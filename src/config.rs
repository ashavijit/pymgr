use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::errors::{ErrorCode, PymgrError, PymgrResult};
use std::sync::atomic::{AtomicBool, Ordering};

static OFFLINE_MODE: AtomicBool = AtomicBool::new(false);

pub fn set_offline_mode(offline: bool) {
    OFFLINE_MODE.store(offline, Ordering::SeqCst);
}

pub fn is_offline_mode() -> bool {
    OFFLINE_MODE.load(Ordering::SeqCst)
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct PymgrConfig {
    #[serde(default)]
    pub python: Option<String>,
    #[serde(default)]
    pub env_path: Option<String>,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(default, rename = "dev-dependencies")]
    pub dev_dependencies: HashMap<String, String>,
    #[serde(default)]
    pub groups: HashMap<String, HashMap<String, String>>,
    #[serde(default)]
    pub hooks: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct WorkspaceConfig {
    #[serde(default)]
    pub members: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default, rename = "env-strategy")]
    pub env_strategy: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PyprojectToml {
    tool: Option<PyprojectTool>,
}

#[derive(Debug, Deserialize)]
struct PyprojectTool {
    pymgr: Option<PymgrConfig>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct PymgrToml {
    #[serde(default)]
    pub python: Option<String>,
    #[serde(default)]
    pub env_path: Option<String>,
    #[serde(default)]
    pub index_url: Option<String>,
    #[serde(default)]
    pub workspace: Option<WorkspaceConfig>,
}

impl PymgrConfig {
    pub fn load(project_dir: &Path) -> PymgrResult<Self> {
        let pyproject = project_dir.join("pyproject.toml");
        if pyproject.exists() {
            let content = std::fs::read_to_string(&pyproject)?;
            let parsed: PyprojectToml = toml::from_str(&content)?;
            if let Some(tool) = parsed.tool {
                if let Some(config) = tool.pymgr {
                    return Ok(config);
                }
            }
        }

        let pymgr_toml = project_dir.join(".pymgr.toml");
        if pymgr_toml.exists() {
            let content = std::fs::read_to_string(&pymgr_toml)?;
            let toml_config: PymgrToml = toml::from_str(&content)?;
            return Ok(PymgrConfig {
                python: toml_config.python,
                env_path: toml_config.env_path,
                ..Default::default()
            });
        }

        Ok(PymgrConfig::default())
    }

    pub fn find_project_root() -> PymgrResult<PathBuf> {
        let mut current = std::env::current_dir()?;
        loop {
            if current.join("pyproject.toml").exists()
                || current.join(".pymgr.toml").exists()
                || current.join(".pymgr").exists()
            {
                return Ok(current);
            }
            if !current.pop() {
                return Ok(std::env::current_dir()?);
            }
        }
    }

    pub fn env_dir(&self) -> String {
        self.env_path
            .clone()
            .unwrap_or_else(|| ".pymgr/env".to_string())
    }

    pub fn all_dependencies(&self) -> HashMap<String, String> {
        let mut all = self.dependencies.clone();
        all.extend(self.dev_dependencies.clone());
        all
    }

    pub fn save_pyproject(&self, project_dir: &Path) -> PymgrResult<()> {
        let pyproject = project_dir.join("pyproject.toml");
        let mut doc = if pyproject.exists() {
            let content = std::fs::read_to_string(&pyproject)?;
            content
        } else {
            String::new()
        };

        let _config_str = toml::to_string_pretty(self)
            .map_err(|e| PymgrError::coded(ErrorCode::ConfigError, e.to_string()))?;

        if doc.contains("[tool.pymgr]") {
            return Ok(());
        }

        doc.push_str("\n[tool.pymgr]\n");
        if let Some(ref py) = self.python {
            doc.push_str(&format!("python = \"{}\"\n", py));
        }
        doc.push_str(&format!("env-path = \"{}\"\n", self.env_dir()));

        if !self.dependencies.is_empty() {
            doc.push_str("\n[tool.pymgr.dependencies]\n");
            for (name, version) in &self.dependencies {
                doc.push_str(&format!("{} = \"{}\"\n", name, version));
            }
        }

        if !self.dev_dependencies.is_empty() {
            doc.push_str("\n[tool.pymgr.dev-dependencies]\n");
            for (name, version) in &self.dev_dependencies {
                doc.push_str(&format!("{} = \"{}\"\n", name, version));
            }
        }

        std::fs::write(&pyproject, doc)?;
        Ok(())
    }
}

pub fn pymgr_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".pymgr")
}

pub fn ensure_pymgr_dirs() -> PymgrResult<()> {
    let home = pymgr_home();
    let dirs_to_create = [
        home.clone(),
        home.join("cache"),
        home.join("cache/metadata"),
        home.join("cache/wheels"),
        home.join("cache/python"),
        home.join("envs"),
        home.join("python"),
    ];
    for dir in &dirs_to_create {
        std::fs::create_dir_all(dir)?;
    }
    Ok(())
}
