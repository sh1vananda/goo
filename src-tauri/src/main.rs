#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct HistoryPayload {
    entries: Vec<goo::enrich::EnrichedEntry>,
    cache_warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct StoredSettings {
    log_path: Option<String>,
    cache_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
struct SettingsPayload {
    log_path: Option<String>,
    cache_path: Option<String>,
    tmdb_key_present: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct SettingsInput {
    log_path: Option<String>,
    cache_path: Option<String>,
    tmdb_api_key: Option<String>,
}

#[tauri::command]
fn load_history(
    log_path: Option<String>,
    cache_path: Option<String>,
    tmdb_api_key: Option<String>,
) -> Result<HistoryPayload, String> {
    let settings = read_settings();
    let log_path = resolve_log_path(log_path.or(settings.log_path))?;
    let cache_path = cache_path.or(settings.cache_path);
    let api_key = tmdb_api_key
        .and_then(normalize_key)
        .or_else(read_tmdb_key);

    let cache_path = cache_path.as_deref().map(Path::new);
    let api_key = api_key.as_deref();
    let history =
        goo::app::load_enriched_history(&log_path, cache_path, api_key).map_err(|err| err.to_string())?;

    Ok(HistoryPayload {
        entries: history.entries,
        cache_warning: history.cache_warning,
    })
}

#[tauri::command]
fn load_settings() -> Result<SettingsPayload, String> {
    let settings = read_settings();
    let tmdb_key_present = read_tmdb_key().is_some();
    Ok(SettingsPayload {
        log_path: settings.log_path,
        cache_path: settings.cache_path,
        tmdb_key_present,
    })
}

#[tauri::command]
fn save_settings(settings: SettingsInput) -> Result<(), String> {
    let stored = StoredSettings {
        log_path: settings.log_path,
        cache_path: settings.cache_path,
    };
    write_settings(&stored)?;
    if let Some(key) = settings.tmdb_api_key.and_then(normalize_key) {
        store_tmdb_key(&key)?;
    }
    Ok(())
}

#[tauri::command]
fn clear_tmdb_key() -> Result<(), String> {
    delete_tmdb_key()
}

#[tauri::command]
fn delete_log(log_path: Option<String>) -> Result<(), String> {
    let settings = read_settings();
    let log_path = resolve_log_path(log_path.or(settings.log_path))?;
    delete_log_file(&log_path)
}

#[tauri::command]
fn delete_entry(
    log_path: Option<String>,
    cleaned_title: String,
    release_year: Option<i32>,
) -> Result<(), String> {
    let settings = read_settings();
    let log_path = resolve_log_path(log_path.or(settings.log_path))?;
    delete_log_entries(&log_path, &cleaned_title, release_year)
}

fn resolve_log_path(arg: Option<String>) -> Result<PathBuf, String> {
    if let Some(value) = arg {
        return Ok(PathBuf::from(value));
    }

    goo::app::default_log_path().ok_or_else(|| {
        "Log path not found. Set GOO_LOG_PATH or pass a path.".to_string()
    })
}

fn read_settings() -> StoredSettings {
    let Some(path) = settings_path() else {
        return StoredSettings::default();
    };

    let Ok(content) = fs::read_to_string(path) else {
        return StoredSettings::default();
    };

    serde_json::from_str(&content).unwrap_or_default()
}

fn write_settings(settings: &StoredSettings) -> Result<(), String> {
    let Some(path) = settings_path() else {
        return Err("Settings path not available".to_string());
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let payload = serde_json::to_string_pretty(settings).map_err(|err| err.to_string())?;
    fs::write(path, payload).map_err(|err| err.to_string())
}

fn settings_path() -> Option<PathBuf> {
    let base = config_base_dir()?;
    Some(base.join("settings.json"))
}

fn config_base_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(|base| PathBuf::from(base).join("goo"))
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME")
            .map(|base| PathBuf::from(base).join("Library").join("Application Support").join("goo"))
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(base) = std::env::var_os("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(base).join("goo"));
        }
        std::env::var_os("HOME").map(|base| PathBuf::from(base).join(".config").join("goo"))
    }
}

fn normalize_key(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(target_os = "windows")]
fn read_tmdb_key() -> Option<String> {
    let entry = keyring::Entry::new("goo", "tmdb_api_key").ok()?;
    match entry.get_password() {
        Ok(value) => normalize_key(value),
        Err(_) => None,
    }
}

#[cfg(not(target_os = "windows"))]
fn read_tmdb_key() -> Option<String> {
    None
}

#[cfg(target_os = "windows")]
fn store_tmdb_key(value: &str) -> Result<(), String> {
    let entry = keyring::Entry::new("goo", "tmdb_api_key").map_err(|err| err.to_string())?;
    entry.set_password(value).map_err(|err| err.to_string())
}

#[cfg(not(target_os = "windows"))]
fn store_tmdb_key(_value: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn delete_tmdb_key() -> Result<(), String> {
    let entry = keyring::Entry::new("goo", "tmdb_api_key").map_err(|err| err.to_string())?;
    entry.delete_password().map_err(|err| err.to_string())
}

#[cfg(not(target_os = "windows"))]
fn delete_tmdb_key() -> Result<(), String> {
    Ok(())
}

fn install_vlc_logger() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let Some(appdata) = std::env::var_os("APPDATA") else {
            return Ok(());
        };
        let vlc_dir = PathBuf::from(appdata).join("vlc");
        let intf_dir = vlc_dir.join("lua").join("intf");
        fs::create_dir_all(&intf_dir).map_err(|err| err.to_string())?;

        let target = intf_dir.join("goo_logger_intf.lua");
        let payload = include_bytes!("../../vlc/goo_logger_intf.lua");
        if fs::read(&target).map(|existing| existing == payload).unwrap_or(false) {
            ensure_vlcrc(&vlc_dir)?;
            return Ok(());
        }

        fs::write(&target, payload).map_err(|err| err.to_string())?;
        ensure_vlcrc(&vlc_dir)?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Non-Windows installs are currently manual.
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn ensure_vlcrc(vlc_dir: &Path) -> Result<(), String> {
    let vlcrc_path = vlc_dir.join("vlcrc");
    let content = fs::read_to_string(&vlcrc_path).unwrap_or_default();
    let content = upsert_setting(&content, "lua-intf", "goo_logger_intf");
    let content = upsert_extraintf(&content, "luaintf");
    fs::write(&vlcrc_path, content).map_err(|err| err.to_string())
}

#[cfg(target_os = "windows")]
fn upsert_setting(content: &str, key: &str, value: &str) -> String {
    let mut found = false;
    let mut lines = Vec::new();
    for line in content.lines() {
        if is_setting_line(line, key) {
            lines.push(format!("{key}={value}"));
            found = true;
        } else {
            lines.push(line.to_string());
        }
    }

    if !found {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.push(format!("{key}={value}"));
    }

    lines.join("\n")
}

#[cfg(target_os = "windows")]
fn upsert_extraintf(content: &str, value: &str) -> String {
    let key = "extraintf";
    let mut found = false;
    let mut lines = Vec::new();
    for line in content.lines() {
        if is_setting_line(line, key) {
            found = true;
            lines.push(format!("{key}={}", merge_extraintf_value(line, value)));
        } else {
            lines.push(line.to_string());
        }
    }

    if !found {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.push(format!("{key}={value}"));
    }

    lines.join("\n")
}

#[cfg(target_os = "windows")]
fn merge_extraintf_value(line: &str, value: &str) -> String {
    let Some((_, raw)) = line.split_once('=') else {
        return value.to_string();
    };
    let mut items: Vec<String> = raw
        .split(|ch| ch == ':' || ch == ',')
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(|item| item.to_string())
        .collect();

    if !items.iter().any(|item| item.eq_ignore_ascii_case(value)) {
        items.push(value.to_string());
    }

    items.join(":")
}

#[cfg(target_os = "windows")]
fn is_setting_line(line: &str, key: &str) -> bool {
    let trimmed = line.trim_start();
    let trimmed = trimmed.strip_prefix('#').unwrap_or(trimmed).trim_start();
    trimmed.starts_with(&format!("{key}="))
}

fn main() {
    tauri::Builder::default()
        .setup(|_| {
            if let Err(err) = install_vlc_logger() {
                eprintln!("Failed to install VLC logger: {err}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_history,
            load_settings,
            save_settings,
            clear_tmdb_key,
            delete_log,
            delete_entry
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn delete_log_file(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}

fn delete_log_entries(
    path: &Path,
    cleaned_title: &str,
    release_year: Option<i32>,
) -> Result<(), String> {
    let target = cleaned_title.trim();
    if target.is_empty() {
        return Ok(());
    }

    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err.to_string()),
    };

    let target_normalized = target.to_lowercase();
    let mut kept = Vec::new();
    let mut removed_any = false;

    for line in content.lines() {
        let should_remove = goo::parse_log_line(line)
            .map(|entry| {
                entry.cleaned_title.trim().to_lowercase() == target_normalized
                    && entry.release_year == release_year
            })
            .unwrap_or(false);

        if should_remove {
            removed_any = true;
            continue;
        }

        kept.push(line);
    }

    if !removed_any {
        return Ok(());
    }

    let mut new_content = kept.join("\n");
    if !new_content.is_empty() {
        new_content.push('\n');
    }
    fs::write(path, new_content).map_err(|err| err.to_string())
}
