//! The SFX build (A4c, 05 §7): renders hand-written `sounds.ron` recipes with a small layered
//! synthesizer (sfxr-style voices plus PS1-era crunch), encodes them as Ogg Vorbis and writes a
//! pack's `export/<id>.sfx.ron` and `export/sfx/*.ogg`. Deterministic: the same recipe gives the
//! same bytes, so a diff in `export/` always means a real change.
//!
//! A recipe:
//!
//! ```ron
//! (
//!     pack: "vesper",                     // writes export/vesper.sfx.ron
//!     sounds: [(
//!         name: "bow_draw", events: ["attack.cast"], volume: 0.5, pitch: 0.05,
//!         crush: (bits: 10, hold: 2),  // optional: bit depth, sample-and-hold factor
//!         echo: (delay: 0.07, feedback: 0.3),
//!         layers: [(wave: "noise", freq: (1800.0, 600.0), env: (0.2, 0.0, 0.1), gain: 0.6,
//!                   lowpass: (3000.0, 900.0))],
//!     )],
//! )
//! ```
//!
//! Waves: `sine`, `triangle`, `square` (with `duty`), `saw`, `noise` (a new random value `freq`
//! times a second: hiss high, rumble low) and `metal` (a short 7-step LFSR clocked at `freq`:
//! tonal, metallic noise). `freq` slides from start to end exponentially, `env` is
//! (attack, sustain, decay) in seconds, `delay` starts a layer late, `punch` boosts its attack,
//! `vibrato` is (semitones, Hz), `lowpass` sweeps (start, end) Hz, `highpass` is a fixed corner
//! and `drive` saturates.

use std::f32::consts::TAU;
use std::num::{NonZeroU8, NonZeroU32};
use std::path::{Path, PathBuf};

use nanoserde::DeRon;

/// Every sound renders at 22.05 kHz mono: enough for crunchy SFX, and half the size.
pub const RATE: u32 = 22_050;
const QUALITY: f32 = 0.3;
const PEAK: f32 = 0.89;

#[derive(Clone, Debug, DeRon)]
pub struct Recipe {
    pub pack: String,
    pub sounds: Vec<SoundRecipe>,
}

#[derive(Clone, Debug, DeRon)]
pub struct SoundRecipe {
    pub name: String,
    pub events: Vec<String>,
    pub volume: f32,
    #[nserde(default)]
    pub pitch: f32,
    #[nserde(default)]
    pub crush: Option<Crush>,
    #[nserde(default)]
    pub echo: Option<Echo>,
    pub layers: Vec<Layer>,
}

#[derive(Clone, Copy, Debug, DeRon)]
pub struct Crush {
    pub bits: u32,
    pub hold: u32,
}

#[derive(Clone, Copy, Debug, DeRon)]
pub struct Echo {
    pub delay: f32,
    pub feedback: f32,
}

#[derive(Clone, Debug, DeRon)]
pub struct Layer {
    pub wave: String,
    pub freq: (f32, f32),
    pub env: (f32, f32, f32),
    pub gain: f32,
    #[nserde(default)]
    pub delay: f32,
    #[nserde(default)]
    pub punch: f32,
    #[nserde(default)]
    pub duty: Option<f32>,
    #[nserde(default)]
    pub vibrato: Option<(f32, f32)>,
    #[nserde(default)]
    pub lowpass: Option<(f32, f32)>,
    #[nserde(default)]
    pub highpass: Option<f32>,
    #[nserde(default)]
    pub drive: f32,
}

/// A deterministic xorshift generator.
struct Rng(u32);

impl Rng {
    fn new(seed: &str, salt: usize) -> Rng {
        let h = seed.bytes().fold(0x811c_9dc5u32, |h, b| (h ^ u32::from(b)).wrapping_mul(0x0100_0193));
        Rng((h ^ (salt as u32).wrapping_mul(0x9e37_79b9)) | 1)
    }

    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / (1u32 << 23) as f32 - 1.0
    }
}

fn check_layer(l: &Layer) -> Result<(), String> {
    const WAVES: &[&str] = &["sine", "triangle", "square", "saw", "noise", "metal"];
    if !WAVES.contains(&l.wave.as_str()) {
        return Err(format!("wave `{}` (allowed: {})", l.wave, WAVES.join(", ")));
    }
    let hz = |f: f32| (1.0..=RATE as f32).contains(&f);
    if !hz(l.freq.0) || !hz(l.freq.1) {
        return Err("freq 1 Hz to the sample rate".into());
    }
    if [l.env.0, l.env.1, l.env.2, l.delay].iter().any(|&t| !(0.0..=3.0).contains(&t)) {
        return Err("env and delay 0–3 s".into());
    }
    Ok(())
}

/// Renders one sound to mono samples in -1..1 at [`RATE`].
pub fn render(s: &SoundRecipe) -> Result<Vec<f32>, String> {
    if s.layers.is_empty() {
        return Err(format!("{}: no layers", s.name));
    }
    let sr = RATE as f32;
    let mut len = 0.0f32;
    for l in &s.layers {
        check_layer(l).map_err(|e| format!("{}: {e}", s.name))?;
        len = len.max(l.delay + l.env.0 + l.env.1 + l.env.2);
    }
    if let Some(e) = s.echo {
        // Enough tail for the echoes to fall under -48 dB.
        let repeats = (0.004f32.ln() / e.feedback.clamp(0.01, 0.9).ln()).ceil();
        len += e.delay * repeats;
    }
    let n = ((len.min(max_seconds()) * sr) as usize).max(1);
    let mut mix = vec![0.0f32; n];
    for (li, l) in s.layers.iter().enumerate() {
        let mut rng = Rng::new(&s.name, li);
        let (mut phase, mut held, mut lfsr, mut clock) = (0.0f32, rng.next(), 0x7fu8, 0.0f32);
        let (mut lp1, mut lp2, mut hp) = (0.0f32, 0.0f32, 0.0f32);
        let start = (l.delay * sr) as usize;
        let (a, h, d) = l.env;
        let body = a + h + d;
        for (i, out) in mix.iter_mut().enumerate().skip(start) {
            let t = (i - start) as f32 / sr;
            if t >= body {
                break;
            }
            let u = t / body.max(1e-4);
            // Exponential slide; vibrato on top.
            let mut f = l.freq.0 * (l.freq.1 / l.freq.0).powf(u);
            if let Some((depth, rate)) = l.vibrato {
                f *= 2f32.powf(depth / 12.0 * (TAU * rate * t).sin());
            }
            phase = (phase + f / sr).fract();
            clock += f / sr;
            let x = match l.wave.as_str() {
                "sine" => (TAU * phase).sin(),
                "triangle" => 1.0 - 4.0 * (phase - 0.5).abs(),
                "square" => {
                    if phase < l.duty.unwrap_or(0.5) {
                        1.0
                    } else {
                        -1.0
                    }
                }
                "saw" => 2.0 * phase - 1.0,
                "noise" => {
                    while clock >= 1.0 {
                        clock -= 1.0;
                        held = rng.next();
                    }
                    held
                }
                _ => {
                    // "metal": NES-style short-mode noise.
                    while clock >= 1.0 {
                        clock -= 1.0;
                        let bit = (lfsr ^ (lfsr >> 6)) & 1;
                        lfsr = (lfsr >> 1) | (bit << 6);
                    }
                    if lfsr & 1 == 1 { 1.0 } else { -1.0 }
                }
            };
            let env = if t < a {
                t / a.max(1e-4) * (1.0 + l.punch)
            } else if t < a + h {
                1.0 + l.punch * (1.0 - (t - a) / h.max(1e-4))
            } else {
                let k = 1.0 - (t - a - h) / d.max(1e-4);
                k * k
            };
            let mut y = x;
            if let Some((c0, c1)) = l.lowpass {
                let fc = (c0 * (c1 / c0).powf(u)).min(sr * 0.45);
                let k = 1.0 - (-TAU * fc / sr).exp();
                lp1 += k * (y - lp1);
                lp2 += k * (lp1 - lp2);
                y = lp2;
            }
            if let Some(fc) = l.highpass {
                let k = 1.0 - (-TAU * fc.min(sr * 0.45) / sr).exp();
                hp += k * (y - hp);
                y -= hp;
            }
            if l.drive > 0.0 {
                y = (y * (1.0 + l.drive * 4.0)).tanh();
            }
            *out += y * env * l.gain;
        }
    }
    if let Some(e) = s.echo {
        let d = ((e.delay * sr) as usize).max(1);
        for i in d..n {
            mix[i] += mix[i - d] * e.feedback.clamp(0.0, 0.9);
        }
    }
    if let Some(c) = s.crush {
        let hold = c.hold.clamp(1, 8) as usize;
        let steps = (1u32 << c.bits.clamp(4, 16).saturating_sub(1)) as f32;
        let mut last = 0.0;
        for (i, v) in mix.iter_mut().enumerate() {
            if i % hold == 0 {
                last = (*v * steps).round() / steps;
            }
            *v = last;
        }
    }
    let peak = mix.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    if peak <= 1e-6 {
        return Err(format!("{}: silent", s.name));
    }
    for v in &mut mix {
        *v *= PEAK / peak;
    }
    // Trim the silent tail, then a 5 ms fade so the end never clicks.
    let keep = mix.iter().rposition(|v| v.abs() > 0.001).map_or(1, |i| i + 1);
    mix.truncate(keep);
    let fade = ((0.005 * sr) as usize).min(mix.len());
    let len = mix.len();
    for (k, v) in mix[len - fade..].iter_mut().enumerate() {
        *v *= 1.0 - (k + 1) as f32 / fade as f32;
    }
    Ok(mix)
}

/// Sounds are capped by the pack format.
fn max_seconds() -> f32 {
    mftr_pack::sfx::MAX_SECONDS - 0.01
}

/// Encodes mono samples as Ogg Vorbis with a fixed stream serial (deterministic).
pub fn encode(samples: &[f32], serial: i32) -> Result<Vec<u8>, String> {
    let rate = NonZeroU32::new(RATE).ok_or("rate")?;
    let mono = NonZeroU8::new(1).ok_or("channels")?;
    let mut builder = vorbis_rs::VorbisEncoderBuilder::new_with_serial(rate, mono, Vec::new(), serial);
    builder.bitrate_management_strategy(vorbis_rs::VorbisBitrateManagementStrategy::QualityVbr {
        target_quality: QUALITY,
    });
    let mut enc = builder.build().map_err(|e| e.to_string())?;
    for block in samples.chunks(4096) {
        enc.encode_audio_block([block]).map_err(|e| e.to_string())?;
    }
    enc.finish().map_err(|e| e.to_string())
}

/// What one recipe produced.
pub struct Built {
    pub binding: PathBuf,
    pub sounds: usize,
    pub bytes: usize,
}

/// Builds the recipe at `recipe` (a `sounds.ron`) into `<dir>/export/`.
pub fn build(recipe: &Path) -> Result<Built, String> {
    let text = std::fs::read_to_string(recipe).map_err(|e| format!("{}: {e}", recipe.display()))?;
    let r = Recipe::deserialize_ron(&text).map_err(|e| format!("{}: {e}", recipe.display()))?;
    let export = recipe.parent().unwrap_or(Path::new(".")).join("export");
    let out_dir = export.join("sfx");
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let mut binding = format!(
        "// Generated by `mftr-tools sfx build` from ../sounds.ron. Don't edit: change the recipe.\n(\n    source: \"{}\",\n    sounds: [\n",
        mftr_pack::sfx::source_hash(&text)
    );
    let mut bytes = 0;
    for (i, s) in r.sounds.iter().enumerate() {
        let ogg = encode(&render(s)?, 0x4d46_0000 + i as i32)?;
        bytes += ogg.len();
        std::fs::write(out_dir.join(format!("{}.ogg", s.name)), &ogg).map_err(|e| e.to_string())?;
        let events: Vec<String> = s.events.iter().map(|e| format!("\"{e}\"")).collect();
        binding.push_str(&format!(
            "        (name: \"{}\", events: [{}], volume: {:.2}, pitch: {:.2}),\n",
            s.name,
            events.join(", "),
            s.volume,
            s.pitch
        ));
    }
    binding.push_str("    ],\n)\n");
    // Sounds the recipe no longer has.
    for e in std::fs::read_dir(&out_dir).map_err(|e| e.to_string())?.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let keep = name.strip_suffix(".ogg").is_some_and(|stem| r.sounds.iter().any(|s| s.name == stem));
        if !keep {
            std::fs::remove_file(e.path()).map_err(|e| e.to_string())?;
        }
    }
    let path = export.join(format!("{}.sfx.ron", r.pack));
    std::fs::write(&path, binding).map_err(|e| e.to_string())?;
    Ok(Built { binding: path, sounds: r.sounds.len(), bytes })
}

/// Every `sounds.ron` under `dir`, in name order.
pub fn find_recipes(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for p in paths {
        if p.is_dir() {
            find_recipes(&p, out);
        } else if p.file_name().is_some_and(|n| n == "sounds.ron") {
            out.push(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blip() -> SoundRecipe {
        SoundRecipe::deserialize_ron(
            r#"(name: "blip", events: ["q.cast"], volume: 0.5,
                crush: (bits: 8, hold: 2), echo: (delay: 0.05, feedback: 0.3),
                layers: [
                    (wave: "square", freq: (880.0, 440.0), env: (0.005, 0.02, 0.1), gain: 0.5),
                    (wave: "noise", freq: (8000.0, 2000.0), env: (0.0, 0.0, 0.05), gain: 0.3, lowpass: (6000.0, 1500.0)),
                    (wave: "metal", freq: (3000.0, 3000.0), env: (0.0, 0.01, 0.05), gain: 0.2, delay: 0.02, highpass: 500.0),
                ])"#,
        )
        .unwrap()
    }

    #[test]
    fn rendering_and_encoding_are_deterministic_and_decodable() {
        let a = render(&blip()).unwrap();
        assert_eq!(a, render(&blip()).unwrap());
        let peak = a.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!((peak - PEAK).abs() < 0.01, "{peak}");
        let ogg = encode(&a, 7).unwrap();
        assert_eq!(ogg, encode(&a, 7).unwrap());
        let (rate, ch, pcm) = mftr_pack::sfx::decode(&ogg).unwrap();
        assert_eq!((rate, ch), (RATE, 1));
        assert_eq!(pcm.len(), a.len());
    }

    #[test]
    fn bad_layers_are_named() {
        let mut s = blip();
        s.layers[0].wave = "laser".into();
        assert!(render(&s).unwrap_err().contains("wave `laser`"));
    }
}
