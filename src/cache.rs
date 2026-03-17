use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::config::pymgr_home;
use crate::errors::PymgrResult;
use crate::output;

const METADATA_TTL_SECS: u64 = 300;

pub fn cache_dir() -> PathBuf {
    pymgr_home().join("cache")
}

pub fn metadata_cache_dir() -> PathBuf {
    cache_dir().join("metadata")
}

pub fn wheel_cache_dir() -> PathBuf {
    cache_dir().join("wheels")
}

pub fn get_cached_metadata(package: &str, version: &str) -> Option<String> {
    let path = metadata_cache_dir()
        .join(package)
        .join(format!("{}.json", version));

    if !path.exists() {
        return None;
    }

    if let Ok(meta) = std::fs::metadata(&path) {
        if let Ok(modified) = meta.modified() {
            let age = SystemTime::now()
                .duration_since(modified)
                .unwrap_or(Duration::MAX);
            if !crate::config::is_offline_mode() && age > Duration::from_secs(METADATA_TTL_SECS) {
                let _ = std::fs::remove_file(&path);
                return None;
            }
        }
    }

    std::fs::read_to_string(&path).ok()
}

pub fn store_metadata(package: &str, version: &str, data: &str) -> PymgrResult<()> {
    let dir = metadata_cache_dir().join(package);
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(format!("{}.json", version)), data)?;
    Ok(())
}

pub fn get_cached_wheel(sha256: &str) -> Option<PathBuf> {
    let shard = &sha256[..4.min(sha256.len())];
    let path = wheel_cache_dir().join(shard).join(sha256);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

pub fn store_wheel(sha256: &str, data: &[u8]) -> PymgrResult<PathBuf> {
    let shard = &sha256[..4.min(sha256.len())];
    let dir = wheel_cache_dir().join(shard).join(sha256);
    std::fs::create_dir_all(&dir)?;
    let wheel_path = dir.join("wheel.whl");
    std::fs::write(&wheel_path, data)?;
    Ok(wheel_path)
}

pub fn install_from_cache(cached_path: &Path, dest: &Path) -> PymgrResult<()> {
    match std::fs::hard_link(cached_path, dest) {
        Ok(_) => {}
        Err(_) => {
            std::fs::copy(cached_path, dest)?;
        }
    }
    Ok(())
}

pub fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

pub fn verify_sha256(data: &[u8], expected: &str) -> bool {
    let actual = compute_sha256(data);
    actual == expected
}

pub fn clear_metadata_cache() -> PymgrResult<()> {
    let dir = metadata_cache_dir();
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
        std::fs::create_dir_all(&dir)?;
    }
    output::print_success("Cleared metadata cache");
    Ok(())
}

pub fn clear_wheel_cache() -> PymgrResult<()> {
    let dir = wheel_cache_dir();
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
        std::fs::create_dir_all(&dir)?;
    }
    output::print_success("Cleared wheel cache");
    Ok(())
}

pub fn cache_stats() -> PymgrResult<CacheStats> {
    let mut stats = CacheStats::default();

    let meta_dir = metadata_cache_dir();
    if meta_dir.exists() {
        for entry in walkdir::WalkDir::new(&meta_dir).into_iter().flatten() {
            if entry.file_type().is_file() {
                stats.metadata_entries += 1;
                stats.metadata_size += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }

    let wheel_dir = wheel_cache_dir();
    if wheel_dir.exists() {
        for entry in walkdir::WalkDir::new(&wheel_dir).into_iter().flatten() {
            if entry.file_type().is_file() {
                stats.wheel_entries += 1;
                stats.wheel_size += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }

    Ok(stats)
}

#[derive(Debug, Default)]
pub struct CacheStats {
    pub metadata_entries: usize,
    pub metadata_size: u64,
    pub wheel_entries: usize,
    pub wheel_size: u64,
}

impl CacheStats {
    pub fn total_size(&self) -> u64 {
        self.metadata_size + self.wheel_size
    }

    pub fn format_size(bytes: u64) -> String {
        if bytes >= 1_073_741_824 {
            format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
        } else if bytes >= 1_048_576 {
            format!("{:.1} MB", bytes as f64 / 1_048_576.0)
        } else if bytes >= 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{} B", bytes)
        }
    }
}
