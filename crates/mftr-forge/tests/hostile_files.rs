//! Hostile champion files (D83, 20 §13.1): a champion file may come from any server or, once
//! players bring their own, from any player. This mutates the example and official files
//! (bytes flipped, ranges cut, repeated and spliced, tokens dropped in, deep nesting, huge and
//! odd numbers) and runs every mutant through what reads them: the strict reader with the
//! limits and the price (`mftr_sim::limits`), the looks reader, and for a sample the forge.
//!
//! Nothing may panic, and nothing may take long: a file is refused with a reason or accepted.
//! No fuzzing crate (D83 adds no dependency): a small seeded generator, so a failure
//! reproduces. `MFTR_FUZZ_ROUNDS` runs more mutants (the default keeps `cargo test` quick).

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// xorshift64*: deterministic, seeded per round.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

const TOKENS: &[&str] = &[
    "(",
    ")",
    "[",
    "]",
    "{",
    "}",
    ",",
    ":",
    "\"",
    "\\",
    "//",
    "/*",
    "-",
    ".",
    "0",
    "-0",
    "1e39",
    "-1e39",
    "1e309",
    "NaN",
    "inf",
    "-inf",
    "18446744073709551616",
    "4294967295",
    "65535",
    "255",
    "256",
    "-1",
    "0.0000001",
    "Some(",
    "None",
    "true",
    "Masculine",
    "Feminine",
    "looks: (",
    "images: [",
    "sculpt: [",
    "outfit: [",
    "paint: [",
    "shape: (",
    "face: (",
    "body: (",
    "moves: (",
    "\"body\"",
    "\"humanoid_slim\"",
    "\"humanoid_heavy\"",
    "\"../../x\"",
    "\"C:/x\"",
    "\u{0}",
    "\u{202e}",
    "é",
    "\u{fffd}",
];

fn corpus() -> Vec<String> {
    let art = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../art/champions");
    let mut files: Vec<PathBuf> = Vec::new();
    for dir in std::fs::read_dir(&art).unwrap().flatten() {
        if let Ok(inner) = std::fs::read_dir(dir.path()) {
            files.extend(inner.flatten().map(|e| e.path()).filter(|p| p.to_string_lossy().ends_with(".champion.ron")));
        }
    }
    files.sort();
    let texts: Vec<String> = files.iter().map(|p| std::fs::read_to_string(p).unwrap()).collect();
    assert!(texts.len() >= 10, "the example and official champion files: found {}", texts.len());
    texts
}

/// One mutant of `base` (another corpus file for splices).
fn mutate(rng: &mut Rng, base: &str, other: &str) -> String {
    let mut b = base.as_bytes().to_vec();
    for _ in 0..1 + rng.below(4) {
        let at = rng.below(b.len() + 1);
        match rng.below(9) {
            0 if !b.is_empty() => {
                let i = rng.below(b.len());
                b[i] ^= 1 << rng.below(8);
            }
            1 if !b.is_empty() => {
                let n = 1 + rng.below(64.min(b.len()));
                let i = rng.below(b.len() - n + 1);
                b.drain(i..i + n);
            }
            2 if !b.is_empty() => {
                // Repeat a range (lists past their caps).
                let n = 1 + rng.below(200.min(b.len()));
                let i = rng.below(b.len() - n + 1);
                let piece: Vec<u8> = b[i..i + n].to_vec();
                for _ in 0..1 + rng.below(40) {
                    b.splice(i..i, piece.iter().copied());
                }
            }
            3 => {
                let o = other.as_bytes();
                let i = rng.below(o.len());
                let n = rng.below(o.len() - i + 1).min(400);
                b.splice(at..at, o[i..i + n].iter().copied());
            }
            4 => b.truncate(at),
            5 => {
                // Deep nesting.
                let depth = 1 + rng.below(64);
                let open = if rng.below(2) == 0 { b'(' } else { b'[' };
                b.splice(at..at, std::iter::repeat_n(open, depth));
            }
            6 => {
                // A number swapped for an odd one.
                let t = TOKENS[rng.below(TOKENS.len())];
                if let Some(i) = (at..b.len()).find(|&i| b[i].is_ascii_digit()) {
                    let end = (i..b.len()).find(|&j| !(b[j].is_ascii_digit() || b[j] == b'.')).unwrap_or(b.len());
                    b.splice(i..end, t.bytes());
                }
            }
            _ => {
                let t = TOKENS[rng.below(TOKENS.len())];
                b.splice(at..at, t.bytes());
            }
        }
    }
    // As the client takes a fetched file.
    String::from_utf8_lossy(&b).into_owned()
}

fn rounds() -> usize {
    std::env::var("MFTR_FUZZ_ROUNDS").ok().and_then(|v| v.parse().ok()).unwrap_or(3000)
}

/// A file is refused or read, within this, by the reader, the limits and the price together.
const READ_CAP: Duration = Duration::from_millis(250);
/// The forge's cap on a hostile file: well above a real champion's few seconds, so a test
/// machine's load doesn't trip it, while a blow-up does.
const FORGE_CAP: Duration = Duration::from_secs(30);

#[test]
fn hostile_champion_files_are_refused_never_crash() {
    let corpus = corpus();
    let mut read = 0;
    let mut looks_ok = Vec::new();
    for round in 0..rounds() {
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15 ^ (round as u64).wrapping_mul(0x1000_0000_01b3));
        let base = &corpus[rng.below(corpus.len())];
        let other = &corpus[rng.below(corpus.len())];
        let text = mutate(&mut rng, base, other);

        let t = Instant::now();
        // The guard inside turns a panic into this refusal; it must never be needed.
        let r = mftr_sim::limits::read_champion_file(&text);
        if let Err(e) = &r {
            assert!(!e.contains("broke the reader"), "round {round}: the reader panicked\n{text}");
        } else {
            read += 1;
        }
        let looks = catch_unwind(AssertUnwindSafe(|| mftr_forge::looks::of_champion_file(&text)));
        let looks = looks.unwrap_or_else(|_| panic!("round {round}: the looks reader panicked\n{text}"));
        let took = t.elapsed();
        assert!(took < READ_CAP, "round {round}: reading took {took:?}\n{text}");
        if let Ok(Some(l)) = looks
            && looks_ok.len() < 400
        {
            looks_ok.push((round, l));
        }
    }
    eprintln!("{} mutants: {read} kits read, {} looks read (of the first 400)", rounds(), looks_ok.len());
    // Mutants that still read have to exist, or this tests only the first error.
    assert!(read > 0 && !looks_ok.is_empty(), "no mutant read: {read} kits, {} looks", looks_ok.len());

    // The forge, on a spread of mutants whose looks read (forging takes seconds each).
    let art = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../art");
    let cat = mftr_forge::Catalog::load(&art).unwrap();
    let step = (looks_ok.len() / 6).max(1);
    for (round, l) in looks_ok.iter().step_by(step).take(6) {
        let t = Instant::now();
        let r = catch_unwind(AssertUnwindSafe(|| cat.forge("Fuzzed", l)));
        assert!(r.is_ok(), "round {round}: the forge panicked on {l:?}");
        let took = t.elapsed();
        assert!(took < FORGE_CAP, "round {round}: forging took {took:?}");
    }
}
