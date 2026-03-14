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
) -> PymgrResult<Vec<PathBuf>> {
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_DOWNLOADS));
    let pb = output::create_progress_bar(packages.len() as u64, "Installing packages");

    let mut tasks = Vec::new();

    for pkg in packages {
        let sem = Arc::clone(&semaphore);
        let pkg = pkg.clone();
        let dest = site_packages.clone();
        let pb = pb.clone();

        let task = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let result = download_and_install(&pkg, &dest).await;
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

async fn download_and_install(pkg: &LockedPackage, site_packages: &PathBuf) -> PymgrResult<PathBuf> {
    if let Some(cached_path) = cache::get_cached_wheel(&pkg.sha256) {
        output::print_verbose(&format!("Cache hit: {} {}", pkg.name, pkg.version));
        let dest = site_packages.clone();
        crate::installer::extractor::extract_wheel(&cached_path.join("wheel.whl"), &dest)?;
        return Ok(dest);
    }

    output::print_verbose(&format!("Downloading {} {}...", pkg.name, pkg.version));

    let info = pypi::fetch_version_info(&pkg.name, &pkg.version).await?;
    let wheel = pypi::find_best_wheel(&info.urls).ok_or_else(|| {
        PymgrError::coded(
            ErrorCode::PackageNotFound,
            format!("No wheel found for {} {}", pkg.name, pkg.version),
        )
    })?;

    let client = reqwest::Client::new();
    let response = client
        .get(&wheel.url)
        .send()
        .await
        .map_err(|e| {
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

    let _cached_path = cache::store_wheel(&pkg.sha256, &bytes)?;
    crate::installer::extractor::extract_wheel_bytes(&bytes, site_packages)?;

    Ok(site_packages.clone())
}

pub async fn download_single(
    name: &str,
    version: &str,
    site_packages: &PathBuf,
) -> PymgrResult<()> {
    let info = pypi::fetch_version_info(name, version).await?;
    let wheel = pypi::find_best_wheel(&info.urls).ok_or_else(|| {
        PymgrError::coded(
            ErrorCode::PackageNotFound,
            format!("No wheel found for {} {}", name, version),
        )
    })?;

    let client = reqwest::Client::new();
    let response = client.get(&wheel.url).send().await.map_err(|e| {
        PymgrError::coded(
            ErrorCode::NetworkError,
            format!("Failed to download {}: {}", name, e),
        )
    })?;

    let bytes = response.bytes().await?;

    if !cache::verify_sha256(&bytes, &wheel.digests.sha256) {
        return Err(PymgrError::coded(
            ErrorCode::ChecksumMismatch,
            format!("SHA-256 mismatch for {} {}", name, version),
        ));
    }

    let _ = cache::store_wheel(&wheel.digests.sha256, &bytes);
    crate::installer::extractor::extract_wheel_bytes(&bytes, site_packages)?;

    Ok(())
}
