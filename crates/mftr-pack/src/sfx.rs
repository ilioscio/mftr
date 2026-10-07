//! A pack's sounds (A4c, 11 §3.2, 05 §7): `<id>.sfx.ron` next to the model binds events to Ogg
//! Vorbis files in `sfx/`. First-party packs generate both with `mftr-tools sfx build` from a
//! hand-written recipe; a community pack may ship recorded sounds instead.
//!
//! Events are those of the VFX (`<action>.<phase>`, including `fire`, with the extra phase `cast`
//! for a windup's start), plus `unit.<what>` (`foot`, `death`, `respawn`, `recall`, `emote`) and the shared
//! hard-CC accent `cc.hard` (05 §7: hard CC has a learnable signature). Several sounds on one
//! event are variants: the client picks one, never the same twice in a row.
//!
//! Ogg is decoded by `lewton` (memory-safe Rust, 11 §4) with every cap checked before the samples
//! are allocated: file size, channels, rate and the stream's length from its last page.

use std::io::Cursor;

use nanoserde::DeRon;

/// The binding file itself.
pub const MAX_SFX_BYTES: usize = 16 * 1024;
pub const MAX_SOUNDS: usize = 48;
pub const MAX_VARIANTS: usize = 4;
/// 10 §9: SFX ≤ 550 KB per champion.
pub const MAX_TOTAL_BYTES: usize = 550 * 1000;
pub const MAX_FILE_BYTES: usize = 128 * 1024;
pub const MAX_SECONDS: f32 = 3.0;
pub const RATES: (u32, u32) = (8_000, 48_000);

pub const ACTIONS: &[&str] = &["attack", "q", "w", "e", "r", "d", "f", "*"];
pub const PHASES: &[&str] = &["cast", "release", "impact", "detonate", "start", "land", "fire"];
pub const UNIT_EVENTS: &[&str] = &["foot", "death", "respawn", "recall", "emote"];

#[derive(Clone, Debug, DeRon)]
pub struct SfxFile {
    /// `fnv1a64:<hex>` of the recipe the file was built from, when generated (a staleness check
    /// for first-party packs; ignored by the game).
    #[nserde(default)]
    pub source: Option<String>,
    pub sounds: Vec<SoundSpec>,
}

#[derive(Clone, Debug, DeRon)]
pub struct SoundSpec {
    /// `sfx/<name>.ogg`.
    pub name: String,
    pub events: Vec<String>,
    /// Linear gain, 0–1.
    pub volume: f32,
    /// Random pitch variation per play, ± this fraction (0–0.25).
    #[nserde(default)]
    pub pitch: f32,
}

/// A decoded sound, ready for the client.
#[derive(Clone, Debug)]
pub struct Sound {
    pub spec: SoundSpec,
    pub rate: u32,
    pub channels: u8,
    /// Interleaved 16-bit samples.
    pub pcm: Vec<i16>,
}

pub fn parse(text: &str) -> Result<SfxFile, String> {
    if text.len() > MAX_SFX_BYTES {
        return Err(format!("sfx file is {} bytes, over the {MAX_SFX_BYTES} byte cap", text.len()));
    }
    SfxFile::deserialize_ron(text).map_err(|e| format!("sfx: {e}"))
}

fn event_ok(event: &str) -> Result<(), String> {
    if event == "cc.hard" {
        return Ok(());
    }
    match event.split_once('.') {
        Some(("unit", what)) if UNIT_EVENTS.contains(&what) => Ok(()),
        Some(("unit", what)) => Err(format!("unknown unit event `{what}` (allowed: {})", UNIT_EVENTS.join(", "))),
        Some((action, phase)) => {
            if !ACTIONS.contains(&action) {
                Err(format!("unknown action `{action}`"))
            } else if !PHASES.contains(&phase) {
                Err(format!("unknown phase `{phase}`"))
            } else {
                Ok(())
            }
        }
        None => Err("events are `<action>.<phase>`, `unit.<what>` or `cc.hard`".into()),
    }
}

/// Checks the binding file alone (names, events, knobs, caps).
pub fn check(f: &SfxFile) -> Vec<String> {
    let mut out = Vec::new();
    if f.sounds.len() > MAX_SOUNDS {
        out.push(format!("{} sounds, at most {MAX_SOUNDS}", f.sounds.len()));
    }
    let mut names: Vec<&str> = Vec::new();
    let mut events: Vec<&str> = Vec::new();
    for s in &f.sounds {
        let at = |msg: String| format!("sfx `{}`: {msg}", s.name);
        let name_ok = !s.name.is_empty()
            && s.name.len() <= 32
            && s.name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
        if !name_ok {
            out.push(at("names are 1–32 of a–z, 0–9 and _".into()));
        }
        if names.contains(&s.name.as_str()) {
            out.push(at("duplicate name".into()));
        }
        names.push(&s.name);
        if s.events.is_empty() {
            out.push(at("no events".into()));
        }
        for e in &s.events {
            if let Err(msg) = event_ok(e) {
                out.push(at(format!("`{e}`: {msg}")));
            }
            events.push(e);
        }
        if !(0.0..=1.0).contains(&s.volume) {
            out.push(at("volume 0–1".into()));
        }
        if !(0.0..=0.25).contains(&s.pitch) {
            out.push(at("pitch variation 0–0.25".into()));
        }
    }
    events.sort();
    for w in events.chunk_by(|a, b| a == b) {
        if w.len() > MAX_VARIANTS {
            out.push(format!("sfx event `{}` has {} sounds, at most {MAX_VARIANTS} variants", w[0], w.len()));
        }
    }
    out
}

/// The granule position (samples per channel) of a stream's last page, read before decoding.
fn last_granule(bytes: &[u8]) -> Option<u64> {
    let at = bytes.windows(4).rposition(|w| w == b"OggS")?;
    let g = bytes.get(at + 6..at + 14)?;
    Some(u64::from_le_bytes(g.try_into().ok()?))
}

/// Decodes one Ogg Vorbis file under the caps.
pub fn decode(bytes: &[u8]) -> Result<(u32, u8, Vec<i16>), String> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(format!("{} bytes, over the {MAX_FILE_BYTES} byte cap", bytes.len()));
    }
    if !bytes.starts_with(b"OggS") {
        return Err("not an Ogg stream".into());
    }
    let mut reader = lewton::inside_ogg::OggStreamReader::new(Cursor::new(bytes)).map_err(|e| format!("ogg: {e}"))?;
    let channels = reader.ident_hdr.audio_channels;
    let rate = reader.ident_hdr.audio_sample_rate;
    if !(1..=2).contains(&channels) {
        return Err(format!("{channels} channels (mono or stereo)"));
    }
    if !(RATES.0..=RATES.1).contains(&rate) {
        return Err(format!("{rate} Hz ({}–{} Hz)", RATES.0, RATES.1));
    }
    let cap = (MAX_SECONDS * rate as f32) as u64;
    let frames = last_granule(bytes).ok_or("no Ogg pages")?;
    if frames > cap {
        return Err(format!("{:.2} s, over the {MAX_SECONDS} s cap", frames as f32 / rate as f32));
    }
    let limit = (cap as usize + 4096) * channels as usize;
    let mut pcm = Vec::with_capacity(frames as usize * channels as usize);
    while let Some(packet) = reader.read_dec_packet_itl().map_err(|e| format!("vorbis: {e}"))? {
        if pcm.len() + packet.len() > limit {
            return Err("decodes longer than its pages say".into());
        }
        pcm.extend_from_slice(&packet);
    }
    // The last page's granule is where the stream really ends; lewton doesn't trim to it.
    pcm.truncate(frames as usize * channels as usize);
    if pcm.is_empty() {
        return Err("no audio".into());
    }
    Ok((rate, channels, pcm))
}

/// Checks and decodes a pack's sounds: `files(name)` returns `sfx/<name>.ogg`'s bytes, and
/// `present` lists the `.ogg` files actually there (unreferenced ones are errors, 11 §4).
pub fn load(
    f: &SfxFile,
    files: impl Fn(&str) -> Result<Vec<u8>, String>,
    present: &[String],
) -> (Vec<Sound>, usize, Vec<String>) {
    let mut out = check(f);
    let mut sounds = Vec::new();
    let mut total = 0;
    for s in &f.sounds {
        match files(&s.name).and_then(|b| {
            total += b.len();
            decode(&b)
        }) {
            Ok((rate, channels, pcm)) => sounds.push(Sound { spec: s.clone(), rate, channels, pcm }),
            Err(e) => out.push(format!("sfx/{}.ogg: {e}", s.name)),
        }
    }
    for p in present {
        if !f.sounds.iter().any(|s| &s.name == p) {
            out.push(format!("sfx/{p}.ogg is not bound to any event"));
        }
    }
    if total > MAX_TOTAL_BYTES {
        out.push(format!("sounds total {total} bytes, over the {MAX_TOTAL_BYTES} byte budget (10 §9)"));
    }
    (sounds, total, out)
}

/// The recipe hash recorded in a generated file's `source` (FNV-1a 64 over the text without
/// carriage returns, so a Windows checkout hashes the same).
pub fn source_hash(text: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes().filter(|&b| b != b'\r') {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("fnv1a64:{h:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_pass_and_mistakes_are_named() {
        let ok = r#"(sounds: [
            (name: "bow_draw", events: ["attack.cast", "q.cast"], volume: 0.6, pitch: 0.05),
            (name: "step_1", events: ["unit.foot"], volume: 0.3),
            (name: "snare", events: ["cc.hard"], volume: 0.9),
        ])"#;
        assert!(check(&parse(ok).unwrap()).is_empty());
        for (bad, needle) in [
            (r#"(sounds: [(name: "Bad Name", events: ["q.cast"], volume: 0.5)])"#, "names are"),
            (r#"(sounds: [(name: "a", events: ["q.explode"], volume: 0.5)])"#, "unknown phase"),
            (r#"(sounds: [(name: "a", events: ["unit.dance"], volume: 0.5)])"#, "unknown unit event"),
            (r#"(sounds: [(name: "a", events: ["q.cast"], volume: 2.0)])"#, "volume"),
            (r#"(sounds: [(name: "a", events: ["q.cast"], volume: 0.5, pitch: 0.9)])"#, "pitch"),
            (r#"(sounds: [(name: "a", events: [], volume: 0.5)])"#, "no events"),
            (
                r#"(sounds: [(name: "a", events: ["q.cast"], volume: 0.5), (name: "a", events: ["w.cast"], volume: 0.5)])"#,
                "duplicate",
            ),
        ] {
            let errs = check(&parse(bad).unwrap());
            assert!(errs.iter().any(|e| e.contains(needle)), "{needle}: {errs:?}");
        }
    }

    #[test]
    fn garbage_is_not_decoded() {
        assert!(decode(b"RIFF....WAVE").unwrap_err().contains("not an Ogg"));
        assert!(decode(&vec![0u8; MAX_FILE_BYTES + 1]).unwrap_err().contains("cap"));
        assert!(decode(b"OggS\0\x02garbage").is_err());
    }

    #[test]
    fn the_source_hash_ignores_line_endings() {
        assert_eq!(source_hash("a\r\nb\r\n"), source_hash("a\nb\n"));
        assert_ne!(source_hash("a"), source_hash("b"));
    }
}
