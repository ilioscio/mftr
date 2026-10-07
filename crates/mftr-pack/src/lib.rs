//! `mftr-pack`: the content-pack format and its validator (docs/design/11-content-packs-and-mods.md).
//!
//! This crate is the only code that reads pack bytes, on the server, on the client and in
//! `mftr-tools`. A1 scope: the model (`<id>.glb`, a strict glTF binary subset) and its marker
//! sidecar (`<id>.anims.ron`), checked against the rig standard, the caps, the clip catalogue
//! and the timing markers (10 §8.4). The container, signatures and push come later (11 §8).

pub mod glb;
pub mod json;
pub mod rules;
pub mod sidecar;
pub mod validate;

pub use validate::{Finding, Level, Report, validate, validate_model};

use std::path::{Path, PathBuf};

/// The sidecar that belongs to a `.glb`: `<id>.glb` → `<id>.anims.ron`.
pub fn sidecar_path(glb: &Path) -> PathBuf {
    glb.with_extension("anims.ron")
}

/// Reads and validates `<id>.glb` and its sidecar from disk.
pub fn validate_file(glb_path: &Path) -> Report {
    let read = |p: &Path| std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()));
    let side_path = sidecar_path(glb_path);
    match (read(glb_path), read(&side_path)) {
        (Ok(g), Ok(s)) => match String::from_utf8(s) {
            Ok(text) => validate(&g, &text),
            Err(_) => error_report(format!("{}: not UTF-8", side_path.display())),
        },
        (Err(e), _) | (_, Err(e)) => error_report(e),
    }
}

fn error_report(msg: String) -> Report {
    Report { findings: vec![Finding { level: Level::Error, msg }], summary: Vec::new() }
}
