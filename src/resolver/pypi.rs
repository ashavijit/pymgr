use serde::Deserialize;
use std::collections::HashMap;

use crate::cache;
use crate::errors::{ErrorCode, PymgrError, PymgrResult};

const PYPI_API_URL: &str = "https://pypi.org/pypi";

#[derive(Debug, Deserialize)]
pub struct PypiPackageResponse {
    pub info: PypiPackageInfo,
    #[serde(default)]
    pub releases: HashMap<String, Vec<PypiRelease>>,
    #[serde(default)]
    pub urls: Vec<PypiRelease>,
}

#[derive(Debug, Deserialize)]
pub struct PypiPackageInfo {
    pub name: String,
    pub version: String,
    pub summary: Option<String>,
    pub requires_python: Option<String>,
    pub requires_dist: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PypiRelease {
    pub filename: String,
    pub url: String,
    pub packagetype: String,
    pub python_requires: Option<String>,
    #[serde(default)]
    pub requires_dist: Option<Vec<String>>,
    pub digests: PypiDigests,
    pub size: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PypiDigests {
    pub sha256: String,
    #[serde(default)]
    pub md5: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    pub name: String,
    pub version: String,
    pub url: String,
    pub sha256: String,
    pub requires_python: Option<String>,
    pub dependencies: Vec<String>,
}

pub async fn fetch_package_info(name: &str) -> PymgrResult<PypiPackageResponse> {
    if let Some(cached) = cache::get_cached_metadata(name, "latest") {
        if let Ok(resp) = serde_json::from_str::<PypiPackageResponse>(&cached) {
            return Ok(resp);
        }
    }

    let url = format!("{}/{}/json", PYPI_API_URL, name);
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await.map_err(|e| {
        PymgrError::coded(
            ErrorCode::NetworkError,
            format!("Failed to fetch package info for '{}': {}", name, e),
        )
    })?;

    if response.status() == 404 {
        return Err(PymgrError::coded(
            ErrorCode::PackageNotFound,
            format!("Package '{}' not found on PyPI", name),
        )
        .with_suggestion("Check the package name for typos"));
    }

    if !response.status().is_success() {
        return Err(PymgrError::coded(
            ErrorCode::NetworkError,
            format!("PyPI returned HTTP {}", response.status()),
        ));
    }

    let text = response.text().await.map_err(|e| {
        PymgrError::coded(ErrorCode::NetworkError, format!("Failed to read response: {}", e))
    })?;

    let _ = cache::store_metadata(name, "latest", &text);

    let resp: PypiPackageResponse = serde_json::from_str(&text)?;
    Ok(resp)
}

pub async fn fetch_version_info(name: &str, version: &str) -> PymgrResult<PypiPackageResponse> {
    if let Some(cached) = cache::get_cached_metadata(name, version) {
        if let Ok(resp) = serde_json::from_str::<PypiPackageResponse>(&cached) {
            return Ok(resp);
        }
    }

    let url = format!("{}/{}/{}/json", PYPI_API_URL, name, version);
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await.map_err(|e| {
        PymgrError::coded(
            ErrorCode::NetworkError,
            format!("Failed to fetch version info: {}", e),
        )
    })?;

    if !response.status().is_success() {
        return Err(PymgrError::coded(
            ErrorCode::PackageNotFound,
            format!("Package '{}' version '{}' not found", name, version),
        ));
    }

    let text = response.text().await?;
    let _ = cache::store_metadata(name, version, &text);

    let resp: PypiPackageResponse = serde_json::from_str(&text)?;
    Ok(resp)
}

pub fn find_best_wheel(releases: &[PypiRelease]) -> Option<&PypiRelease> {
    let platform_tag = if cfg!(target_os = "windows") {
        "win"
    } else if cfg!(target_os = "macos") {
        "macosx"
    } else {
        "linux"
    };

    let arch_tag = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x86_64"
    };

    let wheels: Vec<&PypiRelease> = releases
        .iter()
        .filter(|r| r.packagetype == "bdist_wheel")
        .collect();

    if let Some(w) = wheels.iter().find(|w| {
        w.filename.contains(platform_tag) && w.filename.contains(arch_tag)
    }) {
        return Some(w);
    }

    if let Some(w) = wheels.iter().find(|w| {
        w.filename.contains(platform_tag)
    }) {
        return Some(w);
    }

    if let Some(w) = wheels.iter().find(|w| {
        w.filename.contains("none-any")
    }) {
        return Some(w);
    }

    wheels.first().copied()
}

pub fn parse_requirement(req: &str) -> (String, Option<String>) {
    let req = req.trim();

    if let Some(pos) = req.find(">=") {
        return (req[..pos].trim().to_string(), Some(req[pos..].trim().to_string()));
    }
    if let Some(pos) = req.find("<=") {
        return (req[..pos].trim().to_string(), Some(req[pos..].trim().to_string()));
    }
    if let Some(pos) = req.find("==") {
        return (req[..pos].trim().to_string(), Some(req[pos..].trim().to_string()));
    }
    if let Some(pos) = req.find("!=") {
        return (req[..pos].trim().to_string(), Some(req[pos..].trim().to_string()));
    }
    if let Some(pos) = req.find("~=") {
        return (req[..pos].trim().to_string(), Some(req[pos..].trim().to_string()));
    }

    let name = req
        .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_' && c != '.')
        .next()
        .unwrap_or(req);

    (name.to_string(), None)
}
