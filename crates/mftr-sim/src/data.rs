//! Champion data as text (CR1, D8, D73, [20 §11](../../docs/design/20-champion-creator.md)):
//! a strict reader and a writer for the small subset of RON that champion files use. The
//! official champions are files (`champions/<id>.ron`, embedded at build time); recipes from the
//! champion creator carry the same `Champion(...)` kit.
//!
//! The sim stays dependency-free (D15), so this is a hand-written recursive-descent reader. It is
//! strict, because recipes come from other players (11 §2):
//! - unknown, duplicate and missing fields are errors; optional fields default to zero or none;
//! - every number is finite and non-negative, every integer in its type's range;
//! - input, nesting, names and list lengths are capped before anything is allocated for them.
//!
//! Durations are written in whole milliseconds (`*_ms`, `Stun(ms: 1250)`) and become sub-ticks by
//! [`SimDuration::from_millis`]. Numbers are read with Rust's own parser, so `0.65` in a file is
//! exactly the `0.65f32` the old Rust constants held.

use std::collections::BTreeSet;
use std::fmt::{self, Write as _};
use std::sync::Mutex;

use crate::ability::{
    Ability, Beam, Blink, Cc, Damage, DamageKind, Dash, DelayedArea, Effect, LineSkillshot, Lunge, Pulses, RankScaling,
    ReactionClass, Shield, Support, Transforms,
};
use crate::champion::{AttackSpec, ChampionDef, Class, Resource, ResourceKind, Stats};
use crate::items::Passive;
use crate::time::{SUBTICKS_PER_SECOND, SimDuration};
use crate::veil::VeilSpec;

/// The champion file format this reader understands. Format 0 writes the kit inline as the sim's
/// own shapes; the creator's catalog moves (CR6) expand into them.
pub const FORMAT: u64 = 0;
/// A champion file at most this long (bytes).
pub const MAX_INPUT: usize = 64 * 1024;
const MAX_DEPTH: usize = 16;
/// Items in a list (a creator sculpt's offsets, CR9; the input's size bounds the work).
const MAX_ITEMS: usize = 1024;
/// Champion and ability names: 1–32 bytes.
pub const MAX_NAME: usize = 32;
/// The longest duration a champion file may hold (10 minutes).
const MAX_MS: u64 = 600_000;

/// Where reading failed (`abilities[2].effect.radius`, or `line 4, column 9` for syntax) and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataError {
    pub at: String,
    pub msg: String,
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.at, self.msg)
    }
}

impl std::error::Error for DataError {}

fn err<T>(at: &str, msg: impl Into<String>) -> Result<T, DataError> {
    Err(DataError { at: at.to_string(), msg: msg.into() })
}

// --- Syntax -----------------------------------------------------------------------------------

/// A parsed value: numbers keep their text until a field says what type they are. Public so
/// other readers of champion files (the creator's `looks`, `mftr-forge`) share this one.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Num(String),
    Str(String),
    /// A bare identifier: a unit variant (`Magic`, `None`).
    Ident(String),
    /// `Name(a: 1, b: 2)` or `(a: 1)`.
    Fields(Option<String>, Vec<(String, Value)>),
    /// `[a, b]`, or `Name(a, b)` (a tuple variant, which no field uses yet but which reads).
    List(Option<String>, Vec<Value>),
}

struct Parser<'a> {
    src: &'a [u8],
    pos: usize,
    depth: usize,
}

impl Parser<'_> {
    fn here(&self) -> String {
        let before = &self.src[..self.pos.min(self.src.len())];
        let line = before.iter().filter(|&&b| b == b'\n').count() + 1;
        let col = before.iter().rev().take_while(|&&b| b != b'\n').count() + 1;
        format!("line {line}, column {col}")
    }

    fn fail<T>(&self, msg: impl Into<String>) -> Result<T, DataError> {
        err(&self.here(), msg)
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn skip_ws(&mut self) -> Result<(), DataError> {
        loop {
            match self.peek() {
                Some(b' ' | b'\t' | b'\r' | b'\n') => self.pos += 1,
                Some(b'/') if self.src.get(self.pos + 1) == Some(&b'/') => {
                    while self.peek().is_some_and(|b| b != b'\n') {
                        self.pos += 1;
                    }
                }
                Some(b'/') if self.src.get(self.pos + 1) == Some(&b'*') => {
                    self.pos += 2;
                    loop {
                        match self.peek() {
                            None => return self.fail("unclosed comment"),
                            Some(b'*') if self.src.get(self.pos + 1) == Some(&b'/') => {
                                self.pos += 2;
                                break;
                            }
                            _ => self.pos += 1,
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    fn expect(&mut self, b: u8) -> Result<(), DataError> {
        self.skip_ws()?;
        if self.peek() == Some(b) {
            self.pos += 1;
            Ok(())
        } else {
            self.fail(format!("expected `{}`", b as char))
        }
    }

    fn ident(&mut self) -> Option<String> {
        let start = self.pos;
        while self.peek().is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_') {
            self.pos += 1;
        }
        let s = &self.src[start..self.pos];
        if s.is_empty() || s[0].is_ascii_digit() {
            self.pos = start;
            return None;
        }
        Some(String::from_utf8_lossy(s).into_owned())
    }

    fn value(&mut self) -> Result<Value, DataError> {
        self.skip_ws()?;
        match self.peek() {
            None => self.fail("unexpected end of input"),
            Some(b'"') => self.string(),
            Some(b'[') => {
                self.pos += 1;
                let items = self.items(b']')?;
                Ok(Value::List(None, items))
            }
            Some(b'(') => self.body(None),
            Some(b) if b.is_ascii_digit() || b == b'-' || b == b'+' || b == b'.' => self.number(),
            Some(_) => {
                let Some(name) = self.ident() else { return self.fail("expected a value") };
                self.skip_ws()?;
                if self.peek() == Some(b'(') { self.body(Some(name)) } else { Ok(Value::Ident(name)) }
            }
        }
    }

    fn enter(&mut self) -> Result<(), DataError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH { self.fail("nested too deeply") } else { Ok(()) }
    }

    /// After a name, at `(`: named fields if the first thing inside is `ident:`, else items.
    fn body(&mut self, name: Option<String>) -> Result<Value, DataError> {
        self.expect(b'(')?;
        self.skip_ws()?;
        let mark = self.pos;
        let named = self.ident().is_some() && {
            self.skip_ws()?;
            self.peek() == Some(b':')
        };
        self.pos = mark;
        if !named {
            let items = self.items(b')')?;
            return Ok(if items.is_empty() { Value::Fields(name, Vec::new()) } else { Value::List(name, items) });
        }
        self.enter()?;
        let mut fields: Vec<(String, Value)> = Vec::new();
        loop {
            self.skip_ws()?;
            if self.peek() == Some(b')') {
                self.pos += 1;
                break;
            }
            let Some(key) = self.ident() else { return self.fail("expected a field name") };
            if fields.iter().any(|(k, _)| *k == key) {
                return self.fail(format!("`{key}` is given twice"));
            }
            if fields.len() >= MAX_ITEMS {
                return self.fail("too many fields");
            }
            self.expect(b':')?;
            let v = self.value()?;
            fields.push((key, v));
            self.skip_ws()?;
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b')') => {}
                _ => return self.fail("expected `,` or `)`"),
            }
        }
        self.depth -= 1;
        Ok(Value::Fields(name, fields))
    }

    fn items(&mut self, close: u8) -> Result<Vec<Value>, DataError> {
        self.enter()?;
        let mut out = Vec::new();
        loop {
            self.skip_ws()?;
            if self.peek() == Some(close) {
                self.pos += 1;
                break;
            }
            if out.len() >= MAX_ITEMS {
                return self.fail("too many items");
            }
            out.push(self.value()?);
            self.skip_ws()?;
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(c) if c == close => {}
                _ => return self.fail(format!("expected `,` or `{}`", close as char)),
            }
        }
        self.depth -= 1;
        Ok(out)
    }

    fn number(&mut self) -> Result<Value, DataError> {
        let start = self.pos;
        while self.peek().is_some_and(|b| b.is_ascii_digit() || matches!(b, b'-' | b'+' | b'.' | b'e' | b'E' | b'_')) {
            self.pos += 1;
        }
        let text: String = String::from_utf8_lossy(&self.src[start..self.pos]).replace('_', "");
        if text.len() > 32 {
            return self.fail("number too long");
        }
        Ok(Value::Num(text))
    }

    fn string(&mut self) -> Result<Value, DataError> {
        self.pos += 1;
        let mut out = Vec::new();
        loop {
            match self.peek() {
                None | Some(b'\n') => return self.fail("unclosed string"),
                Some(b'"') => {
                    self.pos += 1;
                    break;
                }
                Some(b'\\') => {
                    let c = match self.src.get(self.pos + 1) {
                        Some(b'"') => b'"',
                        Some(b'\\') => b'\\',
                        _ => return self.fail("unsupported escape"),
                    };
                    out.push(c);
                    self.pos += 2;
                }
                Some(b) => {
                    out.push(b);
                    self.pos += 1;
                }
            }
            if out.len() > 4 * MAX_NAME {
                return self.fail("string too long");
            }
        }
        match String::from_utf8(out) {
            Ok(s) => Ok(Value::Str(s)),
            Err(_) => self.fail("string isn't UTF-8"),
        }
    }
}

/// Parse a value (the whole text, capped at [`MAX_INPUT`]).
pub fn parse(text: &str) -> Result<Value, DataError> {
    if text.len() > MAX_INPUT {
        return err("input", format!("longer than {MAX_INPUT} bytes"));
    }
    let mut p = Parser { src: text.as_bytes(), pos: 0, depth: 0 };
    let v = p.value()?;
    p.skip_ws()?;
    if p.pos != p.src.len() {
        return p.fail("unexpected text after the value");
    }
    Ok(v)
}

// --- Typed reading ----------------------------------------------------------------------------

/// A struct's fields, taken one by one; [`Fields::done`] fails on any left over.
pub struct Fields {
    at: String,
    list: Vec<(String, Value)>,
}

impl Fields {
    pub fn of(v: Value, at: &str, want: Option<&str>) -> Result<Fields, DataError> {
        match v {
            Value::Fields(name, list) => {
                match (name.as_deref(), want) {
                    (Some(n), Some(w)) if n != w => return err(at, format!("expected `{w}(...)`, found `{n}(...)`")),
                    (None, Some(w)) => return err(at, format!("expected `{w}(...)`")),
                    (Some(_), None) => return err(at, "expected `(...)` without a name"),
                    _ => {}
                }
                Ok(Fields { at: at.to_string(), list })
            }
            _ => err(at, format!("expected {}", want.map_or("`(...)`".to_string(), |w| format!("`{w}(...)`")))),
        }
    }

    fn path(&self, key: &str) -> String {
        if self.at.is_empty() { key.to_string() } else { format!("{}.{key}", self.at) }
    }

    pub fn take(&mut self, key: &str) -> Option<(Value, String)> {
        let i = self.list.iter().position(|(k, _)| k == key)?;
        let (_, v) = self.list.remove(i);
        Some((v, self.path(key)))
    }

    pub fn req(&mut self, key: &str) -> Result<(Value, String), DataError> {
        match self.take(key) {
            Some(x) => Ok(x),
            None => err(&self.at_or_top(), format!("`{key}` is missing")),
        }
    }

    fn at_or_top(&self) -> String {
        if self.at.is_empty() { "champion".to_string() } else { self.at.clone() }
    }

    pub fn f32(&mut self, key: &str) -> Result<f32, DataError> {
        let (v, at) = self.req(key)?;
        num(v, &at)
    }

    pub fn f32_or(&mut self, key: &str, default: f32) -> Result<f32, DataError> {
        match self.take(key) {
            Some((v, at)) => num(v, &at),
            None => Ok(default),
        }
    }

    pub fn ms(&mut self, key: &str) -> Result<SimDuration, DataError> {
        let (v, at) = self.req(key)?;
        ms(v, &at)
    }

    pub fn ms_or(&mut self, key: &str) -> Result<SimDuration, DataError> {
        match self.take(key) {
            Some((v, at)) => ms(v, &at),
            None => Ok(SimDuration(0)),
        }
    }

    pub fn done(self) -> Result<(), DataError> {
        match self.list.first() {
            Some((k, _)) => err(&self.path(k), "unknown field"),
            None => Ok(()),
        }
    }
}

/// A finite number ≥ 0.
pub fn num(v: Value, at: &str) -> Result<f32, DataError> {
    let Value::Num(t) = v else { return err(at, "expected a number") };
    match t.parse::<f32>() {
        Ok(x) if x.is_finite() && x >= 0.0 => Ok(x),
        Ok(_) => err(at, "must be a finite number ≥ 0"),
        Err(_) => err(at, format!("`{t}` isn't a number")),
    }
}

/// A whole number from 0 to `max`.
pub fn int(v: Value, at: &str, max: u64) -> Result<u64, DataError> {
    let Value::Num(t) = v else { return err(at, "expected a whole number") };
    match t.parse::<u64>() {
        Ok(x) if x <= max => Ok(x),
        Ok(_) => err(at, format!("at most {max}")),
        Err(_) => err(at, format!("`{t}` isn't a whole number ≥ 0")),
    }
}

fn ms(v: Value, at: &str) -> Result<SimDuration, DataError> {
    Ok(SimDuration::from_millis(int(v, at, MAX_MS)?))
}

/// An enum value: `Name` or `Name(fields...)`.
pub fn variant(v: Value, at: &str) -> Result<(String, Option<Fields>), DataError> {
    match v {
        Value::Ident(name) => Ok((name, None)),
        Value::Fields(Some(name), list) => Ok((name, Some(Fields { at: at.to_string(), list }))),
        _ => err(at, "expected a variant: `Name` or `Name(...)`"),
    }
}

/// A bare name (a unit variant).
pub fn unit(v: Value, at: &str) -> Result<String, DataError> {
    match v {
        Value::Ident(name) => Ok(name),
        _ => err(at, "expected a name"),
    }
}

/// Names live as long as the program (the sim's types hold `&'static str`), interned so reading
/// the same recipe again doesn't grow memory. Capped so a stream of distinct names can't either.
fn intern(s: &str, at: &str) -> Result<&'static str, DataError> {
    static NAMES: Mutex<BTreeSet<&'static str>> = Mutex::new(BTreeSet::new());
    const MAX_NAMES: usize = 1 << 16;
    let mut names = NAMES.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(&n) = names.get(s) {
        return Ok(n);
    }
    if names.len() >= MAX_NAMES {
        return err(at, "too many distinct names in this process");
    }
    let leaked: &'static str = Box::leak(s.to_string().into_boxed_str());
    names.insert(leaked);
    Ok(leaked)
}

fn name(v: Value, at: &str) -> Result<&'static str, DataError> {
    let Value::Str(s) = v else { return err(at, "expected a string") };
    if s.trim().is_empty() || s.len() > MAX_NAME || s.chars().any(char::is_control) {
        return err(at, format!("1–{MAX_NAME} bytes, no control characters"));
    }
    intern(&s, at)
}

const STAT_FIELDS: [&str; 7] =
    ["max_health", "health_regen", "armor", "magic_resist", "attack_damage", "ability_power", "move_speed"];

fn stats(v: Value, at: &str) -> Result<Stats, DataError> {
    let mut f = Fields::of(v, at, None)?;
    let mut s = Stats::NONE;
    let mut vals = [0.0; 7];
    for (i, k) in STAT_FIELDS.iter().enumerate() {
        vals[i] = f.f32_or(k, 0.0)?;
    }
    f.done()?;
    [s.max_health, s.health_regen, s.armor, s.magic_resist, s.attack_damage, s.ability_power, s.move_speed] = vals;
    Ok(s)
}

fn damage(v: Value, at: &str) -> Result<Damage, DataError> {
    let mut f = Fields::of(v, at, None)?;
    let (k, kat) = f.req("kind")?;
    let kind = match unit(k, &kat)?.as_str() {
        "Physical" => DamageKind::Physical,
        "Magic" => DamageKind::Magic,
        "True" => DamageKind::True,
        other => return err(&kat, format!("`{other}`: expected Physical, Magic or True")),
    };
    let d = Damage {
        kind,
        base: f.f32("base")?,
        ad_ratio: f.f32_or("ad_ratio", 0.0)?,
        ap_ratio: f.f32_or("ap_ratio", 0.0)?,
    };
    f.done()?;
    Ok(d)
}

fn cc(v: Value, at: &str) -> Result<Cc, DataError> {
    let (name, fields) = variant(v, at)?;
    let Some(mut f) = fields else {
        return match name.as_str() {
            "None" => Ok(Cc::None),
            other => err(at, format!("`{other}`: expected None, Stun, Root, Knockup, Pull or Slow")),
        };
    };
    let out = match name.as_str() {
        "Stun" => Cc::Stun(f.ms("ms")?),
        "Root" => Cc::Root(f.ms("ms")?),
        "Knockup" => Cc::Knockup(f.ms("ms")?),
        "Pull" => {
            let (v, a) = f.req("to")?;
            Cc::Pull(int(v, &a, u16::MAX as u64)? as u16)
        }
        "Slow" => {
            let (v, a) = f.req("pct")?;
            let pct = int(v, &a, 99)? as u8;
            Cc::Slow { pct, duration: f.ms("ms")? }
        }
        other => return err(at, format!("`{other}(...)`: expected Stun, Root, Knockup, Pull or Slow")),
    };
    f.done()?;
    Ok(out)
}

fn cc_or(f: &mut Fields) -> Result<Cc, DataError> {
    match f.take("cc") {
        Some((v, at)) => cc(v, &at),
        None => Ok(Cc::None),
    }
}

fn effect(v: Value, at: &str) -> Result<Effect, DataError> {
    let (kind, fields) = variant(v, at)?;
    let Some(mut f) = fields else { return err(at, format!("`{kind}` needs its fields: `{kind}(...)`")) };
    let dmg = |f: &mut Fields| -> Result<Damage, DataError> {
        let (v, a) = f.req("damage")?;
        damage(v, &a)
    };
    let out = match kind.as_str() {
        "Line" => Effect::Line(LineSkillshot {
            windup: f.ms("windup_ms")?,
            speed: f.f32("speed")?,
            radius: f.f32("radius")?,
            range: f.f32("range")?,
            damage: dmg(&mut f)?,
            cc: cc_or(&mut f)?,
        }),
        "Area" => Effect::Area(DelayedArea {
            windup: f.ms("windup_ms")?,
            range: f.f32_or("range", 0.0)?,
            radius: f.f32("radius")?,
            delay: f.ms_or("delay_ms")?,
            damage: dmg(&mut f)?,
            cc: cc_or(&mut f)?,
            pulses: match f.take("pulses") {
                Some((v, a)) => {
                    let mut p = Fields::of(v, &a, None)?;
                    let (c, ca) = p.req("count")?;
                    let out = Pulses {
                        count: int(c, &ca, 64)? as u8,
                        every: p.ms("every_ms")?,
                        share: p.f32("share")?,
                        cc: cc_or(&mut p)?,
                    };
                    p.done()?;
                    out
                }
                None => Pulses::NONE,
            },
        }),
        "Dash" => Effect::Dash(Dash {
            range: f.f32("range")?,
            speed: f.f32("speed")?,
            veil: match f.take("veil") {
                Some((v, a)) => {
                    let mut p = Fields::of(v, &a, None)?;
                    let out = VeilSpec { radius: p.f32("radius")?, duration: p.ms("duration_ms")? };
                    p.done()?;
                    out
                }
                None => VeilSpec::NONE,
            },
        }),
        "Blink" => Effect::Blink(Blink { range: f.f32("range")? }),
        "Shield" => Effect::Shield(Shield { amount: f.f32("amount")?, duration: f.ms("duration_ms")? }),
        "Support" => Effect::Support(Support {
            range: f.f32_or("range", 0.0)?,
            heal: f.f32_or("heal", 0.0)?,
            heal_ap: f.f32_or("heal_ap", 0.0)?,
            heal_missing: f.f32_or("heal_missing", 0.0)?,
            shield: f.f32_or("shield", 0.0)?,
            shield_ap: f.f32_or("shield_ap", 0.0)?,
            duration: f.ms_or("duration_ms")?,
        }),
        "Lunge" => Effect::Lunge(Lunge {
            range: f.f32("range")?,
            speed: f.f32("speed")?,
            damage: dmg(&mut f)?,
            cc: cc_or(&mut f)?,
        }),
        "Beam" => Effect::Beam(Beam {
            windup: f.ms("windup_ms")?,
            range: f.f32("range")?,
            radius: f.f32("radius")?,
            duration: f.ms("duration_ms")?,
            every: f.ms("every_ms")?,
            damage: dmg(&mut f)?,
            cc: cc_or(&mut f)?,
        }),
        "Claim" => return err(at, "Claim is a utility spell, not a champion ability"),
        other => {
            return err(at, format!("`{other}`: expected Line, Area, Dash, Blink, Shield, Support, Lunge or Beam"));
        }
    };
    f.done()?;
    Ok(out)
}

const TRANSFORMS: [(&str, u8); 3] = [("Multishot", Transforms::MULTISHOT), ("Echo", Transforms::ECHO), ("Wide", 4)];

fn ability(v: Value, at: &str) -> Result<Ability, DataError> {
    let mut f = Fields::of(v, at, None)?;
    let (n, nat) = f.req("name")?;
    let name = name(n, &nat)?;
    let cooldown = f.ms("cooldown_ms")?;
    let (e, eat) = f.req("effect")?;
    let effect = effect(e, &eat)?;
    let (r, rat) = f.req("reaction")?;
    let reaction = match unit(r, &rat)?.as_str() {
        "HardCc" => ReactionClass::HardCc,
        "Burst" => ReactionClass::Burst,
        "Poke" => ReactionClass::Poke,
        "None" => ReactionClass::None,
        other => return err(&rat, format!("`{other}`: expected HardCc, Burst, Poke or None")),
    };
    let per_rank = match f.take("per_rank") {
        Some((v, a)) => {
            let mut p = Fields::of(v, &a, None)?;
            let out = RankScaling { damage: p.f32_or("damage", 0.0)?, cooldown: p.ms_or("cooldown_ms")? };
            p.done()?;
            out
        }
        None => RankScaling::NONE,
    };
    let transforms = match f.take("transforms") {
        Some((Value::List(None, items), a)) => {
            let mut bits = 0u8;
            for (i, t) in items.into_iter().enumerate() {
                let tat = format!("{a}[{i}]");
                let t = unit(t, &tat)?;
                match TRANSFORMS.iter().find(|(n, _)| *n == t) {
                    Some((_, b)) => bits |= b,
                    None => return err(&tat, format!("`{t}`: expected Multishot, Echo or Wide")),
                }
            }
            Transforms(bits)
        }
        Some((_, a)) => return err(&a, "expected a list: `[Multishot, Echo, Wide]`"),
        None => Transforms::NONE,
    };
    f.done()?;
    Ok(Ability { name, cooldown, effect, reaction, per_rank, transforms })
}

/// One ability as a champion file writes it (`(name: …, cooldown_ms: …, effect: …, reaction:
/// …)`): the champion creator's own moves, which no champion carries, are read with it (20 §8.2).
pub fn read_ability(v: Value, at: &str) -> Result<Ability, DataError> {
    ability(v, at)
}

/// Read a champion file (`Champion(format: 0, ...)`).
pub fn read_champion(text: &str) -> Result<ChampionDef, DataError> {
    read_champion_and_looks(text).map(|(def, _)| def)
}

/// A champion file's kit, and its `looks` (how the creator built it, CR2) unread: the sim
/// doesn't use them, and `mftr-forge` checks them.
pub fn read_champion_and_looks(text: &str) -> Result<(ChampionDef, Option<(Value, String)>), DataError> {
    let mut f = Fields::of(parse(text)?, "", Some("Champion"))?;
    let looks = f.take("looks");
    let (v, at) = f.req("format")?;
    let format = int(v, &at, u32::MAX as u64)?;
    if format != FORMAT {
        return err(&at, format!("format {format} isn't supported (this build reads {FORMAT})"));
    }
    let (n, nat) = f.req("name")?;
    let name = name(n, &nat)?;
    let (c, cat) = f.req("class")?;
    let class_name = unit(c, &cat)?;
    let Some(class) = Class::by_name(&class_name) else {
        let all: Vec<&str> = Class::ALL.iter().map(|c| c.name()).collect();
        return err(&cat, format!("`{class_name}`: expected one of {}", all.join(", ")));
    };
    let (s, sat) = f.req("stats")?;
    let stats = stats(s, &sat)?;
    let growth = match f.take("growth") {
        Some((g, gat)) => self::stats(g, &gat)?,
        None => Stats::NONE,
    };
    let (a, aat) = f.req("attack")?;
    let attack = {
        let mut a = Fields::of(a, &aat, None)?;
        let out = AttackSpec {
            range: a.f32("range")?,
            attack_speed: a.f32("attack_speed")?,
            windup_fraction: a.f32("windup_fraction")?,
            bolt_speed: a.f32_or("bolt_speed", 0.0)?,
        };
        a.done()?;
        if out.attack_speed <= 0.0 || out.windup_fraction >= 1.0 {
            return err(&aat, "attack_speed must be > 0 and windup_fraction < 1");
        }
        out
    };
    let (l, lat) = f.req("abilities")?;
    let Value::List(None, items) = l else { return err(&lat, "expected a list of four abilities") };
    if items.len() != 4 {
        return err(&lat, format!("expected four abilities (Q W E R), found {}", items.len()));
    }
    let mut abilities = [crate::ability::BLINK; 4];
    for (i, item) in items.into_iter().enumerate() {
        abilities[i] = ability(item, &format!("{lat}[{i}]"))?;
    }
    let passive = match f.take("passive") {
        Some((v, at)) => passive(v, &at)?,
        None => Passive::None,
    };
    let resource = match f.take("resource") {
        Some((v, at)) => resource(v, &at)?,
        None => Resource::NONE,
    };
    f.done()?;
    Ok((ChampionDef { name, class, stats, growth, attack, abilities, passive, resource }, looks))
}

/// A kit's resource (01 §9): `Mana(max, regen, costs: [Q, W, E, R])`, `Energy(...)`,
/// `Rage(max, regen: per hit, costs)`, `Health(costs)`, or `None`.
fn resource(v: Value, at: &str) -> Result<Resource, DataError> {
    let (kind, fields) = variant(v, at)?;
    let kind = match kind.as_str() {
        "None" => return Ok(Resource::NONE),
        "Mana" => ResourceKind::Mana,
        "Energy" => ResourceKind::Energy,
        "Rage" => ResourceKind::Rage,
        "Health" => ResourceKind::Health,
        other => return err(at, format!("`{other}`: a resource is Mana, Energy, Rage, Health or None")),
    };
    let Some(mut f) = fields else { return err(at, "a resource needs its costs: `Mana(..., costs: [...])`") };
    let (max, regen) = if kind == ResourceKind::Health { (0.0, 0.0) } else { (f.f32("max")?, f.f32("regen")?) };
    let (c, cat) = f.req("costs")?;
    let Value::List(None, items) = c else { return err(&cat, "expected four costs: `[Q, W, E, R]`") };
    if items.len() != 4 {
        return err(&cat, "expected four costs: `[Q, W, E, R]`");
    }
    let mut costs = [0.0; 4];
    for (i, item) in items.into_iter().enumerate() {
        costs[i] = num(item, &format!("{cat}[{i}]"))?;
    }
    f.done()?;
    Ok(Resource { kind, max, regen, costs })
}

/// A champion's own passive (20 §8.1): the kinds a champion may take, with their numbers.
/// Durations are milliseconds; shares and ratios are fractions.
fn passive(v: Value, at: &str) -> Result<Passive, DataError> {
    let (kind, fields) = variant(v, at)?;
    let Some(mut f) = fields else { return err(at, format!("`{kind}` needs its fields: `{kind}(...)`")) };
    let millis = |f: &mut Fields, key: &str| -> Result<u64, DataError> {
        let (v, a) = f.req(key)?;
        int(v, &a, 600_000)
    };
    let out = match kind.as_str() {
        "OnHitMagic" => Passive::OnHitMagic { base: f.f32("base")?, ap_ratio: f.f32_or("ap_ratio", 0.0)? },
        "ThirdHit" => Passive::ThirdHit { base: f.f32("base")?, bonus_ad: f.f32_or("bonus_ad", 0.0)? },
        "Spellblade" => Passive::Spellblade { base_ad: f.f32("base_ad")? },
        "Cleave" => Passive::Cleave { melee: f.f32("melee")?, ranged: f.f32("ranged")?, radius: f.f32("radius")? },
        "Quicken" => Passive::Quicken { speed: f.f32("speed")?, ms: millis(&mut f, "ms")? },
        "Sunder" => {
            let per_stack = f.f32("per_stack")?;
            let (s, sa) = f.req("stacks")?;
            Passive::Sunder { per_stack, stacks: int(s, &sa, 20)? as u8, ms: millis(&mut f, "ms")? }
        }
        "Thorns" => Passive::Thorns {
            base: f.f32("base")?,
            bonus_armor: f.f32_or("bonus_armor", 0.0)?,
            wounds: f.f32_or("wounds", 0.0)?,
        },
        "Rally" => Passive::Rally {
            shield: f.f32("shield")?,
            bonus_hp: f.f32_or("bonus_hp", 0.0)?,
            threshold: f.f32("threshold")?,
            duration_ms: millis(&mut f, "duration_ms")?,
            cooldown_ms: millis(&mut f, "cooldown_ms")?,
        },
        "Warmth" => Passive::Warmth { max_hp: f.f32("max_hp")?, after_ms: millis(&mut f, "after_ms")? },
        other => {
            return err(at, format!("`{other}`: a champion's passive is one of {}", PASSIVE_KINDS.join(", ")));
        }
    };
    f.done()?;
    Ok(out)
}

// --- Writing ----------------------------------------------------------------------------------

/// Milliseconds that [`SimDuration::from_millis`] turns back into `d` exactly.
fn ms_of(d: SimDuration) -> u64 {
    let m = d.0 * 1000 / SUBTICKS_PER_SECOND;
    (m..=m + 1).find(|&m| SimDuration::from_millis(m) == d).unwrap_or_else(|| panic!("{d:?} isn't a whole millisecond"))
}

/// Joins `key: value` pairs, leaving out the ones equal to their default.
struct Line(String);

impl Line {
    fn new() -> Line {
        Line(String::new())
    }

    fn put(&mut self, key: &str, value: impl fmt::Display) -> &mut Line {
        if !self.0.is_empty() {
            self.0.push_str(", ");
        }
        let _ = write!(self.0, "{key}: {value}");
        self
    }

    fn opt(&mut self, key: &str, value: f32) -> &mut Line {
        if value != 0.0 {
            self.put(key, value);
        }
        self
    }

    fn opt_ms(&mut self, key: &str, d: SimDuration) -> &mut Line {
        if d.0 != 0 {
            self.put(key, ms_of(d));
        }
        self
    }

    fn wrap(&self, name: &str) -> String {
        format!("{name}({})", self.0)
    }
}

fn write_stats(s: &Stats) -> String {
    let rest = Stats {
        max_health: 0.0,
        health_regen: 0.0,
        armor: 0.0,
        magic_resist: 0.0,
        attack_damage: 0.0,
        ability_power: 0.0,
        move_speed: 0.0,
        ..*s
    };
    assert_eq!(rest, Stats::NONE, "champion files hold the seven champion stats only");
    let vals = [s.max_health, s.health_regen, s.armor, s.magic_resist, s.attack_damage, s.ability_power, s.move_speed];
    let mut l = Line::new();
    for (k, v) in STAT_FIELDS.iter().zip(vals) {
        l.opt(k, v);
    }
    l.wrap("")
}

fn write_damage(d: &Damage) -> String {
    let kind = match d.kind {
        DamageKind::Physical => "Physical",
        DamageKind::Magic => "Magic",
        DamageKind::True => "True",
    };
    Line::new().put("kind", kind).put("base", d.base).opt("ad_ratio", d.ad_ratio).opt("ap_ratio", d.ap_ratio).wrap("")
}

fn write_cc(c: Cc) -> String {
    match c {
        Cc::None => "None".to_string(),
        Cc::Stun(d) => Line::new().put("ms", ms_of(d)).wrap("Stun"),
        Cc::Root(d) => Line::new().put("ms", ms_of(d)).wrap("Root"),
        Cc::Knockup(d) => Line::new().put("ms", ms_of(d)).wrap("Knockup"),
        Cc::Pull(to) => Line::new().put("to", to).wrap("Pull"),
        Cc::Slow { pct, duration } => Line::new().put("pct", pct).put("ms", ms_of(duration)).wrap("Slow"),
    }
}

fn cc_field(l: &mut Line, c: Cc) {
    if c != Cc::None {
        l.put("cc", write_cc(c));
    }
}

fn write_effect(e: &Effect) -> String {
    match *e {
        Effect::Line(s) => {
            let mut l = Line::new();
            l.put("windup_ms", ms_of(s.windup)).put("speed", s.speed).put("radius", s.radius).put("range", s.range);
            l.put("damage", write_damage(&s.damage));
            cc_field(&mut l, s.cc);
            l.wrap("Line")
        }
        Effect::Area(s) => {
            let mut l = Line::new();
            l.put("windup_ms", ms_of(s.windup))
                .opt("range", s.range)
                .put("radius", s.radius)
                .opt_ms("delay_ms", s.delay);
            l.put("damage", write_damage(&s.damage));
            cc_field(&mut l, s.cc);
            if s.pulses != Pulses::NONE {
                let p = s.pulses;
                let mut pl = Line::new();
                pl.put("count", p.count).put("every_ms", ms_of(p.every)).put("share", p.share);
                cc_field(&mut pl, p.cc);
                l.put("pulses", pl.wrap(""));
            }
            l.wrap("Area")
        }
        Effect::Dash(d) => {
            let mut l = Line::new();
            l.put("range", d.range).put("speed", d.speed);
            if d.veil != VeilSpec::NONE {
                l.put(
                    "veil",
                    Line::new().put("radius", d.veil.radius).put("duration_ms", ms_of(d.veil.duration)).wrap(""),
                );
            }
            l.wrap("Dash")
        }
        Effect::Blink(b) => Line::new().put("range", b.range).wrap("Blink"),
        Effect::Shield(s) => Line::new().put("amount", s.amount).put("duration_ms", ms_of(s.duration)).wrap("Shield"),
        Effect::Support(s) => Line::new()
            .opt("range", s.range)
            .opt("heal", s.heal)
            .opt("heal_ap", s.heal_ap)
            .opt("heal_missing", s.heal_missing)
            .opt("shield", s.shield)
            .opt("shield_ap", s.shield_ap)
            .opt_ms("duration_ms", s.duration)
            .wrap("Support"),
        Effect::Lunge(s) => {
            let mut l = Line::new();
            l.put("range", s.range).put("speed", s.speed).put("damage", write_damage(&s.damage));
            cc_field(&mut l, s.cc);
            l.wrap("Lunge")
        }
        Effect::Beam(s) => {
            let mut l = Line::new();
            l.put("windup_ms", ms_of(s.windup)).put("range", s.range).put("radius", s.radius);
            l.put("duration_ms", ms_of(s.duration))
                .put("every_ms", ms_of(s.every))
                .put("damage", write_damage(&s.damage));
            cc_field(&mut l, s.cc);
            l.wrap("Beam")
        }
        Effect::Claim(_) => panic!("Claim is a utility spell, not a champion ability"),
    }
}

/// The passive kinds a champion file takes ([`passive`]).
pub const PASSIVE_KINDS: [&str; 9] =
    ["OnHitMagic", "ThirdHit", "Spellblade", "Cleave", "Quicken", "Sunder", "Thorns", "Rally", "Warmth"];

fn write_passive(p: &Passive) -> Option<String> {
    Some(match *p {
        Passive::None => return None,
        Passive::OnHitMagic { base, ap_ratio } => {
            Line::new().put("base", base).opt("ap_ratio", ap_ratio).wrap("OnHitMagic")
        }
        Passive::ThirdHit { base, bonus_ad } => {
            Line::new().put("base", base).opt("bonus_ad", bonus_ad).wrap("ThirdHit")
        }
        Passive::Spellblade { base_ad } => Line::new().put("base_ad", base_ad).wrap("Spellblade"),
        Passive::Cleave { melee, ranged, radius } => {
            Line::new().put("melee", melee).put("ranged", ranged).put("radius", radius).wrap("Cleave")
        }
        Passive::Quicken { speed, ms } => Line::new().put("speed", speed).put("ms", ms).wrap("Quicken"),
        Passive::Sunder { per_stack, stacks, ms } => {
            Line::new().put("per_stack", per_stack).put("stacks", stacks).put("ms", ms).wrap("Sunder")
        }
        Passive::Thorns { base, bonus_armor, wounds } => {
            Line::new().put("base", base).opt("bonus_armor", bonus_armor).opt("wounds", wounds).wrap("Thorns")
        }
        Passive::Rally { shield, bonus_hp, threshold, duration_ms, cooldown_ms } => Line::new()
            .put("shield", shield)
            .opt("bonus_hp", bonus_hp)
            .put("threshold", threshold)
            .put("duration_ms", duration_ms)
            .put("cooldown_ms", cooldown_ms)
            .wrap("Rally"),
        Passive::Warmth { max_hp, after_ms } => {
            Line::new().put("max_hp", max_hp).put("after_ms", after_ms).wrap("Warmth")
        }
        other => panic!("{other:?} isn't a champion passive"),
    })
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Write a champion file. `comment` (one or more lines) goes at the top as `//` lines.
/// [`read_champion`] gives back exactly `def`.
pub fn write_champion(def: &ChampionDef, comment: &str) -> String {
    write_champion_as(def, def.name, comment, "")
}

/// [`write_champion`] under another name, with `extra` (whole field lines, like a recipe's
/// `looks`) after the format line.
pub fn write_champion_as(def: &ChampionDef, name: &str, comment: &str, extra: &str) -> String {
    let mut out = String::new();
    for line in comment.lines() {
        let _ = writeln!(out, "// {line}");
    }
    let _ = writeln!(out, "Champion(");
    let _ = writeln!(out, "    format: {FORMAT},");
    out += extra;
    let _ = writeln!(out, "    name: {},", quote(name));
    let _ = writeln!(out, "    class: {},", def.class.name());
    let _ = writeln!(out, "    stats: {},", write_stats(&def.stats));
    let _ = writeln!(out, "    growth: {},", write_stats(&def.growth));
    let a = &def.attack;
    let attack = Line::new()
        .put("range", a.range)
        .put("attack_speed", a.attack_speed)
        .put("windup_fraction", a.windup_fraction)
        .opt("bolt_speed", a.bolt_speed)
        .wrap("");
    let _ = writeln!(out, "    attack: {attack},");
    if let Some(p) = write_passive(&def.passive) {
        let _ = writeln!(out, "    passive: {p},");
    }
    let r = &def.resource;
    if r.kind != ResourceKind::None {
        let costs: Vec<String> = r.costs.iter().map(|c| c.to_string()).collect();
        let name = match r.kind {
            ResourceKind::Mana => "Mana",
            ResourceKind::Energy => "Energy",
            ResourceKind::Rage => "Rage",
            _ => "Health",
        };
        let pool = if r.kind == ResourceKind::Health {
            String::new()
        } else {
            format!("max: {}, regen: {}, ", r.max, r.regen)
        };
        let _ = writeln!(out, "    resource: {name}({pool}costs: [{}]),", costs.join(", "));
    }
    let _ = writeln!(out, "    abilities: [");
    for ab in &def.abilities {
        let _ = writeln!(out, "        (");
        let _ = writeln!(out, "            name: {},", quote(ab.name));
        let _ = writeln!(out, "            cooldown_ms: {},", ms_of(ab.cooldown));
        let _ = writeln!(out, "            effect: {},", write_effect(&ab.effect));
        let reaction = match ab.reaction {
            ReactionClass::HardCc => "HardCc",
            ReactionClass::Burst => "Burst",
            ReactionClass::Poke => "Poke",
            ReactionClass::None => "None",
        };
        let _ = writeln!(out, "            reaction: {reaction},");
        if ab.per_rank != RankScaling::NONE {
            let pr = Line::new().opt("damage", ab.per_rank.damage).opt_ms("cooldown_ms", ab.per_rank.cooldown).wrap("");
            let _ = writeln!(out, "            per_rank: {pr},");
        }
        let names: Vec<&str> = TRANSFORMS.iter().filter(|(_, b)| ab.transforms.accepts(*b)).map(|(n, _)| *n).collect();
        let known: u8 = TRANSFORMS.iter().map(|(_, b)| b).sum();
        assert_eq!(ab.transforms.0 & !known, 0, "{}: unknown transform bits", ab.name);
        if !names.is_empty() {
            let _ = writeln!(out, "            transforms: [{}],", names.join(", "));
        }
        let _ = writeln!(out, "        ),");
    }
    let _ = writeln!(out, "    ],");
    let _ = writeln!(out, ")");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::champion::ChampionId;

    #[test]
    fn every_official_champion_round_trips() {
        for c in ChampionId::ALL {
            let def = c.def();
            let text = write_champion(def, "");
            assert_eq!(read_champion(&text).as_ref(), Ok(def), "{}\n{text}", def.name);
        }
    }

    #[test]
    fn resources_round_trip() {
        for r in [
            Resource { kind: ResourceKind::Mana, max: 400.0, regen: 8.0, costs: [50.0, 60.0, 70.0, 100.0] },
            Resource { kind: ResourceKind::Energy, max: 200.0, regen: 50.0, costs: [40.0, 30.0, 0.0, 0.0] },
            Resource { kind: ResourceKind::Rage, max: 100.0, regen: 8.0, costs: [0.0, 50.0, 0.0, 100.0] },
            Resource { kind: ResourceKind::Health, max: 0.0, regen: 0.0, costs: [30.0, 0.0, 60.0, 0.0] },
        ] {
            let def = ChampionDef { resource: r, ..*ChampionId::Rook.def() };
            let text = write_champion(&def, "");
            assert_eq!(read_champion(&text), Ok(def), "{text}");
        }
    }

    #[test]
    fn passives_round_trip_and_odd_ones_are_refused() {
        use crate::items::Passive;
        for p in [
            Passive::OnHitMagic { base: 18.0, ap_ratio: 0.2 },
            Passive::ThirdHit { base: 45.0, bonus_ad: 0.0 },
            Passive::Spellblade { base_ad: 0.8 },
            Passive::Cleave { melee: 0.35, ranged: 0.15, radius: 300.0 },
            Passive::Quicken { speed: 25.0, ms: 2000 },
            Passive::Sunder { per_stack: 0.04, stacks: 5, ms: 5000 },
            Passive::Thorns { base: 12.0, bonus_armor: 0.15, wounds: 0.3 },
            Passive::Rally { shield: 120.0, bonus_hp: 0.3, threshold: 0.3, duration_ms: 3000, cooldown_ms: 60_000 },
            Passive::Warmth { max_hp: 0.02, after_ms: 6000 },
        ] {
            let def = ChampionDef { passive: p, ..*ChampionId::Rook.def() };
            let text = write_champion(&def, "");
            assert_eq!(read_champion(&text), Ok(def), "{text}");
        }
        let text = write_champion(ChampionId::Rook.def(), "").replace(
            "    abilities: [",
            "    passive: Smolder(base: 5, bonus_hp: 0, radius: 300),
    abilities: [",
        );
        let e = read_champion(&text).unwrap_err().to_string();
        assert!(e.contains("a champion's passive is one of"), "{e}");
    }

    fn ember() -> String {
        write_champion(ChampionId::Ember.def(), "Ember")
    }

    fn fails(text: &str, at: &str, msg: &str) {
        match read_champion(text) {
            Ok(_) => panic!("read:\n{text}"),
            Err(e) => {
                assert!(e.at.contains(at), "{e}");
                assert!(e.msg.contains(msg), "{e}");
            }
        }
    }

    #[test]
    fn comments_and_trailing_commas_read() {
        let text = ember().replace("    format: 0,", "    /* a block */ format: 0, // a line\n");
        assert_eq!(read_champion(&text).unwrap(), *ChampionId::Ember.def());
    }

    #[test]
    fn strict_about_fields() {
        let e = ember();
        fails(&e.replace("format: 0,", "format: 0, colour: 3,"), "colour", "unknown field");
        fails(&e.replace("    format: 0,\n", ""), "champion", "`format` is missing");
        fails(&e.replace("format: 0,", "format: 0, format: 0,"), "line", "given twice");
        fails(&e.replace("format: 0", "format: 1"), "format", "isn't supported");
        fails(&e.replace("Champion(", "Hero("), "", "expected `Champion(...)`");
        fails(&e.replace("class: Mage", "class: Wizard"), "class", "Wizard");
    }

    #[test]
    fn strict_about_values() {
        let e = ember();
        fails(&e.replace("max_health: 600", "max_health: -600"), "stats.max_health", "≥ 0");
        fails(&e.replace("max_health: 600", "max_health: inf"), "stats.max_health", "expected a number");
        fails(&e.replace("max_health: 600", "max_health: 1e40"), "stats.max_health", "finite");
        fails(&e.replace("cooldown_ms: 4000", "cooldown_ms: 4000.5"), "abilities[0].cooldown_ms", "whole number");
        fails(&e.replace("cooldown_ms: 4000", "cooldown_ms: 900000"), "abilities[0].cooldown_ms", "at most");
        fails(
            &e.replace("Stun(ms: 1250)", "Stun(ms: 1250, extra: 1)"),
            "abilities[3].effect.cc.extra",
            "unknown field",
        );
        fails(&e.replace("Stun(ms: 1250)", "Freeze(ms: 1250)"), "abilities[3].effect.cc", "Freeze");
        fails(&e.replace("kind: Magic", "kind: Fire"), "abilities[0].effect.damage.kind", "Fire");
        fails(&e.replace("reaction: Burst", "reaction: Slow"), "abilities[0].reaction", "Slow");
        fails(&e.replace("[Multishot, Echo, Wide]", "[Split]"), "abilities[0].transforms[0]", "Split");
        fails(&e.replace("\"Ember Lance\"", "\"\""), "abilities[0].name", "1–32 bytes");
        fails(&e.replace("\"Ember Lance\"", &format!("\"{}\"", "x".repeat(40))), "abilities[0].name", "1–32 bytes");
    }

    #[test]
    fn strict_about_shape() {
        let e = ember();
        let three = {
            let start = e.find("        (\n            name: \"Ember Lance\"").unwrap();
            let end = e.find("        (\n            name: \"Cinder Bloom\"").unwrap();
            format!("{}{}", &e[..start], &e[end..])
        };
        fails(&three, "abilities", "found 3");
        fails(&format!("{e} extra"), "line", "after the value");
        fails(&e[..e.len() / 2], "line", "");
        fails(&"(".repeat(40), "line", "nested too deeply");
        fails(&" ".repeat(MAX_INPUT + 1), "input", "longer than");
        fails(
            &e.replace("effect: Blink(range: 350)", "effect: Claim(range: 500)"),
            "abilities[2].effect",
            "utility spell",
        );
    }

    #[test]
    fn names_are_interned() {
        let a = read_champion(&ember()).unwrap();
        let b = read_champion(&ember()).unwrap();
        assert!(std::ptr::eq(a.name, b.name));
    }
}
