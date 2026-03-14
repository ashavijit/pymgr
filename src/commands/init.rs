use std::path::Path;

use crate::config::{ensure_pymgr_dirs, PymgrConfig};
use crate::errors::PymgrResult;
use crate::output;

pub fn exec(project_dir: &Path, python: Option<&str>) -> PymgrResult<()> {
    ensure_pymgr_dirs()?;

    let _env_dir = crate::env::manager::create_env(project_dir, python)?;

    let config = PymgrConfig::load(project_dir).unwrap_or_default();
    if config.python.is_none() {
        let mut new_config = config;
        if let Some(py) = python {
            new_config.python = Some(py.to_string());
        }
        let _ = new_config.save_pyproject(project_dir);
    }

    if output::is_json_mode() {
        let info = crate::env::manager::env_info(project_dir)?;
        output::print_json(&serde_json::json!({
            "status": "success",
            "environment": {
                "path": info.env_path,
                "python_version": info.python_version,
                "python_path": info.python_path,
                "created_at": info.created_at,
            }
        }));
    }

    Ok(())
}
