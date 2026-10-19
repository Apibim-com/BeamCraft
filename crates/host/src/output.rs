// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Atomic publication of one output file (E01 section 14, "Output publication").
//!
//! The bytes are written to a temporary file in the destination directory, flushed and synced,
//! and installed under the final name only after the caller's writer succeeded. An existing file
//! is not replaced unless the caller asks for it; a symbolic link or Windows reparse point at the
//! output name or at its parent directory is refused. A failure removes the temporary file and
//! leaves no output. The path is an I/O-edge argument and authorises nothing else.

use crate::storage::is_link;
use beamcraft_model::error::ErrorCode;
use std::fs::{self, File, OpenOptions};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Why an output could not be published.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputError {
    /// The F04 code of the failure.
    pub code: ErrorCode,
    /// A fixed message that never echoes the path.
    pub message: &'static str,
}

impl OutputError {
    const fn new(code: ErrorCode, message: &'static str) -> Self {
        Self { code, message }
    }
}

impl std::fmt::Display for OutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for OutputError {}

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct Temporary(PathBuf);

impl Drop for Temporary {
    fn drop(&mut self) {
        // Best effort: the file may already be installed or gone.
        let _ = fs::remove_file(&self.0);
    }
}

/// Publishes the file `target` with the bytes `write` produces.
///
/// `write` receives the temporary file; its error aborts the publication. With `overwrite` an
/// existing regular file is replaced by rename; without it the install is a non-replacing hard
/// link, and a file system that cannot make one fails with `BACKEND_UNAVAILABLE` (there is no
/// rename fallback that could replace a file created in the meantime).
pub fn publish_file<F>(target: &Path, overwrite: bool, write: F) -> Result<(), OutputError>
where
    F: FnOnce(&mut File) -> Result<(), OutputError>,
{
    let name = target
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.is_empty() && n.len() <= 200)
        .ok_or(OutputError::new(
            ErrorCode::InvalidArgument,
            "the output path has no usable file name",
        ))?;
    let parent = match target.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    let parent_meta = fs::symlink_metadata(parent).map_err(|_| {
        OutputError::new(
            ErrorCode::InvalidArgument,
            "the output directory does not exist",
        )
    })?;
    if is_link(&parent_meta) || !parent_meta.is_dir() {
        return Err(OutputError::new(
            ErrorCode::InvalidArgument,
            "the output directory is a link or not a directory",
        ));
    }
    match fs::symlink_metadata(target) {
        Ok(meta) => {
            if is_link(&meta) || !meta.is_file() {
                return Err(OutputError::new(
                    ErrorCode::InvalidArgument,
                    "the output name is a link or not a regular file",
                ));
            }
            if !overwrite {
                return Err(OutputError::new(
                    ErrorCode::InvalidArgument,
                    "the output file exists and overwrite was not requested",
                ));
            }
        }
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        Err(_) => {
            return Err(OutputError::new(
                ErrorCode::BackendUnavailable,
                "the output name could not be examined",
            ));
        }
    }
    let unavailable = |_: std::io::Error| {
        OutputError::new(
            ErrorCode::BackendUnavailable,
            "the output could not be written",
        )
    };
    let temporary_path = parent.join(format!(
        ".{name}.tmp-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary_path)
        .map_err(unavailable)?;
    let temporary = Temporary(temporary_path);
    write(&mut file)?;
    file.sync_all().map_err(unavailable)?;
    drop(file);
    if overwrite {
        fs::rename(&temporary.0, target).map_err(unavailable)?;
    } else {
        match fs::hard_link(&temporary.0, target) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                return Err(OutputError::new(
                    ErrorCode::InvalidArgument,
                    "the output file exists and overwrite was not requested",
                ));
            }
            Err(e) => return Err(unavailable(e)),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "beamcraft-output-{tag}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn put(text: &'static str) -> impl FnOnce(&mut File) -> Result<(), OutputError> {
        move |f| {
            f.write_all(text.as_bytes())
                .map_err(|_| OutputError::new(ErrorCode::BackendUnavailable, "write"))
        }
    }

    fn leftovers(dir: &Path) -> Vec<String> {
        fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect()
    }

    #[test]
    fn a_new_file_is_installed_whole_and_no_temporary_remains() {
        let dir = scratch("new");
        let target = dir.join("a.ifc");
        publish_file(&target, false, put("hello")).unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "hello");
        assert_eq!(leftovers(&dir), ["a.ifc"]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_existing_file_is_kept_unless_overwrite_is_asked() {
        let dir = scratch("exists");
        let target = dir.join("a.ifc");
        publish_file(&target, false, put("one")).unwrap();
        let refused = publish_file(&target, false, put("two")).unwrap_err();
        assert_eq!(refused.code, ErrorCode::InvalidArgument);
        assert_eq!(fs::read_to_string(&target).unwrap(), "one");
        publish_file(&target, true, put("two")).unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "two");
        assert_eq!(leftovers(&dir), ["a.ifc"]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_failing_writer_leaves_no_output_and_no_temporary() {
        let dir = scratch("fail");
        let target = dir.join("a.ifc");
        let error = publish_file(&target, false, |_| {
            Err(OutputError::new(ErrorCode::LimitExceeded, "too big"))
        })
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::LimitExceeded);
        assert!(leftovers(&dir).is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_missing_directory_and_a_directory_target_are_refused() {
        let dir = scratch("dirs");
        let missing = dir.join("nope").join("a.ifc");
        assert_eq!(
            publish_file(&missing, false, put("x")).unwrap_err().code,
            ErrorCode::InvalidArgument
        );
        let as_dir = dir.join("sub");
        fs::create_dir(&as_dir).unwrap();
        assert_eq!(
            publish_file(&as_dir, true, put("x")).unwrap_err().code,
            ErrorCode::InvalidArgument
        );
        assert_eq!(
            publish_file(Path::new(""), false, put("x"))
                .unwrap_err()
                .code,
            ErrorCode::InvalidArgument
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_link_at_the_name_or_the_parent_is_refused() {
        let dir = scratch("links");
        let real = dir.join("real.ifc");
        fs::write(&real, "keep").unwrap();
        let link = dir.join("link.ifc");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        for overwrite in [false, true] {
            let error = publish_file(&link, overwrite, put("x")).unwrap_err();
            assert_eq!(error.code, ErrorCode::InvalidArgument);
        }
        assert_eq!(fs::read_to_string(&real).unwrap(), "keep");
        let sub = dir.join("sub");
        fs::create_dir(&sub).unwrap();
        let dir_link = dir.join("dirlink");
        std::os::unix::fs::symlink(&sub, &dir_link).unwrap();
        let error = publish_file(&dir_link.join("a.ifc"), false, put("x")).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidArgument);
        assert!(leftovers(&sub).is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }
}
