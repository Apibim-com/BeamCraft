// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! `--screenshot FILE`: the window runs as usual, draws [`FRAMES`] frames, saves its own picture
//! as a PNG to `FILE` and closes. A check of the build (or a person) gets an image of the window
//! without a screen-capture tool; nothing is read from or written to the model.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex};

/// Frames drawn before the picture is taken, so the window has laid out and loaded.
pub const FRAMES: u32 = 120;

/// What became of the picture: `None` while the window runs, then the save's outcome.
pub type Outcome = Arc<Mutex<Option<Result<(), String>>>>;

/// An app that takes its own picture after [`FRAMES`] frames and then closes the window.
pub struct Capture<'a> {
    inner: Box<dyn eframe::App + 'a>,
    path: PathBuf,
    frames: u32,
    shot: Option<Receiver<Arc<egui::ColorImage>>>,
    outcome: Outcome,
}

impl<'a> Capture<'a> {
    /// Wraps `inner`; the picture goes to `path`, the outcome to `outcome`.
    pub fn new(inner: Box<dyn eframe::App + 'a>, path: PathBuf, outcome: Outcome) -> Self {
        Self {
            inner,
            path,
            frames: 0,
            shot: None,
            outcome,
        }
    }

    fn finish(&self, result: Result<(), String>) {
        if let Ok(mut slot) = self.outcome.lock() {
            *slot = Some(result);
        }
    }
}

/// Writes `image` as a PNG to `path`.
pub fn save_png(image: &egui::ColorImage, path: &std::path::Path) -> Result<(), String> {
    let [width, height] = image.size;
    let bytes: Vec<u8> = image
        .pixels
        .iter()
        .flat_map(|p| p.to_srgba_unmultiplied())
        .collect();
    let (w, h) = (
        u32::try_from(width).map_err(|_| "the picture is too wide".to_owned())?,
        u32::try_from(height).map_err(|_| "the picture is too tall".to_owned())?,
    );
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("the folder could not be made: {e}"))?;
    }
    image::save_buffer_with_format(
        path,
        &bytes,
        w,
        h,
        image::ExtendedColorType::Rgba8,
        image::ImageFormat::Png,
    )
    .map_err(|e| format!("the picture could not be saved: {e}"))
}

impl eframe::App for Capture<'_> {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.inner.ui(ui, frame);
        let ctx = ui.ctx().clone();
        ctx.request_repaint();
        if self.frames == u32::MAX {
            // The picture is taken; the window is closing.
            return;
        }
        if let Some(rx) = &self.shot {
            match rx.try_recv() {
                Ok(image) => {
                    self.finish(save_png(&image, &self.path));
                    self.frames = u32::MAX;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.finish(Err("the window gave no picture".into()));
                    self.frames = u32::MAX;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
            return;
        }
        self.frames = self.frames.saturating_add(1).min(u32::MAX - 1);
        if self.frames >= FRAMES {
            let (tx, rx) = std::sync::mpsc::channel();
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
                egui::ScreenshotCallback::new(move |image| {
                    let _ = tx.send(image);
                }),
            ));
            self.shot = Some(rx);
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_is_saved_as_png() {
        let dir = std::env::temp_dir().join(format!("beamcraft-capture-{}", std::process::id()));
        let path = dir.join("shot.png");
        let image = egui::ColorImage::new([4, 3], vec![egui::Color32::from_rgb(10, 20, 30); 12]);
        save_png(&image, &path).unwrap();
        let back = image::open(&path).unwrap().to_rgba8();
        assert_eq!(back.dimensions(), (4, 3));
        assert_eq!(back.get_pixel(1, 1).0, [10, 20, 30, 255]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
