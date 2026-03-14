use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::errors::PymgrResult;

#[derive(Debug, Serialize, Deserialize)]
pub struct Lockfile {
    pub metadata: LockfileMetadata,
    #[serde(rename = "package", default)]
    pub packages: Vec<LockedPackage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LockfileMetadata {
    pub python: String,
    pub resolver: String,
    #[serde(rename = "generated-at")]
    pub generated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default)]
    pub sha256: String,
    #[serde(default, rename = "requires-python")]
    pub requires_python: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<LockedDependency>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LockedDependency {
    pub name: String,
    pub version: String,
}

fn default_source() -> String {
    "pypi".to_string()
}

impl Lockfile {
    pub fn new(python_version: &str) -> Self {
        Self {
            metadata: LockfileMetadata {
                python: python_version.to_string(),
                resolver: "pubgrub-v1".to_string(),
                generated_at: Utc::now().to_rfc3339(),
            },
            packages: Vec::new(),
        }
    }

    pub fn load(path: &Path) -> PymgrResult<Self> {
        let content = std::fs::read_to_string(path)?;
        let lockfile: Lockfile = toml::from_str(&content)?;
        Ok(lockfile)
    }

    pub fn save(&self, path: &Path) -> PymgrResult<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| crate::errors::PymgrError::Other(e.to_string()))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn add_package(&mut self, package: LockedPackage) {
        self.packages.retain(|p| p.name != package.name);
        self.packages.push(package);
        self.packages.sort_by(|a, b| a.name.cmp(&b.name));
        self.metadata.generated_at = Utc::now().to_rfc3339();
    }

    pub fn remove_package(&mut self, name: &str) {
        self.packages.retain(|p| p.name != name);
        self.metadata.generated_at = Utc::now().to_rfc3339();
    }

    pub fn find_package(&self, name: &str) -> Option<&LockedPackage> {
        self.packages.iter().find(|p| p.name == name)
    }
}
