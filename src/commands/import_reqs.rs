use crate::commands::packages::exec_add;
use crate::config::PymgrConfig;
use crate::errors::{ErrorCode, PymgrError, PymgrResult};

pub async fn exec(file: &str) -> PymgrResult<()> {
    let project_dir = PymgrConfig::find_project_root()?;
    let bytes = std::fs::read(file).map_err(|e| {
        PymgrError::coded(
            ErrorCode::ConfigError,
            format!("Failed to read {}: {}", file, e),
        )
    })?;

    let content = if bytes.starts_with(&[0xFF, 0xFE]) {
        let u16_data: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        String::from_utf16_lossy(&u16_data)
    } else if bytes.starts_with(&[0xFE, 0xFF]) {
        let u16_data: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect();
        String::from_utf16_lossy(&u16_data)
    } else {
        String::from_utf8_lossy(&bytes).to_string()
    };

    let mut packages = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let token = line.split_whitespace().next().unwrap_or(line);
        if token.starts_with('-') {
            continue;
        }

        packages.push(token.to_string());
    }

    if packages.is_empty() {
        println!("No valid packages found to import.");
        return Ok(());
    }

    println!("Importing {} packages from {}...", packages.len(), file);
    exec_add(&project_dir, &packages, false, false).await?;

    Ok(())
}
