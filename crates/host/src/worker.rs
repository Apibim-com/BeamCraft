// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! One serialized private-pipe worker session, with bounded supervision and no network.

use crate::wire;
use beamcraft_model::{
    ErrorCode,
    digest::{ContentDigest, Sha256},
};
use beamcraft_protocol::{
    Limits, TransportCode as T,
    json::Json,
    message::{self, envelope},
};
use std::{
    collections::hash_map::RandomState,
    fs::File,
    hash::{BuildHasher, Hasher},
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU8, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

/// A bounded, fixed safe host error; raw child diagnostics never escape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostError {
    /// Logical error vocabulary.
    pub code: ErrorCode,
    /// Transport cause, if present.
    pub transport: Option<T>,
}
impl HostError {
    /// Creates a logical failure.
    pub const fn logical(code: ErrorCode) -> Self {
        Self {
            code,
            transport: None,
        }
    }
    /// Fixed safe text; does not include file paths or submitted values.
    pub const fn message(self) -> &'static str {
        "operation unavailable or rejected by validated read boundary"
    }
    /// Stable CLI exit mapping.
    pub const fn exit_code(self) -> u8 {
        match self.code {
            ErrorCode::Cancelled => 130,
            ErrorCode::BackendUnavailable => {
                if matches!(self.transport, Some(T::Deadline)) {
                    124
                } else {
                    3
                }
            }
            ErrorCode::OutcomeUnknown => 4,
            _ => 2,
        }
    }
}
impl From<T> for HostError {
    fn from(t: T) -> Self {
        Self {
            code: t.f04(),
            transport: Some(t),
        }
    }
}
impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message())
    }
}
impl std::error::Error for HostError {}

/// Explicit cooperative cancellation of a host operation.
#[derive(Clone, Default)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    /// Requests cancellation. No acceptance/rollback is inferred from this flag.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    /// Whether the caller has requested cancellation.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}
/// Incarnation-specific host supervision handle. It cannot terminate a replacement child.
#[derive(Clone, Default)]
pub struct Termination(Arc<AtomicBool>);
impl Termination {
    /// Requests forced termination at the next host supervision poll. This is backend loss,
    /// not a cancellation acknowledgement or a statement about durable application outcomes.
    pub fn request(&self) {
        self.0.store(true, Ordering::Release);
    }
}
/// One host deadline across launch, transmission, recovery and validation.
pub struct Control {
    /// Monotonic absolute deadline.
    pub deadline: Instant,
    /// Cancellation token.
    pub cancellation: Cancellation,
}
impl Control {
    /// Applies F05's 30 second default and 1..120000 ms bounds.
    pub fn new(timeout_ms: Option<u64>, cancellation: Cancellation) -> Result<Self, HostError> {
        let timeout = timeout_ms.unwrap_or(30_000);
        if !(1..=120_000).contains(&timeout) {
            return Err(HostError::logical(ErrorCode::InvalidArgument));
        }
        Ok(Self {
            deadline: Instant::now() + Duration::from_millis(timeout),
            cancellation,
        })
    }
    /// Remaining whole milliseconds; never resets an enclosing deadline.
    pub fn remaining(&self) -> Result<u64, HostError> {
        if self.cancellation.is_cancelled() {
            return Err(HostError::logical(ErrorCode::Cancelled));
        }
        let ms = self
            .deadline
            .saturating_duration_since(Instant::now())
            .as_millis();
        if ms == 0 {
            return Err(T::Deadline.into());
        }
        u64::try_from(ms.min(120_000)).map_err(|_| T::Deadline.into())
    }
}
/// Packaged executable selected by trusted host configuration, never by a USD model.
#[derive(Clone)]
pub struct WorkerConfig {
    /// Absolute executable path.
    pub executable: PathBuf,
    /// Verified package's complete executable digest.
    pub executable_digest: ContentDigest,
    /// Existing protected U05 root, also the controlled working directory.
    pub store_root: PathBuf,
}
/// Verifies a capped regular executable before each launch. The package namespace must be
/// protected from concurrent replacement, as with the U05 root.
pub fn executable_digest(path: &Path) -> Result<ContentDigest, HostError> {
    hash_executable(path, || Ok(()))
}
fn hash_executable(
    path: &Path,
    check: impl Fn() -> Result<(), HostError>,
) -> Result<ContentDigest, HostError> {
    let meta = std::fs::symlink_metadata(path).map_err(|_| HostError::from(T::BackendLost))?;
    if !path.is_absolute()
        || !meta.is_file()
        || meta.file_type().is_symlink()
        || meta.len() > 128 * 1024 * 1024
    {
        return Err(HostError::logical(ErrorCode::PermissionDenied));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return Err(HostError::logical(ErrorCode::PermissionDenied));
        }
    }
    let mut file = File::open(path).map_err(|_| HostError::from(T::BackendLost))?;
    let mut hash = Sha256::new();
    let mut bytes = [0u8; 65536];
    let mut total = 0usize;
    loop {
        check()?;
        let n = file
            .read(&mut bytes)
            .map_err(|_| HostError::from(T::BackendLost))?;
        if n == 0 {
            break;
        }
        total = total.checked_add(n).ok_or(T::Limit)?;
        if total > 128 * 1024 * 1024 {
            return Err(T::Limit.into());
        }
        hash.update(bytes.get(..n).ok_or(T::Limit)?)
            .map_err(|_| HostError::from(T::Limit))?;
    }
    Ok(hash.finalize())
}
/// Fresh non-authorizing token from independently OS-seeded standard hash states.
/// This is not a cryptographic entropy API; exact F05 random-byte admission remains open.
pub fn fresh_token() -> ContentDigest {
    let mut token = [0u8; 32];
    for slot in token.as_chunks_mut::<8>().0 {
        let mut h = RandomState::new().build_hasher();
        h.write(b"BeamCraft/U04/incarnation");
        slot.copy_from_slice(&h.finish().to_be_bytes());
    }
    ContentDigest::from_bytes(token)
}

/// One live child session. Exclusive mutable calls enforce a single request owner.
pub struct Session {
    child: Child,
    outgoing: Option<SyncSender<Json>>,
    incoming: Option<Receiver<Result<Json, T>>>,
    threads: Vec<JoinHandle<()>>,
    failed: Arc<AtomicU8>,
    starts: Receiver<Instant>,
    assembly: Option<Instant>,
    termination: Termination,
    nonce: String,
    limits: Limits,
    next_id: u32,
    stopped: bool,
    last_ping: Instant,
    ping_id: u32,
}
impl Session {
    /// Verifies the package, spawns with anonymous pipes and a cleared environment, then
    /// performs all four handshake messages before stage access.
    pub fn spawn(config: &WorkerConfig, control: &Control) -> Result<Self, HostError> {
        control.remaining()?;
        if hash_executable(&config.executable, || control.remaining().map(|_| ()))?
            != config.executable_digest
        {
            return Err(HostError::logical(ErrorCode::IntegrityError));
        }
        crate::storage::BlobStore::new(&config.store_root)
            .map_err(|e| HostError::logical(e.code()))?;
        control.remaining()?;
        let mut command = Command::new(&config.executable);
        command
            .arg("--store")
            .arg(&config.store_root)
            .current_dir(&config.store_root)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW, no elevation.
            if let Some(root) = std::env::var_os("SystemRoot") {
                command.env("SystemRoot", root);
            }
        }
        let mut child = command
            .spawn()
            .map_err(|_| HostError::from(T::BackendLost))?;
        let pipes = (child.stdin.take(), child.stdout.take(), child.stderr.take());
        let (Some(mut input), Some(mut output), Some(mut diagnostics)) = pipes else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(T::BackendLost.into());
        };
        let (tx, writer_rx) = mpsc::sync_channel::<Json>(1);
        let (reader_tx, rx) = mpsc::sync_channel(0);
        let failed = Arc::new(AtomicU8::new(0));
        let (start_tx, starts) = mpsc::sync_channel(1);
        let wf = failed.clone();
        let writer = thread::spawn(move || {
            while let Ok(j) = writer_rx.recv() {
                if wire::write(&mut input, &j, &message::read_limits()).is_err() {
                    wf.store(1, Ordering::Release);
                    break;
                }
            }
        });
        let reader = thread::spawn(move || {
            loop {
                let j = wire::read_tracked(&mut output, &message::read_limits(), || {
                    let _ = start_tx.try_send(Instant::now());
                });
                let end = j.is_err();
                if reader_tx.send(j).is_err() || end {
                    break;
                }
            }
        });
        let df = failed.clone();
        let diagnostic = thread::spawn(move || {
            let mut accepted = 0usize;
            let mut bytes = [0; 1024];
            // Retain zero bytes: even panic-hook paths and values cannot reach a client.
            loop {
                match diagnostics.read(&mut bytes) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        accepted = accepted.saturating_add(n);
                        if accepted > 65536 {
                            df.store(2, Ordering::Release);
                            break;
                        }
                    }
                }
            }
        });
        let mut s = Self {
            child,
            outgoing: Some(tx),
            incoming: Some(rx),
            threads: vec![writer, reader, diagnostic],
            failed,
            starts,
            assembly: None,
            termination: Termination::default(),
            nonce: fresh_token().to_string(),
            limits: message::read_limits(),
            next_id: 1,
            stopped: false,
            last_ping: Instant::now(),
            ping_id: 0,
        };
        let deadline = control
            .deadline
            .min(Instant::now() + Duration::from_secs(5));
        let handshake = Control {
            deadline,
            cancellation: control.cancellation.clone(),
        };
        if let Err(error) = s.handshake(&handshake) {
            let _ = s.stop();
            return Err(error);
        }
        Ok(s)
    }
    fn send(&self, j: Json) -> Result<(), HostError> {
        self.outgoing
            .as_ref()
            .ok_or(T::BackendLost)?
            .try_send(j)
            .map_err(|_| T::BackendLost.into())
    }
    fn receive(&mut self, control: &Control) -> Result<Json, HostError> {
        loop {
            if self.termination.0.load(Ordering::Acquire) {
                self.child
                    .kill()
                    .map_err(|_| HostError::from(T::BackendLost))?;
                return Err(T::BackendLost.into());
            }
            control.remaining()?;
            match self.failed.load(Ordering::Acquire) {
                0 => (),
                1 => return Err(T::BackendLost.into()),
                _ => return Err(T::Limit.into()),
            }
            if let Ok(start) = self.starts.try_recv() {
                self.assembly = Some(start);
            }
            if self
                .assembly
                .is_some_and(|s| s.elapsed() >= Duration::from_secs(10))
            {
                return Err(T::Deadline.into());
            }
            let wait = Duration::from_millis(control.remaining()?.min(10));
            match self
                .incoming
                .as_ref()
                .ok_or(T::BackendLost)?
                .recv_timeout(wait)
            {
                Ok(j) => {
                    self.assembly = None;
                    return j.map_err(HostError::from);
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => return Err(T::BackendLost.into()),
                Err(mpsc::RecvTimeoutError::Timeout) => (),
            }
        }
    }
    fn handshake(&mut self, c: &Control) -> Result<(), HostError> {
        let features = Json::Array(message::FEATURES.into_iter().map(Json::text).collect());
        self.send(envelope(
            "hello",
            &self.nonce,
            [
                ("offered_versions", Json::Array(vec![Json::text("0.1.0")])),
                ("model_contract", Json::text("0.1.0")),
                (
                    "expected_schema_digest",
                    Json::text(message::schema_digest().to_string()),
                ),
                ("required_features", features.clone()),
                ("host_limits", message::limits_json(self.limits)),
            ],
        ))?;
        let cap = self.receive(c)?;
        cap.closed(&[
            "type",
            "buffers",
            "session_nonce",
            "protocol_version",
            "model_contract",
            "backend_name",
            "backend_version",
            "sdk_version",
            "python_version",
            "schema_package_digest",
            "supported_features",
            "worker_limits",
        ])?;
        message::common(&cap, "capabilities", &self.nonce)?;
        for (key, expected) in [
            ("protocol_version", "0.1.0"),
            ("model_contract", "0.1.0"),
            ("backend_name", "beamcraft-rust-openusd"),
            ("backend_version", "0.8.0"),
            ("sdk_version", message::ARCHIVE),
        ] {
            if cap.get(key)?.string()? != expected {
                return Err(T::CapabilityMismatch.into());
            }
        }
        if *cap.get("python_version")? != Json::Null
            || *cap.get("supported_features")? != features
            || message::digest(cap.get("schema_package_digest")?)? != message::schema_digest()
        {
            return Err(T::CapabilityMismatch.into());
        }
        self.limits = Limits::effective(
            &self.limits,
            &message::parse_limits(cap.get("worker_limits")?)?,
        )?;
        // This initial profile admits only the exact tested limit set.
        if self.limits != message::read_limits() {
            return Err(T::CapabilityMismatch.into());
        }
        let fields = [
            ("protocol_version", Json::text("0.1.0")),
            ("model_contract", Json::text("0.1.0")),
            (
                "schema_package_digest",
                Json::text(message::schema_digest().to_string()),
            ),
            ("effective_limits", message::limits_json(self.limits)),
        ];
        let ready = envelope("ready", &self.nonce, fields);
        self.send(ready.clone())?;
        let ack = self.receive(c)?;
        let mut expected = ready;
        if let Json::Object(o) = &mut expected {
            o.insert("type".into(), Json::text("ready_ack"));
        }
        if ack != expected {
            return Err(T::CapabilityMismatch.into());
        }
        Ok(())
    }
    /// Returns the real process ID for diagnostics/process containment owned by the host.
    pub fn process_id(&self) -> u32 {
        self.child.id()
    }
    /// Obtains a host-owned supervision handle for this child only.
    pub fn termination_handle(&self) -> Termination {
        self.termination.clone()
    }
    /// Sends one bounded request. Errors close the session; a worker acknowledgement can
    /// never become recorded acceptance. Only read-profile methods are dispatchable.
    pub fn call(
        &mut self,
        method: &str,
        context: Json,
        body: Json,
        c: &Control,
    ) -> Result<Json, HostError> {
        let result = self.call_inner(method, context, body, c);
        if result.is_err() {
            self.stop()?;
        }
        result
    }
    fn call_inner(
        &mut self,
        method: &str,
        context: Json,
        body: Json,
        c: &Control,
    ) -> Result<Json, HostError> {
        if !matches!(
            method,
            "OpenStage" | "ReadSnapshot" | "CloseStage" | "QueryOutcome"
        ) {
            return Err(HostError::logical(ErrorCode::UnsupportedCommand));
        }
        message::read_request(method, &context, &body)?;
        if self.next_id > 65535 {
            return Err(T::Limit.into());
        }
        if let Some(rx) = &self.incoming {
            match rx.try_recv() {
                Ok(_) => return Err(T::UnknownId.into()),
                Err(mpsc::TryRecvError::Disconnected) => return Err(T::BackendLost.into()),
                Err(mpsc::TryRecvError::Empty) => (),
            }
        }
        let id = self.next_id;
        self.next_id += 1;
        self.send(envelope(
            "request",
            &self.nonce,
            [
                ("request_id", Json::uint(u64::from(id))),
                ("method", Json::text(method)),
                ("timeout_ms", Json::uint(c.remaining()?)),
                ("context", context),
                ("body", body),
            ],
        ))?;
        let mut pending_ping = None;
        loop {
            let response = match self.receive(&Control {
                deadline: c.deadline.min(Instant::now() + Duration::from_millis(25)),
                cancellation: c.cancellation.clone(),
            }) {
                Ok(j) => j,
                Err(e) if e.transport == Some(T::Deadline) && Instant::now() < c.deadline => {
                    if self.last_ping.elapsed() >= Duration::from_secs(5) && pending_ping.is_none()
                    {
                        self.ping_id += 1;
                        if self.ping_id > 65535 {
                            return Err(T::Limit.into());
                        }
                        self.send(envelope(
                            "ping",
                            &self.nonce,
                            [("ping_id", Json::uint(u64::from(self.ping_id)))],
                        ))?;
                        self.last_ping = Instant::now();
                        pending_ping = Some(self.last_ping);
                    }
                    if pending_ping.is_some_and(|p: Instant| p.elapsed() > Duration::from_secs(15))
                    {
                        return Err(T::Deadline.into());
                    }
                    continue;
                }
                Err(e) => {
                    if e.code == ErrorCode::Cancelled || e.transport == Some(T::Deadline) {
                        let _ = self.send(envelope(
                            "cancel",
                            &self.nonce,
                            [("target_request_id", Json::uint(u64::from(id)))],
                        ));
                        // Read-only: termination discards the projection; no mutation is admitted.
                    }
                    return Err(e);
                }
            };
            match response.get("type")?.string()? {
                "pong" => {
                    response.closed(&["type", "buffers", "session_nonce", "ping_id"])?;
                    message::common(&response, "pong", &self.nonce)?;
                    if pending_ping.take().is_none()
                        || response.get("ping_id")?.unsigned()? != u64::from(self.ping_id)
                    {
                        return Err(T::UnknownId.into());
                    }
                }
                "response" => {
                    response.closed(&[
                        "type",
                        "buffers",
                        "session_nonce",
                        "request_id",
                        "outcome",
                        "code",
                        "message",
                        "observed_context",
                        "body",
                    ])?;
                    message::common(&response, "response", &self.nonce)?;
                    if response.get("request_id")?.unsigned()? != u64::from(id) {
                        return Err(T::UnknownId.into());
                    }
                    // Safe vocabulary only; raw message strings are never forwarded.
                    if response.get("message")?.string()? != "read operation completed"
                        && response.get("message")?.string()? != "read operation rejected"
                    {
                        return Err(T::Message.into());
                    }
                    c.remaining()?;
                    return Ok(response);
                }
                _ => return Err(T::Message.into()),
            }
        }
    }
    /// Terminates and confirms reaping before a replacement may launch. Joins all pipe tasks
    /// after closing channels; never waits indefinitely for an untrusted child.
    pub fn stop(&mut self) -> Result<(), HostError> {
        if self.stopped {
            return Ok(());
        }
        self.outgoing.take();
        self.incoming.take();
        let _ = self.child.kill();
        let until = Instant::now() + Duration::from_secs(2);
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => (),
                Err(_) => return Err(T::BackendLost.into()),
            }
            if Instant::now() >= until {
                return Err(T::BackendLost.into());
            }
            thread::sleep(Duration::from_millis(5));
        }
        for t in self.threads.drain(..) {
            while !t.is_finished() && Instant::now() < until {
                thread::sleep(Duration::from_millis(5));
            }
            if !t.is_finished() {
                return Err(T::BackendLost.into());
            }
            let _ = t.join();
        }
        self.stopped = true;
        Ok(())
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
