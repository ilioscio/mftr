//! `mftr-pack`: the content-pack format and its validator (docs/design/11-content-packs-and-mods.md).
//!
//! This crate is the only code that reads pack bytes, on the server, on the client and in
//! `mftr-tools`. A1 scope: the model (`<id>.glb`, a strict glTF binary subset) and its marker
//! sidecar (`<id>.anims.ron`), checked against the rig standard, the caps, the clip catalogue
//! and the timing markers (10 §8.4). The container, signatures and push come later (11 §8).

pub mod animator;
pub mod glb;
pub mod json;
pub mod pose;
pub mod rules;
pub mod sidecar;
pub mod validate;
pub mod vfx;

pub use validate::{Finding, Level, Report, validate, validate_model};

use std::path::{Path, PathBuf};

/// The sidecar that belongs to a `.glb`: `<id>.glb` → `<id>.anims.ron`.
pub fn sidecar_path(glb: &Path) -> PathBuf {
    glb.with_extension("anims.ron")
}

/// A validated pack, parsed and ready to animate: the model (mesh data) and its clip library.
pub struct Loaded {
    pub model: glb::Model,
    pub library: pose::Library,
    /// `<id>.vfx.ron`, when the pack ships one (A4b).
    pub vfx: Option<vfx::VfxFile>,
}

/// A pack's optional VFX file: `<id>.glb` → `<id>.vfx.ron`.
pub fn vfx_path(glb: &Path) -> PathBuf {
    glb.with_extension("vfx.ron")
}

/// Reads, validates and parses `<id>.glb` and its sidecar. Refuses anything with errors: the
/// client never draws an invalid pack (11 §4).
pub fn load_file(glb_path: &Path) -> Result<Loaded, String> {
    let report = validate_file(glb_path);
    if report.errors() > 0 {
        let first = report.findings.iter().find(|f| f.level == Level::Error).map_or(String::new(), |f| f.msg.clone());
        return Err(format!("{}: {} errors (first: {first})", glb_path.display(), report.errors()));
    }
    let bytes = std::fs::read(glb_path).map_err(|e| e.to_string())?;
    let text = std::fs::read_to_string(sidecar_path(glb_path)).map_err(|e| e.to_string())?;
    let model = glb::parse(&bytes)?;
    let side = sidecar::parse(&text)?;
    let library = pose::Library::new(&model, &side)?;
    let vfx_file = vfx_path(glb_path);
    let vfx = if vfx_file.exists() {
        Some(vfx::parse(&std::fs::read_to_string(&vfx_file).map_err(|e| e.to_string())?)?)
    } else {
        None
    };
    Ok(Loaded { model, library, vfx })
}

/// Reads and validates `<id>.glb` and its sidecar from disk.
pub fn validate_file(glb_path: &Path) -> Report {
    let read = |p: &Path| std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()));
    let side_path = sidecar_path(glb_path);
    let mut report = match (read(glb_path), read(&side_path)) {
        (Ok(g), Ok(s)) => match String::from_utf8(s) {
            Ok(text) => validate(&g, &text),
            Err(_) => error_report(format!("{}: not UTF-8", side_path.display())),
        },
        (Err(e), _) | (_, Err(e)) => error_report(e),
    };
    let vfx_file = vfx_path(glb_path);
    if vfx_file.exists() {
        let problems = match std::fs::read_to_string(&vfx_file).map_err(|e| e.to_string()).and_then(|t| vfx::parse(&t))
        {
            Ok(f) => {
                report.summary.push(format!("vfx: {} effects", f.effects.len()));
                vfx::check(&f)
            }
            Err(e) => vec![e],
        };
        report.findings.extend(problems.into_iter().map(|msg| Finding { level: Level::Error, msg }));
    }
    report
}

fn error_report(msg: String) -> Report {
    Report { findings: vec![Finding { level: Level::Error, msg }], summary: Vec::new() }
}
