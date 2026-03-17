use crate::cache;
use crate::errors::{ErrorCode, PymgrError, PymgrResult};
use crate::output;
use crate::resolver::pypi;

pub fn exec_clear(target: Option<&str>) -> PymgrResult<()> {
    match target {
        Some("metadata") => cache::clear_metadata_cache()?,
        Some("wheels") => cache::clear_wheel_cache()?,
        Some("all") | None => {
            cache::clear_metadata_cache()?;
            cache::clear_wheel_cache()?;
        }
        Some(other) => {
            return Err(PymgrError::coded(
                ErrorCode::ConfigError,
                format!("Unknown cache target: {}. Use 'metadata', 'wheels', or 'all'.", other),
            ));
        }
    }
    Ok(())
}

pub fn exec_info() -> PymgrResult<()> {
    let stats = cache::cache_stats()?;
    let mut table = comfy_table::Table::new();
    table.load_preset(comfy_table::presets::UTF8_FULL);
    table.set_header(vec!["Metric", "Value"]);
    
    table.add_row(vec!["Metadata Cache Entries", &stats.metadata_entries.to_string()]);
    table.add_row(vec!["Metadata Cache Size", &cache::CacheStats::format_size(stats.metadata_size)]);
    table.add_row(vec!["Wheel Cache Entries", &stats.wheel_entries.to_string()]);
    table.add_row(vec!["Wheel Cache Size", &cache::CacheStats::format_size(stats.wheel_size)]);
    table.add_row(vec!["Total Cache Size", &cache::CacheStats::format_size(stats.total_size())]);
    
    println!("Cache Information:\n{}", table);
    Ok(())
}

pub fn exec_gc(_dry_run: bool, _aggressive: bool) -> PymgrResult<()> {
    output::print_info("Garbage collection is a work in progress. Use `pymgr cache clear` for now.");
    Ok(())
}

pub async fn exec_warm(packages: &[String]) -> PymgrResult<()> {
    if packages.is_empty() {
        return Ok(());
    }

    let pb = output::create_progress_bar(packages.len() as u64, "Warming cache");

    for pkg in packages {
        let (name, version) = pypi::parse_requirement(pkg);
        if let Some(v) = version {
            let clean_v = v.replace(">=", "").replace("==", "").replace("<=", "").replace("~=", "").replace("!=", "");
            let _ = pypi::fetch_version_info(&name, &clean_v).await;
        } else {
            if let Ok(info) = pypi::fetch_package_info(&name).await {
                let _ = pypi::fetch_version_info(&name, &info.info.version).await;
            }
        }
        pb.inc(1);
    }

    pb.finish_and_clear();
    output::print_success(&format!("Warmed metadata cache for {} packages", packages.len()));
    Ok(())
}
