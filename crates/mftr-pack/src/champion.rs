//! Champion files (CR1b, 11 §3.3, 20 §11.1): a champion made in the creator, or by hand, as one
//! `<id>.champion.ron`. Format 0 holds the kit and stats only (`Champion(format: 0, ...)`,
//! read by `mftr_sim::data`); looks arrive with the creator's later formats.
//!
//! A file is accepted when it reads (the strict reader) and passes the limits every kit must
//! respect (`mftr_sim::limits`: D10, D36, D44 and bounds). Accepting it registers the champion,
//! so its name works wherever champions are picked by name.

use std::path::Path;

use mftr_sim::ChampionId;
use mftr_sim::champion::ChampionDef;

use crate::validate::{Finding, Level, Report};

/// The suffix of a champion file.
pub const SUFFIX: &str = ".champion.ron";

/// Whether `path` names a champion file.
pub fn is_champion_file(path: &Path) -> bool {
    path.file_name().is_some_and(|n| n.to_string_lossy().ends_with(SUFFIX))
}

/// Read and check a champion file's text. `Err` lists every problem.
pub fn read(text: &str) -> Result<ChampionDef, Vec<String>> {
    let def = mftr_sim::data::read_champion(text).map_err(|e| vec![e.to_string()])?;
    let mut problems = mftr_sim::limits::problems(&def);
    if let Some(over) = mftr_sim::balance::over(&def) {
        problems.push(format!(
            "costs {:.0} points, over the {:.0}-point budget by {over:.0} (20 §9)",
            mftr_sim::balance::price(&def).total(),
            mftr_sim::balance::BUDGET
        ));
    }
    if problems.is_empty() { Ok(def) } else { Err(problems) }
}

/// A champion file's report for `mftr-tools pack validate`.
pub fn validate_text(text: &str) -> Report {
    let mut report = Report::default();
    match read(text) {
        Ok(def) => {
            let names: Vec<&str> = def.abilities.iter().map(|a| a.name).collect();
            report.summary.push(format!("champion `{}`: {}", def.name, names.join(" / ")));
            report.summary.push(format!(
                "price {:.1} of {:.0} points",
                mftr_sim::balance::price(&def).total(),
                mftr_sim::balance::BUDGET
            ));
            if ChampionId::ALL.iter().any(|c| c.def().name.eq_ignore_ascii_case(def.name) && *c.def() != def) {
                report.findings.push(Finding {
                    level: Level::Warning,
                    msg: format!("`{}` is an official champion's name; by name, the official one is picked", def.name),
                });
            }
        }
        Err(problems) => {
            report.findings.extend(problems.into_iter().map(|msg| Finding { level: Level::Error, msg }));
        }
    }
    report
}

/// [`validate_text`] on a file.
pub fn validate_file(path: &Path) -> Report {
    match std::fs::read_to_string(path) {
        Ok(text) => validate_text(&text),
        Err(e) => {
            let mut r = Report::default();
            r.findings.push(Finding { level: Level::Error, msg: format!("{}: {e}", path.display()) });
            r
        }
    }
}

/// Read, check and register a champion file: its id (the same one if it was registered
/// already).
pub fn load_text(text: &str) -> Result<ChampionId, String> {
    let def = read(text).map_err(|p| p.join("; "))?;
    ChampionId::register(def).map_err(|_| "too many champions registered".to_string())
}

/// [`load_text`] on a file.
pub fn load_file(path: &Path) -> Result<ChampionId, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    load_text(&text).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rookish(name: &str, speed: f32) -> String {
        let mut d = *ChampionId::Rook.def();
        d.name = Box::leak(name.to_string().into_boxed_str());
        d.stats.move_speed = speed;
        mftr_sim::data::write_champion(&d, "a test champion")
    }

    #[test]
    fn a_good_file_loads_and_registers() {
        let text = rookish("Pack Test Brute", 330.0);
        assert_eq!(validate_text(&text).errors(), 0);
        let id = load_text(&text).unwrap();
        assert_eq!(ChampionId::by_name("pack test brute"), Some(id));
        assert_eq!(load_text(&text), Ok(id));
    }

    #[test]
    fn a_bad_file_says_why() {
        // Too fast for the bounds (and so, over the budget too).
        let report = validate_text(&rookish("Pack Test Speedy", 900.0));
        assert_eq!(report.errors(), 2, "{:?}", report.findings);
        assert!(report.findings[0].msg.contains("stats.move_speed"), "{:?}", report.findings);
        assert!(report.findings[1].msg.contains("budget"), "{:?}", report.findings);
        let report = validate_text("Champion(format: 0)");
        assert!(report.findings[0].msg.contains("`name` is missing"), "{:?}", report.findings);
        assert!(load_text(&rookish("Pack Test Speedy", 900.0)).is_err());
    }

    #[test]
    fn an_official_name_warns() {
        let report = validate_text(&rookish("Rook", 330.0));
        assert_eq!((report.errors(), report.warnings()), (0, 1));
    }

    #[test]
    fn the_example_file_loads() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../art/champions/examples/grumble.champion.ron");
        let report = validate_file(&path);
        assert_eq!(report.errors(), 0, "{:?}", report.findings);
        assert!(is_champion_file(&path));
    }
}
