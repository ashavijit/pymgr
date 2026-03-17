use crate::config::PymgrConfig;
use crate::errors::{ErrorCode, PymgrError, PymgrResult};
use crate::output;
use std::fs;

pub fn exec_create(id: &str) -> PymgrResult<()> {
    let project_dir = PymgrConfig::find_project_root()?;
    let snapshots_dir = project_dir.join(".pymgr").join("snapshots");
    fs::create_dir_all(&snapshots_dir)?;

    let lockfile = project_dir.join("pymgr.lock");
    if !lockfile.exists() {
        return Err(PymgrError::coded(ErrorCode::LockStale, "No pymgr.lock to snapshot (run `pymgr install` first)"));
    }

    let target = snapshots_dir.join(format!("{}.lock", id));
    fs::copy(&lockfile, &target)?;
    output::print_success(&format!("Snapshot '{}' created", id));
    Ok(())
}

pub fn exec_list() -> PymgrResult<()> {
    let project_dir = PymgrConfig::find_project_root()?;
    let snapshots_dir = project_dir.join(".pymgr").join("snapshots");

    if !snapshots_dir.exists() {
        output::print_info("No snapshots found");
        return Ok(());
    }

    let mut found = false;
    for entry in fs::read_dir(snapshots_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".lock") {
            if !found {
                output::print_header("Snapshots");
                found = true;
            }
            let id = name.trim_end_matches(".lock");
            println!("  • {}", id);
        }
    }

    if !found {
        output::print_info("No snapshots found");
    }

    Ok(())
}

pub fn exec_rollback(id: Option<&str>) -> PymgrResult<()> {
    let project_dir = PymgrConfig::find_project_root()?;
    let snapshots_dir = project_dir.join(".pymgr").join("snapshots");

    let snapshot_id = match id {
        Some(name) => name.to_string(),
        None => {
            return Err(PymgrError::coded(ErrorCode::ConfigError, "Please specify a snapshot ID to rollback to"));
        }
    };

    let target = snapshots_dir.join(format!("{}.lock", snapshot_id));
    if !target.exists() {
        return Err(PymgrError::coded(ErrorCode::IoError, format!("Snapshot '{}' does not exist", snapshot_id)));
    }

    let lockfile = project_dir.join("pymgr.lock");
    fs::copy(&target, &lockfile)?;
    
    output::print_success(&format!("Rolled back to snapshot '{}'. Run `pymgr sync` to apply changes to the environment.", snapshot_id));
    Ok(())
}

pub fn exec_diff(id: &str) -> PymgrResult<()> {
    let project_dir = PymgrConfig::find_project_root()?;
    let snapshots_dir = project_dir.join(".pymgr").join("snapshots");
    let target = snapshots_dir.join(format!("{}.lock", id));

    if !target.exists() {
        return Err(PymgrError::coded(ErrorCode::IoError, format!("Snapshot '{}' does not exist", id)));
    }

    let current_lock_path = project_dir.join("pymgr.lock");
    if !current_lock_path.exists() {
        return Err(PymgrError::coded(ErrorCode::LockStale, "No current pymgr.lock exists to diff against."));
    }

    let current_lock = crate::lockfile::Lockfile::load(&current_lock_path)?;
    let snapshot_lock = crate::lockfile::Lockfile::load(&target)?;

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();

    let mut current_map = std::collections::HashMap::new();
    for pkg in &current_lock.packages {
        current_map.insert(pkg.name.clone(), pkg.version.clone());
    }

    let mut snap_map = std::collections::HashMap::new();
    for pkg in &snapshot_lock.packages {
        snap_map.insert(pkg.name.clone(), pkg.version.clone());
    }

    for (name, ver) in &current_map {
        if let Some(snap_ver) = snap_map.get(name) {
            if ver != snap_ver {
                changed.push((name.clone(), snap_ver.clone(), ver.clone()));
            }
        } else {
            added.push((name.clone(), ver.clone()));
        }
    }

    for (name, ver) in &snap_map {
        if !current_map.contains_key(name) {
            removed.push((name.clone(), ver.clone()));
        }
    }

    output::print_header(&format!("Diff: Snapshot '{}' -> Current Environment", id));

    let mut has_changes = false;

    if !added.is_empty() {
        println!("\n  {} Added:", console::style("+").green());
        for (name, ver) in added {
            println!("    {} {}", console::style(name).green(), console::style(ver).dim());
        }
        has_changes = true;
    }

    if !removed.is_empty() {
        println!("\n  {} Removed:", console::style("-").red());
        for (name, ver) in removed {
            println!("    {} {}", console::style(name).red(), console::style(ver).dim());
        }
        has_changes = true;
    }

    if !changed.is_empty() {
        println!("\n  {} Changed:", console::style("~").yellow());
        for (name, old_v, new_v) in changed {
            println!("    {} {} -> {}", console::style(name).yellow(), console::style(old_v).dim(), console::style(new_v).bold());
        }
        has_changes = true;
    }

    if !has_changes {
        println!("\n  No changes detected. The snapshot perfectly matches the active lockfile.");
    } else {
        println!();
    }

    Ok(())
}

pub fn exec_gc() -> PymgrResult<()> {
    let project_dir = PymgrConfig::find_project_root()?;
    let snapshots_dir = project_dir.join(".pymgr").join("snapshots");

    if !snapshots_dir.exists() {
        output::print_info("No snapshots found to clean.");
        return Ok(());
    }

    let mut count = 0;
    for entry in fs::read_dir(snapshots_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("lock") {
            fs::remove_file(path)?;
            count += 1;
        }
    }

    if count > 0 {
        output::print_success(&format!("Garbage collection complete: removed {} snapshot(s).", count));
    } else {
        output::print_info("No snapshots found to clean.");
    }

    Ok(())
}
