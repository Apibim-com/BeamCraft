// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! A bounded JSON reader and a deterministic JSON writer.
//!
//! The reader follows the RFC 8259 grammar strictly and treats every input as untrusted: input
//! size, nesting depth, value count, member count, string length and number length are capped.
//! Duplicate member names are rejected. `NaN`/`Infinity` tokens and numbers that overflow `f64`
//! are reported separately as non-finite numbers.
//!
//! The writer emits two-space indentation, LF line endings and a trailing newline, so the same
//! value always produces the same bytes on every platform.

use std::fmt::Write as _;

/// A parsed JSON value. Object members keep their document order.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A finite number.
    Number(f64),
    /// A string.
    String(String),
    /// An array.
    Array(Vec<Json>),
    /// An object, as name/value pairs in document order. Names are unique.
    Object(Vec<(String, Json)>),
}

impl Json {
    /// The member called `name`, if this is an object that has it.
    pub fn get(&self, name: &str) -> Option<&Json> {
        self.as_object()?
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v)
    }

    /// The string value, if this is a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::String(s) => Some(s),
            _ => None,
        }
    }

    /// The numeric value, if this is a number.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// The elements, if this is an array.
    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Array(a) => Some(a),
            _ => None,
        }
    }

    /// The members, if this is an object.
    pub fn as_object(&self) -> Option<&[(String, Json)]> {
        match self {
            Json::Object(m) => Some(m),
            _ => None,
        }
    }
}

/// Limits applied while parsing untrusted JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseLimits {
    /// Maximum input size in bytes.
    pub max_bytes: usize,
    /// Maximum nesting depth of arrays and objects (the root container has depth 1).
    pub max_depth: usize,
    /// Maximum number of values in the whole document.
    pub max_values: usize,
    /// Maximum decoded length of one string, in bytes.
    pub max_string_bytes: usize,
    /// Maximum length of one number literal, in characters.
    pub max_number_chars: usize,
    /// Maximum number of members in one object.
    pub max_members: usize,
}

/// Why a JSON document was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The bytes are not a valid JSON document (including invalid UTF-8 and duplicate names).
    Malformed {
        /// Byte offset where the problem was found.
        offset: usize,
        /// Human-readable reason.
        reason: &'static str,
    },
    /// A `NaN`/`Infinity` token, or a number literal that does not fit a finite `f64`.
    NonFinite {
        /// Byte offset of the number.
        offset: usize,
    },
    /// A [`ParseLimits`] bound was exceeded.
    Limit {
        /// Which limit.
        what: &'static str,
    },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Malformed { offset, reason } => write!(f, "{reason} at byte {offset}"),
            ParseError::NonFinite { offset } => write!(f, "non-finite number at byte {offset}"),
            ParseError::Limit { what } => write!(f, "{what} limit exceeded"),
        }
    }
}

/// Parses one JSON document from `bytes` within `limits`.
pub fn parse(bytes: &[u8], limits: &ParseLimits) -> Result<Json, ParseError> {
    if bytes.len() > limits.max_bytes {
        return Err(ParseError::Limit { what: "input size" });
    }
    if let Err(e) = std::str::from_utf8(bytes) {
        return Err(ParseError::Malformed {
            offset: e.valid_up_to(),
            reason: "invalid UTF-8",
        });
    }
    let mut parser = Parser {
        bytes,
        pos: 0,
        values: 0,
        limits,
    };
    let value = parser.value(0)?;
    parser.skip_ws();
    if parser.pos != bytes.len() {
        return Err(parser.malformed("trailing data after the document"));
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
    values: usize,
    limits: &'a ParseLimits,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn bump(&mut self) {
        self.pos = self.pos.saturating_add(1);
    }

    fn malformed(&self, reason: &'static str) -> ParseError {
        ParseError::Malformed {
            offset: self.pos,
            reason,
        }
    }

    fn skip_ws(&mut self) {
        while let Some(b' ' | b'\t' | b'\n' | b'\r') = self.peek() {
            self.bump();
        }
    }

    fn starts_with(&self, literal: &[u8]) -> bool {
        self.bytes
            .get(self.pos..)
            .is_some_and(|rest| rest.starts_with(literal))
    }

    fn literal(&mut self, literal: &[u8], value: Json) -> Result<Json, ParseError> {
        if self.starts_with(literal) {
            self.pos = self.pos.saturating_add(literal.len());
            Ok(value)
        } else {
            Err(self.malformed("invalid literal"))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Json, ParseError> {
        self.values = self.values.saturating_add(1);
        if self.values > self.limits.max_values {
            return Err(ParseError::Limit {
                what: "value count",
            });
        }
        self.skip_ws();
        match self.peek() {
            None => Err(self.malformed("unexpected end of input")),
            Some(b'{') => self.object(depth.saturating_add(1)),
            Some(b'[') => self.array(depth.saturating_add(1)),
            Some(b'"') => self.string().map(Json::String),
            Some(b't') => self.literal(b"true", Json::Bool(true)),
            Some(b'f') => self.literal(b"false", Json::Bool(false)),
            Some(b'n') => self.literal(b"null", Json::Null),
            Some(b'N' | b'I') | Some(b'-')
                if self.starts_with(b"NaN")
                    || self.starts_with(b"Infinity")
                    || self.starts_with(b"-Infinity") =>
            {
                Err(ParseError::NonFinite { offset: self.pos })
            }
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => Err(self.malformed("unexpected character")),
        }
    }

    fn object(&mut self, depth: usize) -> Result<Json, ParseError> {
        if depth > self.limits.max_depth {
            return Err(ParseError::Limit {
                what: "nesting depth",
            });
        }
        self.bump();
        let mut members: Vec<(String, Json)> = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.bump();
            return Ok(Json::Object(members));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(self.malformed("expected a member name"));
            }
            let name_offset = self.pos;
            let name = self.string()?;
            if members.iter().any(|(n, _)| *n == name) {
                return Err(ParseError::Malformed {
                    offset: name_offset,
                    reason: "duplicate member name",
                });
            }
            self.skip_ws();
            if self.peek() != Some(b':') {
                return Err(self.malformed("expected ':'"));
            }
            self.bump();
            let value = self.value(depth)?;
            if members.len() >= self.limits.max_members {
                return Err(ParseError::Limit {
                    what: "object member count",
                });
            }
            members.push((name, value));
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.bump(),
                Some(b'}') => {
                    self.bump();
                    return Ok(Json::Object(members));
                }
                None => return Err(self.malformed("unexpected end of input")),
                Some(_) => return Err(self.malformed("expected ',' or '}'")),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Json, ParseError> {
        if depth > self.limits.max_depth {
            return Err(ParseError::Limit {
                what: "nesting depth",
            });
        }
        self.bump();
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.bump();
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.value(depth)?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.bump(),
                Some(b']') => {
                    self.bump();
                    return Ok(Json::Array(items));
                }
                None => return Err(self.malformed("unexpected end of input")),
                Some(_) => return Err(self.malformed("expected ',' or ']'")),
            }
        }
    }

    fn string(&mut self) -> Result<String, ParseError> {
        self.bump();
        let mut out: Vec<u8> = Vec::new();
        loop {
            match self.peek() {
                None => return Err(self.malformed("unterminated string")),
                Some(b'"') => {
                    self.bump();
                    break;
                }
                Some(b'\\') => {
                    self.bump();
                    self.escape(&mut out)?;
                }
                Some(0x00..=0x1F) => return Err(self.malformed("control character in string")),
                Some(b) => {
                    out.push(b);
                    self.bump();
                }
            }
            if out.len() > self.limits.max_string_bytes {
                return Err(ParseError::Limit {
                    what: "string length",
                });
            }
        }
        // The input was validated as UTF-8 and escapes only append whole scalar values.
        String::from_utf8(out).map_err(|_| self.malformed("invalid UTF-8 in string"))
    }

    fn escape(&mut self, out: &mut Vec<u8>) -> Result<(), ParseError> {
        let simple = match self.peek() {
            Some(b'"') => Some(b'"'),
            Some(b'\\') => Some(b'\\'),
            Some(b'/') => Some(b'/'),
            Some(b'b') => Some(0x08),
            Some(b'f') => Some(0x0C),
            Some(b'n') => Some(b'\n'),
            Some(b'r') => Some(b'\r'),
            Some(b't') => Some(b'\t'),
            Some(b'u') => None,
            _ => return Err(self.malformed("invalid escape")),
        };
        self.bump();
        if let Some(b) = simple {
            out.push(b);
            return Ok(());
        }
        let first = self.hex4()?;
        let scalar = match first {
            0xD800..=0xDBFF => {
                if !self.starts_with(b"\\u") {
                    return Err(self.malformed("unpaired surrogate"));
                }
                self.pos = self.pos.saturating_add(2);
                let second = self.hex4()?;
                if !(0xDC00..=0xDFFF).contains(&second) {
                    return Err(self.malformed("unpaired surrogate"));
                }
                0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00)
            }
            0xDC00..=0xDFFF => return Err(self.malformed("unpaired surrogate")),
            other => other,
        };
        let ch = char::from_u32(scalar).ok_or_else(|| self.malformed("invalid escape"))?;
        let mut buf = [0u8; 4];
        out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
        Ok(())
    }

    fn hex4(&mut self) -> Result<u32, ParseError> {
        let mut value = 0u32;
        for _ in 0..4 {
            let digit = self
                .peek()
                .and_then(|b| char::from(b).to_digit(16))
                .ok_or_else(|| self.malformed("invalid \\u escape"))?;
            value = (value << 4) | digit;
            self.bump();
        }
        Ok(value)
    }

    fn digits(&mut self) -> usize {
        let mut count = 0usize;
        while let Some(b'0'..=b'9') = self.peek() {
            self.bump();
            count = count.saturating_add(1);
        }
        count
    }

    fn number(&mut self) -> Result<Json, ParseError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.bump();
        }
        match self.peek() {
            Some(b'0') => self.bump(),
            Some(b'1'..=b'9') => {
                self.digits();
            }
            _ => return Err(self.malformed("invalid number")),
        }
        if self.peek() == Some(b'.') {
            self.bump();
            if self.digits() == 0 {
                return Err(self.malformed("invalid number"));
            }
        }
        if let Some(b'e' | b'E') = self.peek() {
            self.bump();
            if let Some(b'+' | b'-') = self.peek() {
                self.bump();
            }
            if self.digits() == 0 {
                return Err(self.malformed("invalid number"));
            }
        }
        if self.pos.saturating_sub(start) > self.limits.max_number_chars {
            return Err(ParseError::Limit {
                what: "number length",
            });
        }
        let text = self
            .bytes
            .get(start..self.pos)
            .and_then(|b| std::str::from_utf8(b).ok())
            .ok_or_else(|| self.malformed("invalid number"))?;
        let value: f64 = text.parse().map_err(|_| self.malformed("invalid number"))?;
        if !value.is_finite() {
            return Err(ParseError::NonFinite { offset: start });
        }
        Ok(Json::Number(value))
    }
}

/// Renders `value` as deterministic, pretty-printed JSON with a trailing newline.
///
/// Arrays that contain only scalars are written on one line. Non-finite numbers cannot be
/// represented in JSON and are written as `null`; callers must not construct them.
pub fn to_pretty_string(value: &Json) -> String {
    let mut out = String::new();
    write_value(value, 0, &mut out);
    out.push('\n');
    out
}

fn is_scalar(value: &Json) -> bool {
    !matches!(value, Json::Array(_) | Json::Object(_))
}

fn indent(level: usize, out: &mut String) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

fn write_value(value: &Json, level: usize, out: &mut String) {
    match value {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Json::Number(n) => write_number(*n, out),
        Json::String(s) => write_string(s, out),
        Json::Array(items) if items.is_empty() => out.push_str("[]"),
        Json::Array(items) if items.iter().all(is_scalar) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write_value(item, level, out);
            }
            out.push(']');
        }
        Json::Array(items) => {
            out.push_str("[\n");
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(",\n");
                }
                indent(level.saturating_add(1), out);
                write_value(item, level.saturating_add(1), out);
            }
            out.push('\n');
            indent(level, out);
            out.push(']');
        }
        Json::Object(members) if members.is_empty() => out.push_str("{}"),
        Json::Object(members) => {
            out.push_str("{\n");
            for (i, (name, item)) in members.iter().enumerate() {
                if i > 0 {
                    out.push_str(",\n");
                }
                indent(level.saturating_add(1), out);
                write_string(name, out);
                out.push_str(": ");
                write_value(item, level.saturating_add(1), out);
            }
            out.push('\n');
            indent(level, out);
            out.push('}');
        }
    }
}

fn write_number(n: f64, out: &mut String) {
    if !n.is_finite() {
        out.push_str("null");
    } else if n == 0.0 {
        // Avoids "-0" for negative zero.
        out.push('0');
    } else {
        // `Display` for f64 is the shortest round-trip decimal and never uses an exponent.
        let _ = write!(out, "{n}");
    }
}

fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: ParseLimits = ParseLimits {
        max_bytes: 4096,
        max_depth: 4,
        max_values: 64,
        max_string_bytes: 16,
        max_number_chars: 24,
        max_members: 8,
    };

    fn p(text: &str) -> Result<Json, ParseError> {
        parse(text.as_bytes(), &LIMITS)
    }

    fn malformed(r: Result<Json, ParseError>) -> bool {
        matches!(r, Err(ParseError::Malformed { .. }))
    }

    #[test]
    fn parses_a_document() {
        let v = p(r#" {"a": [1, -2.5e2, true, null], "b": "x\u00e9\ud83d\ude00"} "#).unwrap();
        assert_eq!(v.get("a").unwrap().as_array().unwrap().len(), 4);
        assert_eq!(
            v.get("a").unwrap().as_array().unwrap()[1].as_f64(),
            Some(-250.0)
        );
        assert_eq!(v.get("b").unwrap().as_str(), Some("xé😀"));
    }

    #[test]
    fn rejects_malformed_syntax() {
        for bad in [
            "",
            "{",
            "[1,]",
            "{\"a\":1,}",
            "01",
            "1.",
            "1e",
            "-",
            "+1",
            "'a'",
            "tru",
            "[1 2]",
            "{\"a\" 1}",
            "{} {}",
            "\"\\x\"",
            "\"\\ud800\"",
            "\"\\udc00\"",
            "\"a\nb\"",
            "\u{feff}{}",
        ] {
            assert!(malformed(p(bad)), "accepted {bad:?}");
        }
    }

    #[test]
    fn rejects_duplicate_member_names() {
        assert!(malformed(p(r#"{"a": 1, "a": 2}"#)));
    }

    #[test]
    fn rejects_invalid_utf8() {
        assert!(malformed(parse(b"\"\xC3\x28\"", &LIMITS)));
    }

    #[test]
    fn reports_non_finite_numbers() {
        for bad in ["NaN", "Infinity", "-Infinity", "[1e400]", "-1e309"] {
            assert!(
                matches!(p(bad), Err(ParseError::NonFinite { .. })),
                "{bad:?}"
            );
        }
        assert_eq!(p("1e-400").unwrap().as_f64(), Some(0.0));
    }

    #[test]
    fn enforces_limits() {
        let limit = |r: Result<Json, ParseError>| matches!(r, Err(ParseError::Limit { .. }));
        assert!(limit(p("[[[[[]]]]]")));
        assert!(p("[[[[]]]]").is_ok());
        assert!(limit(p("\"abcdefghijklmnopq\"")));
        assert!(limit(p("1234567890123456789012345")));
        assert!(limit(p(
            r#"{"a":1,"b":2,"c":3,"d":4,"e":5,"f":6,"g":7,"h":8,"i":9}"#
        )));
        let many = format!("[{}]", vec!["0"; 70].join(","));
        assert!(limit(p(&many)));
        assert!(limit(parse(&[b' '; 4097], &LIMITS)));
    }

    #[test]
    fn writer_is_deterministic_and_round_trips() {
        let value = Json::Object(vec![
            (
                "n".into(),
                Json::Array(vec![
                    Json::Number(640.0),
                    Json::Number(-0.0),
                    Json::Number(0.25),
                ]),
            ),
            ("s".into(), Json::String("q\"\\\n\u{1}".into())),
            (
                "o".into(),
                Json::Array(vec![Json::Object(vec![]), Json::Array(vec![])]),
            ),
        ]);
        let text = to_pretty_string(&value);
        assert_eq!(
            text,
            "{\n  \"n\": [640, 0, 0.25],\n  \"s\": \"q\\\"\\\\\\n\\u0001\",\n  \"o\": [\n    {},\n    []\n  ]\n}\n"
        );
        let back = p(&text).unwrap();
        assert_eq!(back.get("s"), value.get("s"));
        assert_eq!(
            back.get("n").unwrap().as_array().unwrap()[1].as_f64(),
            Some(0.0)
        );
    }
}
