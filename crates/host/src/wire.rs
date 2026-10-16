// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Native stream I/O for U04's zero-attachment F05 read profile.

use beamcraft_protocol::{
    FrameHeader, FrameKind, Limits, TransportCode as T,
    json::{self, Json},
};
use std::io::{Read, Write};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// Shared first-byte deadline owned by one pipe reader. Only that reader starts and retires
/// its current frame; consumers never clear another frame's clock after receiving a result.
#[derive(Clone, Default)]
pub struct Assembly(Arc<Mutex<Option<Instant>>>);
impl Assembly {
    pub(crate) fn begin(&self, now: Instant) -> Result<(), T> {
        *self.0.lock().map_err(|_| T::BackendLost)? = Some(now);
        Ok(())
    }
    fn finish(&self, now: Instant) -> Result<(), T> {
        let start = self.0.lock().map_err(|_| T::BackendLost)?.take();
        if start.is_some_and(|s| now.saturating_duration_since(s) >= Duration::from_secs(10)) {
            return Err(T::Deadline);
        }
        Ok(())
    }
    /// Whether the sole reader's current incomplete frame has reached its fixed deadline.
    pub fn expired(&self, now: Instant) -> Result<bool, T> {
        Ok(self
            .0
            .lock()
            .map_err(|_| T::BackendLost)?
            .is_some_and(|s| now.saturating_duration_since(s) >= Duration::from_secs(10)))
    }
}

/// Reads a bounded frame and retires its assembly clock before handing off the result.
/// A complete frame which exceeded its own deadline is never dispatched.
pub fn read_timed(reader: &mut impl Read, limits: &Limits, assembly: &Assembly) -> Result<Json, T> {
    let result = read_started(reader, limits, || assembly.begin(Instant::now()));
    assembly.finish(Instant::now())?;
    result
}

/// Reads exactly one bounded control frame, with no resynchronization or oversized drain.
/// Callers enforce deadlines by supervising and terminating the owning process.
pub fn read(reader: &mut impl Read, limits: &Limits) -> Result<Json, T> {
    read_tracked(reader, limits, || {})
}
/// Reads a frame and signals its first byte for an independent assembly deadline.
pub fn read_tracked(
    reader: &mut impl Read,
    limits: &Limits,
    on_start: impl FnOnce(),
) -> Result<Json, T> {
    read_started(reader, limits, || {
        on_start();
        Ok(())
    })
}
fn read_started(
    reader: &mut impl Read,
    limits: &Limits,
    on_start: impl FnOnce() -> Result<(), T>,
) -> Result<Json, T> {
    let mut bytes = [0u8; 16];
    match reader.read(&mut bytes[..1]) {
        Ok(0) => return Err(T::BackendLost),
        Ok(_) => (),
        Err(_) => return Err(T::BackendLost),
    }
    on_start()?;
    reader
        .read_exact(&mut bytes[1..])
        .map_err(|_| T::Truncated)?;
    let header = FrameHeader::decode(&bytes, limits).map_err(|e| e.code())?;
    if header.kind() != FrameKind::Control {
        return Err(T::Frame);
    }
    if header.attachment_count() != 0 {
        return Err(T::Buffer);
    }
    let mut payload = vec![0; header.payload_len() as usize];
    reader.read_exact(&mut payload).map_err(|_| T::Truncated)?;
    let value = json::decode(&payload, limits)?;
    if !value.get("buffers")?.array()?.is_empty() {
        return Err(T::Buffer);
    }
    Ok(value)
}
/// Writes a complete bounded control bundle.
pub fn write(writer: &mut impl Write, value: &Json, limits: &Limits) -> Result<(), T> {
    let payload = value.encode(limits)?;
    let len = u32::try_from(payload.len()).map_err(|_| T::Limit)?;
    let header = FrameHeader::new(FrameKind::Control, 0, len, limits).map_err(|e| e.code())?;
    writer
        .write_all(&header.encode())
        .and_then(|()| writer.write_all(&payload))
        .and_then(|()| writer.flush())
        .map_err(|_| T::BackendLost)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Cursor, sync::mpsc, thread};
    #[test]
    fn completed_frame_cannot_leave_a_stale_start_or_clear_the_next_frame() {
        let assembly = Assembly::default();
        let mut bytes = Vec::new();
        let j = Json::object([("buffers", Json::Array(vec![]))]);
        write(&mut bytes, &j, &beamcraft_protocol::message::read_limits()).unwrap();
        read_timed(
            &mut Cursor::new(&bytes),
            &beamcraft_protocol::message::read_limits(),
            &assembly,
        )
        .unwrap();
        assert!(
            !assembly
                .expired(Instant::now() + Duration::from_secs(11))
                .unwrap()
        );
        struct Held {
            bytes: Cursor<Vec<u8>>,
            started: mpsc::Sender<()>,
            wait: mpsc::Receiver<()>,
            calls: usize,
        }
        impl Read for Held {
            fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
                self.calls += 1;
                if self.calls == 2 {
                    self.started.send(()).unwrap();
                    self.wait.recv().unwrap();
                }
                self.bytes.read(out)
            }
        }
        let (started, noticed) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let clock = assembly.clone();
        let reader = thread::spawn(move || {
            read_timed(
                &mut Held {
                    bytes: Cursor::new(bytes),
                    started,
                    wait,
                    calls: 0,
                },
                &beamcraft_protocol::message::read_limits(),
                &clock,
            )
        });
        noticed.recv_timeout(Duration::from_secs(2)).unwrap();
        // Consuming the previous frame must not retire this incomplete successor's deadline.
        assert!(
            assembly
                .expired(Instant::now() + Duration::from_secs(11))
                .unwrap()
        );
        release.send(()).unwrap();
        reader.join().unwrap().unwrap();
        assert!(
            !assembly
                .expired(Instant::now() + Duration::from_secs(11))
                .unwrap()
        );
    }
}
