//! The Windows client updating itself (D66): install a newer release's archive over the running
//! game.
//!
//! The client downloads the archive the signed manifest names (`update`) into `.update/` in its
//! install folder, then [`install`] checks its size and SHA-256 against the manifest, unpacks it
//! into `.update/staged/`, and swaps it in: every top-level entry of the install folder that the
//! new release has (`mftr.exe`, `mftr_gdext.dll`, `art/`) is renamed into `.update/old/`, and the
//! new one renamed into its place. Windows lets a running program and its loaded libraries be
//! renamed, not overwritten, which is why it renames. If any step fails, what was moved goes back,
//! so the game that was there keeps working. The client then starts the new `mftr.exe` and quits;
//! the next start removes `.update/` ([`clean`]).
//!
//! The player's data (settings, known servers, replays) lives in Godot's user folder, not here, so
//! it carries over. macOS players install a new version as they did the first one, and Linux
//! players through their package manager.

use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The folder in the install folder that holds a download, the unpacked release and the files
/// it replaced.
pub const UPDATE_DIR: &str = ".update";
/// What a Windows release must hold to be installed: the game, its extension, and the packs.
pub const WINDOWS_FILES: [&str; 3] = ["mftr.exe", "mftr_gdext.dll", "art/library/biped/export/biped_library.glb"];

pub fn update_dir(install: &Path) -> PathBuf {
    install.join(UPDATE_DIR)
}

/// Whether the install folder takes an update: a file can be made in it (not in a folder only an
/// administrator may write, like Program Files). Leaves nothing behind.
pub fn writable(install: &Path) -> bool {
    let probe = install.join(".update-probe");
    fs::write(&probe, b"").is_ok() && fs::remove_file(&probe).is_ok()
}

/// A file's SHA-256, in hex.
pub fn sha256_file(path: &Path) -> io::Result<String> {
    let mut hasher = Sha256::new();
    io::copy(&mut fs::File::open(path)?, &mut hasher)?;
    Ok(crate::update::hex(&hasher.finalize()))
}

/// Install the downloaded `archive` (whose size and SHA-256 the signed manifest gave) over the
/// game in `install`, if it holds every one of `required`. On an error, the game there is as it
/// was.
pub fn install(archive: &Path, size: u64, sha256: &str, install: &Path, required: &[&str]) -> Result<(), String> {
    let staged = stage(archive, size, sha256, install, required)?;
    swap(install, &staged, &mut |from, to| fs::rename(from, to))?;
    let _ = fs::remove_dir_all(&staged);
    let _ = fs::remove_file(archive);
    Ok(())
}

/// Remove what an earlier update left in `.update/` (the replaced files: their program has
/// exited by now). Best effort.
pub fn clean(install: &Path) {
    let _ = fs::remove_dir_all(update_dir(install));
}

/// Check the archive and unpack it into `.update/staged/`.
fn stage(archive: &Path, size: u64, sha256: &str, install: &Path, required: &[&str]) -> Result<PathBuf, String> {
    let got = fs::metadata(archive).map_err(|e| format!("the download is missing: {e}"))?.len();
    if got != size {
        return Err(format!("the download is {got} bytes, not {size}"));
    }
    let hash = sha256_file(archive).map_err(|e| format!("the download doesn't read: {e}"))?;
    if !hash.eq_ignore_ascii_case(sha256) {
        return Err("the download isn't the release's archive (its SHA-256 differs)".into());
    }
    let staged = update_dir(install).join("staged");
    if staged.exists() {
        fs::remove_dir_all(&staged).map_err(|e| format!("could not clear {}: {e}", staged.display()))?;
    }
    fs::create_dir_all(&staged).map_err(|e| format!("could not make {}: {e}", staged.display()))?;
    unzip(archive, &staged)?;
    if let Some(missing) = required.iter().find(|f| !staged.join(f).is_file()) {
        return Err(format!("the archive has no {missing}"));
    }
    Ok(staged)
}

fn unzip(archive: &Path, dest: &Path) -> Result<(), String> {
    let file = fs::File::open(archive).map_err(|e| e.to_string())?;
    let mut zip =
        zip::ZipArchive::new(io::BufReader::new(file)).map_err(|e| format!("the archive doesn't read: {e}"))?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| format!("the archive doesn't read: {e}"))?;
        // Names that would leave the folder ("../", absolute) are refused.
        let rel = entry
            .enclosed_name()
            .ok_or_else(|| format!("the archive has an unsafe name: {}", entry.name().unwrap_or_default()))?;
        let path = dest.join(rel);
        if entry.is_dir() {
            fs::create_dir_all(&path).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out = fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        io::copy(&mut entry, &mut out).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

/// Move each top-level entry of `staged` into `install`, the entry it replaces into
/// `.update/old/` first. On a failed rename, undo every one done so far.
fn swap(install: &Path, staged: &Path, rename: &mut dyn FnMut(&Path, &Path) -> io::Result<()>) -> Result<(), String> {
    let old = update_dir(install).join("old");
    if old.exists() {
        fs::remove_dir_all(&old).map_err(|e| format!("could not clear {}: {e}", old.display()))?;
    }
    fs::create_dir_all(&old).map_err(|e| format!("could not make {}: {e}", old.display()))?;
    let mut names: Vec<_> = fs::read_dir(staged).map_err(|e| e.to_string())?.flatten().map(|e| e.file_name()).collect();
    names.sort();
    // Renames done, in order, to undo in reverse.
    let mut done: Vec<(PathBuf, PathBuf)> = Vec::new();
    for name in names {
        let (current, new) = (install.join(&name), staged.join(&name));
        let mut steps = Vec::new();
        if current.exists() {
            steps.push((current.clone(), old.join(&name)));
        }
        steps.push((new, current));
        for (from, to) in steps {
            if let Err(e) = rename(&from, &to) {
                for (from, to) in done.iter().rev() {
                    let _ = rename(to, from);
                }
                return Err(format!("could not replace {}: {e}", from.display()));
            }
            done.push((from, to));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// A fresh folder under the system temp folder.
    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mftr-selfupdate-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A zip of `files` (path, contents) at `path`.
    fn zip_of(path: &Path, files: &[(&str, &str)]) {
        let mut z = zip::ZipWriter::new(fs::File::create(path).unwrap());
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, text) in files {
            z.start_file(*name, opts).unwrap();
            z.write_all(text.as_bytes()).unwrap();
        }
        z.finish().unwrap();
    }

    fn release(version: &str) -> Vec<(&'static str, String)> {
        vec![
            ("mftr.exe", format!("exe {version}")),
            ("mftr_gdext.dll", format!("dll {version}")),
            ("art/library/biped/export/biped_library.glb", format!("glb {version}")),
        ]
    }

    fn install_folder(dir: &Path, version: &str) {
        for (name, text) in release(version) {
            let p = dir.join(name);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, text).unwrap();
        }
    }

    fn download(dir: &Path, version: &str) -> (PathBuf, u64, String) {
        let path = update_dir(dir).join("new.zip");
        fs::create_dir_all(update_dir(dir)).unwrap();
        let files = release(version);
        let refs: Vec<(&str, &str)> = files.iter().map(|(n, t)| (*n, t.as_str())).collect();
        zip_of(&path, &refs);
        (path.clone(), fs::metadata(&path).unwrap().len(), sha256_file(&path).unwrap())
    }

    fn read(dir: &Path, name: &str) -> String {
        fs::read_to_string(dir.join(name)).unwrap()
    }

    #[test]
    fn a_release_replaces_the_game_and_keeps_the_rest() {
        let dir = temp("replace");
        install_folder(&dir, "0.17.0");
        fs::write(dir.join("notes.txt"), "mine").unwrap();
        assert!(writable(&dir));
        assert!(!dir.join(".update-probe").exists() && !update_dir(&dir).exists(), "checking leaves nothing");
        let (zip, size, sha) = download(&dir, "0.18.0");
        install(&zip, size, &sha, &dir, &WINDOWS_FILES).unwrap();
        assert_eq!(read(&dir, "mftr.exe"), "exe 0.18.0");
        assert_eq!(read(&dir, "mftr_gdext.dll"), "dll 0.18.0");
        assert_eq!(read(&dir, "art/library/biped/export/biped_library.glb"), "glb 0.18.0");
        assert_eq!(read(&dir, "notes.txt"), "mine", "files the release doesn't have stay");
        // The replaced files wait in .update/old until the next start cleans up.
        assert_eq!(read(&update_dir(&dir), "old/mftr.exe"), "exe 0.17.0");
        assert!(!zip.exists() && !update_dir(&dir).join("staged").exists());
        clean(&dir);
        assert!(!update_dir(&dir).exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_download_that_isnt_the_release_changes_nothing() {
        let dir = temp("refuse");
        install_folder(&dir, "0.17.0");
        let (zip, size, sha) = download(&dir, "0.18.0");
        let wrong = "00".repeat(32);
        assert!(install(&zip, size, &wrong, &dir, &WINDOWS_FILES).unwrap_err().contains("SHA-256"));
        assert!(install(&zip, size + 1, &sha, &dir, &WINDOWS_FILES).unwrap_err().contains("bytes"));
        // Signed and whole, but not a Windows release.
        let partial = update_dir(&dir).join("partial.zip");
        zip_of(&partial, &[("mftr.exe", "exe 0.18.0")]);
        let (n, h) = (fs::metadata(&partial).unwrap().len(), sha256_file(&partial).unwrap());
        assert!(install(&partial, n, &h, &dir, &WINDOWS_FILES).unwrap_err().contains("mftr_gdext.dll"));
        // A name that would leave the folder.
        let evil = update_dir(&dir).join("evil.zip");
        zip_of(&evil, &[("../escaped.txt", "x")]);
        let (n, h) = (fs::metadata(&evil).unwrap().len(), sha256_file(&evil).unwrap());
        assert!(install(&evil, n, &h, &dir, &WINDOWS_FILES).unwrap_err().contains("unsafe"));
        assert!(!dir.join("../escaped.txt").exists());
        assert_eq!(read(&dir, "mftr.exe"), "exe 0.17.0");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_failed_rename_puts_everything_back() {
        let dir = temp("rollback");
        install_folder(&dir, "0.17.0");
        let (zip, size, sha) = download(&dir, "0.18.0");
        let staged = stage(&zip, size, &sha, &dir, &WINDOWS_FILES).unwrap();
        // Entries go in name order: art, mftr.exe, mftr_gdext.dll. The fifth rename (the old
        // mftr_gdext.dll aside) fails, after art and mftr.exe were swapped.
        let mut calls = 0;
        let mut flaky = |from: &Path, to: &Path| {
            calls += 1;
            if calls == 5 { Err(io::Error::other("in use")) } else { fs::rename(from, to) }
        };
        let err = swap(&dir, &staged, &mut flaky).unwrap_err();
        assert!(err.contains("in use"), "{err}");
        assert_eq!(read(&dir, "mftr.exe"), "exe 0.17.0");
        assert_eq!(read(&dir, "mftr_gdext.dll"), "dll 0.17.0");
        assert_eq!(read(&dir, "art/library/biped/export/biped_library.glb"), "glb 0.17.0");
        // The new release is back where it was staged, ready for another try.
        assert_eq!(read(&staged, "mftr.exe"), "exe 0.18.0");
        swap(&dir, &staged, &mut |f, t| fs::rename(f, t)).unwrap();
        assert_eq!(read(&dir, "mftr.exe"), "exe 0.18.0");
        fs::remove_dir_all(&dir).unwrap();
    }
}
