use crate::errors::{ErrorCode, PymgrError, PymgrResult};
use crate::output;

pub async fn self_update() -> PymgrResult<()> {
    let spinner = output::create_spinner("Checking for updates...");

    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/repos/pymgr/pymgr/releases/latest")
        .header("User-Agent", "pymgr")
        .send()
        .await
        .map_err(|e| {
            PymgrError::coded(
                ErrorCode::NetworkError,
                format!("Failed to check for updates: {}", e),
            )
        })?;

    if !response.status().is_success() {
        spinner.finish_and_clear();
        return Err(PymgrError::coded(
            ErrorCode::NetworkError,
            "Failed to fetch release information",
        ));
    }

    let release: serde_json::Value = response.json().await?;
    let latest_version = release["tag_name"]
        .as_str()
        .unwrap_or("unknown")
        .trim_start_matches('v');

    let current_version = env!("CARGO_PKG_VERSION");

    spinner.finish_and_clear();

    if latest_version == current_version {
        output::print_success(&format!("Already up to date (v{})", current_version));
        return Ok(());
    }

    output::print_info(&format!(
        "New version available: v{} → v{}",
        current_version, latest_version
    ));

    let target = if cfg!(target_os = "windows") {
        "x86_64-pc-windows-msvc"
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "aarch64-apple-darwin"
        } else {
            "x86_64-apple-darwin"
        }
    } else {
        "x86_64-unknown-linux-gnu"
    };

    let _asset_name = format!("pymgr-{}", target);

    if let Some(assets) = release["assets"].as_array() {
        for asset in assets {
            if let Some(name) = asset["name"].as_str() {
                if name.contains(target) {
                    let download_url = asset["browser_download_url"].as_str().unwrap_or("");
                    output::print_info(&format!("Download from: {}", download_url));
                    output::print_info(
                        "Self-update download is not yet implemented — please download manually",
                    );
                    return Ok(());
                }
            }
        }
    }

    output::print_warning("No matching binary found for your platform");
    Ok(())
}
