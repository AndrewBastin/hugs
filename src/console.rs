use std::sync::OnceLock;
use std::time::Instant;

use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;

static BUILD_START: OnceLock<Instant> = OnceLock::new();

fn status_style(verb: &str, color: owo_colors::AnsiColors) -> String {
    format!("{:>12}", verb.color(color).bold())
}

pub fn status(verb: &str, message: impl std::fmt::Display) {
    eprintln!("{} {}", status_style(verb, owo_colors::AnsiColors::Green), message);
}

pub fn status_cyan(verb: &str, message: impl std::fmt::Display) {
    eprintln!("{} {}", status_style(verb, owo_colors::AnsiColors::Cyan), message);
}

pub fn warn(message: impl std::fmt::Display) {
    eprintln!("{} {}", status_style("Warning", owo_colors::AnsiColors::Yellow), message);
}

pub fn start_build() {
    BUILD_START.get_or_init(Instant::now);
}

pub fn finished(summary: impl std::fmt::Display) {
    let elapsed = BUILD_START
        .get()
        .map(|start| start.elapsed())
        .unwrap_or_default();

    eprintln!(
        "{} {} in {:.2}s",
        status_style("Finished", owo_colors::AnsiColors::Green),
        summary,
        elapsed.as_secs_f64()
    );
}

pub fn create_progress_bar(total: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(&format!(
                "{{prefix:>12}} [{{bar:30.cyan/dim}}] {{pos}}/{{len}} {}",
                message
            ))
            .unwrap()
            .progress_chars("━━─"),
    );
    pb.set_prefix("Rendering".green().bold().to_string());
    pb
}

pub fn progress_finish(pb: &ProgressBar) {
    pb.finish_and_clear();
}
