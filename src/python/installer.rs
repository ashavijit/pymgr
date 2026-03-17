use std::path::{Path, PathBuf};

use crate::config::pymgr_home;
use crate::errors::{ErrorCode, PymgrError, PymgrResult};
use crate::output;

pub async fn install_python(version: &str) -> PymgrResult<PathBuf> {
    let install_dir = pymgr_home().join("python").join(version);

    if install_dir.exists() {
        return Err(PymgrError::coded(
            ErrorCode::EnvAlreadyExists,
            format!("Python {} is already installed", version),
        ));
    }

    let spinner = output::create_spinner(&format!("Downloading Python {}...", version));

    let url = build_download_url(version)?;

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| {
            PymgrError::coded(
                ErrorCode::NetworkError,
                format!("Failed to create HTTP client: {}", e),
            )
        })?;

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
                "Failed to download Python {} (HTTP {}). Check that version '{}' exists at python.org",
                version,
                response.status(),
                version
            ),
        )
        .with_suggestions(vec![
            format!("Verify version at https://www.python.org/ftp/python/"),
            format!("Use a full version like 3.12.3 instead of {}", version),
        ]));
    }

    let bytes = response.bytes().await.map_err(|e| {
        PymgrError::coded(
            ErrorCode::NetworkError,
            format!("Download interrupted: {}", e),
        )
    })?;

    spinner.set_message(format!("Extracting Python {}...", version));

    let tmp_dir = tempfile::tempdir()?;

    if cfg!(target_os = "windows") {
        extract_zip(&bytes, &tmp_dir.path().to_path_buf(), &install_dir)?;
    } else {
        let archive_path = tmp_dir.path().join("python.tar.xz");
        std::fs::write(&archive_path, &bytes)?;
        extract_tarball(&archive_path, &install_dir)?;
    }

    spinner.finish_and_clear();
    output::print_success(&format!(
        "Python {} installed to {}",
        version,
        install_dir.display()
    ));

    Ok(install_dir)
}

fn build_download_url(version: &str) -> PymgrResult<String> {
    let base = "https://www.python.org/ftp/python";

    if cfg!(target_os = "windows") {
        let arch = if cfg!(target_arch = "x86_64") {
            "amd64"
        } else if cfg!(target_arch = "aarch64") {
            "arm64"
        } else {
            "amd64"
        };
        Ok(format!(
            "{}/{}/python-{}-embed-{}.zip",
            base, version, version, arch
        ))
    } else if cfg!(target_os = "macos") {
        Ok(format!(
            "{}/{}/python-{}-macos11.pkg",
            base, version, version
        ))
    } else {
        Ok(format!(
            "{}/{}/Python-{}.tar.xz",
            base, version, version
        ))
    }
}

fn extract_zip(data: &[u8], _tmp_dir: &Path, dest: &Path) -> PymgrResult<()> {
    std::fs::create_dir_all(dest)?;

    let reader = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| PymgrError::Other(format!("Invalid zip archive: {}", e)))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| PymgrError::Other(format!("Zip read error: {}", e)))?;

        let outpath = dest.join(file.mangled_name());

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }

    let pth_files: Vec<_> = std::fs::read_dir(dest)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map_or(false, |ext| ext == "pth" || ext == "_pth")
        })
        .collect();

    for pth in &pth_files {
        let name = pth.file_name().to_string_lossy().to_string();
        if name.contains("._pth") {
            let content = std::fs::read_to_string(pth.path()).unwrap_or_default();
            let updated = content.replace("#import site", "import site");
            std::fs::write(pth.path(), updated)?;
        }
    }

    Ok(())
}

fn extract_tarball(archive_path: &Path, dest: &Path) -> PymgrResult<()> {
    std::fs::create_dir_all(dest)?;

    let file = std::fs::File::open(archive_path)?;

    let ext = archive_path
        .to_str()
        .unwrap_or("");

    if ext.ends_with(".tar.xz") || ext.ends_with(".txz") {
        let decoder = xz2::read::XzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        archive.unpack(dest)?;
    } else if ext.ends_with(".tar.gz") || ext.ends_with(".tgz") {
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
