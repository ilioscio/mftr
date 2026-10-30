//! Is a newer release out? The release manifest and its signature.
//!
//! Every GitHub release carries `latest.json` (written by `mftr-tools release-manifest`): its
//! version, its protocol, and each platform's client archive with its size and SHA-256. GitHub
//! serves the newest release's copy at a link that never changes ([`MANIFEST_URL`]), so the
//! client needs nothing else to find updates. `latest.json.sig` beside it is an Ed25519
//! signature of the file's exact bytes by the release key, whose public half every client
//! carries ([`RELEASE_KEY`]): a manifest that doesn't verify is ignored, so the only one a client
//! acts on is one the release workflow signed.
//!
//! On Windows and macOS the notice comes with the new archive's download (the browser fetches
//! it; the player unpacks it, or drags the app in, as on the first install). Linux players
//! update through their package manager or the flake, so they get the release page instead.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use nanoserde::{DeJson, SerJson};
use std::fmt;

/// The repository whose releases are the game's.
pub const REPOSITORY: &str = "ilioscio/mftr";
/// The newest release's manifest (GitHub redirects to that release's asset); its signature is
/// beside it, at this + ".sig".
pub const MANIFEST_URL: &str = "https://github.com/ilioscio/mftr/releases/latest/download/latest.json";
pub const MANIFEST_FILE: &str = "latest.json";
pub const SIGNATURE_FILE: &str = "latest.json.sig";

/// The public half of the release key (the private half is the `MFTR_RELEASE_KEY` secret of
/// the release workflow; `mftr-tools release-key` makes a pair).
pub const RELEASE_KEY: &str = "5f0b1c73bf3d5233d8a2fdd001956f8dceffc5f315640a390dc92402fe033db2";

/// The platforms releases ship clients for, as named in archives and manifests.
pub const PLATFORMS: [&str; 3] = ["windows", "macos", "linux"];

/// The platform this client was built for.
pub fn platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

/// Whether the client offers a newer release's download here (Linux players update through
/// their package manager or the flake).
pub fn offers_download(platform: &str) -> bool {
    matches!(platform, "windows" | "macos")
}

/// The client archive of `version` for `platform`, as `scripts/package-client.sh` names it.
pub fn archive_name(version: &str, platform: &str) -> String {
    let ext = if platform == "linux" { "tar.gz" } else { "zip" };
    format!("mftr-client-{version}-{platform}.{ext}")
}

/// The release's page on GitHub.
pub fn release_page(version: &str) -> String {
    format!("https://github.com/{REPOSITORY}/releases/tag/v{version}")
}

/// The archive of `version` to download on `platform` (one click starts the download); none on
/// Linux (its players get the release page).
pub fn download_link(version: &str, platform: &str) -> Option<String> {
    offers_download(platform).then(|| {
        format!("https://github.com/{REPOSITORY}/releases/download/v{version}/{}", archive_name(version, platform))
    })
}

/// A release version, `major.minor.patch`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub u32, pub u32, pub u32);

impl Version {
    pub fn parse(s: &str) -> Option<Self> {
        let mut parts = s
            .trim()
            .split('.')
            .map(|p| if p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) { None } else { p.parse().ok() });
        let v = Version(parts.next()??, parts.next()??, parts.next()??);
        parts.next().is_none().then_some(v)
    }

    /// This build's.
    pub fn current() -> Self {
        Self::parse(mftr_net::RELEASE).expect("the workspace version is major.minor.patch")
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

/// Whether `release` (as a server or a manifest names it) is newer than `than`.
pub fn is_newer(release: &str, than: &str) -> bool {
    matches!((Version::parse(release), Version::parse(than)), (Some(a), Some(b)) if a > b)
}

/// `latest.json`.
#[derive(Clone, Debug, PartialEq, SerJson, DeJson)]
pub struct Manifest {
    pub version: String,
    pub protocol: u16,
    /// The release page.
    pub notes: String,
    pub assets: Vec<Asset>,
}

/// One client archive of a release.
#[derive(Clone, Debug, PartialEq, SerJson, DeJson)]
pub struct Asset {
    /// "windows", "macos" or "linux".
    pub platform: String,
    pub name: String,
    pub url: String,
    pub size: u64,
    /// Hex.
    pub sha256: String,
}

impl Manifest {
    pub fn asset(&self, platform: &str) -> Option<&Asset> {
        self.assets.iter().find(|a| a.platform == platform)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ManifestError {
    /// The signature is malformed or doesn't match the file.
    Signature,
    /// The file isn't a manifest.
    Format(String),
    /// Its version isn't `major.minor.patch`, or a link points outside this repository's
    /// releases.
    Invalid(&'static str),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManifestError::Signature => write!(f, "the release manifest's signature doesn't verify"),
            ManifestError::Format(e) => write!(f, "the release manifest doesn't read: {e}"),
            ManifestError::Invalid(what) => write!(f, "the release manifest is invalid: {what}"),
        }
    }
}

/// The release key's public half from its hex.
pub fn release_key() -> Option<[u8; 32]> {
    unhex(RELEASE_KEY)
}

/// Check `json` against its `signature` (hex, as in `latest.json.sig`) with `key`, then read it.
pub fn verify(json: &[u8], signature: &[u8], key: &[u8; 32]) -> Result<Manifest, ManifestError> {
    let sig: [u8; 64] = std::str::from_utf8(signature).ok().and_then(unhex::<64>).ok_or(ManifestError::Signature)?;
    let key = VerifyingKey::from_bytes(key).map_err(|_| ManifestError::Signature)?;
    key.verify_strict(json, &Signature::from_bytes(&sig)).map_err(|_| ManifestError::Signature)?;
    let text = std::str::from_utf8(json).map_err(|e| ManifestError::Format(e.to_string()))?;
    let m = Manifest::deserialize_json(text).map_err(|e| ManifestError::Format(e.to_string()))?;
    if Version::parse(&m.version).is_none() {
        return Err(ManifestError::Invalid("version"));
    }
    let releases = format!("https://github.com/{REPOSITORY}/releases/");
    if !m.notes.starts_with(&releases) || m.assets.iter().any(|a| !a.url.starts_with(&releases)) {
        return Err(ManifestError::Invalid("a link outside the repository's releases"));
    }
    if m.assets.iter().any(|a| unhex::<32>(&a.sha256).is_none()) {
        return Err(ManifestError::Invalid("sha256"));
    }
    Ok(m)
}

/// The manifest's text, and its signature (hex) by `secret`, the release key's private half.
pub fn sign(manifest: &Manifest, secret: &[u8; 32]) -> (String, String) {
    let json = manifest.serialize_json();
    let sig = SigningKey::from_bytes(secret).sign(json.as_bytes());
    (json, hex(&sig.to_bytes()))
}

/// The public half of a release key.
pub fn public_key(secret: &[u8; 32]) -> [u8; 32] {
    SigningKey::from_bytes(secret).verifying_key().to_bytes()
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn unhex<const N: usize>(s: &str) -> Option<[u8; N]> {
    let s = s.trim();
    if s.len() != N * 2 || !s.is_ascii() {
        return None;
    }
    let mut out = [0u8; N];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(version: &str) -> Manifest {
        Manifest {
            version: version.into(),
            protocol: 31,
            notes: release_page(version),
            assets: PLATFORMS
                .iter()
                .map(|p| Asset {
                    platform: p.to_string(),
                    name: archive_name(version, p),
                    url: format!(
                        "https://github.com/{REPOSITORY}/releases/download/v{version}/{}",
                        archive_name(version, p)
                    ),
                    size: 1234,
                    sha256: "ab".repeat(32),
                })
                .collect(),
        }
    }

    #[test]
    fn versions_compare_by_number() {
        assert!(is_newer("0.17.0", "0.16.1"));
        assert!(is_newer("0.16.10", "0.16.9"), "not as text");
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("0.16.1", "0.16.1"));
        assert!(!is_newer("0.16.0", "0.16.1"));
        for bad in ["", "0.17", "0.17.0.1", "v0.17.0", "0.17.x", "0.17.0-rc.1", "0..1", "+1.2.3"] {
            assert_eq!(Version::parse(bad), None, "{bad}");
            assert!(!is_newer(bad, "0.0.0"), "{bad}");
        }
        assert_eq!(Version::current().to_string(), mftr_net::RELEASE);
    }

    #[test]
    fn a_signed_manifest_verifies_and_reads_back() {
        let secret = [7u8; 32];
        let m = manifest("0.17.0");
        let (json, sig) = sign(&m, &secret);
        assert_eq!(verify(json.as_bytes(), sig.as_bytes(), &public_key(&secret)), Ok(m.clone()));
        // A trailing newline in the signature file is fine.
        assert!(verify(json.as_bytes(), format!("{sig}\n").as_bytes(), &public_key(&secret)).is_ok());
        assert_eq!(m.asset("windows").map(|a| a.name.as_str()), Some("mftr-client-0.17.0-windows.zip"));
        assert_eq!(m.asset("linux").map(|a| a.name.as_str()), Some("mftr-client-0.17.0-linux.tar.gz"));
    }

    #[test]
    fn anything_else_is_refused() {
        let secret = [7u8; 32];
        let key = public_key(&secret);
        let (json, sig) = sign(&manifest("0.17.0"), &secret);
        // Another key, an edited file, a broken signature.
        assert_eq!(verify(json.as_bytes(), sig.as_bytes(), &public_key(&[8; 32])), Err(ManifestError::Signature));
        let edited = json.replace("0.17.0", "0.17.1");
        assert_eq!(verify(edited.as_bytes(), sig.as_bytes(), &key), Err(ManifestError::Signature));
        assert_eq!(verify(json.as_bytes(), b"00", &key), Err(ManifestError::Signature));
        // Signed, but not something to act on.
        let mut m = manifest("0.17.0");
        m.assets[0].url = "https://example.org/mftr.zip".into();
        let (json, sig) = sign(&m, &secret);
        assert!(matches!(verify(json.as_bytes(), sig.as_bytes(), &key), Err(ManifestError::Invalid(_))));
        let (json, sig) = sign(&manifest("latest"), &secret);
        assert_eq!(verify(json.as_bytes(), sig.as_bytes(), &key), Err(ManifestError::Invalid("version")));
    }

    #[test]
    fn newer_manifests_with_more_fields_still_read() {
        let secret = [7u8; 32];
        let json = r#"{"version":"0.18.0","protocol":32,"notes":"https://github.com/ilioscio/mftr/releases/tag/v0.18.0","assets":[],"delta":{"base":"0.17.0"}}"#;
        let sig = hex(&SigningKey::from_bytes(&secret).sign(json.as_bytes()).to_bytes());
        assert_eq!(
            verify(json.as_bytes(), sig.as_bytes(), &public_key(&secret)).map(|m| m.version),
            Ok("0.18.0".into())
        );
    }

    #[test]
    fn links_and_platforms() {
        assert_eq!(release_page("0.17.0"), "https://github.com/ilioscio/mftr/releases/tag/v0.17.0");
        assert_eq!(
            download_link("0.17.0", "windows").as_deref(),
            Some("https://github.com/ilioscio/mftr/releases/download/v0.17.0/mftr-client-0.17.0-windows.zip")
        );
        assert_eq!(
            download_link("0.17.0", "macos").as_deref(),
            Some("https://github.com/ilioscio/mftr/releases/download/v0.17.0/mftr-client-0.17.0-macos.zip")
        );
        assert_eq!(download_link("0.17.0", "linux"), None, "Linux players update through their package manager");
        assert!(PLATFORMS.contains(&platform()));
        assert!(MANIFEST_URL.contains(REPOSITORY) && MANIFEST_URL.ends_with(MANIFEST_FILE));
        assert_eq!(SIGNATURE_FILE, format!("{MANIFEST_FILE}.sig"));
    }

    #[test]
    fn the_release_key_is_a_key() {
        let key = release_key().expect("RELEASE_KEY is 64 hex digits");
        assert!(VerifyingKey::from_bytes(&key).is_ok());
    }
}
