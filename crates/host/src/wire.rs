// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Native stream I/O for U04's zero-attachment F05 read profile.

use beamcraft_protocol::{
    FrameHeader, FrameKind, Limits, TransportCode as T,
    json::{self, Json},
};
use std::io::{Read, Write};

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
    let mut bytes = [0u8; 16];
    match reader.read(&mut bytes[..1]) {
        Ok(0) => return Err(T::BackendLost),
        Ok(_) => (),
        Err(_) => return Err(T::BackendLost),
    }
    on_start();
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
