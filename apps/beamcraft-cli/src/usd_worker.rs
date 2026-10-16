// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

use beamcraft_host::{storage::BlobStore, wire, worker::fresh_token};
use beamcraft_layer_engine as engine;
use beamcraft_model::{ErrorCode, digest::ContentDigest};
use beamcraft_protocol::{
    TransportCode as T,
    json::{self, Json},
    message::{self, Context, Identity, envelope},
};
use std::{
    collections::BTreeMap,
    sync::mpsc,
    time::{Duration, Instant},
};

pub fn run() -> Result<(), T> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let [flag, path] = args.as_slice() else {
        return Err(T::Message);
    };
    // No plugin/search-path/SDK/runtime flags exist; unknown options fail before any input.
    if flag != "--store" {
        return Err(T::ModelExecution);
    }
    let store = BlobStore::new(path).map_err(|_| T::ModelExecution)?;
    let (tx, rx) = mpsc::sync_channel(0);
    let assembly = wire::Assembly::default();
    let reader_assembly = assembly.clone();
    std::thread::spawn(move || {
        let mut input = std::io::stdin().lock();
        loop {
            let j = wire::read_timed(&mut input, &message::read_limits(), &reader_assembly);
            let end = j.is_err();
            if tx.send(j).is_err() || end {
                break;
            }
        }
    });
    let mut output = std::io::stdout().lock();
    let handshake_deadline = Instant::now() + Duration::from_secs(5);
    let recv = |deadline: Instant| {
        rx.recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .map_err(|_| T::Deadline)?
    };
    let hello = recv(handshake_deadline)?;
    hello.closed(&[
        "type",
        "buffers",
        "session_nonce",
        "offered_versions",
        "model_contract",
        "expected_schema_digest",
        "required_features",
        "host_limits",
    ])?;
    let nonce = hello.get("session_nonce")?.string()?.to_string();
    message::digest(hello.get("session_nonce")?)?;
    message::common(&hello, "hello", &nonce)?;
    let features = Json::Array(message::FEATURES.into_iter().map(Json::text).collect());
    if *hello.get("offered_versions")? != Json::Array(vec![Json::text("0.1.0")])
        || hello.get("model_contract")?.string()? != "0.1.0"
        || message::digest(hello.get("expected_schema_digest")?)? != message::schema_digest()
        || *hello.get("required_features")? != features
        || message::parse_limits(hello.get("host_limits")?)? != message::read_limits()
    {
        return Err(T::CapabilityMismatch);
    }
    wire::write(
        &mut output,
        &envelope(
            "capabilities",
            &nonce,
            [
                ("protocol_version", Json::text("0.1.0")),
                ("model_contract", Json::text("0.1.0")),
                ("backend_name", Json::text("beamcraft-rust-openusd")),
                ("backend_version", Json::text(engine::BACKEND.version)),
                ("sdk_version", Json::text(engine::BACKEND.archive)),
                ("python_version", Json::Null),
                (
                    "schema_package_digest",
                    Json::text(message::schema_digest().to_string()),
                ),
                ("supported_features", features),
                (
                    "worker_limits",
                    message::limits_json(message::read_limits()),
                ),
            ],
        ),
        &message::read_limits(),
    )?;
    let ready = recv(handshake_deadline)?;
    let fields = [
        ("protocol_version", Json::text("0.1.0")),
        ("model_contract", Json::text("0.1.0")),
        (
            "schema_package_digest",
            Json::text(message::schema_digest().to_string()),
        ),
        (
            "effective_limits",
            message::limits_json(message::read_limits()),
        ),
    ];
    if ready != envelope("ready", &nonce, fields.clone()) {
        return Err(T::CapabilityMismatch);
    }
    wire::write(
        &mut output,
        &envelope("ready_ack", &nonce, fields),
        &message::read_limits(),
    )?;
    let (jobs_tx, jobs_rx) = mpsc::sync_channel::<Job>(0);
    let (done_tx, done_rx) = mpsc::sync_channel(0);
    // This is the sole stage owner; the IPC loop can process ping/cancel while it composes.
    std::thread::spawn(move || {
        let mut state = State {
            reader: engine::Reader::default(),
            context: None,
            engine_context: None,
            cursors: BTreeMap::new(),
            pages: 0,
        };
        while let Ok(job) = jobs_rx.recv() {
            let result = state.execute(&store, &job);
            if done_tx.send((job.id, result)).is_err() {
                break;
            }
        }
    });
    let mut high = 0u64;
    let mut last = 0u64;
    let mut pending: Option<(u64, engine::Cancellation, bool)> = None;
    let mut ping = 0u64;
    let mut last_input = Instant::now();
    loop {
        if assembly.expired(Instant::now())? || last_input.elapsed() > Duration::from_secs(20) {
            return Err(T::Deadline);
        }
        if let Ok((id, result)) = done_rx.try_recv() {
            if pending.as_ref().map(|p| p.0) != Some(id) {
                return Err(T::UnknownId);
            }
            pending = None;
            last = id;
            let response = match result {
                Ok((outcome, context, body)) => response(&nonce, id, outcome, None, context, body),
                Err(code) => response(
                    &nonce,
                    id,
                    if code == ErrorCode::Cancelled {
                        "cancelled"
                    } else {
                        "rejected"
                    },
                    Some(code),
                    None,
                    Json::object([("diagnostics", Json::Array(vec![]))]),
                ),
            };
            wire::write(&mut output, &response, &message::read_limits())?;
        }
        let j = match rx.recv_timeout(Duration::from_millis(5)) {
            Ok(j) => j?,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(_) => return Err(T::BackendLost),
        };
        last_input = Instant::now();
        match j.get("type")?.string()? {
            "request" => {
                j.closed(&[
                    "type",
                    "buffers",
                    "session_nonce",
                    "request_id",
                    "method",
                    "timeout_ms",
                    "context",
                    "body",
                ])?;
                message::common(&j, "request", &nonce)?;
                let id = j.get("request_id")?.unsigned()?;
                if id == 0 || id > 65535 || id <= high {
                    return Err(T::DuplicateId);
                }
                if pending.is_some() {
                    return Err(T::Busy);
                }
                high = id;
                let timeout = j.get("timeout_ms")?.unsigned()?;
                if !(1..=120000).contains(&timeout) {
                    return Err(T::Message);
                }
                message::read_request(
                    j.get("method")?.string()?,
                    j.get("context")?,
                    j.get("body")?,
                )?;
                let cancel = engine::Cancellation::default();
                pending = Some((id, cancel.clone(), false));
                jobs_tx
                    .send(Job {
                        id,
                        request: j,
                        control: engine::Control {
                            deadline: Instant::now() + Duration::from_millis(timeout),
                            cancellation: cancel,
                        },
                    })
                    .map_err(|_| T::BackendLost)?;
            }
            "cancel" => {
                j.closed(&["type", "buffers", "session_nonce", "target_request_id"])?;
                message::common(&j, "cancel", &nonce)?;
                let target = j.get("target_request_id")?.unsigned()?;
                if let Some((id, cancel, seen)) = &mut pending {
                    if target == *id {
                        if *seen {
                            return Err(T::DuplicateId);
                        }
                        *seen = true;
                        cancel.cancel();
                    } else if target != last || last == 0 {
                        return Err(T::UnknownId);
                    }
                } else if target != last || last == 0 {
                    return Err(T::UnknownId);
                }
            }
            "ping" => {
                j.closed(&["type", "buffers", "session_nonce", "ping_id"])?;
                message::common(&j, "ping", &nonce)?;
                let id = j.get("ping_id")?.unsigned()?;
                if id <= ping || id > 65535 {
                    return Err(T::UnknownId);
                }
                ping = id;
                wire::write(
                    &mut output,
                    &envelope("pong", &nonce, [("ping_id", Json::uint(id))]),
                    &message::read_limits(),
                )?;
            }
            _ => return Err(T::Message),
        }
    }
}
fn response(
    nonce: &str,
    id: u64,
    outcome: &str,
    code: Option<ErrorCode>,
    context: Option<Context>,
    body: Json,
) -> Json {
    envelope(
        "response",
        nonce,
        [
            ("request_id", Json::uint(id)),
            ("outcome", Json::text(outcome)),
            ("code", code.map_or(Json::Null, |c| Json::text(c.as_str()))),
            (
                "message",
                Json::text(if code.is_none() {
                    "read operation completed"
                } else {
                    "read operation rejected"
                }),
            ),
            ("observed_context", context.map_or(Json::Null, |c| c.json())),
            ("body", body),
        ],
    )
}
struct Job {
    id: u64,
    request: Json,
    control: engine::Control,
}
struct State {
    reader: engine::Reader,
    context: Option<Context>,
    engine_context: Option<engine::Context>,
    cursors: BTreeMap<String, engine::Cursor>,
    pages: usize,
}
type OperationResult = Result<(&'static str, Option<Context>, Json), ErrorCode>;
fn mapped<T>(r: Result<T, beamcraft_protocol::TransportCode>) -> Result<T, ErrorCode> {
    r.map_err(beamcraft_protocol::TransportCode::f04)
}
impl State {
    fn execute(&mut self, store: &BlobStore, job: &Job) -> OperationResult {
        self.execute_inner(store, job).map_err(|e| e.code)
    }
    fn execute_inner(
        &mut self,
        store: &BlobStore,
        job: &Job,
    ) -> Result<(&'static str, Option<Context>, Json), engine::Diagnostic> {
        let error = |code| engine::Diagnostic {
            code,
            message: "read profile request rejected",
        };
        let mut work = || -> OperationResult {
            let method = mapped(job.request.get("method").and_then(Json::string))?;
            let context = mapped(job.request.get("context"))?;
            let body = mapped(job.request.get("body"))?;
            if method == "OpenStage" {
                if *context != Json::Null {
                    return Err(ErrorCode::InvalidArgument);
                }
                mapped(body.closed(&[
                    "project_id",
                    "workspace_id",
                    "base_revision",
                    "workspace_generation",
                    "manifest_digest",
                ]))?;
                let identity = mapped(Identity::parse(body))?;
                let grant = mapped(message::digest(mapped(body.get("manifest_digest"))?))?;
                let request = load(store, grant, &identity)?;
                let opened = self
                    .reader
                    .open(request, &job.control)
                    .map_err(|e| e.code)?;
                let context = Context {
                    identity,
                    stage_token: fresh_token(),
                };
                self.context = Some(context.clone());
                self.engine_context = Some(opened.context);
                self.cursors.clear();
                self.pages = 0;
                let body = Json::object([
                    ("backend_archive", Json::text(engine::BACKEND.archive)),
                    ("profile", Json::text(engine::BACKEND.schema)),
                    ("parser_contained", Json::Bool(false)),
                    ("mutations", Json::Bool(false)),
                    ("save", Json::Bool(false)),
                    ("publication", Json::Bool(false)),
                    ("durable_outcomes", Json::Bool(false)),
                    ("relationships", Json::Bool(false)),
                    (
                        "meters_per_unit",
                        mapped(Json::number(opened.meters_per_unit))?,
                    ),
                    ("up_axis", Json::text(opened.up_axis)),
                    ("prim_count", Json::uint(opened.prim_count as u64)),
                    ("metadata", dictionary(opened.metadata)?),
                ]);
                return Ok(("stage_opened", Some(context), body));
            }
            if method == "QueryOutcome" {
                mapped(body.closed(&["project_id", "workspace_id", "op_id"]))?;
                for key in ["project_id", "workspace_id", "op_id"] {
                    if !message::uuid(mapped(mapped(body.get(key))?.string())?) {
                        return Err(ErrorCode::InvalidId);
                    }
                }
                return Ok((
                    "stage_status",
                    None,
                    Json::object([("status", Json::text("not_observed"))]),
                ));
            }
            if !matches!(method, "ReadSnapshot" | "CloseStage") {
                return Err(ErrorCode::UnsupportedCommand);
            }
            let requested = mapped(Context::parse(context))?;
            if self.context.as_ref() != Some(&requested) {
                return Err(ErrorCode::StaleWorkspace);
            }
            let ec = self
                .engine_context
                .as_ref()
                .ok_or(ErrorCode::StaleWorkspace)?;
            if method == "CloseStage" {
                mapped(body.closed(&[]))?;
                self.reader.close(ec, &job.control).map_err(|e| e.code)?;
                self.context = None;
                self.engine_context = None;
                self.cursors.clear();
                self.pages = 0;
                return Ok(("stage_closed", None, Json::object([])));
            }
            mapped(body.closed(&["cursor"]))?;
            let cursor = mapped(body.get("cursor"))?;
            let cursor = if *cursor == Json::Null {
                None
            } else {
                Some(mapped(cursor.string())?)
            };
            if cursor.is_some_and(|s| s.len() > 256) {
                return Err(ErrorCode::LimitExceeded);
            }
            let cursor = match cursor {
                None => {
                    self.pages = 0;
                    None
                }
                Some(c) => Some(self.cursors.get(c).ok_or(ErrorCode::StaleWorkspace)?),
            };
            if self.pages >= 1024 {
                return Err(ErrorCode::LimitExceeded);
            }
            let page = self
                .reader
                .read(ec, cursor, &job.control)
                .map_err(|e| e.code)?;
            // A single continuation is retained; replaying old pages cannot grow a cursor map.
            self.cursors.clear();
            self.pages += 1;
            let next = if let Some(cursor) = page.next {
                let token = fresh_token().to_string();
                self.cursors.insert(token.clone(), cursor);
                Json::text(token)
            } else {
                Json::Null
            };
            let items = page
                .items
                .into_iter()
                .map(prim)
                .collect::<Result<Vec<_>, _>>()?;
            Ok((
                "snapshot_page",
                self.context.clone(),
                Json::object([("items", Json::Array(items)), ("next", next)]),
            ))
        };
        work().map_err(error)
    }
}
fn load(
    store: &BlobStore,
    grant_digest: ContentDigest,
    identity: &Identity,
) -> Result<engine::OpenRequest, ErrorCode> {
    let decode = |d: &ContentDigest| -> Result<Json, ErrorCode> {
        let b = store.read_capped(d, 1_048_576).map_err(|e| e.code())?;
        mapped(json::decode(&b, &message::read_limits()))
    };
    let grant = decode(&grant_digest)?;
    mapped(grant.closed(&["manifest", "owned_layers"]))?;
    let manifest = mapped(message::digest(mapped(grant.get("manifest"))?))?;
    let owned = mapped(mapped(grant.get("owned_layers"))?.array())?;
    if owned.len() > 8 {
        return Err(ErrorCode::LimitExceeded);
    }
    let owned = owned
        .iter()
        .map(|n| mapped(n.string()).map(str::to_string))
        .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
    // Reuse host validation; possession of the private inherited pipe is the grant channel.
    beamcraft_host::dispatch::AuthorizedRead::authorize(
        store,
        manifest,
        identity.clone(),
        owned.clone(),
    )
    .map_err(|e| e.code)?;
    let doc = decode(&manifest)?;
    let mut assets = Vec::new();
    for entry in mapped(mapped(doc.get("assets"))?.array())? {
        let digest = mapped(message::digest(mapped(entry.get("digest"))?))?;
        let bytes = store.read_capped(&digest, 65536).map_err(|e| e.code())?;
        // No plugin system exists. Execution/discovery directives are refused before parsing.
        if requests_execution(&bytes) {
            return Err(ErrorCode::PermissionDenied);
        }
        assets.push(engine::Asset {
            name: mapped(mapped(entry.get("name"))?.string())?.into(),
            digest,
            bytes,
        });
    }
    let mut request = engine::OpenRequest {
        identity: engine::Identity {
            project: identity.project_id.clone(),
            workspace: identity.workspace_id.clone(),
            revision: identity.base_revision,
            generation: identity.workspace_generation,
        },
        root: mapped(mapped(doc.get("root"))?.string())?.into(),
        assets,
        owned_layers: owned,
        manifest: ContentDigest::from_bytes([0; 32]),
        backend: engine::BACKEND,
        profile: engine::Profile::StaticLocal,
        budgets: engine::Budgets::default(),
    };
    request.manifest = request.manifest_digest().map_err(|e| e.code)?;
    Ok(request)
}
fn requests_execution(bytes: &[u8]) -> bool {
    // Reserve two explicit USDA directive names, without treating a comment or inert string
    // mentioning those names as executable. No plugin/script loader exists, including USDC.
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    let mut quoted = false;
    let mut escaped = false;
    let mut comment = false;
    let mut token = String::new();
    for c in text.chars().chain(std::iter::once(' ')) {
        if comment {
            if c == '\n' {
                comment = false;
            }
            continue;
        }
        if quoted {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                quoted = false;
            }
            continue;
        }
        if c == '#' {
            comment = true;
        }
        if c == '"' {
            quoted = true;
        }
        if c.is_ascii_alphanumeric() || matches!(c, ':' | '_') {
            token.push(c);
        } else {
            if matches!(token.as_str(), "beamcraft:plugin" | "beamcraft:script") {
                return true;
            }
            token.clear();
        }
    }
    false
}
fn value(v: engine::Value) -> Result<Json, ErrorCode> {
    let (kind, value) = match v {
        engine::Value::Block => ("block", Json::Null),
        engine::Value::Bool(b) => ("bool", Json::Bool(b)),
        engine::Value::Int(i) => ("int", Json::text(i.to_string())),
        engine::Value::Uint(u) => ("uint", Json::text(u.to_string())),
        engine::Value::Number(n) => ("number", mapped(Json::number(n))?),
        engine::Value::Text(s) => ("text", Json::text(s)),
        engine::Value::Array(a) => (
            "array",
            Json::Array(a.into_iter().map(value).collect::<Result<_, _>>()?),
        ),
        engine::Value::Dictionary(d) => ("dictionary", dictionary(d)?),
    };
    Ok(Json::object([("kind", Json::text(kind)), ("value", value)]))
}
fn dictionary(d: BTreeMap<String, engine::Value>) -> Result<Json, ErrorCode> {
    Ok(Json::Object(
        d.into_iter()
            .map(|(k, v)| value(v).map(|v| (k, v)))
            .collect::<Result<_, _>>()?,
    ))
}
fn prim(p: engine::Prim) -> Result<Json, ErrorCode> {
    Ok(Json::object([
        ("path", Json::text(p.path)),
        ("element_id", p.element_id.map_or(Json::Null, Json::text)),
        ("type_name", p.type_name.map_or(Json::Null, Json::text)),
        ("active", Json::Bool(p.active)),
        (
            "properties",
            Json::Array(
                p.properties
                    .into_iter()
                    .map(|p| {
                        Ok(Json::object([
                            ("name", Json::text(p.name)),
                            ("type_name", Json::text(p.type_name)),
                            (
                                "value",
                                p.value.map(value).transpose()?.unwrap_or(Json::Null),
                            ),
                        ]))
                    })
                    .collect::<Result<_, ErrorCode>>()?,
            ),
        ),
        (
            "sources",
            Json::Array(
                p.sources
                    .into_iter()
                    .map(|s| {
                        Json::object([
                            ("layer", Json::text(s.layer)),
                            ("digest", Json::text(s.digest.to_string())),
                            ("path", Json::text(s.path)),
                            ("owned", Json::Bool(s.owned)),
                        ])
                    })
                    .collect(),
            ),
        ),
    ]))
}
