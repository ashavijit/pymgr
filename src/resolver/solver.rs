use std::collections::HashMap;

use crate::errors::{ErrorCode, PymgrError, PymgrResult};
use crate::lockfile::{LockedDependency, LockedPackage};
use crate::output;
use crate::resolver::pypi;

pub async fn resolve(
    requirements: &HashMap<String, String>,
    _python_version: &str,
) -> PymgrResult<Vec<LockedPackage>> {
    let mut resolved = Vec::new();
    let total = requirements.len();

    let pb = output::create_progress_bar(total as u64, "Resolving dependencies");

    for (name, version_spec) in requirements {
        pb.set_message(format!("Resolving {}", name));

        let package_info = pypi::fetch_package_info(name).await?;

        let version = if version_spec == "*" || version_spec.is_empty() {
            package_info.info.version.clone()
        } else if version_spec.starts_with('^') {
            let base = version_spec.trim_start_matches('^');
            find_compatible_version(&package_info.releases, base)?
        } else if version_spec.starts_with(">=") {
            let min = version_spec.trim_start_matches(">=");
            find_minimum_version(&package_info.releases, min)?
        } else if version_spec.starts_with("==") {
            version_spec.trim_start_matches("==").to_string()
        } else {
            find_compatible_version(&package_info.releases, version_spec)?
        };

        let version_info = pypi::fetch_version_info(name, &version).await?;

        let wheel = pypi::find_best_wheel(&version_info.urls).ok_or_else(|| {
            PymgrError::coded(
                ErrorCode::PackageNotFound,
                format!("No compatible wheel found for {} {}", name, version),
            )
        })?;

        let mut deps = Vec::new();
        if let Some(ref requires_dist) = version_info.info.requires_dist {
            for dep_str in requires_dist {
                if dep_str.contains("extra ==") || dep_str.contains("extra==") {
                    continue;
                }
                let (dep_name, dep_version) = pypi::parse_requirement(dep_str);
                deps.push(LockedDependency {
                    name: dep_name,
                    version: dep_version.unwrap_or_else(|| "*".to_string()),
                });
            }
        }

        resolved.push(LockedPackage {
            name: name.clone(),
            version,
            source: "pypi".to_string(),
            sha256: wheel.digests.sha256.clone(),
            requires_python: version_info.info.requires_python.clone(),
            dependencies: deps,
        });

        pb.inc(1);
    }

    pb.finish_and_clear();
    Ok(resolved)
}

fn find_compatible_version(
    releases: &HashMap<String, Vec<pypi::PypiRelease>>,
    base_version: &str,
) -> PymgrResult<String> {
    let mut versions: Vec<&String> = releases
        .keys()
        .filter(|v| !v.contains("dev") && !v.contains("rc") && !v.contains("alpha") && !v.contains("beta"))
        .filter(|v| !releases[*v].is_empty())
        .collect();

    versions.sort_by(|a, b| compare_versions(b, a));

    let parts: Vec<u64> = base_version
        .split('.')
        .filter_map(|p| p.parse().ok())
        .collect();

    if let Some(major) = parts.first() {
        for v in &versions {
            let v_parts: Vec<u64> = v.split('.').filter_map(|p| p.parse().ok()).collect();
            if let Some(v_major) = v_parts.first() {
                if v_major == major {
                    if parts.len() >= 2 {
                        if let Some(v_minor) = v_parts.get(1) {
                            if *v_minor >= parts[1] {
                                return Ok(v.to_string());
                            }
                        }
                    } else {
                        return Ok(v.to_string());
                    }
                }
            }
        }
    }

    versions
        .first()
        .map(|v| v.to_string())
        .ok_or_else(|| {
            PymgrError::coded(
                ErrorCode::ResolutionConflict,
                format!("No compatible version found for spec {}", base_version),
            )
        })
}

fn find_minimum_version(
    releases: &HashMap<String, Vec<pypi::PypiRelease>>,
    min_version: &str,
) -> PymgrResult<String> {
    let mut versions: Vec<&String> = releases
        .keys()
        .filter(|v| !v.contains("dev") && !v.contains("rc") && !v.contains("alpha") && !v.contains("beta"))
        .filter(|v| !releases[*v].is_empty())
        .collect();

    versions.sort_by(|a, b| compare_versions(b, a));

    for v in &versions {
        if compare_versions(v, min_version) != std::cmp::Ordering::Less {
            return Ok(v.to_string());
        }
    }

    versions
        .first()
        .map(|v| v.to_string())
        .ok_or_else(|| {
            PymgrError::coded(
                ErrorCode::ResolutionConflict,
                format!("No version >= {} found", min_version),
            )
        })
}

fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts: Vec<u64> = a.split('.').filter_map(|p| p.parse().ok()).collect();
    let b_parts: Vec<u64> = b.split('.').filter_map(|p| p.parse().ok()).collect();

    let max_len = a_parts.len().max(b_parts.len());
    for i in 0..max_len {
        let a_val = a_parts.get(i).unwrap_or(&0);
        let b_val = b_parts.get(i).unwrap_or(&0);
        match a_val.cmp(b_val) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}
