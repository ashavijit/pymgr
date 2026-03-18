use futures::future::join_all;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::cache;
use crate::errors::{ErrorCode, PymgrError, PymgrResult};
use crate::lockfile::LockedPackage;
use crate::output;
use crate::resolver::pypi;

const MAX_CONCURRENT_DOWNLOADS: usize = 8;

pub async fn download_packages(
    packages: &[LockedPackage],
    site_packages: &PathBuf,
    python_exe: &PathBuf,
) -> PymgrResult<Vec<PathBuf>> {
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_DOWNLOADS));
    let pb = output::create_progress_bar(packages.len() as u64, "Installing packages");

    let mut tasks = Vec::new();

    for pkg in packages {
        let sem = Arc::clone(&semaphore);
        let pkg = pkg.clone();
        let dest = site_packages.clone();
        let python = python_exe.clone();
        let pb = pb.clone();

        let task = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let result = download_and_install(&pkg, &dest, &python).await;
            pb.inc(1);
            result
        });

        tasks.push(task);
    }

    let mut installed_paths = Vec::new();
    let results = join_all(tasks).await;

    for result in results {
        match result {
            Ok(Ok(path)) => installed_paths.push(path),
            Ok(Err(e)) => return Err(e),
            Err(e) => {
                return Err(PymgrError::coded(
                    ErrorCode::InstallError,
                    format!("Task failed: {}", e),
                ))
            }
        }
    }

    pb.finish_and_clear();
    Ok(installed_paths)
}

async fn download_and_install(
    pkg: &LockedPackage,
    site_packages: &PathBuf,
    python_exe: &PathBuf,
) -> PymgrResult<PathBuf> {
    if let Some(cached_path) = cache::get_cached_wheel(&pkg.sha256) {
        output::print_verbose(&format!("Cache hit: {} {}", pkg.name, pkg.version));
        let dest = site_packages.clone();
        crate::installer::extractor::extract_wheel(&cached_path.join("wheel.whl"), &dest)?;
        return Ok(dest);
    }

    output::print_verbose(&format!("Downloading {} {}...", pkg.name, pkg.version));

    let info = pypi::fetch_version_info(&pkg.name, &pkg.version).await?;

    let is_sdist;
    let release = if let Some(w) = pypi::find_best_wheel(&info.urls) {
        is_sdist = false;
        w
    } else if let Some(s) = pypi::find_source_dist(&info.urls) {
        is_sdist = true;
        s
    } else {
        return Err(PymgrError::coded(
            ErrorCode::PackageNotFound,
            format!("No wheel or source dist found for {}", pkg.name),
        ));
    };

    let client = reqwest::Client::new();
    let response = client.get(&release.url).send().await.map_err(|e| {
        PymgrError::coded(
            ErrorCode::NetworkError,
            format!("Failed to download {}: {}", pkg.name, e),
        )
    })?;

    let bytes = response.bytes().await.map_err(|e| {
        PymgrError::coded(
            ErrorCode::NetworkError,
            format!("Download interrupted for {}: {}", pkg.name, e),
        )
    })?;

    if !cache::verify_sha256(&bytes, &pkg.sha256) {
        return Err(PymgrError::coded(
            ErrorCode::ChecksumMismatch,
            format!(
                "SHA-256 mismatch for {} {} — download may be corrupted",
                pkg.name, pkg.version
            ),
        ));
    }

    if is_sdist {
        let tmp = tempfile::tempdir()?;
        if release.filename.ends_with(".tar.gz") {
            crate::installer::extractor::extract_tar_gz_bytes(&bytes, tmp.path())?;
        } else if release.filename.ends_with(".zip") {
            crate::installer::extractor::extract_wheel_bytes(&bytes, tmp.path())?;
        } else {
            return Err(PymgrError::coded(
                ErrorCode::InstallError,
                "Unknown source distribution format",
            ));
        }

        let mut source_dir = tmp.path().to_path_buf();
        for entry in std::fs::read_dir(tmp.path())? {
            let entry = entry?;
            if entry.path().is_dir() {
                source_dir = entry.path();
                break;
            }
        }

        let out_dir = tempfile::tempdir()?;
        let built_wheel =
            crate::installer::builder::build_sdist(&source_dir, out_dir.path(), python_exe)?;

        let wheel_bytes = std::fs::read(&built_wheel)?;
        crate::installer::extractor::extract_wheel_bytes(&wheel_bytes, site_packages)?;
        let _ = cache::store_wheel(&pkg.sha256, &wheel_bytes);
    } else {
        let _cached_path = cache::store_wheel(&pkg.sha256, &bytes)?;
        crate::installer::extractor::extract_wheel_bytes(&bytes, site_packages)?;
    }

    Ok(site_packages.clone())
}

pub async fn download_single(
    name: &str,
    version: &str,
    site_packages: &PathBuf,
    python_exe: &PathBuf,
) -> PymgrResult<()> {
    let dummy_pkg = LockedPackage {
        name: name.to_string(),
        version: version.to_string(),
        source: "pypi".to_string(),
        sha256: fetch_sha256(name, version).await?,
        requires_python: None,
        dependencies: vec![],
    };

    download_and_install(&dummy_pkg, site_packages, python_exe).await?;
    Ok(())
}

async fn fetch_sha256(name: &str, version: &str) -> PymgrResult<String> {
    let info = pypi::fetch_version_info(name, version).await?;
    let release = pypi::find_best_wheel(&info.urls)
        .or_else(|| pypi::find_source_dist(&info.urls))
        .ok_or_else(|| {
            PymgrError::coded(
                ErrorCode::PackageNotFound,
                format!("No wheel or source dist found for {} {}", name, version),
            )
        })?;
    Ok(release.digests.sha256.clone())
}
