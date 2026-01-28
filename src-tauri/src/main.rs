#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};

#[derive(serde::Serialize)]
struct HistoryPayload {
    entries: Vec<goo::enrich::EnrichedEntry>,
    cache_warning: Option<String>,
}

#[tauri::command]
fn load_history(
    log_path: Option<String>,
    cache_path: Option<String>,
    tmdb_api_key: Option<String>,
) -> Result<HistoryPayload, String> {
    let log_path = resolve_log_path(log_path)?;
    let cache_path = cache_path.as_deref().map(Path::new);
    let api_key = tmdb_api_key.as_deref();
    let history =
        goo::app::load_enriched_history(&log_path, cache_path, api_key).map_err(|err| err.to_string())?;

    Ok(HistoryPayload {
        entries: history.entries,
        cache_warning: history.cache_warning,
    })
}

fn resolve_log_path(arg: Option<String>) -> Result<PathBuf, String> {
    if let Some(value) = arg {
        return Ok(PathBuf::from(value));
    }

    goo::app::default_log_path().ok_or_else(|| {
        "Log path not found. Set GOO_LOG_PATH or pass a path.".to_string()
    })
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![load_history])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
