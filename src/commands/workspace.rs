use crate::config::{PymgrConfig, WorkspaceConfig};
use crate::errors::{ErrorCode, PymgrError, PymgrResult};
use crate::output;
use walkdir::{DirEntry, WalkDir};

pub fn exec_init() -> PymgrResult<()> {
    let project_dir = PymgrConfig::find_project_root()?;
    let mut config = PymgrConfig::load(&project_dir)?;
    
    if config.workspace.is_some() {
        output::print_success("Workspace is already initialized.");
        return Ok(());
    }

    config.workspace = Some(WorkspaceConfig {
        members: vec!["packages/*".to_string()],
        exclude: vec![],
        env_strategy: None,
    });

    config.save_pyproject(&project_dir)?;
    
    let pkgs_dir = project_dir.join("packages");
    if !pkgs_dir.exists() {
        std::fs::create_dir_all(&pkgs_dir)?;
    }

    output::print_success("Initialized workspace. Ready for monorepo scale.");
    Ok(())
}

fn is_hidden_or_ignored(entry: &DirEntry) -> bool {
    let file_name = entry.file_name().to_string_lossy();
    file_name.starts_with('.') || file_name == "node_modules" || file_name == "target" || file_name == ".pymgr" || file_name == "env"
}

pub fn exec_list() -> PymgrResult<()> {
    let project_dir = PymgrConfig::find_project_root()?;
    let config = PymgrConfig::load(&project_dir)?;
    
    let ws = match config.workspace {
        Some(w) => w,
        None => {
            return Err(PymgrError::coded(
                ErrorCode::ConfigError,
                "No workspace configured in pyproject.toml",
            ));
        }
    };

    println!("Workspace Members:");
    for member in &ws.members {
        println!("  - pattern: {}", member);
    }

    let mut found_projects = Vec::new();
    let walker = WalkDir::new(&project_dir).into_iter();
    
    for entry in walker.filter_entry(|e| !is_hidden_or_ignored(e)) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        
        if entry.file_name() == "pyproject.toml" {
            let path = entry.path();
            if path == project_dir.join("pyproject.toml") {
                continue;
            }
            if let Some(parent) = path.parent() {
                if let Ok(rel_path) = parent.strip_prefix(&project_dir) {
                    found_projects.push(rel_path.to_string_lossy().to_string());
                }
            }
        }
    }

    found_projects.sort();

    println!("\nDiscovered Packages:");
    if found_projects.is_empty() {
        println!("  (no packages found yet)");
    } else {
        for proj in found_projects {
            println!("  📦 {}", proj);
        }
    }

    Ok(())
}
