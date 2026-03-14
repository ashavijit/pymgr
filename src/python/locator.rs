use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::pymgr_home;
use crate::errors::{ErrorCode, PymgrError, PymgrResult};

pub struct PythonInfo {
    pub path: PathBuf,
    pub version: String,
}

pub fn locate_python(requested: Option<&str>) -> PymgrResult<PythonInfo> {
    if let Some(version) = requested {
        return find_specific_version(version);
    }

    if let Ok(info) = from_python_version_file() {
        return Ok(info);
    }

    if let Ok(val) = std::env::var("PYMGR_PYTHON") {
        let path = PathBuf::from(&val);
        if path.exists() {
            if let Ok(ver) = get_python_version(&path) {
                return Ok(PythonInfo {
                    path,
                    version: ver,
                });
            }
        }
    }

    if let Ok(info) = scan_path() {
        return Ok(info);
    }

    if let Ok(info) = scan_common_paths() {
        return Ok(info);
    }

    Err(PymgrError::coded(
        ErrorCode::PythonNotFound,
        "No Python 3.x interpreter found",
    )
    .with_suggestion("Install Python 3.x or run `pymgr python install 3.12`"))
}

fn find_specific_version(version: &str) -> PymgrResult<PythonInfo> {
    let managed = pymgr_home().join("python").join(version);
    let bin = if cfg!(windows) {
        managed.join("python.exe")
    } else {
        managed.join("bin").join(&format!("python{}", &version[..version.rfind('.').unwrap_or(version.len())]))
    };
    if bin.exists() {
        if let Ok(ver) = get_python_version(&bin) {
            return Ok(PythonInfo {
                path: bin,
                version: ver,
            });
        }
    }

    let candidates = if cfg!(windows) {
        vec![
            format!("python{}", version),
            format!("python{}.exe", version),
            "python.exe".to_string(),
            "python3.exe".to_string(),
        ]
    } else {
        vec![
            format!("python{}", version),
            format!("python{}", &version[..version.find('.').unwrap_or(version.len()).min(version.len())]),
            "python3".to_string(),
        ]
    };

    for candidate in &candidates {
        if let Ok(path) = which::which(candidate) {
            if let Ok(ver) = get_python_version(&path) {
                if ver.starts_with(version) {
                    return Ok(PythonInfo { path, version: ver });
                }
            }
        }
    }

    Err(PymgrError::coded(
        ErrorCode::PythonNotFound,
        format!("Python {} not found", version),
    )
    .with_suggestion(format!("Run `pymgr python install {}`", version)))
}

fn from_python_version_file() -> PymgrResult<PythonInfo> {
    let cwd = std::env::current_dir()?;
    let version_file = cwd.join(".python-version");
    if version_file.exists() {
        let version = std::fs::read_to_string(&version_file)?
            .trim()
            .to_string();
        return find_specific_version(&version);
    }
    Err(PymgrError::Other("No .python-version file found".into()))
}

fn scan_path() -> PymgrResult<PythonInfo> {
    let candidates = if cfg!(windows) {
        vec!["python3.exe", "python.exe"]
    } else {
        vec!["python3", "python"]
    };

    for name in candidates {
        if let Ok(path) = which::which(name) {
            if let Ok(ver) = get_python_version(&path) {
                if ver.starts_with('3') {
                    return Ok(PythonInfo {
                        path,
                        version: ver,
                    });
                }
            }
        }
    }

    Err(PymgrError::Other("No Python 3.x found in PATH".into()))
}

fn scan_common_paths() -> PymgrResult<PythonInfo> {
    let paths: Vec<PathBuf> = if cfg!(target_os = "windows") {
        let local_app = std::env::var("LOCALAPPDATA").unwrap_or_default();
        vec![
            PathBuf::from(&local_app).join("Programs").join("Python"),
            PathBuf::from("C:\\Python311"),
            PathBuf::from("C:\\Python312"),
            PathBuf::from("C:\\Python313"),
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/Library/Frameworks/Python.framework/Versions"),
        ]
    } else {
        vec![
            PathBuf::from("/usr/bin"),
            PathBuf::from("/usr/local/bin"),
        ]
    };

    for base in &paths {
        if !base.exists() {
            continue;
        }
        let candidates = if cfg!(windows) {
            vec![base.join("python.exe"), base.join("python3.exe")]
        } else {
            vec![base.join("python3"), base.join("python")]
        };
        for candidate in candidates {
            if candidate.exists() {
                if let Ok(ver) = get_python_version(&candidate) {
                    if ver.starts_with('3') {
                        return Ok(PythonInfo {
                            path: candidate,
                            version: ver,
                        });
                    }
                }
            }
        }
    }

    Err(PymgrError::Other("No Python found in common paths".into()))
}

pub fn get_python_version(python_path: &Path) -> PymgrResult<String> {
    let output = Command::new(python_path)
        .args(["-c", "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}')"])
        .output()?;

    if !output.status.success() {
        return Err(PymgrError::Other("Failed to get Python version".into()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn list_managed_versions() -> PymgrResult<Vec<String>> {
    let python_dir = pymgr_home().join("python");
    if !python_dir.exists() {
        return Ok(Vec::new());
    }

    let mut versions = Vec::new();
    for entry in std::fs::read_dir(&python_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                versions.push(name.to_string());
            }
        }
    }
    versions.sort();
    Ok(versions)
}
