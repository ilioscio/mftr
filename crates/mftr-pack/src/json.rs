//! A small, strict JSON reader for the glTF JSON chunk (RFC 8259, no extensions).
//!
//! We parse only what we then check: packs come from untrusted servers and authors
//! (11 §2), so the reader has a nesting limit, rejects duplicate keys and anything malformed,
//! and never guesses.

use std::fmt;

const MAX_DEPTH: usize = 64;

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    /// Keys in document order.
    Obj(Vec<(String, Json)>),
}

#[derive(Debug, PartialEq, Eq)]
pub struct JsonError {
    pub offset: usize,
    pub msg: &'static str,
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid JSON at byte {}: {}", self.offset, self.msg)
    }
}

impl Json {
    pub fn parse(text: &[u8]) -> Result<Json, JsonError> {
        let mut p = Parser { s: text, i: 0 };
        p.ws();
        let v = p.value(0)?;
        p.ws();
        if p.i != p.s.len() {
            return Err(p.err("trailing characters"));
        }
        Ok(v)
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(kv) => kv.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        let kv: &[(String, Json)] = match self {
            Json::Obj(kv) => kv,
            _ => &[],
        };
        kv.iter().map(|(k, _)| k.as_str())
    }

    pub fn as_arr(&self) -> &[Json] {
        match self {
            Json::Arr(a) => a,
            _ => &[],
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }

    /// A non-negative integer that fits in u32.
    pub fn as_index(&self) -> Option<usize> {
        match self {
            Json::Num(n) if *n >= 0.0 && n.fract() == 0.0 && *n <= u32::MAX as f64 => Some(*n as usize),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn err(&self, msg: &'static str) -> JsonError {
        JsonError { offset: self.i, msg }
    }

    fn ws(&mut self) {
        while let Some(b' ' | b'\t' | b'\n' | b'\r') = self.s.get(self.i) {
            self.i += 1;
        }
    }

    fn eat(&mut self, lit: &[u8]) -> bool {
        if self.s[self.i..].starts_with(lit) {
            self.i += lit.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self, depth: usize) -> Result<Json, JsonError> {
        if depth > MAX_DEPTH {
            return Err(self.err("nested too deeply"));
        }
        match self.s.get(self.i) {
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b't') if self.eat(b"true") => Ok(Json::Bool(true)),
            Some(b'f') if self.eat(b"false") => Ok(Json::Bool(false)),
            Some(b'n') if self.eat(b"null") => Ok(Json::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(self.err("expected a value")),
        }
    }

    fn object(&mut self, depth: usize) -> Result<Json, JsonError> {
        self.i += 1;
        let mut kv: Vec<(String, Json)> = Vec::new();
        self.ws();
        if self.eat(b"}") {
            return Ok(Json::Obj(kv));
        }
        loop {
            self.ws();
            if self.s.get(self.i) != Some(&b'"') {
                return Err(self.err("expected a key"));
            }
            let k = self.string()?;
            if kv.iter().any(|(e, _)| *e == k) {
                return Err(self.err("duplicate key"));
            }
            self.ws();
            if !self.eat(b":") {
                return Err(self.err("expected ':'"));
            }
            self.ws();
            let v = self.value(depth + 1)?;
            kv.push((k, v));
            self.ws();
            if self.eat(b",") {
                continue;
            }
            if self.eat(b"}") {
                return Ok(Json::Obj(kv));
            }
            return Err(self.err("expected ',' or '}'"));
        }
    }

    fn array(&mut self, depth: usize) -> Result<Json, JsonError> {
        self.i += 1;
        let mut out = Vec::new();
        self.ws();
        if self.eat(b"]") {
            return Ok(Json::Arr(out));
        }
        loop {
            self.ws();
            out.push(self.value(depth + 1)?);
            self.ws();
            if self.eat(b",") {
                continue;
            }
            if self.eat(b"]") {
                return Ok(Json::Arr(out));
            }
            return Err(self.err("expected ',' or ']'"));
        }
    }

    fn string(&mut self) -> Result<String, JsonError> {
        self.i += 1;
        let mut out = String::new();
        loop {
            let start = self.i;
            while let Some(&b) = self.s.get(self.i) {
                if b == b'"' || b == b'\\' || b < 0x20 {
                    break;
                }
                self.i += 1;
            }
            out.push_str(std::str::from_utf8(&self.s[start..self.i]).map_err(|_| self.err("invalid UTF-8"))?);
            match self.s.get(self.i) {
                Some(b'"') => {
                    self.i += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.i += 1;
                    let c = self.escape()?;
                    out.push(c);
                }
                _ => return Err(self.err("unterminated string")),
            }
        }
    }

    /// After a backslash: one escape, consuming exactly its bytes.
    fn escape(&mut self) -> Result<char, JsonError> {
        let simple = match self.s.get(self.i) {
            Some(b'"') => '"',
            Some(b'\\') => '\\',
            Some(b'/') => '/',
            Some(b'b') => '\u{8}',
            Some(b'f') => '\u{c}',
            Some(b'n') => '\n',
            Some(b'r') => '\r',
            Some(b't') => '\t',
            Some(b'u') => {
                let hi = self.hex4()?;
                if !(0xD800..0xDC00).contains(&hi) {
                    return char::from_u32(hi).ok_or(self.err("unpaired surrogate"));
                }
                if !self.eat(b"\\") || self.s.get(self.i) != Some(&b'u') {
                    return Err(self.err("unpaired surrogate"));
                }
                let lo = self.hex4()?;
                if !(0xDC00..0xE000).contains(&lo) {
                    return Err(self.err("unpaired surrogate"));
                }
                return char::from_u32(0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00))
                    .ok_or(self.err("invalid escape"));
            }
            _ => return Err(self.err("invalid escape")),
        };
        self.i += 1;
        Ok(simple)
    }

    /// Positioned on the 'u' of a `\uXXXX` escape; consumes all five bytes.
    fn hex4(&mut self) -> Result<u32, JsonError> {
        let digits = self.s.get(self.i + 1..self.i + 5).ok_or(self.err("short unicode escape"))?;
        if !digits.iter().all(u8::is_ascii_hexdigit) {
            return Err(self.err("bad unicode escape"));
        }
        let mut v = 0u32;
        for d in digits {
            v = v * 16 + (*d as char).to_digit(16).unwrap_or(0);
        }
        self.i += 5;
        Ok(v)
    }

    fn number(&mut self) -> Result<Json, JsonError> {
        let start = self.i;
        self.eat(b"-");
        match self.s.get(self.i) {
            Some(b'0') => self.i += 1,
            Some(b'1'..=b'9') => self.digits(),
            _ => return Err(self.err("bad number")),
        }
        if self.eat(b".") {
            if !matches!(self.s.get(self.i), Some(b'0'..=b'9')) {
                return Err(self.err("bad number"));
            }
            self.digits();
        }
        if let Some(b'e' | b'E') = self.s.get(self.i) {
            self.i += 1;
            if let Some(b'+' | b'-') = self.s.get(self.i) {
                self.i += 1;
            }
            if !matches!(self.s.get(self.i), Some(b'0'..=b'9')) {
                return Err(self.err("bad number"));
            }
            self.digits();
        }
        let text = std::str::from_utf8(&self.s[start..self.i]).map_err(|_| self.err("bad number"))?;
        let n: f64 = text.parse().map_err(|_| self.err("bad number"))?;
        if !n.is_finite() {
            return Err(self.err("number out of range"));
        }
        Ok(Json::Num(n))
    }

    fn digits(&mut self) {
        while let Some(b'0'..=b'9') = self.s.get(self.i) {
            self.i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_documents() {
        let v = Json::parse(br#" {"a": [1, -2.5e3, true, null], "b": {"c": "x\u00e9\n"}} "#).unwrap();
        assert_eq!(v.get("a").unwrap().as_arr().len(), 4);
        assert_eq!(v.get("a").unwrap().as_arr()[1].as_f64(), Some(-2500.0));
        assert_eq!(v.get("b").unwrap().get("c").unwrap().as_str(), Some("x\u{e9}\n"));
        assert_eq!(Json::parse(br#""\ud83d\ude00""#).unwrap(), Json::Str("\u{1F600}".into()));
    }

    #[test]
    fn rejects_malformed_input() {
        for bad in [
            &b"{\"a\":1,}"[..],
            b"{\"a\":1,\"a\":2}",
            b"[1 2]",
            b"01",
            b"1.",
            b"\"\\x\"",
            b"\"\\ud800\"",
            b"\"abc",
            b"{} x",
            b"nul",
            b"1e999",
        ] {
            assert!(Json::parse(bad).is_err(), "{}", String::from_utf8_lossy(bad));
        }
        let deep = "[".repeat(100) + &"]".repeat(100);
        assert!(Json::parse(deep.as_bytes()).is_err());
    }
}
