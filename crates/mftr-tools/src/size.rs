//! Size report and budgets (M2 slice 6; 00 vision: a v1.0 download under 150 MB, M2 exit: the
//! client download ≤ 60 MB). CI builds the `dist` profile and fails when something grows past
//! its budget, so size creep is a decision, not an accident.

use std::path::{Path, PathBuf};

const MIB: u64 = 1024 * 1024;

/// One measured item: what, where, how big, and its budget.
#[derive(Clone, Debug)]
pub struct SizeItem {
    pub name: &'static str,
    pub path: PathBuf,
    pub bytes: Option<u64>,
    pub budget: u64,
}

impl SizeItem {
    pub fn over(&self) -> bool {
        self.bytes.is_some_and(|b| b > self.budget)
    }
}

/// Total size of a file, or of every file under a directory (skipping `skip` names).
pub fn size_of(path: &Path, skip: &[&str]) -> Option<u64> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.is_file() {
        return Some(meta.len());
    }
    let mut total = 0;
    for entry in std::fs::read_dir(path).ok()?.flatten() {
        if skip.iter().any(|s| entry.file_name() == *s) {
            continue;
        }
        total += size_of(&entry.path(), skip).unwrap_or(0);
    }
    Some(total)
}

/// The first of several platform names that exists (binary and library names differ per OS).
fn first_existing(dir: &Path, names: &[&str]) -> PathBuf {
    names.iter().map(|n| dir.join(n)).find(|p| p.exists()).unwrap_or_else(|| dir.join(names[0]))
}

/// Measure the shipped pieces: `target/dist` binaries, the Godot project, and optionally an
/// exported client package (a directory or archive).
pub fn report(root: &Path, client_package: Option<&Path>) -> Vec<SizeItem> {
    let dist = root.join("target").join("dist");
    let mut items = vec![
        SizeItem {
            name: "server binary",
            path: first_existing(&dist, &["mftr-server", "mftr-server.exe"]),
            bytes: None,
            budget: 5 * MIB,
        },
        SizeItem {
            name: "tools binary",
            path: first_existing(&dist, &["mftr-tools", "mftr-tools.exe"]),
            bytes: None,
            budget: 5 * MIB,
        },
        SizeItem {
            name: "Godot extension",
            path: first_existing(&dist, &["libmftr_gdext.so", "mftr_gdext.dll", "libmftr_gdext.dylib"]),
            bytes: None,
            budget: 15 * MIB,
        },
        SizeItem {
            name: "client project (scripts, shaders)",
            path: root.join("client"),
            bytes: None,
            budget: 20 * MIB,
        },
    ];
    if let Some(p) = client_package {
        items.push(SizeItem { name: "client download (export)", path: p.to_path_buf(), bytes: None, budget: 60 * MIB });
    }
    for item in &mut items {
        item.bytes = size_of(&item.path, &[".godot"]);
    }
    items
}

/// A markdown table (for the CI step summary).
pub fn table(items: &[SizeItem]) -> String {
    let mut s = String::from("| Item | Size | Budget | |\n|---|---:|---:|---|\n");
    for i in items {
        let size = i.bytes.map_or("missing".to_string(), |b| format!("{:.2} MiB", b as f64 / MIB as f64));
        let mark = match i.bytes {
            None => "not built",
            Some(_) if i.over() => "**over budget**",
            Some(_) => "ok",
        };
        s += &format!("| {} | {size} | {} MiB | {mark} |\n", i.name, i.budget / MIB);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_directories_and_flags_overruns() {
        let root = std::env::temp_dir().join(format!("mftr-size-{}", std::process::id()));
        let client = root.join("client");
        std::fs::create_dir_all(client.join(".godot")).unwrap();
        std::fs::write(client.join("a.gd"), vec![0u8; 1000]).unwrap();
        std::fs::write(client.join(".godot").join("cache"), vec![0u8; 5000]).unwrap();
        let items = report(&root, None);
        let c = items.iter().find(|i| i.name.starts_with("client project")).unwrap();
        assert_eq!(c.bytes, Some(1000), "the editor cache doesn't count");
        assert!(items.iter().any(|i| i.bytes.is_none()), "unbuilt binaries are reported as missing");
        let big = SizeItem { name: "x", path: root.clone(), bytes: Some(70 * MIB), budget: 60 * MIB };
        assert!(big.over() && table(&[big]).contains("over budget"));
        std::fs::remove_dir_all(&root).unwrap();
    }
}
