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
pub mod sfx;
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
    /// The sidecar's `kind`: `champion`, `library`, `minion` or `rig`.
    pub kind: String,
    pub model: glb::Model,
    pub library: pose::Library,
    /// `<id>.vfx.ron`, when the pack ships one (A4b).
    pub vfx: Option<vfx::VfxFile>,
    /// The decoded sounds of `<id>.sfx.ron` and `sfx/*.ogg`, when the pack ships them (A4c).
    pub sounds: Vec<sfx::Sound>,
}

/// A pack's optional VFX file: `<id>.glb` → `<id>.vfx.ron`.
pub fn vfx_path(glb: &Path) -> PathBuf {
    glb.with_extension("vfx.ron")
}

/// A pack's optional sound bindings: `<id>.glb` → `<id>.sfx.ron`, with the files in `sfx/`.
pub fn sfx_path(glb: &Path) -> PathBuf {
    glb.with_extension("sfx.ron")
}

/// The folder of a pack's `.ogg` files.
pub fn sfx_dir(glb: &Path) -> PathBuf {
    glb.parent().unwrap_or(Path::new(".")).join("sfx")
}

/// Reads, validates and parses `<id>.glb` and its sidecar. Refuses anything with errors: the
/// client never draws an invalid pack (11 §4).
pub fn load_file(glb_path: &Path) -> Result<Loaded, String> {
    let (report, sounds) = validate_with_sounds(glb_path);
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
    Ok(Loaded { kind: side.kind.clone(), model, library, vfx, sounds })
}

/// Reads and validates `<id>.glb` and its sidecar from disk.
pub fn validate_file(glb_path: &Path) -> Report {
    validate_with_sounds(glb_path).0
}

fn validate_with_sounds(glb_path: &Path) -> (Report, Vec<sfx::Sound>) {
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
    let (sounds, problems) = read_sounds(glb_path, &mut report.summary);
    report.findings.extend(problems.into_iter().map(|msg| Finding { level: Level::Error, msg }));
    (report, sounds)
}

/// `<id>.sfx.ron` and `sfx/*.ogg`, checked and decoded; `sfx/` without a binding file is an error.
fn read_sounds(glb_path: &Path, summary: &mut Vec<String>) -> (Vec<sfx::Sound>, Vec<String>) {
    let (bind, dir) = (sfx_path(glb_path), sfx_dir(glb_path));
    let mut present = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            match name.strip_suffix(".ogg") {
                Some(stem) => present.push(stem.to_string()),
                None => return (Vec::new(), vec![format!("sfx/{name}: only .ogg files belong in sfx/")]),
            }
        }
    }
    present.sort();
    if !bind.exists() {
        let problems =
            if present.is_empty() { Vec::new() } else { vec![format!("sfx/ has sounds but no {}", bind.display())] };
        return (Vec::new(), problems);
    }
    let file = match std::fs::read_to_string(&bind).map_err(|e| e.to_string()).and_then(|t| sfx::parse(&t)) {
        Ok(f) => f,
        Err(e) => return (Vec::new(), vec![e]),
    };
    let read = |name: &str| {
        let p = dir.join(format!("{name}.ogg"));
        let len = std::fs::metadata(&p).map_err(|e| e.to_string())?.len() as usize;
        if len > sfx::MAX_FILE_BYTES {
            return Err(format!("{len} bytes, over the {} byte cap", sfx::MAX_FILE_BYTES));
        }
        std::fs::read(&p).map_err(|e| e.to_string())
    };
    let (sounds, total, problems) = sfx::load(&file, read, &present);
    summary.push(format!("sfx: {} sounds, {:.1} KB", file.sounds.len(), total as f32 / 1000.0));
    (sounds, problems)
}

fn error_report(msg: String) -> Report {
    Report { findings: vec![Finding { level: Level::Error, msg }], summary: Vec::new() }
}
