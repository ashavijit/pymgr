use std::io::Cursor;
use std::path::Path;

use crate::errors::PymgrResult;
use crate::output;
use tar::Archive;
use flate2::read::GzDecoder;

pub fn extract_wheel(wheel_path: &Path, dest: &Path) -> PymgrResult<()> {
    let file = std::fs::File::open(wheel_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| crate::errors::PymgrError::Other(format!("Invalid wheel: {}", e)))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| crate::errors::PymgrError::Other(format!("Zip error: {}", e)))?;

        let outpath = dest.join(file.mangled_name());

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
}

pub fn extract_wheel_bytes(data: &[u8], dest: &Path) -> PymgrResult<()> {
    let reader = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| crate::errors::PymgrError::Other(format!("Invalid wheel: {}", e)))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| crate::errors::PymgrError::Other(format!("Zip error: {}", e)))?;

        let outpath = dest.join(file.mangled_name());

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
}

pub fn extract_tar_gz_bytes(data: &[u8], dest: &Path) -> PymgrResult<()> {
    let cursor = Cursor::new(data);
    let tar = GzDecoder::new(cursor);
    let mut archive = Archive::new(tar);
    archive.unpack(dest).map_err(|e| {
        crate::errors::PymgrError::Other(format!("Failed to unpack tar.gz: {}", e))
    })?;
    Ok(())
}

pub fn uninstall_package(site_packages: &Path, name: &str) -> PymgrResult<()> {
    let normalized = name.replace('-', "_").to_lowercase();

    let entries: Vec<_> = std::fs::read_dir(site_packages)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let fname = e.file_name().to_string_lossy().to_lowercase();
            fname.starts_with(&normalized)
        })
        .collect();

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            std::fs::remove_dir_all(&path)?;
        } else {
            std::fs::remove_file(&path)?;
        }
    }

    output::print_verbose(&format!("Removed {} from site-packages", name));
    Ok(())
}

pub fn list_installed(site_packages: &Path) -> PymgrResult<Vec<InstalledPackage>> {
    let mut packages = Vec::new();

    if !site_packages.exists() {
        return Ok(packages);
    }

    for entry in std::fs::read_dir(site_packages)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();

        if name.ends_with(".dist-info") {
            let pkg_name = name.trim_end_matches(".dist-info");
            if let Some((n, v)) = pkg_name.rsplit_once('-') {
                packages.push(InstalledPackage {
                    name: n.replace('_', "-"),
                    version: v.to_string(),
                });
            }
        }
    }

    packages.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(packages)
}

#[derive(Debug)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
}
