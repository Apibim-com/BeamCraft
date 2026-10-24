// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The operating system's folder dialog.
//!
//! Windows and macOS show their own dialog through `rfd` (no extra runtime). Linux has no
//! dependency-free system dialog, so the desktop's own dialog program is asked instead: `zenity`
//! (GNOME and most desktops), then `kdialog` (KDE). Without either the caller says so and the
//! path is typed. The dialog is modal: the window waits while it is open.

use std::path::{Path, PathBuf};

/// Why no folder could be chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogError {
    /// This system has no folder dialog that BeamCraft can show.
    Unavailable,
    /// The dialog program failed.
    Failed(String),
}

impl std::fmt::Display for DialogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable => f.write_str(
                "no folder dialog is available here (install zenity or kdialog); type the path instead",
            ),
            Self::Failed(why) => write!(f, "the folder dialog failed ({why})"),
        }
    }
}

/// Asks for a folder, starting in `start` when it exists. `Ok(None)` means the dialog was
/// cancelled.
pub fn pick_folder(title: &str, start: Option<&Path>) -> Result<Option<PathBuf>, DialogError> {
    let start = start.filter(|p| p.is_dir());
    #[cfg(any(windows, target_os = "macos"))]
    {
        let mut dialog = rfd::FileDialog::new().set_title(title);
        if let Some(dir) = start {
            dialog = dialog.set_directory(dir);
        }
        Ok(dialog.pick_folder())
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        desktop_dialog(title, start)
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
fn desktop_dialog(title: &str, start: Option<&Path>) -> Result<Option<PathBuf>, DialogError> {
    use std::process::{Command, Stdio};
    let start = start.and_then(Path::to_str).unwrap_or("");
    let programs: [(&str, Vec<String>); 2] = [
        (
            "zenity",
            vec![
                "--file-selection".into(),
                "--directory".into(),
                format!("--title={title}"),
                format!("--filename={start}/"),
            ],
        ),
        (
            "kdialog",
            vec![
                "--title".into(),
                title.into(),
                "--getexistingdirectory".into(),
                if start.is_empty() {
                    ".".into()
                } else {
                    start.into()
                },
            ],
        ),
    ];
    for (program, args) in programs {
        let output = match Command::new(program)
            .args(&args)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
        {
            Ok(output) => output,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(DialogError::Failed(format!("{program}: {}", e.kind()))),
        };
        // Both programs exit with 1 when the dialog is cancelled.
        if output.status.code() == Some(1) {
            return Ok(None);
        }
        if !output.status.success() {
            return Err(DialogError::Failed(format!(
                "{program} exited with {}",
                output.status
            )));
        }
        return Ok(chosen(&output.stdout));
    }
    Err(DialogError::Unavailable)
}

/// The folder a dialog program printed: its first line, without the line end; `None` when empty.
#[cfg_attr(any(windows, target_os = "macos"), allow(dead_code))]
fn chosen(stdout: &[u8]) -> Option<PathBuf> {
    let text = String::from_utf8_lossy(stdout);
    let line = text.lines().next()?.trim_end_matches(['\r', '\n']);
    (!line.is_empty()).then(|| PathBuf::from(line))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;

    #[test]
    fn reads_the_folder_a_dialog_printed() {
        assert_eq!(
            chosen(b"/home/a/Models\n"),
            Some(PathBuf::from("/home/a/Models"))
        );
        assert_eq!(
            chosen(b"/home/a/Model s\r\n"),
            Some(PathBuf::from("/home/a/Model s"))
        );
        assert_eq!(chosen(b"\n"), None);
        assert_eq!(chosen(b""), None);
    }
}
