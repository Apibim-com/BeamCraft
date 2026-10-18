// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! C02 local coordinator transport: an explicitly started loopback server with token
//! authentication and one serialized request queue.
//!
//! Nothing here starts on its own: a caller binds a [`Coordinator`] with at least one client and
//! calls [`Coordinator::serve`]. The listener binds `127.0.0.1` only, and a connection whose peer
//! is not a loopback address is closed unanswered. Each client is an actor identity and a token
//! of 32 to 256 printable bytes read from a file the user made; the coordinator never generates a
//! secret (no audited entropy API is available, see `ids`). A request authenticates by its token,
//! compared in constant time against every client's; the handler receives the authenticated actor,
//! never the token, and the request cannot name an actor of its own.
//!
//! Wire: one request and one response per connection, each a frame of a 4-byte big-endian length
//! (at most [`MAX_FRAME_BYTES`]) followed by a JSON object decoded with the bounded protocol
//! reader. A request is `{"protocol": "beamcraft-coordinator/1", "token": ..., "op": ..., ...}`.
//! Reads and writes have deadlines; at most [`MAX_CONNECTIONS`] connections are served at once and
//! at most [`QUEUE_CAPACITY`] requests wait; beyond either the answer is `LIMIT_EXCEEDED`.
//!
//! Every authenticated request goes through one queue to one handler thread, in arrival order, so
//! the handler never runs two requests at once. This is the "one publish queue" of C02: the
//! publication rules themselves (expected head, changesets) belong to the handler.

use beamcraft_model::error::ErrorCode;
use beamcraft_model::id::ActorId;
use beamcraft_protocol::json::{self, Json};
use beamcraft_protocol::message;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, Shutdown, SocketAddr, SocketAddrV4, TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

/// Protocol tag every request carries.
pub const PROTOCOL: &str = "beamcraft-coordinator/1";
/// Largest request or response frame body.
pub const MAX_FRAME_BYTES: usize = 65_536;
/// Most configured clients.
pub const MAX_CLIENTS: usize = 16;
/// Most connections served at once.
pub const MAX_CONNECTIONS: usize = 16;
/// Most authenticated requests waiting for the handler.
pub const QUEUE_CAPACITY: usize = 64;
/// Shortest token.
pub const MIN_TOKEN_BYTES: usize = 32;
/// Longest token.
pub const MAX_TOKEN_BYTES: usize = 256;
/// Read and write deadline of one connection.
pub const IO_TIMEOUT: Duration = Duration::from_secs(5);
/// Longest wait for the handler's answer to one request.
pub const HANDLER_TIMEOUT: Duration = Duration::from_secs(120);

/// A refused coordinator operation: an F04 code and a fixed message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoordinatorError {
    /// Stable F04 code.
    pub code: ErrorCode,
    /// Fixed ASCII explanation.
    pub message: &'static str,
}

fn error(code: ErrorCode, message: &'static str) -> CoordinatorError {
    CoordinatorError { code, message }
}

/// A shared secret that authenticates one client. Its bytes are never printed.
#[derive(Clone, PartialEq, Eq)]
pub struct Token(Vec<u8>);

impl core::fmt::Debug for Token {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Token(..)")
    }
}

impl Token {
    /// A token from its text: 32 to 256 bytes of `A-Z a-z 0-9 - _ . + / =`.
    pub fn parse(text: &str) -> Result<Self, CoordinatorError> {
        let ok = |b: u8| b.is_ascii_alphanumeric() || b"-_.+/=".contains(&b);
        if !(MIN_TOKEN_BYTES..=MAX_TOKEN_BYTES).contains(&text.len()) || !text.bytes().all(ok) {
            return Err(error(
                ErrorCode::InvalidArgument,
                "a token is 32 to 256 printable bytes",
            ));
        }
        Ok(Self(text.as_bytes().to_vec()))
    }

    /// Reads a token file: its whole content without one trailing newline. On Unix the file must
    /// not be readable or writable by group or others (`PERMISSION_DENIED`).
    pub fn from_file(path: &Path) -> Result<Self, CoordinatorError> {
        let file = std::fs::File::open(path)
            .map_err(|_| error(ErrorCode::NotFound, "token file cannot be opened"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = file
                .metadata()
                .map_err(|_| error(ErrorCode::NotFound, "token file cannot be read"))?
                .permissions()
                .mode();
            if mode & 0o077 != 0 {
                return Err(error(
                    ErrorCode::PermissionDenied,
                    "token file is accessible by group or others",
                ));
            }
        }
        let mut bytes = Vec::new();
        file.take(MAX_TOKEN_BYTES as u64 + 3)
            .read_to_end(&mut bytes)
            .map_err(|_| error(ErrorCode::NotFound, "token file cannot be read"))?;
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| error(ErrorCode::InvalidArgument, "token file is not UTF-8"))?;
        let text = text
            .strip_suffix("\r\n")
            .or_else(|| text.strip_suffix('\n'))
            .unwrap_or(text);
        Self::parse(text)
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0).into_owned()
    }

    /// Constant-time comparison for equal lengths; a length difference is not secret.
    fn matches(&self, other: &[u8]) -> bool {
        self.0.len() == other.len()
            && self
                .0
                .iter()
                .zip(other)
                .fold(0u8, |acc, (a, b)| acc | (a ^ b))
                == 0
    }
}

/// One configured client: the actor a token authenticates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Client {
    /// The authenticated actor.
    pub actor: ActorId,
    /// Its token.
    pub token: Token,
}

/// One authenticated request as the handler sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// The actor the token authenticated.
    pub actor: ActorId,
    /// The operation name.
    pub op: String,
    /// Every other field of the request (`protocol`, `token` and `op` removed).
    pub fields: BTreeMap<String, Json>,
}

/// Decides one request at a time, in queue order.
pub trait Handler: Send + 'static {
    /// The JSON answer to one request. It runs alone: no other request is handled meanwhile.
    fn handle(&mut self, request: Request) -> Json;
}

/// The JSON shape of a refusal.
pub fn rejection(code: ErrorCode, message: &str) -> Json {
    Json::object([
        ("outcome", Json::text("rejected")),
        ("code", Json::text(code.as_str())),
        ("message", Json::text(message)),
    ])
}

fn write_frame(stream: &mut TcpStream, body: &Json) -> Result<(), CoordinatorError> {
    let bytes = body
        .encode(&message::read_limits())
        .map_err(|_| error(ErrorCode::LimitExceeded, "frame cannot be encoded"))?;
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(error(ErrorCode::LimitExceeded, "frame is too large"));
    }
    let length = u32::try_from(bytes.len())
        .map_err(|_| error(ErrorCode::LimitExceeded, "frame is too large"))?;
    stream
        .write_all(&length.to_be_bytes())
        .and_then(|()| stream.write_all(&bytes))
        .and_then(|()| stream.flush())
        .map_err(|_| error(ErrorCode::BackendUnavailable, "frame cannot be written"))
}

fn read_frame(stream: &mut TcpStream) -> Result<Json, CoordinatorError> {
    let mut length = [0u8; 4];
    stream
        .read_exact(&mut length)
        .map_err(|_| error(ErrorCode::MalformedCommand, "frame cannot be read"))?;
    let length = usize::try_from(u32::from_be_bytes(length))
        .map_err(|_| error(ErrorCode::LimitExceeded, "frame is too large"))?;
    if length > MAX_FRAME_BYTES {
        return Err(error(ErrorCode::LimitExceeded, "frame is too large"));
    }
    let mut body = vec![0u8; length];
    stream
        .read_exact(&mut body)
        .map_err(|_| error(ErrorCode::MalformedCommand, "frame cannot be read"))?;
    json::decode(&body, &message::read_limits()).map_err(|_| {
        error(
            ErrorCode::MalformedCommand,
            "frame is not a bounded JSON object",
        )
    })
}

/// Checks the protocol and the token and splits off the handler's fields.
fn authenticate(clients: &[Client], frame: Json) -> Result<Request, CoordinatorError> {
    let Json::Object(mut map) = frame else {
        return Err(error(
            ErrorCode::MalformedCommand,
            "request is not an object",
        ));
    };
    match map.remove("protocol") {
        Some(Json::String(p)) if p == PROTOCOL => {}
        _ => {
            return Err(error(
                ErrorCode::UnsupportedVersion,
                "unknown coordinator protocol",
            ));
        }
    }
    let denied = || error(ErrorCode::PermissionDenied, "authentication failed");
    let token = match map.remove("token") {
        Some(Json::String(t)) => t,
        _ => return Err(denied()),
    };
    // Every client is compared, so the time does not reveal which one matched.
    let mut actor = None;
    for client in clients {
        if client.token.matches(token.as_bytes()) {
            actor = Some(client.actor);
        }
    }
    let actor = actor.ok_or_else(denied)?;
    let op = match map.remove("op") {
        Some(Json::String(op)) if op.len() <= 64 => op,
        _ => return Err(error(ErrorCode::MalformedCommand, "request has no op")),
    };
    Ok(Request {
        actor,
        op,
        fields: map,
    })
}

struct Job {
    request: Request,
    reply: mpsc::Sender<Json>,
}

/// A bound, not yet serving coordinator.
pub struct Coordinator {
    listener: TcpListener,
    clients: Arc<Vec<Client>>,
    max_requests: Option<u64>,
}

impl Coordinator {
    /// Binds `127.0.0.1:port` (0 picks a free port). At least one and at most [`MAX_CLIENTS`]
    /// clients with distinct tokens and actors are required: a coordinator without a token
    /// cannot start. `max_requests` stops the server after that many handled requests.
    pub fn bind(
        port: u16,
        clients: Vec<Client>,
        max_requests: Option<u64>,
    ) -> Result<Self, CoordinatorError> {
        if clients.is_empty() || clients.len() > MAX_CLIENTS {
            return Err(error(
                ErrorCode::InvalidArgument,
                "a coordinator needs 1 to 16 clients with tokens",
            ));
        }
        for (i, a) in clients.iter().enumerate() {
            if clients
                .iter()
                .skip(i + 1)
                .any(|b| a.token == b.token || a.actor == b.actor)
            {
                return Err(error(
                    ErrorCode::InvalidArgument,
                    "clients need distinct tokens and actors",
                ));
            }
        }
        if max_requests == Some(0) {
            return Err(error(
                ErrorCode::InvalidArgument,
                "max requests must be positive",
            ));
        }
        let listener =
            TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)).map_err(|_| {
                error(
                    ErrorCode::BackendUnavailable,
                    "loopback port cannot be bound",
                )
            })?;
        Ok(Self {
            listener,
            clients: Arc::new(clients),
            max_requests,
        })
    }

    /// The bound loopback address.
    pub fn local_addr(&self) -> Result<SocketAddr, CoordinatorError> {
        self.listener
            .local_addr()
            .map_err(|_| error(ErrorCode::BackendUnavailable, "address is not available"))
    }

    /// Serves until `max_requests` requests were handled (forever without it). Returns the number
    /// of handled requests.
    pub fn serve(self, mut handler: impl Handler) -> Result<u64, CoordinatorError> {
        let address = self.local_addr()?;
        let (queue, jobs) = mpsc::sync_channel::<Job>(QUEUE_CAPACITY);
        let stop = Arc::new(AtomicBool::new(false));
        let max = self.max_requests;
        let worker = {
            let stop = Arc::clone(&stop);
            std::thread::spawn(move || {
                let mut handled = 0u64;
                for job in jobs {
                    let answer = handler.handle(job.request);
                    // The connection may have gone away; the request was still handled.
                    let _ = job.reply.send(answer);
                    handled = handled.saturating_add(1);
                    if max.is_some_and(|m| handled >= m) {
                        stop.store(true, Ordering::SeqCst);
                        // Wake the accept loop so it observes the stop flag.
                        let _ = TcpStream::connect_timeout(&address, IO_TIMEOUT);
                        break;
                    }
                }
                handled
            })
        };
        let active = Arc::new(AtomicUsize::new(0));
        for stream in self.listener.incoming() {
            if stop.load(Ordering::SeqCst) {
                break;
            }
            let Ok(mut stream) = stream else {
                continue;
            };
            if !stream.peer_addr().is_ok_and(|p| p.ip().is_loopback()) {
                let _ = stream.shutdown(Shutdown::Both);
                continue;
            }
            let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
            let _ = stream.set_write_timeout(Some(IO_TIMEOUT));
            if active.fetch_add(1, Ordering::SeqCst) >= MAX_CONNECTIONS {
                active.fetch_sub(1, Ordering::SeqCst);
                let _ = write_frame(
                    &mut stream,
                    &rejection(ErrorCode::LimitExceeded, "too many connections"),
                );
                continue;
            }
            let (clients, queue, active) = (
                Arc::clone(&self.clients),
                queue.clone(),
                Arc::clone(&active),
            );
            std::thread::spawn(move || {
                let answer = connection(&mut stream, &clients, &queue);
                let _ = write_frame(&mut stream, &answer);
                let _ = stream.shutdown(Shutdown::Both);
                active.fetch_sub(1, Ordering::SeqCst);
            });
        }
        drop(queue);
        worker.join().map_err(|_| {
            error(
                ErrorCode::BackendUnavailable,
                "the handler stopped unexpectedly",
            )
        })
    }
}

/// Reads, authenticates and queues one request and waits for its answer.
fn connection(stream: &mut TcpStream, clients: &[Client], queue: &mpsc::SyncSender<Job>) -> Json {
    let request = match read_frame(stream).and_then(|frame| authenticate(clients, frame)) {
        Ok(request) => request,
        Err(e) => return rejection(e.code, e.message),
    };
    let (reply, answer) = mpsc::channel();
    match queue.try_send(Job { request, reply }) {
        Ok(()) => {}
        Err(mpsc::TrySendError::Full(_)) => {
            return rejection(ErrorCode::LimitExceeded, "the publish queue is full");
        }
        Err(mpsc::TrySendError::Disconnected(_)) => {
            return rejection(ErrorCode::BackendUnavailable, "the coordinator is stopping");
        }
    }
    match answer.recv_timeout(HANDLER_TIMEOUT) {
        Ok(answer) => answer,
        // The job was dropped unhandled: the coordinator stopped first.
        Err(mpsc::RecvTimeoutError::Disconnected) => rejection(
            ErrorCode::BackendUnavailable,
            "the coordinator stopped before handling the request",
        ),
        Err(mpsc::RecvTimeoutError::Timeout) => rejection(
            ErrorCode::OutcomeUnknown,
            "no answer from the handler in time; query the outcome",
        ),
    }
}

/// Sends one request to a coordinator and returns its answer. Only a loopback address is
/// contacted (`PERMISSION_DENIED` otherwise, before any I/O).
pub fn call(
    address: SocketAddr,
    token: &Token,
    op: &str,
    fields: impl IntoIterator<Item = (String, Json)>,
) -> Result<Json, CoordinatorError> {
    if !address.ip().is_loopback() {
        return Err(error(
            ErrorCode::PermissionDenied,
            "the coordinator is reachable on loopback only",
        ));
    }
    let mut map: BTreeMap<String, Json> = fields.into_iter().collect();
    map.insert("protocol".into(), Json::text(PROTOCOL));
    map.insert("token".into(), Json::text(token.text()));
    map.insert("op".into(), Json::text(op));
    let mut stream = TcpStream::connect_timeout(&address, IO_TIMEOUT).map_err(|_| {
        error(
            ErrorCode::BackendUnavailable,
            "the coordinator is not reachable",
        )
    })?;
    let _ = stream.set_read_timeout(Some(HANDLER_TIMEOUT));
    let _ = stream.set_write_timeout(Some(IO_TIMEOUT));
    write_frame(&mut stream, &Json::Object(map))?;
    read_frame(&mut stream).map_err(|e| CoordinatorError {
        // A missing answer after a sent request leaves the outcome open.
        code: ErrorCode::OutcomeUnknown,
        message: e.message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn token(c: char) -> Token {
        Token::parse(&c.to_string().repeat(40)).unwrap()
    }
    fn actor(n: u8) -> ActorId {
        ActorId::parse(&format!("019a7bf0-0000-7000-8000-0000000c00{n:02}")).unwrap()
    }

    struct Echo(Arc<Mutex<Vec<String>>>);
    impl Handler for Echo {
        fn handle(&mut self, request: Request) -> Json {
            self.0.lock().unwrap().push(request.op.clone());
            Json::object([
                ("outcome", Json::text("echo")),
                ("actor", Json::text(request.actor.to_string())),
                ("op", Json::text(request.op)),
                ("fields", Json::uint(request.fields.len() as u64)),
            ])
        }
    }

    #[test]
    fn tokens_are_bounded_printable_and_never_printed() {
        assert!(Token::parse(&"a".repeat(31)).is_err());
        assert!(Token::parse(&"a".repeat(257)).is_err());
        assert!(Token::parse(&format!("{} ", "a".repeat(40))).is_err());
        let t = token('k');
        assert_eq!(format!("{t:?}"), "Token(..)");
        assert!(t.matches(&[b'k'; 40]) && !t.matches(&[b'k'; 39]) && !t.matches(&[b'x'; 40]));
    }

    #[test]
    fn a_coordinator_needs_clients_and_binds_loopback_only() {
        assert!(Coordinator::bind(0, Vec::new(), None).is_err());
        let twice = vec![
            Client {
                actor: actor(1),
                token: token('a'),
            },
            Client {
                actor: actor(2),
                token: token('a'),
            },
        ];
        assert!(Coordinator::bind(0, twice, None).is_err());
        let c = Coordinator::bind(
            0,
            vec![Client {
                actor: actor(1),
                token: token('a'),
            }],
            Some(1),
        )
        .unwrap();
        assert!(c.local_addr().unwrap().ip().is_loopback());
        let far = SocketAddr::from(([192, 0, 2, 1], 9));
        assert_eq!(
            call(far, &token('a'), "head", []).unwrap_err().code,
            ErrorCode::PermissionDenied
        );
    }

    #[test]
    fn requests_are_authenticated_and_handled_one_at_a_time_in_order() {
        let clients = vec![
            Client {
                actor: actor(1),
                token: token('a'),
            },
            Client {
                actor: actor(2),
                token: token('b'),
            },
        ];
        let c = Coordinator::bind(0, clients, Some(6)).unwrap();
        let address = c.local_addr().unwrap();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let server = {
            let seen = Arc::clone(&seen);
            std::thread::spawn(move || c.serve(Echo(seen)))
        };
        // A wrong token and a missing protocol never reach the handler.
        let denied = call(address, &token('z'), "head", []).unwrap();
        assert_eq!(
            denied.get("code").unwrap().string().unwrap(),
            "PERMISSION_DENIED"
        );
        // The token decides the actor; a request cannot name one.
        let answer = call(
            address,
            &token('b'),
            "first",
            [("actor".to_owned(), Json::text(actor(1).to_string()))],
        )
        .unwrap();
        assert_eq!(
            answer.get("actor").unwrap().string().unwrap(),
            actor(2).to_string()
        );
        let handles: Vec<_> = (0..5)
            .map(|i| std::thread::spawn(move || call(address, &token('a'), &format!("op{i}"), [])))
            .collect();
        for h in handles {
            assert_eq!(
                h.join()
                    .unwrap()
                    .unwrap()
                    .get("outcome")
                    .unwrap()
                    .string()
                    .unwrap(),
                "echo"
            );
        }
        assert_eq!(server.join().unwrap().unwrap(), 6);
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 6);
        assert_eq!(seen[0], "first");
    }

    #[test]
    fn oversized_and_malformed_frames_are_refused() {
        let c = Coordinator::bind(
            0,
            vec![Client {
                actor: actor(1),
                token: token('a'),
            }],
            Some(1),
        )
        .unwrap();
        let address = c.local_addr().unwrap();
        let server = std::thread::spawn(move || c.serve(Echo(Arc::default())));
        let raw = |bytes: &[u8]| {
            let mut s = TcpStream::connect(address).unwrap();
            s.write_all(bytes).unwrap();
            read_frame(&mut s).unwrap()
        };
        let code = |j: Json| j.get("code").unwrap().string().unwrap().to_owned();
        assert_eq!(code(raw(&[0xff, 0xff, 0xff, 0xff])), "LIMIT_EXCEEDED");
        assert_eq!(
            code(raw(&[0, 0, 0, 3, b'[', b']', b' '])),
            "MALFORMED_COMMAND"
        );
        assert_eq!(code(raw(&[0, 0, 0, 2, b'{', b'}'])), "UNSUPPORTED_VERSION");
        // One authenticated request is handled, then the server stops.
        call(address, &token('a'), "last", []).unwrap();
        assert_eq!(server.join().unwrap().unwrap(), 1);
    }
}
