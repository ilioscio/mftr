//! Releases for the start menu: `MftrRelease` says which version this is, checks the newest
//! release's manifest (`mftr_client::update`; the client fetches it with `HTTPRequest`), gives
//! the links to get a version, and on Windows installs a downloaded one over the game
//! (`mftr_client::selfupdate`).

use godot::prelude::*;
use mftr_client::{selfupdate, update};
use std::path::Path;

#[derive(GodotClass)]
#[class(base = RefCounted, no_init)]
pub struct MftrRelease {
    base: Base<RefCounted>,
}

#[godot_api]
impl MftrRelease {
    /// This build's release ("0.16.1").
    #[func]
    fn current() -> GString {
        mftr_net::RELEASE.into()
    }

    /// "windows", "macos" or "linux".
    #[func]
    fn platform() -> GString {
        update::platform().into()
    }

    /// Whether this client offers a newer release's download (Windows and macOS); Linux players
    /// update through their package manager or the flake, and get the release page instead.
    #[func]
    fn offers_download() -> bool {
        update::offers_download(update::platform())
    }

    /// Whether this client installs a newer release itself (Windows); macOS players install the
    /// download as they did the first one.
    #[func]
    fn updates_itself() -> bool {
        update::updates_itself(update::platform())
    }

    /// Whether the game in `install_dir` can be updated in place (the folder is writable).
    #[func]
    fn can_install(install_dir: GString) -> bool {
        selfupdate::writable(Path::new(&install_dir.to_string()))
    }

    /// Install the downloaded `archive` (with the manifest's `size` and `sha256`) over the game in
    /// `install_dir`. Empty when done (start the new `mftr.exe`), else why it wasn't; then the
    /// game there is as it was.
    #[func]
    fn install(archive: GString, size: i64, sha256: GString, install_dir: GString) -> GString {
        let (archive, dir) = (archive.to_string(), install_dir.to_string());
        let required = &selfupdate::WINDOWS_FILES;
        match selfupdate::install(Path::new(&archive), size as u64, &sha256.to_string(), Path::new(&dir), required) {
            Ok(()) => GString::new(),
            Err(e) => GString::from(&e),
        }
    }

    /// Remove what an earlier update left in `install_dir` (the files it replaced).
    #[func]
    fn clean(install_dir: GString) {
        selfupdate::clean(Path::new(&install_dir.to_string()));
    }

    /// The newest release's `latest.json` (its signature is at this + ".sig").
    #[func]
    fn manifest_url() -> GString {
        update::MANIFEST_URL.into()
    }

    /// Check a fetched manifest: `{ version, protocol, newer, page, download, size, sha256 }`
    /// when its signature is the release key's (`newer`: newer than this build; `download`: this
    /// platform's archive, empty on Linux; `size` and `sha256`: that archive's, for installing
    /// it), else `{ error }`.
    #[func]
    fn check(json: PackedByteArray, signature: PackedByteArray) -> VarDictionary {
        let mut d = VarDictionary::new();
        let key = update::release_key().expect("RELEASE_KEY is a key");
        match update::verify(json.as_slice(), signature.as_slice(), &key) {
            Ok(m) => {
                d.set("version", m.version.as_str());
                d.set("protocol", m.protocol as i64);
                d.set("newer", update::is_newer(&m.version, mftr_net::RELEASE));
                d.set("page", m.notes.as_str());
                let download = update::download_link(&m.version, update::platform()).unwrap_or_default();
                d.set("download", download.as_str());
                let asset = m.asset(update::platform()).filter(|a| a.url == download);
                d.set("size", asset.map_or(0, |a| a.size as i64));
                d.set("sha256", asset.map_or("", |a| a.sha256.as_str()));
            }
            Err(e) => d.set("error", e.to_string().as_str()),
        }
        d
    }

    /// Whether `release` ("0.17.0") is newer than this build.
    #[func]
    fn is_newer(release: GString) -> bool {
        update::is_newer(&release.to_string(), mftr_net::RELEASE)
    }

    /// `release`'s archive for this platform (empty on Linux, and unless it's a
    /// `major.minor.patch` version).
    #[func]
    fn download_link(release: GString) -> GString {
        let release = release.to_string();
        if update::Version::parse(&release).is_none() {
            return GString::new();
        }
        GString::from(&update::download_link(&release, update::platform()).unwrap_or_default())
    }

    /// `release`'s page on GitHub (empty unless it's a `major.minor.patch` version).
    #[func]
    fn release_page(release: GString) -> GString {
        let release = release.to_string();
        if update::Version::parse(&release).is_none() {
            return GString::new();
        }
        GString::from(&update::release_page(&release))
    }
}
