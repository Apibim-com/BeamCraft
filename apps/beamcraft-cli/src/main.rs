// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Headless CLI using the same native dispatcher as application callers.

mod revision;

use beamcraft_host::{
    dispatch::{self, AuthorizedRead, Command, Dispatcher},
    storage::BlobStore,
    worker::{Cancellation, Control, HostError, WorkerConfig},
};
use beamcraft_model::{ErrorCode, digest::ContentDigest};
use beamcraft_protocol::{
    json::Json,
    message::{self, Identity},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::PathBuf,
};

mod plate_cmd;
mod view;

fn invalid() -> HostError {
    HostError::logical(ErrorCode::InvalidArgument)
}
fn main() -> std::process::ExitCode {
    let args = std::env::args_os().skip(1).take(50).collect::<Vec<_>>();
    if args.len() == 1 && (args.first().is_some_and(|s| s == "--version" || s == "-V")) {
        println!("beamcraft {}", env!("CARGO_PKG_VERSION"));
        return std::process::ExitCode::SUCCESS;
    }
    if args.is_empty()
        || (args.len() == 1 && args.first().is_some_and(|s| s == "--help" || s == "-h"))
    {
        println!(
            "BeamCraft: open | read | query-outcome | blob-put | blob-verify | revision-commit | revision-open | revision-recover | revision-outcome | create-plate | plate-inspect | define-material | define-profile | definitions | create-member | view-snapshot | view-changes\nRead: --backend rust --store ROOT --worker EXE --worker-digest SHA256 --project-id UUID --workspace-id UUID --revision SHA256 --generation U64\nInput: --manifest SHA256 [--owned NAME,...] or --file FILE --digest SHA256 [--owned root.usda]\nControl: [--timeout-ms 1..120000] [--cancel-after-ms 0..120000]\nRevision: revision-commit --store ROOT --project-id UUID --workspace-id UUID --revision-id UUID --op-id UUID --generation U64 --base none|REVISION_UUID --schema SHA256 --root-layer NAME --layers ROLE:NAME:SHA256,... (ROLE authored|derived|external; blobs must be stored first); revision-open --store ROOT\nRecovery: revision-recover --store ROOT --min-age-secs U64 (0 asserts that no commit is running; closes unresolved intents from the root, removes abandoned temporary files, reports unreferenced blobs, deletes no blob); revision-outcome --store ROOT --op-id UUID (host-ledger QueryOutcome: committed | aborted | not recorded | OUTCOME_UNKNOWN)\nPlate (P01, committed USD revision): create-plate --store ROOT --width MM --length MM --thickness MM [--name TEXT] [--translation X,Y,Z] [--material-digest SHA256] [--hole-rect X,Y,W,H[;...]] [--hole-round X,Y,D[;...]] [--hole-segments N] [--new-id UUID] [--op-id UUID] [--revision-id UUID] [--actor-id UUID] [--project-id UUID] [--workspace-id UUID] (rectangular origin plate only; a retry of the same --op-id needs the same --new-id and arguments and is replayed); plate-inspect --store ROOT (verified head revision, plates, meshes, snapshot)\nDefinitions (P02, immutable USD assets pinned by full digest; a material without --density keeps an unknown density, so mass stays unknown): define-material --store ROOT [--density KG_M3] [--revision-id UUID] [--actor-id UUID] [--project-id UUID] [--workspace-id UUID]; define-profile --store ROOT --width MM --height MM [same ids] (rectangle section; no command consumes a profile yet); definitions --store ROOT (verified head: digest, kind, units, values, plates that pin it). A definition already in the head is not rewritten (definition_exists). create-plate --material-digest pins an existing material.\nMember (P03): create-member --store ROOT --start X,Y,Z --end X,Y,Z --profile-digest SHA256 [--kind beam|column] [--roll RAD] [--material-digest SHA256] [--name TEXT] [--translation X,Y,Z] [--new-id UUID] [--op-id UUID] [--revision-id UUID] [--actor-id UUID] [--project-id UUID] [--workspace-id UUID] (straight member with a pinned rectangle profile; plate-inspect lists plates and members)\nView (synthetic kernel plate scene, no worker or store): view-snapshot --plates 1..500 [--hide KIND,...] [--representation full|simplified] [--stand-in-simplified true|false] [--select ID,...] [--revision SHA256] [--generation U64]; view-changes --plates 1..500 --edit move-first|resize-first|add-one|remove-last,... [--since TOKEN] [--stand-in-simplified true|false]\nOnly the experimental static read profile is available; save/mutation/SDK fallback are unavailable; revision-commit verifies blobs but not USD content; create-plate verifies the USD content (U03 index, structural check, kernel re-evaluation) in-process."
        );
        return std::process::ExitCode::SUCCESS;
    }
    let (json, exit) = if args
        .first()
        .is_some_and(|c| c == "view-snapshot" || c == "view-changes")
    {
        view::execute(&args)
    } else if args.first().is_some_and(|c| {
        c == "create-plate"
            || c == "plate-inspect"
            || c == "define-material"
            || c == "define-profile"
            || c == "definitions"
            || c == "create-member"
    }) {
        plate_cmd::execute(&args)
    } else {
        match run(args) {
            Ok(j) => (j, 0),
            Err((e, observed)) => (
                Json::object([
                    (
                        "outcome",
                        Json::text(if e.code == ErrorCode::OutcomeUnknown {
                            "outcome_unknown"
                        } else if e.code == ErrorCode::Cancelled {
                            "cancelled"
                        } else {
                            "rejected"
                        }),
                    ),
                    ("code", Json::text(e.code.as_str())),
                    ("message", Json::text(e.message())),
                    (
                        "transport_code",
                        e.transport.map_or(Json::Null, |t| Json::text(t.as_str())),
                    ),
                    ("observed_context", observed),
                    ("diagnostics", Json::Array(vec![])),
                ]),
                e.exit_code(),
            ),
        }
    };
    match json.encode(&message::read_limits()) {
        Ok(bytes) => {
            use std::io::Write;
            if std::io::stdout().lock().write_all(&bytes).is_err() {
                return std::process::ExitCode::from(3);
            }
        }
        Err(_) => {
            println!("{{\"outcome\":\"rejected\",\"code\":\"LIMIT_EXCEEDED\"}}");
            return std::process::ExitCode::from(2);
        }
    }
    std::process::ExitCode::from(exit)
}
fn run(args: Vec<std::ffi::OsString>) -> Result<Json, (HostError, Json)> {
    let mut dispatcher = None;
    let mut known_workspace = Json::Null;
    let result = (|| -> Result<Json, HostError> {
        if args.len() > 49 {
            return Err(invalid());
        }
        let args = args
            .into_iter()
            .map(|s| s.into_string().map_err(|_| invalid()))
            .collect::<Result<Vec<_>, _>>()?;
        if args.iter().any(|s| s.len() > 4096) {
            return Err(invalid());
        }
        let command = args.first().ok_or_else(invalid)?;
        let tail = args.get(1..).ok_or_else(invalid)?;
        if !tail.len().is_multiple_of(2) {
            return Err(invalid());
        }
        let mut options = BTreeMap::new();
        for pair in tail.as_chunks::<2>().0 {
            let [key, value] = pair;
            if !matches!(
                key.as_str(),
                "--backend"
                    | "--store"
                    | "--worker"
                    | "--worker-digest"
                    | "--project-id"
                    | "--workspace-id"
                    | "--revision"
                    | "--generation"
                    | "--manifest"
                    | "--owned"
                    | "--file"
                    | "--digest"
                    | "--timeout-ms"
                    | "--cancel-after-ms"
                    | "--op-id"
                    | "--revision-id"
                    | "--base"
                    | "--schema"
                    | "--root-layer"
                    | "--layers"
                    | "--min-age-secs"
            ) || options.insert(key.as_str(), value.as_str()).is_some()
            {
                return Err(invalid());
            }
        }
        let get = |k: &str| options.get(k).copied().ok_or_else(invalid);
        let timeout = options
            .get("--timeout-ms")
            .map(|s| s.parse::<u64>().map_err(|_| invalid()))
            .transpose()?;
        let cancellation = Cancellation::default();
        if let Some(ms) = options.get("--cancel-after-ms") {
            let ms = ms.parse::<u64>().map_err(|_| invalid())?;
            if ms > 120000 {
                return Err(invalid());
            }
            if ms == 0 {
                cancellation.cancel();
            } else {
                let token = cancellation.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(ms));
                    token.cancel();
                });
            }
        }
        let control = Control::new(timeout, cancellation)?;
        control.remaining()?;
        let store = BlobStore::new(get("--store")?).map_err(|e| HostError::logical(e.code()))?;
        let parse_digest = |s: &str| s.parse::<ContentDigest>().map_err(|_| invalid());
        if command == "blob-put" {
            let bytes = read_file(get("--file")?, 32 * 1024 * 1024)?;
            let (digest, _) = store
                .publish(&bytes)
                .map_err(|e| HostError::logical(e.code()))?;
            control.remaining()?;
            return Ok(Json::object([
                ("outcome", Json::text("blob_stored")),
                ("digest", Json::text(digest.to_string())),
            ]));
        }
        if command == "blob-verify" {
            let digest = parse_digest(get("--digest")?)?;
            store
                .verify(&digest)
                .map_err(|e| HostError::logical(e.code()))?;
            control.remaining()?;
            return Ok(Json::object([
                ("outcome", Json::text("blob_verified")),
                ("digest", Json::text(digest.to_string())),
            ]));
        }
        if command == "revision-commit" {
            // The deadline and cancellation are checked before and during the commit (not
            // after the root switch: a timeout would hide a committed revision). A retry with
            // the same op id returns the revision.
            return revision::commit(&store, &options, &control);
        }
        if command == "revision-open" {
            let result = revision::open(&store)?;
            control.remaining()?;
            return Ok(result);
        }
        // Recovery and outcome queries only restate what the root proves; a deadline after the
        // work would hide a result that is already durable, so none is checked afterwards.
        if command == "revision-recover" {
            return revision::recover(&store, &options);
        }
        if command == "revision-outcome" {
            return revision::outcome(&store, &options);
        }
        if !matches!(command.as_str(), "open" | "read" | "query-outcome") {
            return Err(HostError::logical(ErrorCode::UnsupportedCommand));
        }
        if get("--backend")? != "rust" {
            return Err(HostError::logical(ErrorCode::BackendUnavailable));
        }
        let identity = Identity {
            project_id: get("--project-id")?.into(),
            workspace_id: get("--workspace-id")?.into(),
            base_revision: parse_digest(get("--revision")?)?,
            workspace_generation: message::generation(&Json::text(get("--generation")?))?,
        };
        identity.validate()?;
        // QueryOutcome is available without a worker or a claimed durable ledger result.
        if command == "query-outcome" {
            if !message::uuid(get("--op-id")?) {
                return Err(HostError::logical(ErrorCode::InvalidId));
            }
            known_workspace = identity.json();
            return Err(HostError::logical(ErrorCode::OutcomeUnknown));
        }
        let manifest = if let Some(m) = options.get("--manifest") {
            if options.contains_key("--file") || options.contains_key("--digest") {
                return Err(invalid());
            }
            parse_digest(m)?
        } else {
            let bytes = read_file(get("--file")?, 65536)?;
            let expected = parse_digest(get("--digest")?)?;
            let (digest, _) = store
                .publish(&bytes)
                .map_err(|e| HostError::logical(e.code()))?;
            if digest != expected {
                return Err(HostError::logical(ErrorCode::IntegrityError));
            }
            let manifest = dispatch::read_manifest(
                &identity,
                "root.usda",
                &[("root.usda".into(), digest, bytes.len() as u64)],
            );
            store
                .publish(&manifest.encode(&message::read_limits())?)
                .map_err(|e| HostError::logical(e.code()))?
                .0
        };
        let owned = options.get("--owned").map_or_else(BTreeSet::new, |s| {
            s.split(',').map(str::to_string).collect()
        });
        let grant = AuthorizedRead::authorize(&store, manifest, identity, owned)?;
        known_workspace = grant.identity().json();
        let host = dispatcher.insert(Dispatcher::new(WorkerConfig {
            executable: PathBuf::from(get("--worker")?),
            executable_digest: parse_digest(get("--worker-digest")?)?,
            store_root: PathBuf::from(get("--store")?),
        }));
        let opened = host.execute(Command::Open(grant), &control)?;
        if command == "open" {
            return Ok(opened.json());
        }
        let context = opened.context.ok_or_else(invalid)?;
        host.execute(
            Command::Read {
                context,
                cursor: None,
            },
            &control,
        )
        .map(|r| r.json())
        .map_err(HostError::from)
    })();
    result.map_err(|e| {
        (
            e,
            dispatcher
                .as_ref()
                .and_then(Dispatcher::observed_context)
                .map_or(known_workspace, |context| context.identity.json()),
        )
    })
}
fn read_file(path: &str, cap: usize) -> Result<Vec<u8>, HostError> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    // Check the opened handle below; a path metadata check cannot close a replacement race.
    // O_NONBLOCK prevents a FIFO replacement from blocking the open itself.
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        #[cfg(any(target_os = "linux", target_os = "android"))]
        const O_NONBLOCK: i32 = 0x800;
        #[cfg(any(
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "dragonfly"
        ))]
        const O_NONBLOCK: i32 = 0x4;
        #[cfg(not(any(
            target_os = "linux",
            target_os = "android",
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "dragonfly"
        )))]
        return Err(invalid());
        #[cfg(any(
            target_os = "linux",
            target_os = "android",
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "dragonfly"
        ))]
        options.custom_flags(O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::{
            os::windows::fs::{MetadataExt, OpenOptionsExt},
            path::{Component, Prefix},
        };
        if matches!(std::path::Path::new(path).components().next(), Some(Component::Prefix(p)) if !matches!(p.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)))
        {
            return Err(invalid());
        }
        let metadata =
            std::fs::symlink_metadata(path).map_err(|_| HostError::logical(ErrorCode::NotFound))?;
        if !metadata.is_file() || metadata.file_attributes() & 0x400 != 0 {
            return Err(invalid());
        }
        // Open a concurrently substituted reparse point itself rather than its target.
        options.custom_flags(0x0020_0000); // FILE_FLAG_OPEN_REPARSE_POINT
    }
    let file = options
        .open(path)
        .map_err(|_| HostError::logical(ErrorCode::NotFound))?;
    let metadata = file.metadata().map_err(|_| invalid())?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(invalid());
        }
    }
    if !metadata.is_file() || metadata.len() > cap as u64 {
        return Err(HostError::logical(ErrorCode::LimitExceeded));
    }
    let mut data = Vec::new();
    file.take(cap as u64 + 1)
        .read_to_end(&mut data)
        .map_err(|_| invalid())?;
    if data.len() > cap {
        return Err(HostError::logical(ErrorCode::LimitExceeded));
    }
    Ok(data)
}
