use std::path::Path;

use crate::cache;
use crate::config::{pymgr_home, PymgrConfig};
use crate::errors::PymgrResult;
use crate::output;
use crate::python::locator;

pub fn run_diagnostics(project_dir: &Path) -> PymgrResult<()> {
    output::print_header("pymgr doctor");
    let mut issues = 0;

    output::print_info("Checking Python...");
    match locator::locate_python(None) {
        Ok(info) => {
            output::print_key_value("Python", &format!("{} ({})", info.version, info.path.display()));
        }
        Err(e) => {
            output::print_error(&format!("Python: {}", e));
            issues += 1;
        }
    }

    output::print_info("Checking environment...");
    let config = PymgrConfig::load(project_dir).unwrap_or_default();
    let env_dir = project_dir.join(config.env_dir());
    if env_dir.exists() {
        let python = if cfg!(windows) {
            env_dir.join("Scripts").join("python.exe")
        } else {
            env_dir.join("bin").join("python")
        };
        if python.exists() {
            output::print_key_value("Environment", &env_dir.to_string_lossy());
        } else {
            output::print_error("Environment exists but Python binary is missing");
            issues += 1;
        }

        let pyvenv_cfg = env_dir.join("pyvenv.cfg");
        if pyvenv_cfg.exists() {
            output::print_key_value("pyvenv.cfg", "OK");
        } else {
            output::print_warning("pyvenv.cfg is missing");
            issues += 1;
        }
    } else {
        output::print_warning("No environment found in this project");
    }

    output::print_info("Checking lockfile...");
    let lockfile = project_dir.join("pymgr.lock");
    if lockfile.exists() {
        output::print_key_value("Lockfile", "Found");
    } else {
        output::print_key_value("Lockfile", "Not found (no dependencies locked)");
    }

    output::print_info("Checking cache...");
    match cache::cache_stats() {
        Ok(stats) => {
            output::print_key_value(
                "Metadata cache",
                &format!("{} entries ({})", stats.metadata_entries, cache::CacheStats::format_size(stats.metadata_size)),
            );
            output::print_key_value(
                "Wheel cache",
                &format!("{} entries ({})", stats.wheel_entries, cache::CacheStats::format_size(stats.wheel_size)),
            );
        }
        Err(_) => {
            output::print_warning("Cache directory not accessible");
        }
    }

    output::print_info("Checking pymgr home...");
    let home = pymgr_home();
    output::print_key_value("Home directory", &home.to_string_lossy());
    if home.exists() {
        output::print_key_value("Home", "OK");
    } else {
        output::print_warning("Home directory does not exist");
        issues += 1;
    }

    let python_dir = home.join("python");
    if python_dir.exists() {
        let versions = crate::python::locator::list_managed_versions().unwrap_or_default();
        if versions.is_empty() {
            output::print_key_value("Managed Pythons", "None");
        } else {
            output::print_key_value("Managed Pythons", &versions.join(", "));
        }
    }

    println!();
    if issues == 0 {
        output::print_success("No issues found");
    } else {
        output::print_warning(&format!("{} issue(s) found", issues));
    }

    Ok(())
}
