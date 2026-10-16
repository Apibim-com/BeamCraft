// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Independent admission checks. These bounds are deliberately smaller than F11's proposal.

use crate::{Budgets, Control, Diagnostic, corrupt, fail, limit, unsupported};
use beamcraft_model::error::ErrorCode;

pub(crate) fn name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 256
        && s.is_ascii()
        && s.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
        })
        && (s.ends_with(".usda") || s.ends_with(".usdc"))
}

pub(crate) fn anchored(asset: &str, parent: &str) -> Result<String, Diagnostic> {
    if !name(asset) {
        return Err(fail(
            ErrorCode::PermissionDenied,
            "asset identifier is not contained",
        ));
    }
    let result = if let Some((directory, _)) = parent.rsplit_once('/') {
        format!("{directory}/{asset}")
    } else {
        asset.to_owned()
    };
    if !name(&result) {
        return Err(limit());
    }
    Ok(result)
}

pub(crate) fn text(bytes: &[u8], b: Budgets, control: &Control) -> Result<(), Diagnostic> {
    let text = std::str::from_utf8(bytes).map_err(|_| corrupt())?;
    if !text.starts_with("#usda 1.0\n") && !text.starts_with("#usda 1.0\r\n") {
        return Err(corrupt());
    }
    let mut i = 0usize;
    let mut delimiters = Vec::new();
    let mut counts = Vec::new();
    let mut tokens = 0usize;
    let mut prims = 0usize;
    while let Some(&ch) = bytes.get(i) {
        control.check()?;
        if ch.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if ch == b'#' {
            while bytes.get(i).is_some_and(|c| *c != b'\n') {
                i += 1;
            }
            continue;
        }
        tokens += 1;
        // A field entails multiple tokens; this is a pre-parser work ceiling, not a field count.
        if tokens > b.values.saturating_mul(16) {
            return Err(limit());
        }
        // Count actual leaf tokens in every enclosing array, including tuple scalars.
        // Delimiters/separators themselves do not allocate array elements.
        if !b"{}[]()=,;".contains(&ch) {
            for (delimiter, count) in delimiters.iter().zip(&mut counts) {
                if *delimiter == b'[' {
                    *count += 1usize;
                    if *count > b.array_items {
                        return Err(limit());
                    }
                }
            }
        }
        match ch {
            b'\'' => return Err(unsupported()),
            b'"' | b'@' | b'<' => {
                let close = if ch == b'<' { b'>' } else { ch };
                i += 1;
                let start = i;
                if ch == b'"' && bytes.get(i..i.saturating_add(2)) == Some(b"\"\"") {
                    return Err(unsupported());
                }
                loop {
                    let c = bytes.get(i).copied().ok_or_else(corrupt)?;
                    if c == close {
                        break;
                    }
                    if c == b'\\' && ch == b'"' {
                        i = i.checked_add(1).ok_or_else(limit)?;
                    }
                    i = i.checked_add(1).ok_or_else(limit)?;
                    if i.saturating_sub(start) > 1024 {
                        return Err(limit());
                    }
                }
                if ch == b'@' && !name(text.get(start..i).ok_or_else(corrupt)?) {
                    return Err(fail(
                        ErrorCode::PermissionDenied,
                        "asset identifier is not contained",
                    ));
                }
                i += 1;
            }
            b'{' | b'[' | b'(' => {
                if delimiters.len() >= b.depth {
                    return Err(limit());
                }
                delimiters.push(ch);
                counts.push(0usize);
                i += 1;
            }
            b'}' | b']' | b')' => {
                let expected = match ch {
                    b'}' => b'{',
                    b']' => b'[',
                    _ => b'(',
                };
                if delimiters.pop() != Some(expected) {
                    return Err(corrupt());
                }
                counts.pop();
                i += 1;
            }
            b',' | b'=' | b';' => {
                i += 1;
            }
            _ => {
                let start = i;
                while bytes
                    .get(i)
                    .is_some_and(|c| !c.is_ascii_whitespace() && !b"{}[]()=,;\"@<>#".contains(c))
                {
                    i += 1;
                }
                if start == i {
                    return Err(corrupt());
                }
                let token = text.get(start..i).ok_or_else(corrupt)?;
                if matches!(token, "def" | "over" | "class") {
                    prims += 1;
                    if prims > b.prims {
                        return Err(limit());
                    }
                }
                if token.len() > 256 {
                    return Err(limit());
                }
                if matches!(
                    token,
                    "nan" | "NaN" | "inf" | "-inf" | "+inf" | "Infinity" | "-Infinity"
                ) {
                    return Err(fail(ErrorCode::NonFiniteNumber, "non-finite numeric value"));
                }
                if token
                    .bytes()
                    .next()
                    .is_some_and(|c| c.is_ascii_digit() || matches!(c, b'+' | b'-' | b'.'))
                {
                    let n: f64 = token.parse().map_err(|_| corrupt())?;
                    if !n.is_finite() {
                        return Err(fail(ErrorCode::NonFiniteNumber, "non-finite numeric value"));
                    }
                    if n == 0.0
                        && token
                            .split(['e', 'E'])
                            .next()
                            .is_some_and(|m| m.bytes().any(|c| matches!(c, b'1'..=b'9')))
                    {
                        return Err(fail(
                            ErrorCode::NonFiniteNumber,
                            "nonzero numeric value underflows binary64",
                        ));
                    }
                }
            }
        }
    }
    if !delimiters.is_empty() {
        return Err(corrupt());
    }
    Ok(())
}

struct Bytes<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl<'a> Bytes<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Diagnostic> {
        let end = self.pos.checked_add(n).ok_or_else(limit)?;
        let slice = self.bytes.get(self.pos..end).ok_or_else(corrupt)?;
        self.pos = end;
        Ok(slice)
    }
    fn u64(&mut self) -> Result<usize, Diagnostic> {
        let n = u64::from_le_bytes(self.take(8)?.try_into().map_err(|_| corrupt())?);
        usize::try_from(n).map_err(|_| limit())
    }
    fn block(&mut self) -> Result<(), Diagnostic> {
        let size = self.u64()?;
        self.take(size)?;
        Ok(())
    }
}

pub(crate) fn charge(total: &mut usize, n: usize, max: usize) -> Result<(), Diagnostic> {
    *total = total
        .checked_add(n)
        .filter(|n| *n <= max)
        .ok_or_else(limit)?;
    Ok(())
}

/// Before CrateFile::open: table bounds, counts and decompression destination estimates.
pub(crate) fn binary(bytes: &[u8], b: Budgets, decoded: &mut usize) -> Result<(), Diagnostic> {
    if bytes.get(..8) != Some(b"PXR-USDC") {
        return Err(corrupt());
    }
    // This exact restricted profile uses 64-bit array lengths (crate >= 0.8).
    if !matches!(bytes.get(8..11), Some([0, 8..=12, 0])) {
        return Err(unsupported());
    }
    let mut header = Bytes { bytes, pos: 16 };
    let toc = header.u64()?;
    let mut table = Bytes { bytes, pos: toc };
    let n = table.u64()?;
    if n > 6 {
        return Err(limit());
    }
    if n != 6 {
        return Err(corrupt());
    }
    let mut spans = Vec::new();
    let mut names = std::collections::BTreeSet::new();
    for _ in 0..n {
        let label = table.take(16)?;
        let label = label.split(|c| *c == 0).next().ok_or_else(corrupt)?;
        let label = std::str::from_utf8(label).map_err(|_| corrupt())?;
        if !names.insert(label) {
            return Err(corrupt());
        }
        let start = table.u64()?;
        let size = table.u64()?;
        let end = start.checked_add(size).ok_or_else(limit)?;
        if start < 32 || end > toc || spans.iter().any(|(s, e)| start < *e && *s < end) {
            return Err(corrupt());
        }
        spans.push((start, end));
        let slice = bytes.get(start..end).ok_or_else(corrupt)?;
        let mut section = Bytes {
            bytes: slice,
            pos: 0,
        };
        let count = section.u64()?;
        if count > b.values {
            return Err(limit());
        }
        charge(
            decoded,
            count.checked_mul(1024).ok_or_else(limit)?,
            b.decoded_bytes,
        )?;
        match label {
            "TOKENS" => {
                let expanded = section.u64()?;
                if count > expanded || expanded > b.decoded_bytes {
                    return Err(limit());
                }
                charge(decoded, expanded, b.decoded_bytes)?;
                section.block()?;
            }
            "STRINGS" => {
                section.take(count.checked_mul(4).ok_or_else(limit)?)?;
            }
            "FIELDS" => {
                section.block()?;
                section.block()?;
            }
            "FIELDSETS" => {
                section.block()?;
            }
            "PATHS" => {
                let encoded = section.u64()?;
                if encoded > b.values || count.checked_sub(encoded).is_none_or(|n| n > 1) {
                    return Err(limit());
                }
                for _ in 0..3 {
                    section.block()?;
                }
            }
            "SPECS" => {
                for _ in 0..3 {
                    section.block()?;
                }
            }
            _ => return Err(unsupported()),
        }
        if section.pos != slice.len() {
            return Err(corrupt());
        }
    }
    if table.pos != bytes.len() {
        return Err(corrupt());
    }
    Ok(())
}

/// Before any lazy value decode: refuse opaque nested/compressed encodings in this profile.
pub(crate) fn binary_values(
    file: &openusd::usdc::CrateFile,
    bytes: &[u8],
    b: Budgets,
    decoded: &mut usize,
) -> Result<(), Diagnostic> {
    for field in &file.fields {
        let rep = field.value_rep;
        let ty = rep.ty().map_err(|_| corrupt())? as u32;
        // No container recursion or reference custom dictionaries are decoded from USDC yet.
        if rep.is_compressed() {
            return Err(unsupported());
        }
        let vector = rep.is_array() || matches!(ty, 40 | 41 | 50);
        if vector {
            if rep.is_inlined() {
                return Err(corrupt());
            }
            if !matches!(ty, 1..=12 | 19..=30 | 40 | 41 | 50) {
                return Err(unsupported());
            }
            let offset = usize::try_from(rep.payload()).map_err(|_| limit())?;
            let mut cursor = Bytes { bytes, pos: offset };
            // Empty array rep payload zero is upstream's empty-array encoding.
            let count = if offset == 0 && rep.is_array() {
                0
            } else {
                cursor.u64()?
            };
            if count > b.array_items {
                return Err(limit());
            }
            charge(
                decoded,
                count.checked_mul(1024).ok_or_else(limit)?,
                b.decoded_bytes,
            )?;
        } else if !matches!(ty, 1..=30 | 42..=44 | 51) {
            // Empty inline dictionary is bounded and used by some independent writers.
            if !(ty == 31 && rep.is_inlined()) {
                return Err(unsupported());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn count_and_decompression_caps_precede_backend() {
        // Original deliberately incomplete crate: oversized TOC is rejected without allocation.
        let mut bytes = vec![0; 40];
        bytes.get_mut(..8).unwrap().copy_from_slice(b"PXR-USDC");
        bytes.get_mut(8..11).unwrap().copy_from_slice(&[0, 8, 0]);
        bytes
            .get_mut(16..24)
            .unwrap()
            .copy_from_slice(&32u64.to_le_bytes());
        bytes
            .get_mut(32..40)
            .unwrap()
            .copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(
            binary(&bytes, Budgets::default(), &mut 0).unwrap_err().code,
            ErrorCode::LimitExceeded
        );
    }
}
