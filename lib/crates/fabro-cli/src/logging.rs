use std::fs::OpenOptions;
use std::path::PathBuf;

use anyhow::Result;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_tracing(debug: bool, config_log_level: Option<&str>, log_prefix: &str) -> Result<()> {
    let default_level = if debug {
        "debug"
    } else {
        config_log_level.unwrap_or("info")
    };
    let filter =
        EnvFilter::try_from_env("FABRO_LOG").unwrap_or_else(|_| EnvFilter::new(default_level));
    let run_log_writer = fabro_util::run_log::init();
    if let Some(file_writer) = build_file_log_writer(log_prefix) {
        tracing_subscriber::registry()
            .with(filter)
            .with(
                fmt::layer()
                    .with_writer(file_writer)
                    .with_target(true)
                    .with_ansi(false),
            )
            .with(
                fmt::layer()
                    .with_writer(run_log_writer)
                    .with_target(true)
                    .with_ansi(false),
            )
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(
                fmt::layer()
                    .with_writer(run_log_writer)
                    .with_target(true)
                    .with_ansi(false),
            )
            .init();
    }

    Ok(())
}

fn build_file_log_writer(log_prefix: &str) -> Option<BoxMakeWriter> {
    let filename = chrono::Local::now()
        .format(&format!("{log_prefix}-%Y-%m-%d.log"))
        .to_string();
    let log_dir = first_writable_log_dir(&log_dir_candidates(), &filename)?;
    Some(BoxMakeWriter::new(tracing_appender::rolling::never(
        log_dir, filename,
    )))
}

fn log_dir_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(path) = std::env::var("FABRO_LOG_DIR") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            candidates.push(PathBuf::from(trimmed));
        }
    }
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".fabro").join("logs"));
    }
    candidates.push(PathBuf::from(".fabro").join("logs"));
    candidates.push(std::env::temp_dir().join("fabro").join("logs"));
    candidates.dedup();
    candidates
}

fn first_writable_log_dir(candidates: &[PathBuf], filename: &str) -> Option<PathBuf> {
    for candidate in candidates {
        if std::fs::create_dir_all(candidate).is_err() {
            continue;
        }
        let log_path = candidate.join(filename);
        if OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .is_ok()
        {
            return Some(candidate.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{first_writable_log_dir, log_dir_candidates};

    #[test]
    fn first_writable_log_dir_skips_unusable_candidates() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let blocked = tempdir.path().join("blocked");
        std::fs::write(&blocked, "not a directory").expect("blocked marker");
        let fallback = tempdir.path().join("fallback");
        let selected = first_writable_log_dir(&[blocked, fallback.clone()], "cli.log");
        assert_eq!(selected, Some(fallback));
    }

    #[test]
    fn log_dir_candidates_include_local_and_temp_fallbacks() {
        let candidates = log_dir_candidates();
        assert!(candidates
            .iter()
            .any(|path| path == &PathBuf::from(".fabro").join("logs")));
        assert!(candidates
            .iter()
            .any(|path| path == &std::env::temp_dir().join("fabro").join("logs")));
    }
}
