//! Auto-update installer. Downloads a release asset, verifies its
//! SHA-256 against the sibling .sha256 file from the release, and
//! hands the archive to platform-specific install logic.
//!
//! Network + filesystem only — no UI side effects. Caller (the
//! Message handler in app/snapdash.rs) is responsible for messaging
//! progress back into iced state.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use self_update::backends::github::ReleaseList;
use self_update::update::Release;
use sha2::{Digest, Sha256};

const REPO_OWNER: &str = "schizza";
const REPO_NAME: &str = "snapdash";

/// Substring that must appear in the asset filename for the host
/// platform/arch. Returns None for platforms we don't ship.
fn target_pattern() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Some("macos-aarch64"),
        ("linux", "x86_64") => Some("linux-x86-64"),
        ("windows", "x86_64") => Some("winows-x86_64"),
        _ => None,
    }
}

/// Asset descriptor — what we're going to download for a given release.
pub struct UpdateAsset {
    pub archive_url: String,
    pub checksum_url: String,
    pub archive_name: String,
}

/// Reslove matching archive + .sha256 for the  host platform.
pub fn pick_asset(release: &Release) -> Result<UpdateAsset> {
    let pattern =
        target_pattern().ok_or_else(|| anyhow!("no update asset for this platform/arch"))?;

    let archive = release
        .assets
        .iter()
        .find(|a| {
            a.name.contains(pattern) && (a.name.ends_with(".tar.gz") || a.name.ends_with(".zip"))
        })
        .ok_or_else(|| {
            anyhow!(
                "no archive matching '{pattern}' in release {}",
                release.version
            )
        })?;

    let checksum_name = format!("{}.sha256", archive.name);
    let checksum = release
        .assets
        .iter()
        .find(|a| a.name == checksum_name)
        .ok_or_else(|| anyhow!("missing {checksum_name} in release"))?;

    Ok(UpdateAsset {
        archive_url: archive.download_url.clone(),
        checksum_url: checksum.download_url.clone(),
        archive_name: archive.name.clone(),
    })
}

/// Fetches the latest GitHub release.
/// Synchronous - call from tokio::task::spawn_blocking inside Task::perform.
pub fn fetch_latest_release() -> Result<Release> {
    let releases = ReleaseList::configure()
        .repo_name(REPO_NAME)
        .repo_owner(REPO_OWNER)
        .build()
        .context("build release list")?
        .fetch()
        .context("fetch release")?;

    releases
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("no releases published"))
}

/// Download `url` into `dest_dir/file_name` and returns the path.
pub fn download_to(url: &str, dest_dir: &Path, file_name: &str) -> Result<PathBuf> {
    let path = dest_dir.join(file_name);

    let bytes = reqwest::blocking::Client::builder()
        .user_agent("snapdash")
        .build()
        .context("build http clinet")?
        .get(url)
        .send()
        .context("HTTP GET")?
        .error_for_status()
        .context("HTTP error")?
        .bytes()
        .context("read request body")?;

    std::fs::write(&path, &bytes).context("write archive")?;
    Ok(path)
}

/// Verifies SHA-256 of `archive` against the contents of `checksum_file`.
///
/// `.sha256` file format: hex digest, optionally followed by whitespace
/// + filename. We accept any of: `<hex>`, `<hex>  filename`, `<hex>  *filename`.
pub fn verify_checksum(archive: &Path, checksum_file: &Path) -> Result<()> {
    use std::fmt::Write as _;

    let raw = std::fs::read_to_string(checksum_file).context("read .sha256")?;
    let expected = raw
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow!("empty .256 file"))?
        .to_lowercase();

    let bytes = std::fs::read(archive).context("read archive")?;
    let digest = Sha256::digest(&bytes);

    let actual: String = digest.iter().map(|b| format!("{b:02x}")).collect();

    if actual != expected {
        return Err(anyhow!(
            "checksum mismatch: expected {expected}, got {actual}"
        ));
    }

    Ok(())
}
