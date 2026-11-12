use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

static GUARD: OnceLock<WorkerGuard> = OnceLock::new();

pub fn data_dir() -> PathBuf {
    if let Ok(value) = std::env::var("VERBATIM_DATA_DIR") {
        return PathBuf::from(value);
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".verbatim")
}

pub fn log_dir() -> PathBuf {
    let dir = std::env::var("VERBATIM_LOG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| data_dir().join("logs"));
    let _ = fs::create_dir_all(&dir);
    dir
}

/// Rotating daily logs with seven-day retention. No console spam in release.
pub fn init() -> Result<PathBuf, String> {
    let dir = log_dir();
    let file_appender = tracing_appender::rolling::daily(&dir, "verbatim.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let _ = GUARD.set(guard);

    let filter = EnvFilter::try_from_env("VERBATIM_LOG")
        .unwrap_or_else(|_| EnvFilter::new("info,verbatim_lib=debug"));
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .with_target(true)
        .with_writer(non_blocking);
    let _ = subscriber.try_init();

    cleanup_old_logs(&dir, 7);
    tracing::info!(path = %dir.display(), "Verbatim logging initialized");
    Ok(dir)
}

fn cleanup_old_logs(dir: &PathBuf, keep_days: u64) {
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(keep_days * 24 * 60 * 60))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let is_ours = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with("verbatim.log") || name.starts_with("asr-service") || name.starts_with("ocr-service"))
            .unwrap_or(false);
        if !is_ours {
            continue;
        }
        if let Ok(modified) = entry.metadata().and_then(|meta| meta.modified()) {
            if modified < cutoff {
                let _ = fs::remove_file(path);
            }
        }
    }
}
