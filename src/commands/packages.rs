use std::collections::HashMap;
use std::path::Path;

use crate::config::PymgrConfig;
use crate::errors::{ErrorCode, PymgrError, PymgrResult};
use crate::installer::{downloader, extractor};
use crate::lockfile::{LockedPackage, Lockfile};
use crate::output;
use crate::resolver::{pypi, solver};

pub async fn exec_add(
    project_dir: &Path,
    packages: &[String],
    dev: bool,
    editable: bool,
) -> PymgrResult<()> {
    let mut config = PymgrConfig::load(project_dir).unwrap_or_default();
    let python_version = get_python_version(project_dir)?;

    let mut lockfile = load_or_create_lockfile(project_dir, &python_version)?;
    let site_packages = get_site_packages(project_dir, &python_version)?;

    if editable {
        let python_exe = crate::env::manager::get_env_python(project_dir)?;
        for pkg_path in packages {
            let path = Path::new(pkg_path);
            let abs_path = if path.is_absolute() {
                path.to_path_buf()
            } else {
                std::env::current_dir()?.join(path)
            };

            let spinner = output::create_spinner(&format!("Adding editable {}...", pkg_path));

            let status = std::process::Command::new(&python_exe)
                .args([
                    "-m",
                    "pip",
                    "install",
                    "--no-deps",
                    "-e",
                    abs_path.to_str().unwrap(),
                ])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()?;

            if !status.success() {
                return Err(PymgrError::coded(
                    ErrorCode::InstallError,
                    format!("Failed to install editable package at {}", pkg_path),
                ));
            }

            let name = abs_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            lockfile.add_package(LockedPackage {
                name: name.clone(),
                version: "0.0.0-editable".to_string(),
                source: "local".to_string(),
                sha256: "".to_string(),
                requires_python: None,
                dependencies: vec![],
            });

            if dev {
                config
                    .dev_dependencies
                    .insert(name.clone(), pkg_path.to_string());
            } else {
                config
                    .dependencies
                    .insert(name.clone(), pkg_path.to_string());
            }

            spinner.finish_and_clear();
            output::print_success(&format!("Added editable package: {}", name));
        }

        lockfile.save(&project_dir.join("pymgr.lock"))?;
        config.save_pyproject(project_dir)?;
        return Ok(());
    }

    for pkg_spec in packages {
        let (name, version_spec) = parse_pkg_spec(pkg_spec);

        let spinner = output::create_spinner(&format!("Adding {}...", name));

        let info = pypi::fetch_package_info(&name).await?;
        let resolved_version = info.info.version.clone();

        let release = pypi::find_best_wheel(&info.urls)
            .or_else(|| pypi::find_source_dist(&info.urls))
            .ok_or_else(|| {
                PymgrError::coded(
                    ErrorCode::PackageNotFound,
                    format!("No compatible wheel or source dist for {}", name),
                )
            })?;

        let python_exe = crate::env::manager::get_env_python(project_dir)?;
        downloader::download_single(&name, &resolved_version, &site_packages, &python_exe).await?;

        let mut deps = Vec::new();
        if let Some(ref requires_dist) = info.info.requires_dist {
            for dep_str in requires_dist {
                if dep_str.contains("extra ==") || dep_str.contains("extra==") {
                    continue;
                }
                let (dep_name, dep_ver) = pypi::parse_requirement(dep_str);
                deps.push(crate::lockfile::LockedDependency {
                    name: dep_name,
                    version: dep_ver.unwrap_or_else(|| "*".to_string()),
                });
            }
        }

        lockfile.add_package(LockedPackage {
            name: name.clone(),
            version: resolved_version.clone(),
            source: "pypi".to_string(),
            sha256: release.digests.sha256.clone(),
            requires_python: info.info.requires_python.clone(),
            dependencies: deps,
        });

        let version_str = version_spec.unwrap_or_else(|| format!(">={}", resolved_version));
        if dev {
            config.dev_dependencies.insert(name.clone(), version_str);
        } else {
            config.dependencies.insert(name.clone(), version_str);
        }

        spinner.finish_and_clear();
        output::print_success(&format!("Added {} {}", name, resolved_version));
    }

    lockfile.save(&project_dir.join("pymgr.lock"))?;
    config.save_pyproject(project_dir)?;

    if output::is_json_mode() {
        let pkg_list: Vec<serde_json::Value> = packages
            .iter()
            .map(|p| serde_json::json!({"name": p}))
            .collect();
        output::print_json(&serde_json::json!({
            "status": "success",
            "added": pkg_list,
        }));
    }

    Ok(())
}

pub async fn exec_remove(project_dir: &Path, packages: &[String]) -> PymgrResult<()> {
    let mut config = PymgrConfig::load(project_dir).unwrap_or_default();
    let python_version = get_python_version(project_dir)?;
    let mut lockfile = load_or_create_lockfile(project_dir, &python_version)?;
    let site_packages = get_site_packages(project_dir, &python_version)?;

    for name in packages {
        extractor::uninstall_package(&site_packages, name)?;
        lockfile.remove_package(name);
        config.dependencies.remove(name);
        config.dev_dependencies.remove(name);
        output::print_success(&format!("Removed {}", name));
    }

    lockfile.save(&project_dir.join("pymgr.lock"))?;
    config.save_pyproject(project_dir)?;

    Ok(())
}

pub async fn exec_install(project_dir: &Path, frozen: bool) -> PymgrResult<()> {
    let python_version = get_python_version(project_dir)?;
    let lockfile_path = project_dir.join("pymgr.lock");

    if lockfile_path.exists() {
        if frozen {
            output::print_verbose("Installing from lockfile (frozen mode)");
        }
        let lockfile = Lockfile::load(&lockfile_path)?;
        let site_packages = get_site_packages(project_dir, &python_version)?;
        let python_exe = crate::env::manager::get_env_python(project_dir)?;
        downloader::download_packages(&lockfile.packages, &site_packages, &python_exe).await?;
        output::print_success(&format!(
            "Installed {} packages from lockfile",
            lockfile.packages.len()
        ));
    } else {
        let config = PymgrConfig::load(project_dir).unwrap_or_default();
        let all_deps = config.all_dependencies();

        if all_deps.is_empty() {
            output::print_info("No dependencies to install");
            return Ok(());
        }

        let resolved = solver::resolve(&all_deps, &python_version).await?;
        let site_packages = get_site_packages(project_dir, &python_version)?;
        let python_exe = crate::env::manager::get_env_python(project_dir)?;
        downloader::download_packages(&resolved, &site_packages, &python_exe).await?;

        let mut lockfile = Lockfile::new(&python_version);
        for pkg in resolved {
            lockfile.add_package(pkg);
        }
        lockfile.save(&lockfile_path)?;

        output::print_success(&format!("Installed {} packages", lockfile.packages.len()));
    }

    Ok(())
}

pub async fn exec_update(project_dir: &Path, packages: &[String]) -> PymgrResult<()> {
    let config = PymgrConfig::load(project_dir).unwrap_or_default();
    let python_version = get_python_version(project_dir)?;

    let deps_to_update: HashMap<String, String> = if packages.is_empty() {
        config.all_dependencies()
    } else {
        config
            .all_dependencies()
            .into_iter()
            .filter(|(k, _)| packages.contains(k))
            .collect()
    };

    if deps_to_update.is_empty() {
        output::print_info("No packages to update");
        return Ok(());
    }

    let mut update_deps = HashMap::new();
    for (name, _) in &deps_to_update {
        update_deps.insert(name.clone(), "*".to_string());
    }

    let resolved = solver::resolve(&update_deps, &python_version).await?;
    let site_packages = get_site_packages(project_dir, &python_version)?;
    let python_exe = crate::env::manager::get_env_python(project_dir)?;
    downloader::download_packages(&resolved, &site_packages, &python_exe).await?;

    let mut lockfile = load_or_create_lockfile(project_dir, &python_version)?;
    for pkg in resolved {
        lockfile.add_package(pkg);
    }
    lockfile.save(&project_dir.join("pymgr.lock"))?;

    output::print_success("Packages updated");
    Ok(())
}

pub async fn exec_sync(project_dir: &Path) -> PymgrResult<()> {
    let lockfile_path = project_dir.join("pymgr.lock");
    if !lockfile_path.exists() {
        return Err(
            PymgrError::coded(ErrorCode::LockStale, "No pymgr.lock found")
                .with_suggestion("Run `pymgr install` first"),
        );
    }

    let python_version = get_python_version(project_dir)?;
    let lockfile = Lockfile::load(&lockfile_path)?;
    let site_packages = get_site_packages(project_dir, &python_version)?;

    if site_packages.exists() {
        for entry in std::fs::read_dir(&site_packages)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "__pycache__" {
                continue;
            }
            let path = entry.path();
            if path.is_dir() {
                std::fs::remove_dir_all(&path)?;
            } else {
                std::fs::remove_file(&path)?;
            }
        }
    }

    let python_exe = crate::env::manager::get_env_python(project_dir)?;
    downloader::download_packages(&lockfile.packages, &site_packages, &python_exe).await?;
    output::print_success("Environment synced to lockfile");
    Ok(())
}

pub fn exec_list(project_dir: &Path) -> PymgrResult<()> {
    let python_version = get_python_version(project_dir)?;
    let site_packages = get_site_packages(project_dir, &python_version)?;

    let installed = extractor::list_installed(&site_packages)?;

    if output::is_json_mode() {
        let pkgs: Vec<serde_json::Value> = installed
            .iter()
            .map(|p| serde_json::json!({"name": p.name, "version": p.version}))
            .collect();
        output::print_json(&serde_json::json!({"packages": pkgs}));
        return Ok(());
    }

    if installed.is_empty() {
        output::print_info("No packages installed");
    } else {
        output::print_header("Installed packages");
        let mut table = comfy_table::Table::new();
        table.load_preset(comfy_table::presets::UTF8_FULL);
        table.set_header(vec!["Package", "Version"]);

        for pkg in &installed {
            table.add_row(vec![pkg.name.clone(), pkg.version.clone()]);
        }
        println!("\n{}", table);
    }

    Ok(())
}

fn parse_pkg_spec(spec: &str) -> (String, Option<String>) {
    if let Some(pos) = spec.find("==") {
        return (
            spec[..pos].to_string(),
            Some(format!(">={}", &spec[pos + 2..])),
        );
    }
    if let Some(pos) = spec.find(">=") {
        return (spec[..pos].to_string(), Some(spec[pos..].to_string()));
    }
    (spec.to_string(), None)
}

fn get_python_version(project_dir: &Path) -> PymgrResult<String> {
    let meta_path = project_dir.join(".pymgr").join("env.json");
    if meta_path.exists() {
        let content = std::fs::read_to_string(&meta_path)?;
        let meta: crate::env::manager::EnvMetadata = serde_json::from_str(&content)?;
        return Ok(meta.python_version);
    }

    let config = PymgrConfig::load(project_dir).unwrap_or_default();
    if let Some(ref py) = config.python {
        return Ok(py.clone());
    }

    match crate::python::locator::locate_python(None) {
        Ok(info) => Ok(info.version),
        Err(_) => Ok("3.11".to_string()),
    }
}

fn get_site_packages(project_dir: &Path, python_version: &str) -> PymgrResult<std::path::PathBuf> {
    let config = PymgrConfig::load(project_dir).unwrap_or_default();
    let env_dir = project_dir.join(config.env_dir());
    let version_short = python_version
        .split('.')
        .take(2)
        .collect::<Vec<_>>()
        .join(".");

    let sp = if cfg!(windows) {
        env_dir.join("Lib").join("site-packages")
    } else {
        env_dir
            .join("lib")
            .join(format!("python{}", version_short))
            .join("site-packages")
    };

    std::fs::create_dir_all(&sp)?;
    Ok(sp)
}

fn load_or_create_lockfile(project_dir: &Path, python_version: &str) -> PymgrResult<Lockfile> {
    let lockfile_path = project_dir.join("pymgr.lock");
    if lockfile_path.exists() {
        Lockfile::load(&lockfile_path)
    } else {
        Ok(Lockfile::new(python_version))
    }
}
