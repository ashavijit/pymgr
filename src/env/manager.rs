use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{pymgr_home, PymgrConfig};
use crate::errors::{ErrorCode, PymgrError, PymgrResult};
use crate::output;
use crate::python::locator::{self, PythonInfo};

#[derive(Debug, Serialize, Deserialize)]
pub struct EnvMetadata {
    pub version: String,
    pub python_version: String,
    pub python_path: String,
    pub env_path: String,
    pub created_at: String,
    pub platform: String,
}

pub fn create_env(project_dir: &Path, python_request: Option<&str>) -> PymgrResult<PathBuf> {
    let config = PymgrConfig::load(project_dir).unwrap_or_default();
    let python_spec = python_request.or(config.python.as_deref());

    let python = locator::locate_python(python_spec)?;

    output::print_verbose(&format!(
        "Using Python {} at {}",
        python.version,
        python.path.display()
    ));

    let env_dir = project_dir.join(config.env_dir());

    if env_dir.exists() {
        return Err(PymgrError::coded(
            ErrorCode::EnvAlreadyExists,
            format!("Environment already exists at {}", env_dir.display()),
        )
        .with_suggestion("Remove it first with `pymgr env remove` or use a different path"));
    }

    create_env_skeleton(&env_dir, &python)?;
    write_pyvenv_cfg(&env_dir, &python)?;
    crate::env::activation::generate_all(&env_dir, project_dir)?;
    bootstrap_pip(&env_dir)?;
    write_env_metadata(&env_dir, project_dir, &python)?;

    output::print_success(&format!(
        "Created environment with Python {} at {}",
        python.version,
        env_dir.display()
    ));

    Ok(env_dir)
}

fn create_env_skeleton(env_dir: &Path, python: &PythonInfo) -> PymgrResult<()> {
    let version_short =
        &python.version[..python.version.rfind('.').unwrap_or(python.version.len())];

    if cfg!(windows) {
        std::fs::create_dir_all(env_dir.join("Scripts"))?;
    } else {
        std::fs::create_dir_all(env_dir.join("bin"))?;
    }

    let site_packages = if cfg!(windows) {
        env_dir.join("Lib").join("site-packages")
    } else {
        env_dir
            .join("lib")
            .join(format!("python{}", version_short))
            .join("site-packages")
    };
    std::fs::create_dir_all(&site_packages)?;
    std::fs::write(site_packages.join(".gitkeep"), "")?;

    std::fs::create_dir_all(env_dir.join("include"))?;

    link_python(env_dir, python)?;

    Ok(())
}

fn link_python(env_dir: &Path, python: &PythonInfo) -> PymgrResult<()> {
    if cfg!(windows) {
        let scripts_dir = env_dir.join("Scripts");
        let dest = scripts_dir.join("python.exe");
        std::fs::copy(&python.path, &dest)?;

        let python_dir = python.path.parent().unwrap_or(Path::new(""));
        for dll in &[
            "python3.dll",
            "python311.dll",
            "python312.dll",
            "python313.dll",
            "vcruntime140.dll",
            "vcruntime140_1.dll",
        ] {
            let src = python_dir.join(dll);
            if src.exists() {
                let _ = std::fs::copy(&src, scripts_dir.join(dll));
            }
        }

        let dest3 = scripts_dir.join("python3.exe");
        if !dest3.exists() {
            let _ = std::fs::copy(&dest, &dest3);
        }
    } else {
        let bin_dir = env_dir.join("bin");
        let dest = bin_dir.join("python");

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&python.path, &dest)?;
            let dest3 = bin_dir.join("python3");
            if !dest3.exists() {
                std::os::unix::fs::symlink(&python.path, &dest3)?;
            }
        }

        #[cfg(not(unix))]
        {
            std::fs::copy(&python.path, &dest)?;
        }
    }

    Ok(())
}

fn write_pyvenv_cfg(env_dir: &Path, python: &PythonInfo) -> PymgrResult<()> {
    let home = python
        .path
        .parent()
        .unwrap_or(Path::new(""))
        .to_string_lossy();

    let cfg = format!(
        "home = {}\ninclude-system-site-packages = false\nversion = {}\npymgr = true\npymgr-version = {}\n",
        home,
        python.version,
        env!("CARGO_PKG_VERSION")
    );

    std::fs::write(env_dir.join("pyvenv.cfg"), cfg)?;
    Ok(())
}

fn bootstrap_pip(env_dir: &Path) -> PymgrResult<()> {
    let python_exe = if cfg!(windows) {
        env_dir.join("Scripts").join("python.exe")
    } else {
        env_dir.join("bin").join("python")
    };

    output::print_verbose("Bootstrapping pip via ensurepip...");

    let status = Command::new(&python_exe)
        .args(["-m", "ensurepip", "--upgrade", "--default-pip"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    match status {
        Ok(s) if s.success() => {
            output::print_verbose("pip bootstrapped successfully");
        }
        _ => {
            output::print_warning("ensurepip not available — pip may need manual installation");
        }
    }

    Ok(())
}

fn write_env_metadata(env_dir: &Path, project_dir: &Path, python: &PythonInfo) -> PymgrResult<()> {
    let pymgr_dir = project_dir.join(".pymgr");
    std::fs::create_dir_all(&pymgr_dir)?;

    let metadata = EnvMetadata {
        version: env!("CARGO_PKG_VERSION").to_string(),
        python_version: python.version.clone(),
        python_path: python.path.to_string_lossy().to_string(),
        env_path: env_dir.to_string_lossy().to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
    };

    let json = serde_json::to_string_pretty(&metadata)?;
    std::fs::write(pymgr_dir.join("env.json"), json)?;

    Ok(())
}

pub fn remove_env(env_dir: &Path) -> PymgrResult<()> {
    if !env_dir.exists() {
        return Err(PymgrError::coded(
            ErrorCode::EnvNotFound,
            format!("No environment at {}", env_dir.display()),
        ));
    }
    std::fs::remove_dir_all(env_dir)?;
    output::print_success(&format!("Removed environment at {}", env_dir.display()));
    Ok(())
}

pub fn env_info(project_dir: &Path) -> PymgrResult<EnvMetadata> {
    let meta_path = project_dir.join(".pymgr").join("env.json");
    if !meta_path.exists() {
        return Err(PymgrError::coded(
            ErrorCode::EnvNotFound,
            "No environment found in this project",
        )
        .with_suggestion("Run `pymgr init` to create one"));
    }
    let content = std::fs::read_to_string(&meta_path)?;
    let metadata: EnvMetadata = serde_json::from_str(&content)?;
    Ok(metadata)
}

pub fn list_named_envs() -> PymgrResult<Vec<String>> {
    let envs_dir = pymgr_home().join("envs");
    if !envs_dir.exists() {
        return Ok(Vec::new());
    }

    let mut names = Vec::new();
    for entry in std::fs::read_dir(&envs_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

pub fn create_named_env(name: &str, python_request: Option<&str>) -> PymgrResult<PathBuf> {
    let env_dir = pymgr_home().join("envs").join(name);

    if env_dir.exists() {
        return Err(PymgrError::coded(
            ErrorCode::EnvAlreadyExists,
            format!("Environment '{}' already exists", name),
        ));
    }

    let python = locator::locate_python(python_request)?;

    create_env_skeleton(&env_dir, &python)?;
    write_pyvenv_cfg(&env_dir, &python)?;

    let project_dir = &env_dir;
    crate::env::activation::generate_all(&env_dir, project_dir)?;
    bootstrap_pip(&env_dir)?;

    output::print_success(&format!(
        "Created named environment '{}' with Python {}",
        name, python.version
    ));

    Ok(env_dir)
}

pub fn remove_named_env(name: &str) -> PymgrResult<()> {
    let env_dir = pymgr_home().join("envs").join(name);
    remove_env(&env_dir)
}

pub fn get_env_python(project_dir: &Path) -> PymgrResult<PathBuf> {
    let config = PymgrConfig::load(project_dir).unwrap_or_default();
    let env_dir = project_dir.join(config.env_dir());

    let python_path = if cfg!(windows) {
        env_dir.join("Scripts").join("python.exe")
    } else {
        env_dir.join("bin").join("python")
    };

    if !python_path.exists() {
        return Err(
            PymgrError::coded(ErrorCode::EnvNotFound, "No environment found")
                .with_suggestion("Run `pymgr init` to create one"),
        );
    }

    Ok(python_path)
}
