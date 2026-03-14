use std::path::Path;

use crate::config::pymgr_home;
use crate::env::manager;
use crate::errors::PymgrResult;
use crate::output;

pub fn exec_list() -> PymgrResult<()> {
    let envs = manager::list_named_envs()?;

    if output::is_json_mode() {
        output::print_json(&serde_json::json!({
            "environments": envs,
        }));
        return Ok(());
    }

    output::print_header("Environments");

    if envs.is_empty() {
        output::print_info("No named environments found");
    } else {
        for name in &envs {
            let path = pymgr_home().join("envs").join(name);
            output::print_key_value(name, &path.to_string_lossy());
        }
    }

    Ok(())
}

pub fn exec_remove(name: &str) -> PymgrResult<()> {
    manager::remove_named_env(name)?;
    Ok(())
}

pub fn exec_info(project_dir: &Path) -> PymgrResult<()> {
    let info = manager::env_info(project_dir)?;

    if output::is_json_mode() {
        output::print_json(&serde_json::json!({
            "version": info.version,
            "python_version": info.python_version,
            "python_path": info.python_path,
            "env_path": info.env_path,
            "created_at": info.created_at,
            "platform": info.platform,
        }));
        return Ok(());
    }

    output::print_header("Environment Info");
    output::print_key_value("pymgr version", &info.version);
    output::print_key_value("Python version", &info.python_version);
    output::print_key_value("Python path", &info.python_path);
    output::print_key_value("Environment path", &info.env_path);
    output::print_key_value("Created at", &info.created_at);
    output::print_key_value("Platform", &info.platform);

    Ok(())
}
