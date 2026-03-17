use crate::config::PymgrConfig;
use crate::errors::{ErrorCode, PymgrError, PymgrResult};
use crate::output;
use std::path::Path;

pub fn exec(name: &str) -> PymgrResult<()> {
    let project_dir = PymgrConfig::find_project_root()?;
    let python_exe = crate::env::manager::get_env_python(&project_dir)?;
    
    // We want a relative path for IDEs if possible to keep them portable
    let relative_exe = if let Ok(rel) = python_exe.strip_prefix(&project_dir) {
        rel.to_path_buf()
    } else {
        python_exe.clone()
    };

    let rel_str = relative_exe.to_string_lossy().replace('\\', "/");

    match name.to_lowercase().as_str() {
        "vscode" => {
            let vscode_dir = project_dir.join(".vscode");
            std::fs::create_dir_all(&vscode_dir)?;
            let settings_path = vscode_dir.join("settings.json");
            
            let mut settings = serde_json::json!({});
            if settings_path.exists() {
                let content = std::fs::read_to_string(&settings_path).unwrap_or_default();
                if let Ok(parsed) = serde_json::from_str(&content) {
                    settings = parsed;
                }
            }

            if let Some(obj) = settings.as_object_mut() {
                obj.insert("python.defaultInterpreterPath".to_string(), serde_json::json!(rel_str));
            }

            std::fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;
            output::print_success("Configured VSCode interpreter path in .vscode/settings.json");
        }
        "pyright" => {
            let pyright_path = project_dir.join("pyrightconfig.json");
            
            let mut settings = serde_json::json!({});
            if pyright_path.exists() {
                let content = std::fs::read_to_string(&pyright_path).unwrap_or_default();
                if let Ok(parsed) = serde_json::from_str(&content) {
                    settings = parsed;
                }
            }

            let config = PymgrConfig::load(&project_dir).unwrap_or_default();
            let env_dir = config.env_dir();
            let env_path = Path::new(&env_dir);
            let venv_parent = env_path.parent().unwrap_or(Path::new(".")).to_string_lossy().to_string();
            let venv_name = env_path.file_name().unwrap_or_default().to_string_lossy().to_string();

            if let Some(obj) = settings.as_object_mut() {
                obj.insert("venvPath".to_string(), serde_json::json!(venv_parent));
                obj.insert("venv".to_string(), serde_json::json!(venv_name));
            }

            std::fs::write(&pyright_path, serde_json::to_string_pretty(&settings)?)?;
            output::print_success("Configured Pyright venv environment in pyrightconfig.json");
        }
        "pycharm" => {
            output::print_info("To configure PyCharm:");
            output::print_info("1. Go to Settings -> Project -> Python Interpreter");
            output::print_info("2. Click 'Add Interpreter' -> 'Add Local Interpreter'");
            output::print_info("3. Select 'Existing environment'");
            output::print_info(&format!("4. Choose the path: {}", python_exe.display()));
        }
        _ => {
            return Err(PymgrError::coded(
                ErrorCode::ConfigError,
                format!("Unknown IDE: {}. Use 'vscode', 'pyright', or 'pycharm'.", name),
            ));
        }
    }

    Ok(())
}
