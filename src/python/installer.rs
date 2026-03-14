use std::path::{Path, PathBuf};

use crate::config::pymgr_home;
use crate::errors::{ErrorCode, PymgrError, PymgrResult};
use crate::output;

const PYTHON_BUILD_STANDALONE_URL: &str =
    "https://github.com/indygreg/python-build-standalone/releases/download";

pub async fn install_python(version: &str) -> PymgrResult<PathBuf> {
    let install_dir = pymgr_home().join("python").join(version);

    if install_dir.exists() {
        return Err(PymgrError::coded(
            ErrorCode::EnvAlreadyExists,
            format!("Python {} is already installed", version),
        ));
    }

    let spinner = output::create_spinner(&format!("Downloading Python {}...", version));

    let (url, archive_name) = build_download_url(version)?;

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await.map_err(|e| {
        PymgrError::coded(
            ErrorCode::NetworkError,
            format!("Failed to download Python {}: {}", version, e),
        )
    })?;

    if !response.status().is_success() {
        spinner.finish_and_clear();
        return Err(PymgrError::coded(
            ErrorCode::NetworkError,
            format!(
                "Failed to download Python {} (HTTP {})",
                version,
                response.status()
            ),
        ));
    }

    let bytes = response.bytes().await.map_err(|e| {
        PymgrError::coded(
            ErrorCode::NetworkError,
            format!("Download interrupted: {}", e),
        )
    })?;

    spinner.set_message(format!("Extracting Python {}...", version));

    let tmp_dir = tempfile::tempdir()?;
    let archive_path = tmp_dir.path().join(&archive_name);
    std::fs::write(&archive_path, &bytes)?;

    extract_archive(&archive_path, &install_dir)?;

    spinner.finish_and_clear();
    output::print_success(&format!("Python {} installed to {}", version, install_dir.display()));

    Ok(install_dir)
}

fn build_download_url(version: &str) -> PymgrResult<(String, String)> {
    let (os, arch, ext) = if cfg!(target_os = "windows") {
        if cfg!(target_arch = "x86_64") {
            ("windows", "x86_64", "tar.zst")
        } else {
            ("windows", "aarch64", "tar.zst")
        }
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            ("apple-darwin", "aarch64", "tar.gz")
        } else {
            ("apple-darwin", "x86_64", "tar.gz")
        }
    } else {
        if cfg!(target_arch = "aarch64") {
            ("unknown-linux-gnu", "aarch64", "tar.gz")
        } else {
            ("unknown-linux-gnu", "x86_64", "tar.gz")
        }
    };

    let archive_name = format!(
        "cpython-{}+{}-{}-{}-install_only.{}",
        version,
        chrono::Utc::now().format("%Y%m%d"),
        arch,
        os,
        ext
    );

    let tag = format!("{}", chrono::Utc::now().format("%Y%m%d"));
    let url = format!(
        "https://github.com/indygreg/python-build-standalone/releases/download/{}/cpython-{}-{}-{}-install_only.tar.gz",
        tag, version, arch, os
    );

    Ok((url, archive_name))
}

fn extract_archive(archive_path: &Path, dest: &Path) -> PymgrResult<()> {
    std::fs::create_dir_all(dest)?;

    let file = std::fs::File::open(archive_path)?;

    if archive_path.extension().and_then(|e| e.to_str()) == Some("gz")
        || archive_path
            .to_str()
            .map_or(false, |s| s.ends_with(".tar.gz"))
    {
        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        archive.unpack(dest)?;
    } else {
        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        archive.unpack(dest)?;
    }

    Ok(())
}

pub fn remove_python(version: &str) -> PymgrResult<()> {
    let install_dir = pymgr_home().join("python").join(version);

    if !install_dir.exists() {
        return Err(PymgrError::coded(
            ErrorCode::EnvNotFound,
            format!("Python {} is not installed", version),
        ));
    }

    std::fs::remove_dir_all(&install_dir)?;
    output::print_success(&format!("Removed Python {}", version));
    Ok(())
}
