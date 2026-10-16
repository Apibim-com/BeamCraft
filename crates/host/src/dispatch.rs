// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Shared headless dispatcher. Only implemented read/storage capabilities are exposed.

use crate::{
    storage::BlobStore,
    worker::{Control, HostError, Session, WorkerConfig},
};
use beamcraft_model::{ErrorCode, digest::ContentDigest};
use beamcraft_protocol::{
    TransportCode as T,
    json::{self, Json},
    message::{self, Context, Identity},
};
use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};

/// Host-authorized immutable read input. Construction requires an independently selected
/// project/workspace tuple and an explicit owned-layer grant; model metadata grants nothing.
#[derive(Clone)]
pub struct AuthorizedRead {
    identity: Identity,
    manifest: ContentDigest,
    assets: Vec<(String, ContentDigest)>,
    owned: BTreeSet<String>,
}
impl AuthorizedRead {
    /// Validates a bounded read manifest from U05 against the caller's trusted authorization.
    /// The protected store and actor authorization are prerequisites of this local host API.
    pub fn authorize(
        store: &BlobStore,
        manifest: ContentDigest,
        expected: Identity,
        owned: BTreeSet<String>,
    ) -> Result<Self, HostError> {
        expected.validate()?;
        let bytes = store
            .read_capped(&manifest, 1_048_576)
            .map_err(|e| HostError::logical(e.code()))?;
        let doc = json::decode(&bytes, &message::read_limits())?;
        doc.closed(&["identity", "root", "assets", "backend_archive", "profile"])?;
        doc.get("identity")?.closed(&[
            "project_id",
            "workspace_id",
            "base_revision",
            "workspace_generation",
        ])?;
        if Identity::parse(doc.get("identity")?)? != expected {
            return Err(HostError::logical(ErrorCode::PermissionDenied));
        }
        if doc.get("backend_archive")?.string()? != message::ARCHIVE
            || doc.get("profile")?.string()? != "u01-static-read-v1"
        {
            return Err(T::CapabilityMismatch.into());
        }
        let root = doc.get("root")?.string()?;
        if !message::asset_name(root) {
            return Err(HostError::logical(ErrorCode::PermissionDenied));
        }
        let entries = doc.get("assets")?.array()?;
        if entries.is_empty() || entries.len() > 8 || owned.len() > 8 {
            return Err(T::Limit.into());
        }
        let mut names = BTreeSet::new();
        let mut assets = Vec::new();
        let mut total = 0u64;
        for entry in entries {
            entry.closed(&["name", "digest", "byte_length"])?;
            let name = entry.get("name")?.string()?;
            if !message::asset_name(name) || !names.insert(name.to_string()) {
                return Err(HostError::logical(ErrorCode::IntegrityError));
            }
            let digest = message::digest(entry.get("digest")?)?;
            let len = entry.get("byte_length")?.unsigned()?;
            total = total.checked_add(len).ok_or(T::Limit)?;
            if len > 65536 || total > 262144 {
                return Err(T::Limit.into());
            }
            let data = store
                .read_capped(&digest, 65536)
                .map_err(|e| HostError::logical(e.code()))?;
            if data.len() as u64 != len {
                return Err(HostError::logical(ErrorCode::IntegrityError));
            }
            assets.push((name.to_string(), digest));
        }
        if !names.contains(root) || !owned.is_subset(&names) {
            return Err(HostError::logical(ErrorCode::PermissionDenied));
        }
        // Publish a distinct host grant, never trust an owned-layer claim from USD or its manifest.
        let grant = Json::object([
            ("manifest", Json::text(manifest.to_string())),
            (
                "owned_layers",
                Json::Array(owned.iter().map(Json::text).collect()),
            ),
        ]);
        let (grant_digest, _) = store
            .publish(&grant.encode(&message::read_limits())?)
            .map_err(|e| HostError::logical(e.code()))?;
        Ok(Self {
            identity: expected,
            manifest: grant_digest,
            assets,
            owned,
        })
    }
    /// Identity independently approved by the host.
    pub fn identity(&self) -> &Identity {
        &self.identity
    }
    /// Digest of the bounded host-produced grant for the worker.
    pub fn grant_digest(&self) -> ContentDigest {
        self.manifest
    }
    fn open_body(&self) -> Json {
        let mut j = self.identity.json();
        if let Json::Object(o) = &mut j {
            o.insert(
                "manifest_digest".into(),
                Json::text(self.manifest.to_string()),
            );
        }
        j
    }
}
/// Statically registered shared commands. U02/U03 add typed variants when implemented;
/// no dynamic plugin discovery or raw setter dispatch exists.
pub enum Command {
    /// Open verified read input.
    Open(AuthorizedRead),
    /// Read one bounded page from the pinned live context.
    Read {
        /// Full context returned by open.
        context: Context,
        /// Opaque cursor from this incarnation, or the initial page.
        cursor: Option<String>,
    },
    /// Release the matching read view.
    Close(Context),
    /// Host-ledger query; no ledger exists yet, so always explicitly uncertain.
    QueryOutcome {
        /// Authorized project/workspace tuple.
        identity: Identity,
        /// Full operation identity.
        op_id: String,
    },
}
/// Same typed logical result for native callers and CLI serialization.
#[derive(Clone, Debug)]
pub struct ResultValue {
    /// Stable logical outcome.
    pub outcome: &'static str,
    /// Verified context; absent for host-ledger uncertainty.
    pub context: Option<Context>,
    /// Closed, validated profile body.
    pub body: Json,
}
impl ResultValue {
    /// Machine-readable result with the profile's actual capabilities.
    pub fn json(&self) -> Json {
        Json::object([
            ("outcome", Json::text(self.outcome)),
            (
                "context",
                self.context.as_ref().map_or(Json::Null, Context::json),
            ),
            ("body", self.body.clone()),
        ])
    }
}
/// A safe logical failure together with the independently known workspace tuple.
/// No worker acknowledgement or stale stage token is invented after process loss.
#[derive(Debug)]
pub struct DispatchFailure {
    /// Bounded fixed logical/transport error.
    pub error: HostError,
    /// Authorized observed identity, when the host has one.
    pub observed_identity: Option<Identity>,
}
impl std::ops::Deref for DispatchFailure {
    type Target = HostError;
    fn deref(&self) -> &Self::Target {
        &self.error
    }
}
impl From<DispatchFailure> for HostError {
    fn from(failure: DispatchFailure) -> Self {
        failure.error
    }
}
impl std::fmt::Display for DispatchFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl std::error::Error for DispatchFailure {}
/// Shared native/CLI host entry point, one stage owner and one request at a time.
pub struct Dispatcher {
    config: WorkerConfig,
    session: Option<Session>,
    grant: Option<AuthorizedRead>,
    live: Option<Context>,
    blocked_reap: bool,
    total: u64,
    delivered: BTreeSet<String>,
    next_cursor: Option<String>,
    pages: u32,
}
impl Dispatcher {
    /// Does not spawn a process or start a service until a read command is explicitly invoked.
    pub fn new(config: WorkerConfig) -> Self {
        Self {
            config,
            session: None,
            grant: None,
            live: None,
            blocked_reap: false,
            total: 0,
            delivered: BTreeSet::new(),
            next_cursor: None,
            pages: 0,
        }
    }
    /// Current real worker ID, if a session exists.
    pub fn process_id(&self) -> Option<u32> {
        self.session.as_ref().map(Session::process_id)
    }
    /// Incarnation-specific supervision hook, usable while an operation holds this dispatcher.
    /// Forced loss preserves the authorized read input and never records application acceptance.
    pub fn termination_handle(&self) -> Option<crate::worker::Termination> {
        self.session.as_ref().map(Session::termination_handle)
    }
    /// Host-known context, including after backend loss. A tuple is not proof of acceptance.
    pub fn observed_context(&self) -> Option<&Context> {
        self.live.as_ref()
    }
    fn stop(&mut self) -> Result<(), HostError> {
        if let Some(mut s) = self.session.take()
            && let Err(e) = s.stop()
        {
            self.blocked_reap = true;
            return Err(e);
        }
        Ok(())
    }
    fn start(&mut self, c: &Control) -> Result<(), HostError> {
        if self.blocked_reap {
            return Err(T::BackendLost.into());
        }
        if self.session.is_none() {
            self.session = Some(Session::spawn(&self.config, c)?);
        }
        Ok(())
    }
    fn raw(
        &mut self,
        method: &str,
        context: Json,
        body: Json,
        c: &Control,
    ) -> Result<Json, HostError> {
        self.session
            .as_mut()
            .ok_or(T::BackendLost)?
            .call(method, context, body, c)
    }
    /// Dispatches implemented read commands. Recovery retains the exact authorized grant,
    /// including nonzero generation; no unsent application change is deleted or accepted.
    pub fn execute(
        &mut self,
        command: Command,
        c: &Control,
    ) -> Result<ResultValue, DispatchFailure> {
        let requested = match &command {
            Command::QueryOutcome { identity, .. } if identity.validate().is_ok() => {
                Some(identity.clone())
            }
            Command::Open(grant) => Some(grant.identity.clone()),
            _ => None,
        };
        self.execute_inner(command, c)
            .map_err(|error| DispatchFailure {
                error,
                observed_identity: self.live.as_ref().map(|c| c.identity.clone()).or(requested),
            })
    }
    fn execute_inner(&mut self, command: Command, c: &Control) -> Result<ResultValue, HostError> {
        c.remaining()?;
        let result = match command {
            Command::Open(grant) => self.open(grant, c),
            Command::Read { context, cursor } => self.read(context, cursor, c),
            Command::Close(context) => {
                self.check_context(&context)?;
                let response = self.raw("CloseStage", context.json(), Json::object([]), c)?;
                let result = self.validate_response(response, "stage_closed", None)?;
                result.body.closed(&[])?;
                self.live = None;
                self.grant = None;
                Ok(result)
            }
            Command::QueryOutcome { identity, op_id } => {
                identity.validate()?;
                if !message::uuid(&op_id) {
                    return Err(HostError::logical(ErrorCode::InvalidId));
                }
                // No backend probe can substitute for U07 durable ledger proof.
                Err(HostError::logical(ErrorCode::OutcomeUnknown))
            }
        };
        if let Err(e) = result {
            if e.transport.is_some_and(|t| !matches!(t, T::StaleStage)) {
                self.stop()?;
            }
            return Err(e);
        }
        c.remaining()?;
        result
    }
    fn check_context(&self, context: &Context) -> Result<(), HostError> {
        let live = self.live.as_ref().ok_or(T::StaleStage)?;
        if live.stage_token != context.stage_token {
            return Err(T::StaleStage.into());
        }
        if live.identity.project_id != context.identity.project_id
            || live.identity.workspace_id != context.identity.workspace_id
        {
            return Err(HostError::logical(ErrorCode::PermissionDenied));
        }
        if live.identity.base_revision != context.identity.base_revision {
            return Err(HostError::logical(ErrorCode::BaseRevisionMismatch));
        }
        if live.identity.workspace_generation != context.identity.workspace_generation {
            return Err(HostError::logical(ErrorCode::StaleWorkspace));
        }
        Ok(())
    }
    fn delay(c: &Control, ms: u64) -> Result<(), HostError> {
        let end = Instant::now() + Duration::from_millis(ms);
        while Instant::now() < end {
            c.remaining()?;
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }
    fn retryable(e: HostError) -> bool {
        matches!(e.transport, Some(T::BackendLost | T::Truncated))
    }
    fn open(&mut self, grant: AuthorizedRead, c: &Control) -> Result<ResultValue, HostError> {
        for attempt in 0..3 {
            if attempt > 0 {
                self.stop()?;
                Self::delay(c, if attempt == 1 { 250 } else { 1000 })?;
            }
            match self.open_once(grant.clone(), c) {
                Ok(r) => return Ok(r),
                Err(e) if Self::retryable(e) && attempt < 2 => continue,
                Err(e) => return Err(e),
            }
        }
        Err(T::BackendLost.into())
    }
    fn open_once(&mut self, grant: AuthorizedRead, c: &Control) -> Result<ResultValue, HostError> {
        self.start(c)?;
        let response = self.raw("OpenStage", Json::Null, grant.open_body(), c)?;
        let result = self.validate_response(response, "stage_opened", Some(&grant.identity))?;
        validate_opened(&result.body)?;
        self.total = result.body.get("prim_count")?.unsigned()?;
        self.delivered.clear();
        self.next_cursor = None;
        self.pages = 0;
        self.live = result.context.clone();
        self.grant = Some(grant);
        Ok(result)
    }
    fn read(
        &mut self,
        context: Context,
        cursor: Option<String>,
        c: &Control,
    ) -> Result<ResultValue, HostError> {
        self.check_context(&context)?;
        if cursor.as_ref().is_some_and(|s| s.len() > 256) {
            return Err(T::Limit.into());
        }
        if cursor.is_some() && cursor != self.next_cursor {
            return Err(T::StaleStage.into());
        }
        let mut response = None;
        for attempt in 0..3 {
            if attempt > 0 {
                self.stop()?;
                Self::delay(c, if attempt == 1 { 250 } else { 1000 })?;
                let grant = self.grant.clone().ok_or(T::BackendLost)?;
                match self.open_once(grant, c) {
                    Ok(_) => (),
                    Err(e) if Self::retryable(e) && attempt < 2 => continue,
                    Err(e) => return Err(e),
                }
                // A continuation from the dead incarnation cannot authorize a resumed delivery.
                if cursor.is_some() {
                    return Err(T::StaleStage.into());
                }
            }
            let live = self.live.as_ref().ok_or(T::BackendLost)?.json();
            match self.raw(
                "ReadSnapshot",
                live,
                Json::object([("cursor", cursor.as_ref().map_or(Json::Null, Json::text))]),
                c,
            ) {
                Ok(r) => {
                    response = Some(r);
                    break;
                }
                Err(e) if Self::retryable(e) && attempt < 2 => continue,
                Err(e) => return Err(e),
            }
        }
        let response = response.ok_or(T::BackendLost)?;
        let result = self.validate_response(response, "snapshot_page", Some(&context.identity))?;
        if result.context.as_ref() != self.live.as_ref() {
            return Err(T::StaleStage.into());
        }
        let grant = self.grant.as_ref().ok_or(T::StaleStage)?;
        validate_page(&result.body, grant)?;
        if cursor.is_none() {
            self.delivered.clear();
            self.pages = 0;
        }
        self.pages = self.pages.checked_add(1).ok_or(T::Limit)?;
        if self.pages > 1024 {
            return Err(T::Limit.into());
        }
        for prim in result.body.get("items")?.array()? {
            let path = prim.get("path")?.string()?;
            if self.delivered.len() >= 256 || !self.delivered.insert(path.into()) {
                return Err(T::Message.into());
            }
        }
        let next = result.body.get("next")?;
        self.next_cursor = if *next == Json::Null {
            None
        } else {
            Some(next.string()?.into())
        };
        if self.next_cursor.is_some() && self.next_cursor == cursor {
            return Err(T::Message.into());
        }
        let count = self.delivered.len() as u64;
        if count > self.total
            || (self.next_cursor.is_none() && count != self.total)
            || (self.next_cursor.is_some() && count >= self.total)
        {
            return Err(T::Message.into());
        }
        Ok(result)
    }
    fn validate_response(
        &self,
        j: Json,
        expected: &'static str,
        identity: Option<&Identity>,
    ) -> Result<ResultValue, HostError> {
        let outcome = j.get("outcome")?.string()?;
        let observed = j.get("observed_context")?;
        let context = if *observed == Json::Null {
            None
        } else {
            Some(Context::parse(observed)?)
        };
        if let Some(context) = &context
            && identity.is_some_and(|i| i != &context.identity)
        {
            return Err(T::Session.into());
        }
        if matches!(outcome, "rejected" | "cancelled" | "outcome_unknown") {
            let code = ErrorCode::parse(j.get("code")?.string()?).ok_or(T::Message)?;
            if (outcome == "cancelled" && code != ErrorCode::Cancelled)
                || (outcome == "outcome_unknown" && code != ErrorCode::OutcomeUnknown)
            {
                return Err(T::Message.into());
            }
            let body = j.get("body")?;
            body.closed(&["diagnostics"])?;
            // This profile emits no diagnostic records; all other schemas remain unavailable.
            if !body.get("diagnostics")?.array()?.is_empty() {
                return Err(T::Message.into());
            }
            return Err(HostError::logical(code));
        }
        if outcome != expected
            || *j.get("code")? != Json::Null
            || (identity.is_some() && context.is_none())
        {
            return Err(T::Message.into());
        }
        Ok(ResultValue {
            outcome: expected,
            context,
            body: j.get("body")?.clone(),
        })
    }
}
fn validate_opened(j: &Json) -> Result<(), HostError> {
    j.closed(&[
        "backend_archive",
        "profile",
        "parser_contained",
        "mutations",
        "save",
        "publication",
        "durable_outcomes",
        "relationships",
        "meters_per_unit",
        "up_axis",
        "prim_count",
        "metadata",
    ])?;
    if j.get("backend_archive")?.string()? != message::ARCHIVE
        || j.get("profile")?.string()? != "u01-static-read-v1"
        || j.get("up_axis")?.string()? != "Z"
        || j.get("meters_per_unit")?.float()? != 0.001
    {
        return Err(T::Message.into());
    }
    for k in [
        "parser_contained",
        "mutations",
        "save",
        "publication",
        "durable_outcomes",
        "relationships",
    ] {
        if j.get(k)?.boolean()? {
            return Err(T::CapabilityMismatch.into());
        }
    }
    if j.get("prim_count")?.unsigned()? > 256 {
        return Err(T::Limit.into());
    }
    let Json::Object(metadata) = j.get("metadata")? else {
        return Err(T::Message.into());
    };
    let mut count = 0;
    for value in metadata.values() {
        message::validate_value(value, 0, &mut count)?;
    }
    Ok(())
}
fn validate_page(j: &Json, grant: &AuthorizedRead) -> Result<(), HostError> {
    j.closed(&["items", "next"])?;
    let items = j.get("items")?.array()?;
    if items.len() > 64 {
        return Err(T::Limit.into());
    }
    if *j.get("next")? != Json::Null {
        let next = j.get("next")?.string()?;
        if next.len() != 64 || next.parse::<ContentDigest>().is_err() || items.is_empty() {
            return Err(T::Message.into());
        }
    }
    let mut paths = BTreeSet::new();
    let mut count = 0;
    for prim in items {
        prim.closed(&[
            "path",
            "element_id",
            "type_name",
            "active",
            "properties",
            "sources",
        ])?;
        let path = prim.get("path")?.string()?;
        if !message::prim_path(path) || !paths.insert(path) {
            return Err(T::Message.into());
        }
        if *prim.get("element_id")? != Json::Null
            && !message::uuid(prim.get("element_id")?.string()?)
        {
            return Err(T::Message.into());
        }
        if *prim.get("type_name")? != Json::Null {
            token(prim.get("type_name")?.string()?)?;
        }
        prim.get("active")?.boolean()?;
        let properties = prim.get("properties")?.array()?;
        if properties.len() > 4096 {
            return Err(T::Limit.into());
        }
        let mut names = BTreeSet::new();
        for property in properties {
            property.closed(&["name", "type_name", "value"])?;
            let name = property.get("name")?.string()?;
            token(name)?;
            token(property.get("type_name")?.string()?)?;
            if !names.insert(name) {
                return Err(T::Message.into());
            }
            if *property.get("value")? != Json::Null {
                message::validate_value(property.get("value")?, 0, &mut count)?;
            }
        }
        let leaf = path.rsplit('/').next().ok_or(T::Message)?;
        let id = prim.get("element_id")?;
        let bim = properties
            .iter()
            .find(|p| p.get("name").and_then(Json::string) == Ok("bim:id"));
        if *id == Json::Null {
            if leaf.starts_with("E_") || bim.is_some() {
                return Err(T::Message.into());
            }
        } else {
            let id = id.string()?;
            if leaf != format!("E_{}", id.replace('-', "")) {
                return Err(T::Message.into());
            }
            let property = bim.ok_or(T::Message)?;
            let value = property.get("value")?;
            if !matches!(property.get("type_name")?.string()?, "string" | "token")
                || value.get("kind")?.string()? != "text"
                || value.get("value")?.string()? != id
            {
                return Err(T::Message.into());
            }
        }
        let sources = prim.get("sources")?.array()?;
        if sources.is_empty() || sources.len() > 4096 {
            return Err(T::Limit.into());
        }
        for source in sources {
            source.closed(&["layer", "digest", "path", "owned"])?;
            let layer = source.get("layer")?.string()?;
            let digest = message::digest(source.get("digest")?)?;
            if !grant.assets.iter().any(|(n, d)| n == layer && *d == digest)
                || !message::prim_path(source.get("path")?.string()?)
            {
                return Err(T::Message.into());
            }
            if source.get("owned")?.boolean()? != grant.owned.contains(layer) {
                return Err(T::Message.into());
            }
        }
    }
    Ok(())
}
fn token(s: &str) -> Result<(), HostError> {
    if s.is_empty() || s.len() > 1024 || s.chars().any(char::is_control) {
        Err(T::Message.into())
    } else {
        Ok(())
    }
}

/// Constructs a manifest for explicitly supplied local bytes. Publication here stores bytes
/// only; neither the manifest nor its digest is a U06 accepted revision.
pub fn read_manifest(
    identity: &Identity,
    root: &str,
    assets: &[(String, ContentDigest, u64)],
) -> Json {
    Json::object([
        ("identity", identity.json()),
        ("root", Json::text(root)),
        ("backend_archive", Json::text(message::ARCHIVE)),
        ("profile", Json::text("u01-static-read-v1")),
        (
            "assets",
            Json::Array(
                assets
                    .iter()
                    .map(|(n, d, l)| {
                        Json::object([
                            ("name", Json::text(n)),
                            ("digest", Json::text(d.to_string())),
                            ("byte_length", Json::uint(*l)),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    fn grant() -> AuthorizedRead {
        let digest = ContentDigest::from_bytes([0; 32]);
        AuthorizedRead {
            identity: Identity {
                project_id: "0199e7a0-0000-7000-8000-000000000001".into(),
                workspace_id: "0199e7a0-0000-7000-8000-000000000002".into(),
                base_revision: digest,
                workspace_generation: 42,
            },
            manifest: digest,
            assets: vec![("foreign.usda".into(), digest)],
            owned: BTreeSet::new(),
        }
    }
    fn prim() -> Json {
        Json::object([
            ("path", Json::text("/Foreign")),
            ("element_id", Json::Null),
            ("type_name", Json::text("Xform")),
            ("active", Json::Bool(true)),
            ("properties", Json::Array(vec![])),
            (
                "sources",
                Json::Array(vec![Json::object([
                    ("layer", Json::text("foreign.usda")),
                    (
                        "digest",
                        Json::text(ContentDigest::from_bytes([0; 32]).to_string()),
                    ),
                    ("path", Json::text("/Foreign")),
                    ("owned", Json::Bool(false)),
                ])]),
            ),
        ])
    }
    fn page(p: Json) -> Json {
        Json::object([("items", Json::Array(vec![p])), ("next", Json::Null)])
    }
    #[test]
    fn hostile_worker_pages_cannot_invent_ownership_identity_or_unbounded_data() {
        let grant = grant();
        assert!(validate_page(&page(prim()), &grant).is_ok());
        let mut dishonest = prim();
        if let Json::Object(o) = &mut dishonest {
            o.insert(
                "sources".into(),
                Json::Array(vec![Json::object([
                    ("layer", Json::text("foreign.usda")),
                    (
                        "digest",
                        Json::text(ContentDigest::from_bytes([0; 32]).to_string()),
                    ),
                    ("path", Json::text("/Foreign")),
                    ("owned", Json::Bool(true)),
                ])]),
            );
        }
        assert_eq!(
            validate_page(&page(dishonest), &grant)
                .unwrap_err()
                .transport,
            Some(T::Message)
        );
        let oversized = Json::object([
            ("items", Json::Array(vec![prim(); 65])),
            ("next", Json::Null),
        ]);
        assert_eq!(
            validate_page(&oversized, &grant).unwrap_err().transport,
            Some(T::Limit)
        );
        for (key, value) in [
            (
                "element_id",
                Json::text("0199e7a0-0000-7000-8000-000000000001"),
            ),
            ("path", Json::text("/../Escape")),
            ("path", Json::text("/".repeat(1025))),
            ("extra", Json::Bool(true)),
        ] {
            let mut p = prim();
            if let Json::Object(o) = &mut p {
                o.insert(key.into(), value);
            }
            assert!(validate_page(&page(p), &grant).is_err());
        }
    }
}
