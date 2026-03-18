use console::{style, Term};
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::atomic::{AtomicBool, Ordering};

static JSON_MODE: AtomicBool = AtomicBool::new(false);
static VERBOSE_MODE: AtomicBool = AtomicBool::new(false);
static NO_COLOR: AtomicBool = AtomicBool::new(false);

pub fn set_json_mode(enabled: bool) {
    JSON_MODE.store(enabled, Ordering::SeqCst);
}

pub fn set_verbose_mode(enabled: bool) {
    VERBOSE_MODE.store(enabled, Ordering::SeqCst);
}

pub fn set_no_color(enabled: bool) {
    NO_COLOR.store(enabled, Ordering::SeqCst);
}

pub fn is_json_mode() -> bool {
    JSON_MODE.load(Ordering::SeqCst)
}

pub fn is_verbose() -> bool {
    VERBOSE_MODE.load(Ordering::SeqCst)
}

pub fn print_success(message: &str) {
    if is_json_mode() {
        println!(
            "{}",
            serde_json::json!({"status": "success", "message": message})
        );
    } else {
        let term = Term::stderr();
        let _ = term.write_line(&format!("{} {}", style("✓").green().bold(), message));
    }
}

pub fn print_info(message: &str) {
    if is_json_mode() {
        return;
    }
    let term = Term::stderr();
    let _ = term.write_line(&format!("{} {}", style("ℹ").blue().bold(), message));
}

pub fn print_warning(message: &str) {
    if is_json_mode() {
        return;
    }
    let term = Term::stderr();
    let _ = term.write_line(&format!("{} {}", style("⚠").yellow().bold(), message));
}

pub fn print_error(message: &str) {
    if is_json_mode() {
        eprintln!(
            "{}",
            serde_json::json!({"status": "error", "message": message})
        );
    } else {
        let term = Term::stderr();
        let _ = term.write_line(&format!("{} {}", style("✗").red().bold(), message));
    }
}

pub fn print_verbose(message: &str) {
    if !is_verbose() || is_json_mode() {
        return;
    }
    let term = Term::stderr();
    let _ = term.write_line(&format!("{} {}", style("→").dim(), style(message).dim()));
}

pub fn print_json(value: &serde_json::Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_default()
    );
}

pub fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}

pub fn create_progress_bar(len: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.cyan} {msg} [{bar:30.cyan/dim}] {pos}/{len}")
            .unwrap()
            .progress_chars("━╸─"),
    );
    pb.set_message(message.to_string());
    pb
}

pub fn print_header(title: &str) {
    if is_json_mode() {
        return;
    }
    let term = Term::stderr();
    let _ = term.write_line(&format!("\n{}\n", style(title).bold().underlined()));
}

pub fn print_key_value(key: &str, value: &str) {
    if is_json_mode() {
        return;
    }
    let term = Term::stderr();
    let _ = term.write_line(&format!("  {}: {}", style(key).bold(), value));
}
