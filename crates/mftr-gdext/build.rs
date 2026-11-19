//! `MFTR_FORGE_BUILD`: a hash of the code a forged model depends on (the forge, the pack format
//! and the sim's champion data). It's part of every forged-model cache key (D78), so a client
//! built from changed forge code rebuilds its champions rather than load ones the old code made.

use std::path::{Path, PathBuf};

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            files(&p, out);
        } else {
            out.push(p);
        }
    }
}

fn main() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut all = Vec::new();
    for dir in ["mftr-forge/src", "mftr-pack/src", "mftr-sim/src", "mftr-sim/champions"] {
        let d = crates.join(dir);
        println!("cargo:rerun-if-changed={}", d.display());
        files(&d, &mut all);
    }
    all.sort();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for p in &all {
        let name = p.strip_prefix(&crates).unwrap_or(p).to_string_lossy().replace('\\', "/");
        for b in name.bytes().chain(std::fs::read(p).unwrap_or_default()) {
            h ^= b as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    println!("cargo:rustc-env=MFTR_FORGE_BUILD={h:016x}");
}
