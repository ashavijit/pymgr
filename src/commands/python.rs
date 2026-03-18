use std::path::Path;

use crate::errors::PymgrResult;
use crate::output;
use crate::python::{installer, locator, version};

pub async fn exec_install(version_str: &str) -> PymgrResult<()> {
    installer::install_python(version_str).await?;
    Ok(())
}

pub fn exec_list() -> PymgrResult<()> {
    let managed = locator::list_managed_versions()?;

    if output::is_json_mode() {
        output::print_json(&serde_json::json!({
            "installed_versions": managed,
        }));
        return Ok(());
    }

    output::print_header("Installed Python versions");

    if managed.is_empty() {
        output::print_info("No managed Python versions installed");
        output::print_info("Install one with: pymgr python install <version>");
    } else {
        for v in &managed {
            output::print_key_value("  ", v);
        }
    }

    match locator::locate_python(None) {
        Ok(info) => {
            println!();
            output::print_key_value(
                "System Python",
                &format!("{} ({})", info.version, info.path.display()),
            );
        }
        Err(_) => {}
    }

    Ok(())
}

pub fn exec_use(project_dir: &Path, version_str: &str) -> PymgrResult<()> {
    version::write_python_version_file(project_dir, version_str)?;
    output::print_success(&format!("Pinned Python version to {}", version_str));

    if output::is_json_mode() {
        output::print_json(&serde_json::json!({
            "status": "success",
            "python_version": version_str,
        }));
    }

    Ok(())
}

pub fn exec_remove(version_str: &str) -> PymgrResult<()> {
    installer::remove_python(version_str)?;
    Ok(())
}
