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
        ("linux", "x86_64") => Some("linux-x86_64"),
        ("windows", "x86_64") => Some("windows-x86_64"),
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
        .header("Accept", "application/octet-stream, text/plain")
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

pub fn cleanup_stale_artefacts() {
    let result: Result<()> = (|| {
        #[cfg(target_os = "macos")]
        if let Some(bundle) = detect_app_bundle() {
            let backup = bundle.with_extension("app.old");
            if backup.exists() {
                std::fs::remove_dir_all(&backup)
                    .with_context(|| format!("remove {}", backup.display()))?;
                tracing::info!(path = %backup.display(), "cleaned up old bundle");
            }
        }

        #[cfg(target_os = "linux")]
        {
            let exe = std::env::current_exe()?;
            let backup = exe.with_extension("old");
            if backup.exists() {
                std::fs::remove_file(&backup)
                    .with_context(|| format!("remove {}", backup.display()))?;
                tracing::info!(path = %backup.display(), "cleaned up old binary");
            }
        }

        #[cfg(target_os = "windows")]
        {
            let exe = std::env::current_exe()?;
            let backup = exe.with_extension("exe.old");
            if backup.exists() {
                std::fs::remove_file(&backup)
                    .with_context(|| format!("remove {}", backup.display()))?;
                tracing::info!(path = %backup.display(), "cleaned up old binary");
            }
        }

        Ok(())
    })();

    if let Err(e) = result {
        tracing::warn!(error = %e, "failed to clean stale update artefacts");
    }
}

/// Recursively walks a directory looking for a file with the given
/// `name`. Used to find the binary inside an extracted tarball/zip
/// where the structure (subdirs, .app bundle internals) varies per
/// platform. For Linux and Winows platforms
#[cfg(not(target_os = "macos"))]
fn find_file_recursive(dir: &Path, name: &str) -> Option<PathBuf> {
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path: PathBuf = entry.path();
        if path.is_file() && path.file_name().and_then(|n| n.to_str()) == Some(name) {
            return Some(path);
        }
        if path.is_dir()
            && let Some(found) = find_file_recursive(&path, name)
        {
            return Some(found);
        }
    }
    None
}

//   macOS
#[cfg(target_os = "macos")]
pub fn install_archive(archive: &Path) -> Result<PathBuf> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let bundle_path = detect_app_bundle().ok_or_else(|| {
        anyhow!(
            "Auto-update is only available in packaged .app builds. \
                 Download the latest release from \
                 https://github.com/{REPO_OWNER}/{REPO_NAME}/releases"
        )
    })?;

    let parent = bundle_path
        .parent()
        .ok_or_else(|| anyhow!("bundle has no parent directory"))?;

    // Staging dir next to bundle → atomic rename
    let staging = tempfile::TempDir::new_in(parent).context("staging dir")?;

    let file = std::fs::File::open(archive).context("open archive")?;
    Archive::new(GzDecoder::new(file))
        .unpack(staging.path())
        .context("extract tar.gz")?;

    let new_bundle = std::fs::read_dir(staging.path())?
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().and_then(|s| s.to_str()) == Some("app"))
        .ok_or_else(|| anyhow!("no .app found in archive"))?
        .path();

    // Strip quarantine xattr
    let xattr_status = std::process::Command::new("xattr")
        .args(["-cr", new_bundle.to_str().unwrap()])
        .status()
        .context("run xattr -cr")?;
    if !xattr_status.success() {
        return Err(anyhow!("xattr -cr failed with status {xattr_status}"));
    }

    // Atomic swap
    let backup = bundle_path.with_extension("app.old");
    let _ = std::fs::remove_dir_all(&backup);
    std::fs::rename(&bundle_path, &backup).context("rename old bundle")?;
    std::fs::rename(&new_bundle, &bundle_path).context("rename new bundle")?;

    let exec_name = read_bundle_executable_name(&bundle_path)
        .unwrap_or_else(|| env!("CARGO_PKG_NAME").to_string());

    Ok(bundle_path.join("Contents/MacOS").join(exec_name))
}

#[cfg(target_os = "macos")]
pub(crate) fn detect_app_bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let mut path: &Path = exe.as_path();
    while let Some(parent) = path.parent() {
        if parent.extension().and_then(|s| s.to_str()) == Some("app") {
            return Some(parent.to_path_buf());
        }
        path = parent;
    }
    None
}

#[cfg(target_os = "macos")]
fn read_bundle_executable_name(bundle: &Path) -> Option<String> {
    // Parses CFBundleExecutable out of Info.plist without a full plist
    // dependency. The bundle is generated by our own release workflow,
    // so the format is stable: `<key>CFBundleExecutable</key><string>NAME</string>`.
    let plist = std::fs::read_to_string(bundle.join("Contents/Info.plist")).ok()?;

    let key_pos = plist.find("<key>CFBundleExecutable</key>")?;
    let after_key = &plist[key_pos..];
    let open = after_key.find("<string>")? + "<string>".len();
    let close = after_key[open..].find("</string>")?;

    Some(after_key[open..open + close].trim().to_string())
}

//   Linux
#[cfg(target_os = "linux")]
pub fn install_archive(archive: &Path) -> Result<PathBuf> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let current_exe = std::env::current_exe().context("current_exe")?;
    let parent = current_exe
        .parent()
        .ok_or_else(|| anyhow!("exe has no parent"))?;

    let staging = tempfile::TempDir::new_in(parent).context("staging dir")?;

    let file = std::fs::File::open(archive).context("open archive")?;
    Archive::new(GzDecoder::new(file))
        .unpack(staging.path())
        .context("extract tar.gz")?;

    let bin_name = env!("CARGO_PKG_NAME");
    let new_exe = find_file_recursive(staging.path(), bin_name)
        .ok_or_else(|| anyhow!("'{bin_name}' not found in archive"))?;

    // Linux mmap pattern: rename old, move new — both atomic. The running
    // process keeps its mmap intact regardless.
    let backup = current_exe.with_extension("old");
    let _ = std::fs::remove_file(&backup);
    std::fs::rename(&current_exe, &backup).context("backup current exe")?;
    std::fs::rename(&new_exe, &current_exe).context("install new exe")?;

    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(&current_exe)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&current_exe, perms)?;

    Ok(current_exe)
}

//    Windows

#[cfg(target_os = "windows")]
pub fn install_archive(archive: &Path) -> Result<PathBuf> {
    let current_exe = std::env::current_exe().context("current_exe")?;
    let parent = current_exe
        .parent()
        .ok_or_else(|| anyhow!("exe has no parent"))?;

    let staging = tempfile::TempDir::new_in(parent).context("staging dir")?;

    let file = std::fs::File::open(archive).context("open archive")?;
    let mut zip = zip::ZipArchive::new(file).context("read zip")?;
    zip.extract(staging.path()).context("extract zip")?;

    let bin_name = format!("{}.exe", env!("CARGO_PKG_NAME"));
    let new_exe = find_file_recursive(staging.path(), &bin_name)
        .ok_or_else(|| anyhow!("'{bin_name}' not found in archive"))?;

    // Windows can rename a locked .exe even though it can't overwrite it.
    let backup = current_exe.with_extension("exe.old");
    let _ = std::fs::remove_file(&backup);
    std::fs::rename(&current_exe, &backup).context("backup current exe")?;
    std::fs::rename(&new_exe, &current_exe).context("install new exe")?;

    Ok(current_exe)
}
