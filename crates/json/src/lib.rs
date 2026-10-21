// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Bounded RFC 8259 JSON values: the machine-readable output of the CLI, which the native UI
//! and the local coordinator read back.
//!
//! [`decode`] scans the complete input before it builds a tree: grammar, nesting depth, token
//! count, decoded string bytes, a conservative scratch reservation, numeric overflow and nonzero
//! underflow, escapes and surrogate pairs are all checked first, and tree construction rejects
//! duplicate decoded keys. [`Json::encode`] revalidates its output under the same limits. The
//! crate performs no I/O. This is not an RFC 8785 (JCS) canonicalizer.

use beamcraft_model::ErrorCode;
use core::fmt;
use std::collections::BTreeMap;

/// Why a JSON value was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Error {
    /// Invalid UTF-8, JSON grammar, number spelling or duplicate decoded keys.
    Syntax,
    /// A byte, depth, token, string or scratch limit was exceeded.
    Limit,
    /// The value does not have the requested shape: a missing field, an unexpected type or an
    /// object that is not exactly the named fields.
    Shape,
}

impl Error {
    /// The F04 code reported for this refusal.
    pub const fn code(self) -> ErrorCode {
        match self {
            Self::Syntax | Self::Shape => ErrorCode::MalformedCommand,
            Self::Limit => ErrorCode::LimitExceeded,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Syntax => "invalid JSON",
            Self::Limit => "JSON limit exceeded",
            Self::Shape => "unexpected JSON shape",
        })
    }
}

impl std::error::Error for Error {}

/// The bounds a value is decoded and encoded under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Bytes of the complete encoded value.
    pub bytes: usize,
    /// Nesting depth of arrays and objects.
    pub depth: u32,
    /// Values and object keys.
    pub tokens: u32,
    /// Bytes of one decoded string or key.
    pub string_bytes: usize,
    /// Parser scratch bytes: 256 per token plus every decoded string byte.
    pub scratch_bytes: usize,
}

impl Limits {
    /// The limits of every CLI reply: 1 MiB, depth 16, 65 536 tokens, 4 KiB strings and 8 MiB
    /// of scratch.
    pub const DEFAULT: Self = Self {
        bytes: 1_048_576,
        depth: 16,
        tokens: 65_536,
        string_bytes: 4_096,
        scratch_bytes: 8_388_608,
    };
}

impl Default for Limits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A bounded control value; number spelling is retained for lossless integer validation.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    /// JSON null.
    Null,
    /// Boolean.
    Bool(bool),
    /// Validated JSON number spelling.
    Number(String),
    /// Decoded UTF-8 string.
    String(String),
    /// Ordered values.
    Array(Vec<Json>),
    /// Unique decoded keys.
    Object(BTreeMap<String, Json>),
}
impl Json {
    /// Constructs a trusted control object.
    pub fn object<const N: usize>(entries: [(&str, Json); N]) -> Self {
        Self::Object(entries.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }
    /// Constructs a string.
    pub fn text(value: impl Into<String>) -> Self {
        Self::String(value.into())
    }
    /// Constructs an unsigned integer token.
    pub fn uint(value: u64) -> Self {
        Self::Number(value.to_string())
    }
    /// Constructs a finite numeric token.
    pub fn number(value: f64) -> Result<Self, Error> {
        if !value.is_finite() {
            return Err(Error::Syntax);
        }
        Ok(Self::Number(value.to_string()))
    }
    /// Requires an object with exactly the named fields.
    pub fn closed(&self, keys: &[&str]) -> Result<(), Error> {
        match self {
            Self::Object(o) if o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)) => {
                Ok(())
            }
            _ => Err(Error::Shape),
        }
    }
    /// Gets a mandatory object field.
    pub fn get(&self, key: &str) -> Result<&Self, Error> {
        match self {
            Self::Object(o) => o.get(key).ok_or(Error::Shape),
            _ => Err(Error::Shape),
        }
    }
    /// Requires a decoded string.
    pub fn string(&self) -> Result<&str, Error> {
        match self {
            Self::String(s) => Ok(s),
            _ => Err(Error::Shape),
        }
    }
    /// Requires an unsigned integer token (no fractional or exponent form).
    pub fn unsigned(&self) -> Result<u64, Error> {
        match self {
            Self::Number(s) if s.bytes().all(|b| b.is_ascii_digit()) => {
                s.parse().map_err(|_| Error::Shape)
            }
            _ => Err(Error::Shape),
        }
    }
    /// Requires a finite binary64 number.
    pub fn float(&self) -> Result<f64, Error> {
        match self {
            Self::Number(s) => s.parse().map_err(|_| Error::Syntax),
            _ => Err(Error::Shape),
        }
    }
    /// Requires a boolean.
    pub fn boolean(&self) -> Result<bool, Error> {
        match self {
            Self::Bool(b) => Ok(*b),
            _ => Err(Error::Shape),
        }
    }
    /// Requires an array.
    pub fn array(&self) -> Result<&[Self], Error> {
        match self {
            Self::Array(a) => Ok(a),
            _ => Err(Error::Shape),
        }
    }
    /// Encodes and revalidates a bounded control value.
    pub fn encode(&self, limits: &Limits) -> Result<Vec<u8>, Error> {
        let mut out = String::new();
        self.write(&mut out, 0, limits)?;
        decode(out.as_bytes(), limits)?;
        Ok(out.into_bytes())
    }
    fn write(&self, out: &mut String, depth: u32, limits: &Limits) -> Result<(), Error> {
        if depth > limits.depth || out.len() > limits.bytes {
            return Err(Error::Limit);
        }
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Self::Number(s) => out.push_str(s),
            Self::String(s) => {
                if s.len() > limits.string_bytes {
                    return Err(Error::Limit);
                }
                quote(s, out);
            }
            Self::Array(a) => {
                out.push('[');
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.write(out, depth + 1, limits)?;
                }
                out.push(']');
            }
            Self::Object(o) => {
                out.push('{');
                for (i, (k, v)) in o.iter().enumerate() {
                    if k.len() > limits.string_bytes {
                        return Err(Error::Limit);
                    }
                    if i > 0 {
                        out.push(',');
                    }
                    quote(k, out);
                    out.push(':');
                    v.write(out, depth + 1, limits)?;
                }
                out.push('}');
            }
        }
        if out.len() > limits.bytes {
            return Err(Error::Limit);
        }
        Ok(())
    }
}
fn quote(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c < ' ' => {
                use std::fmt::Write;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Decodes after a complete allocation-free scan. Depth, token, string and scratch caps
/// precede construction of the object tree; decoded duplicate keys are rejected.
pub fn decode(bytes: &[u8], limits: &Limits) -> Result<Json, Error> {
    if bytes.is_empty() || bytes.len() > limits.bytes {
        return Err(Error::Limit);
    }
    let source = std::str::from_utf8(bytes).map_err(|_| Error::Syntax)?;
    let mut scan = Parser {
        source,
        at: 0,
        tokens: 0,
        scratch: 0,
        limits,
        build: false,
    };
    scan.value(0)?;
    scan.space();
    if scan.at != source.len() {
        return Err(Error::Syntax);
    }
    let mut parser = Parser {
        source,
        at: 0,
        tokens: 0,
        scratch: 0,
        limits,
        build: true,
    };
    parser.value(0)
}
struct Parser<'a> {
    source: &'a str,
    at: usize,
    tokens: u32,
    scratch: usize,
    limits: &'a Limits,
    build: bool,
}
impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.source.as_bytes().get(self.at).copied()
    }
    fn space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.at += 1;
        }
    }
    fn eat(&mut self, b: u8) -> Result<(), Error> {
        if self.peek() != Some(b) {
            return Err(Error::Syntax);
        }
        self.at += 1;
        Ok(())
    }
    fn token(&mut self) -> Result<(), Error> {
        self.tokens = self.tokens.checked_add(1).ok_or(Error::Limit)?;
        self.scratch = self.scratch.checked_add(256).ok_or(Error::Limit)?;
        if self.tokens > self.limits.tokens || self.scratch > self.limits.scratch_bytes {
            return Err(Error::Limit);
        }
        Ok(())
    }
    fn value(&mut self, depth: u32) -> Result<Json, Error> {
        self.space();
        self.token()?;
        match self.peek() {
            Some(b'{' | b'[') => {
                if depth >= self.limits.depth {
                    return Err(Error::Limit);
                }
                let object = self.peek() == Some(b'{');
                self.at += 1;
                self.space();
                let close = if object { b'}' } else { b']' };
                let mut array = Vec::new();
                let mut map = BTreeMap::new();
                if self.peek() == Some(close) {
                    self.at += 1;
                    return Ok(if object {
                        Json::Object(map)
                    } else {
                        Json::Array(array)
                    });
                }
                loop {
                    self.space();
                    let key = if object {
                        self.token()?;
                        let k = self.string()?;
                        self.space();
                        self.eat(b':')?;
                        Some(k)
                    } else {
                        None
                    };
                    let value = self.value(depth + 1)?;
                    if self.build {
                        if let Some(k) = key {
                            if map.insert(k, value).is_some() {
                                return Err(Error::Syntax);
                            }
                        } else {
                            array.push(value);
                        }
                    }
                    self.space();
                    if self.peek() == Some(close) {
                        self.at += 1;
                        break;
                    }
                    self.eat(b',')?;
                }
                Ok(if object {
                    Json::Object(map)
                } else {
                    Json::Array(array)
                })
            }
            Some(b'"') => self.string().map(Json::String),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(b'n') => {
                self.literal("null")?;
                Ok(Json::Null)
            }
            Some(b't') => {
                self.literal("true")?;
                Ok(Json::Bool(true))
            }
            Some(b'f') => {
                self.literal("false")?;
                Ok(Json::Bool(false))
            }
            _ => Err(Error::Syntax),
        }
    }
    fn literal(&mut self, s: &str) -> Result<(), Error> {
        for b in s.bytes() {
            self.eat(b)?;
        }
        Ok(())
    }
    fn hex(&mut self) -> Result<u16, Error> {
        let mut n = 0u16;
        for _ in 0..4 {
            let b = self.peek().ok_or(Error::Syntax)?;
            let v = (b as char).to_digit(16).ok_or(Error::Syntax)?;
            n = n * 16 + v as u16;
            self.at += 1;
        }
        Ok(n)
    }
    fn string(&mut self) -> Result<String, Error> {
        self.eat(b'"')?;
        let mut out = String::new();
        let mut size = 0usize;
        loop {
            let b = self.peek().ok_or(Error::Syntax)?;
            if b == b'"' {
                self.at += 1;
                break;
            }
            let c = if b == b'\\' {
                self.at += 1;
                let escape = self.peek().ok_or(Error::Syntax)?;
                self.at += 1;
                match escape {
                    b'"' => '"',
                    b'\\' => '\\',
                    b'/' => '/',
                    b'b' => '\u{8}',
                    b'f' => '\u{c}',
                    b'n' => '\n',
                    b'r' => '\r',
                    b't' => '\t',
                    b'u' => {
                        let high = self.hex()?;
                        let code = if (0xd800..=0xdbff).contains(&high) {
                            self.eat(b'\\')?;
                            self.eat(b'u')?;
                            let low = self.hex()?;
                            if !(0xdc00..=0xdfff).contains(&low) {
                                return Err(Error::Syntax);
                            }
                            0x10000 + ((u32::from(high) - 0xd800) << 10) + (u32::from(low) - 0xdc00)
                        } else {
                            u32::from(high)
                        };
                        char::from_u32(code).ok_or(Error::Syntax)?
                    }
                    _ => return Err(Error::Syntax),
                }
            } else {
                if b < 32 {
                    return Err(Error::Syntax);
                }
                let c = self
                    .source
                    .get(self.at..)
                    .and_then(|s| s.chars().next())
                    .ok_or(Error::Syntax)?;
                self.at += c.len_utf8();
                c
            };
            size += c.len_utf8();
            if size > self.limits.string_bytes {
                return Err(Error::Limit);
            }
            if self.build {
                out.push(c);
            }
        }
        self.scratch = self.scratch.checked_add(size).ok_or(Error::Limit)?;
        if self.scratch > self.limits.scratch_bytes {
            return Err(Error::Limit);
        }
        Ok(out)
    }
    fn number(&mut self) -> Result<Json, Error> {
        let start = self.at;
        if self.peek() == Some(b'-') {
            self.at += 1;
        }
        match self.peek() {
            Some(b'0') => self.at += 1,
            Some(b'1'..=b'9') => self.digits()?,
            _ => return Err(Error::Syntax),
        }
        if self.peek() == Some(b'.') {
            self.at += 1;
            self.digits()?;
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.at += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.at += 1;
            }
            self.digits()?;
        }
        let s = self.source.get(start..self.at).ok_or(Error::Syntax)?;
        if s.len() > 128 {
            return Err(Error::Limit);
        }
        let number = s.parse::<f64>().map_err(|_| Error::Syntax)?;
        let mantissa = s.split(['e', 'E']).next().ok_or(Error::Syntax)?;
        if !number.is_finite()
            || (number == 0.0 && mantissa.bytes().any(|b| matches!(b, b'1'..=b'9')))
        {
            return Err(Error::Syntax);
        }
        Ok(if self.build {
            Json::Number(s.into())
        } else {
            Json::Null
        })
    }
    fn digits(&mut self) -> Result<(), Error> {
        let start = self.at;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.at += 1;
        }
        if start == self.at {
            Err(Error::Syntax)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hostile_json_and_decoded_duplicates() {
        for s in [
            r#"{"x":1,"\u0078":2}"#,
            r#""\ud800""#,
            r#""\udc00""#,
            "01",
            "1e400",
            "1e-400",
            "NaN",
            "true false",
            "\u{feff}{}",
            "[1,]",
        ] {
            assert!(decode(s.as_bytes(), &Limits::DEFAULT).is_err(), "{s}");
        }
        let deep = format!("{}0{}", "[".repeat(17), "]".repeat(17));
        assert_eq!(decode(deep.as_bytes(), &Limits::DEFAULT), Err(Error::Limit));
        let j = decode(
            br#"{"s":"\ud83d\ude00","u":18446744073709551615}"#,
            &Limits::DEFAULT,
        )
        .unwrap();
        assert_eq!(j.get("u").unwrap().unsigned(), Ok(u64::MAX));
        assert_eq!(
            decode(&j.encode(&Limits::DEFAULT).unwrap(), &Limits::DEFAULT).unwrap(),
            j
        );
    }
}
