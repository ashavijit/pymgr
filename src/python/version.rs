use std::path::Path;

use crate::errors::PymgrResult;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PythonVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl PythonVersion {
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.trim().split('.').collect();
        match parts.len() {
            2 => Some(Self {
                major: parts[0].parse().ok()?,
                minor: parts[1].parse().ok()?,
                patch: 0,
            }),
            3 => Some(Self {
                major: parts[0].parse().ok()?,
                minor: parts[1].parse().ok()?,
                patch: parts[2].parse().ok()?,
            }),
            _ => None,
        }
    }

    pub fn matches(&self, spec: &str) -> bool {
        let parts: Vec<&str> = spec.trim().split('.').collect();
        match parts.len() {
            1 => {
                if let Ok(major) = parts[0].parse::<u32>() {
                    self.major == major
                } else {
                    false
                }
            }
            2 => {
                if let (Ok(major), Ok(minor)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                    self.major == major && self.minor == minor
                } else {
                    false
                }
            }
            3 => {
                if let Some(other) = Self::parse(spec) {
                    *self == other
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn short(&self) -> String {
        format!("{}.{}", self.major, self.minor)
    }

    pub fn full(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl std::fmt::Display for PythonVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

pub fn read_python_version_file(dir: &Path) -> PymgrResult<Option<String>> {
    let file = dir.join(".python-version");
    if file.exists() {
        let content = std::fs::read_to_string(&file)?;
        Ok(Some(content.trim().to_string()))
    } else {
        Ok(None)
    }
}

pub fn write_python_version_file(dir: &Path, version: &str) -> PymgrResult<()> {
    let file = dir.join(".python-version");
    std::fs::write(&file, format!("{}\n", version))?;
    Ok(())
}
