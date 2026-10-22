//! The release manifest (`mftr_client::update`): the release key, and `latest.json` with its
//! signature for the release workflow to publish beside the client archives.

use mftr_client::update::{self, Asset, Manifest};
use sha2::{Digest, Sha256};
use std::path::Path;

/// Make a release key: its private half into `path` (hex; refused if the file exists), and its
/// public half returned (hex) for `update::RELEASE_KEY`.
pub fn new_key(path: &Path) -> Result<String, String> {
    if path.exists() {
        return Err(format!("{} exists: not replacing a key", path.display()));
    }
    let mut secret = [0u8; 32];
    getrandom::fill(&mut secret).map_err(|e| format!("no randomness: {e}"))?;
    std::fs::write(path, update::hex(&secret) + "\n").map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(update::hex(&update::public_key(&secret)))
}

/// The manifest of this build's release from the client archives in `dir`, which must hold one
/// for every platform.
pub fn manifest(dir: &Path) -> Result<Manifest, String> {
    let version = mftr_net::RELEASE;
    let mut assets = Vec::new();
    for platform in update::PLATFORMS {
        let name = update::archive_name(version, platform);
        let bytes = std::fs::read(dir.join(&name)).map_err(|e| format!("{name}: {e}"))?;
        assets.push(Asset {
            platform: platform.to_string(),
            url: format!("https://github.com/{}/releases/download/v{version}/{name}", update::REPOSITORY),
            name,
            size: bytes.len() as u64,
            sha256: update::hex(&Sha256::digest(&bytes)),
        });
    }
    Ok(Manifest {
        version: version.to_string(),
        protocol: mftr_net::PROTOCOL_VERSION,
        notes: update::release_page(version),
        assets,
    })
}

/// Write `latest.json` and `latest.json.sig` into `dir`, signed with `secret` (hex). Refuses a
/// key whose public half isn't the one clients carry: they would ignore the manifest.
pub fn write(dir: &Path, secret: &str) -> Result<Manifest, String> {
    let secret: [u8; 32] = update::unhex(secret).ok_or("the release key isn't 64 hex digits")?;
    let public = update::hex(&update::public_key(&secret));
    if public != update::RELEASE_KEY {
        return Err(format!(
            "this release key's public half is {public}, but clients carry {} (mftr_client::update::RELEASE_KEY)",
            update::RELEASE_KEY
        ));
    }
    let m = manifest(dir)?;
    let (json, sig) = update::sign(&m, &secret);
    // What clients will check, checked here first.
    update::verify(json.as_bytes(), sig.as_bytes(), &update::release_key().expect("checked above"))
        .map_err(|e| e.to_string())?;
    for (file, text) in [(update::MANIFEST_FILE, json), (update::SIGNATURE_FILE, sig + "\n")] {
        std::fs::write(dir.join(file), text).map_err(|e| format!("{file}: {e}"))?;
    }
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manifest_covers_every_platform() {
        let dir = std::env::temp_dir().join(format!("mftr-release-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(manifest(&dir).is_err(), "no archives");
        for p in update::PLATFORMS {
            std::fs::write(dir.join(update::archive_name(mftr_net::RELEASE, p)), p).unwrap();
        }
        let m = manifest(&dir).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(m.version, mftr_net::RELEASE);
        assert_eq!(m.assets.len(), 3);
        let w = m.asset("windows").unwrap();
        assert_eq!(w.size, 7);
        assert_eq!(w.sha256, update::hex(&Sha256::digest(b"windows")));
        // Signed with some key, it's a manifest clients read (with that key).
        let (json, sig) = update::sign(&m, &[3; 32]);
        assert_eq!(update::verify(json.as_bytes(), sig.as_bytes(), &update::public_key(&[3; 32])), Ok(m));
    }

    #[test]
    fn only_the_release_key_signs() {
        let dir = std::env::temp_dir();
        assert!(write(&dir, &"11".repeat(32)).unwrap_err().contains("clients carry"));
        assert!(write(&dir, "nope").is_err());
    }
}
