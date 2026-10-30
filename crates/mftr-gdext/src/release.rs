//! Releases for the start menu: `MftrRelease` says which version this is, checks the newest
//! release's manifest (`mftr_client::update`; the client fetches it with `HTTPRequest`), and
//! gives the links to get a version.

use godot::prelude::*;
use mftr_client::update;

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

    /// Whether this client updates itself (Windows and macOS); Linux players use their package
    /// manager.
    #[func]
    fn updates_itself() -> bool {
        update::updates_itself(update::platform())
    }

    /// The newest release's `latest.json` (its signature is at this + ".sig").
    #[func]
    fn manifest_url() -> GString {
        update::MANIFEST_URL.into()
    }

    /// Check a fetched manifest: `{ version, protocol, newer, page, download }` when its
    /// signature is the release key's (`newer`: newer than this build; `download`: this
    /// platform's link, see `download_link`), else `{ error }`.
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
                d.set("download", update::download_link(&m.version, update::platform()).as_str());
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

    /// Where to get `release` here: its archive on Windows and macOS, its release page on Linux
    /// (empty unless it's a `major.minor.patch` version).
    #[func]
    fn download_link(release: GString) -> GString {
        let release = release.to_string();
        if update::Version::parse(&release).is_none() {
            return GString::new();
        }
        GString::from(&update::download_link(&release, update::platform()))
    }
}
