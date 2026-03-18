use crate::config::PymgrConfig;
use crate::errors::{ErrorCode, PymgrError, PymgrResult};
use crate::lockfile::{LockedPackage, Lockfile};
use crate::output;
use std::collections::{HashMap, HashSet};

pub fn exec() -> PymgrResult<()> {
    let project_dir = PymgrConfig::find_project_root()?;
    let path = project_dir.join("pymgr.lock");

    if !path.exists() {
        return Err(PymgrError::coded(
            ErrorCode::LockfileMissing,
            "No pymgr.lock found to build tree.",
        ));
    }

    let lockfile = Lockfile::load(&path)?;
    if lockfile.packages.is_empty() {
        output::print_success("No dependencies to show in tree.");
        return Ok(());
    }

    let mut pkg_map: HashMap<&str, &LockedPackage> = HashMap::new();
    for pkg in &lockfile.packages {
        pkg_map.insert(&pkg.name, pkg);
    }

    let mut all_deps: HashSet<&str> = HashSet::new();
    for pkg in &lockfile.packages {
        for dep in &pkg.dependencies {
            all_deps.insert(&dep.name);
        }
    }

    let mut root_pkgs: Vec<&LockedPackage> = Vec::new();
    for pkg in &lockfile.packages {
        if !all_deps.contains(pkg.name.as_str()) {
            root_pkgs.push(pkg);
        }
    }
    
    root_pkgs.sort_by(|a, b| a.name.cmp(&b.name));

    let project_name = project_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project");
        
    println!("{}", project_name);

    for (i, pkg) in root_pkgs.iter().enumerate() {
        let is_last = i == root_pkgs.len() - 1;
        print_tree_node(pkg, &pkg_map, "", is_last, &mut HashSet::new());
    }

    Ok(())
}

fn print_tree_node<'a>(
    pkg: &'a LockedPackage,
    pkg_map: &HashMap<&str, &'a LockedPackage>,
    prefix: &str,
    is_last: bool,
    visited: &mut HashSet<&'a str>,
) {
    let branch = if is_last { "└── " } else { "├── " };
    
    if visited.contains(pkg.name.as_str()) {
        println!("{}{}{}@{} (deduped)", prefix, branch, pkg.name, pkg.version);
        return;
    }
    
    println!("{}{}{}@{}", prefix, branch, pkg.name, pkg.version);
    visited.insert(pkg.name.as_str());

    let child_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });
    
    let deps = &pkg.dependencies;
    let mut sorted_deps: Vec<_> = deps.iter().collect();
    sorted_deps.sort_by(|a, b| a.name.cmp(&b.name));
    
    for (i, dep) in sorted_deps.iter().enumerate() {
        let child_is_last = i == sorted_deps.len() - 1;
        if let Some(&child_pkg) = pkg_map.get(dep.name.as_str()) {
            print_tree_node(child_pkg, pkg_map, &child_prefix, child_is_last, visited);
        } else {
            let child_branch = if child_is_last { "└── " } else { "├── " };
            println!("{}{}{} (missing)", child_prefix, child_branch, dep.name);
        }
    }
}
