// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Bounded RFC 8259 control values. This is not an F04/JCS canonicalizer.

use crate::{Limits, TransportCode};
use std::collections::BTreeMap;

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
    pub fn number(value: f64) -> Result<Self, TransportCode> {
        if !value.is_finite() {
            return Err(TransportCode::Json);
        }
        Ok(Self::Number(value.to_string()))
    }
    /// Requires an object with exactly the named fields.
    pub fn closed(&self, keys: &[&str]) -> Result<(), TransportCode> {
        match self {
            Self::Object(o) if o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)) => {
                Ok(())
            }
            _ => Err(TransportCode::Message),
        }
    }
    /// Gets a mandatory object field.
    pub fn get(&self, key: &str) -> Result<&Self, TransportCode> {
        match self {
            Self::Object(o) => o.get(key).ok_or(TransportCode::Message),
            _ => Err(TransportCode::Message),
        }
    }
    /// Requires a decoded string.
    pub fn string(&self) -> Result<&str, TransportCode> {
        match self {
            Self::String(s) => Ok(s),
            _ => Err(TransportCode::Message),
        }
    }
    /// Requires an unsigned integer token (no fractional or exponent form).
    pub fn unsigned(&self) -> Result<u64, TransportCode> {
        match self {
            Self::Number(s) if s.bytes().all(|b| b.is_ascii_digit()) => {
                s.parse().map_err(|_| TransportCode::Message)
            }
            _ => Err(TransportCode::Message),
        }
    }
    /// Requires a finite binary64 number.
    pub fn float(&self) -> Result<f64, TransportCode> {
        match self {
            Self::Number(s) => s.parse().map_err(|_| TransportCode::Json),
            _ => Err(TransportCode::Message),
        }
    }
    /// Requires a boolean.
    pub fn boolean(&self) -> Result<bool, TransportCode> {
        match self {
            Self::Bool(b) => Ok(*b),
            _ => Err(TransportCode::Message),
        }
    }
    /// Requires an array.
    pub fn array(&self) -> Result<&[Self], TransportCode> {
        match self {
            Self::Array(a) => Ok(a),
            _ => Err(TransportCode::Message),
        }
    }
    /// Encodes and revalidates a bounded control value.
    pub fn encode(&self, limits: &Limits) -> Result<Vec<u8>, TransportCode> {
        let mut out = String::new();
        self.write(&mut out, 0, limits.control_bytes as usize)?;
        decode(out.as_bytes(), limits)?;
        Ok(out.into_bytes())
    }
    fn write(&self, out: &mut String, depth: usize, cap: usize) -> Result<(), TransportCode> {
        if depth > 16 || out.len() > cap {
            return Err(TransportCode::Limit);
        }
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Self::Number(s) => out.push_str(s),
            Self::String(s) => {
                if s.len() > 4096 {
                    return Err(TransportCode::Limit);
                }
                quote(s, out);
            }
            Self::Array(a) => {
                out.push('[');
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.write(out, depth + 1, cap)?;
                }
                out.push(']');
            }
            Self::Object(o) => {
                out.push('{');
                for (i, (k, v)) in o.iter().enumerate() {
                    if k.len() > 4096 {
                        return Err(TransportCode::Limit);
                    }
                    if i > 0 {
                        out.push(',');
                    }
                    quote(k, out);
                    out.push(':');
                    v.write(out, depth + 1, cap)?;
                }
                out.push('}');
            }
        }
        if out.len() > cap {
            return Err(TransportCode::Limit);
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
pub fn decode(bytes: &[u8], limits: &Limits) -> Result<Json, TransportCode> {
    limits.validate()?;
    if bytes.is_empty() || bytes.len() > limits.control_bytes as usize {
        return Err(TransportCode::Limit);
    }
    let source = std::str::from_utf8(bytes).map_err(|_| TransportCode::Json)?;
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
        return Err(TransportCode::Json);
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
    fn eat(&mut self, b: u8) -> Result<(), TransportCode> {
        if self.peek() != Some(b) {
            return Err(TransportCode::Json);
        }
        self.at += 1;
        Ok(())
    }
    fn token(&mut self) -> Result<(), TransportCode> {
        self.tokens = self.tokens.checked_add(1).ok_or(TransportCode::Limit)?;
        self.scratch = self.scratch.checked_add(256).ok_or(TransportCode::Limit)?;
        if self.tokens > self.limits.json_tokens
            || self.scratch > self.limits.scratch_bytes as usize
        {
            return Err(TransportCode::Limit);
        }
        Ok(())
    }
    fn value(&mut self, depth: u32) -> Result<Json, TransportCode> {
        self.space();
        self.token()?;
        match self.peek() {
            Some(b'{' | b'[') => {
                if depth >= self.limits.json_depth {
                    return Err(TransportCode::Limit);
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
                                return Err(TransportCode::Json);
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
            _ => Err(TransportCode::Json),
        }
    }
    fn literal(&mut self, s: &str) -> Result<(), TransportCode> {
        for b in s.bytes() {
            self.eat(b)?;
        }
        Ok(())
    }
    fn hex(&mut self) -> Result<u16, TransportCode> {
        let mut n = 0u16;
        for _ in 0..4 {
            let b = self.peek().ok_or(TransportCode::Json)?;
            let v = (b as char).to_digit(16).ok_or(TransportCode::Json)?;
            n = n * 16 + v as u16;
            self.at += 1;
        }
        Ok(n)
    }
    fn string(&mut self) -> Result<String, TransportCode> {
        self.eat(b'"')?;
        let mut out = String::new();
        let mut size = 0usize;
        loop {
            let b = self.peek().ok_or(TransportCode::Json)?;
            if b == b'"' {
                self.at += 1;
                break;
            }
            let c = if b == b'\\' {
                self.at += 1;
                let escape = self.peek().ok_or(TransportCode::Json)?;
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
                                return Err(TransportCode::Json);
                            }
                            0x10000 + ((u32::from(high) - 0xd800) << 10) + (u32::from(low) - 0xdc00)
                        } else {
                            u32::from(high)
                        };
                        char::from_u32(code).ok_or(TransportCode::Json)?
                    }
                    _ => return Err(TransportCode::Json),
                }
            } else {
                if b < 32 {
                    return Err(TransportCode::Json);
                }
                let c = self
                    .source
                    .get(self.at..)
                    .and_then(|s| s.chars().next())
                    .ok_or(TransportCode::Json)?;
                self.at += c.len_utf8();
                c
            };
            size += c.len_utf8();
            if size > self.limits.string_bytes as usize {
                return Err(TransportCode::Limit);
            }
            if self.build {
                out.push(c);
            }
        }
        self.scratch = self.scratch.checked_add(size).ok_or(TransportCode::Limit)?;
        if self.scratch > self.limits.scratch_bytes as usize {
            return Err(TransportCode::Limit);
        }
        Ok(out)
    }
    fn number(&mut self) -> Result<Json, TransportCode> {
        let start = self.at;
        if self.peek() == Some(b'-') {
            self.at += 1;
        }
        match self.peek() {
            Some(b'0') => self.at += 1,
            Some(b'1'..=b'9') => self.digits()?,
            _ => return Err(TransportCode::Json),
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
        let s = self.source.get(start..self.at).ok_or(TransportCode::Json)?;
        if s.len() > 128 {
            return Err(TransportCode::Limit);
        }
        let number = s.parse::<f64>().map_err(|_| TransportCode::Json)?;
        let mantissa = s.split(['e', 'E']).next().ok_or(TransportCode::Json)?;
        if !number.is_finite()
            || (number == 0.0 && mantissa.bytes().any(|b| matches!(b, b'1'..=b'9')))
        {
            return Err(TransportCode::Json);
        }
        Ok(if self.build {
            Json::Number(s.into())
        } else {
            Json::Null
        })
    }
    fn digits(&mut self) -> Result<(), TransportCode> {
        let start = self.at;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.at += 1;
        }
        if start == self.at {
            Err(TransportCode::Json)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::limits::MAXIMA;
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
            assert!(decode(s.as_bytes(), &MAXIMA).is_err(), "{s}");
        }
        let deep = format!("{}0{}", "[".repeat(17), "]".repeat(17));
        assert_eq!(decode(deep.as_bytes(), &MAXIMA), Err(TransportCode::Limit));
        let j = decode(br#"{"s":"\ud83d\ude00","u":18446744073709551615}"#, &MAXIMA).unwrap();
        assert_eq!(j.get("u").unwrap().unsigned(), Ok(u64::MAX));
        assert_eq!(decode(&j.encode(&MAXIMA).unwrap(), &MAXIMA).unwrap(), j);
    }
}
