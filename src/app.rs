use crate::enrich::{enrich_entries, EnrichedEntry, MovieCache};
use crate::tmdb::{TmdbClient, TmdbError};
use crate::read_watch_log;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum AppError {
    MissingLogPath,
    Io(std::io::Error),
    Tmdb(TmdbError),
}

#[derive(Debug, Clone)]
pub struct EnrichedHistory {
    pub entries: Vec<EnrichedEntry>,
    pub cache_path: PathBuf,
    pub cache_warning: Option<String>,
}

pub fn load_enriched_history(
    log_path: &Path,
    cache_path: Option<&Path>,
    tmdb_api_key: Option<&str>,
) -> Result<EnrichedHistory, AppError> {
    let cache_path = cache_path
        .map(PathBuf::from)
        .unwrap_or_else(|| default_cache_path(log_path));
    let client = if let Some(key) = tmdb_api_key {
        TmdbClient::new(key)
    } else {
        TmdbClient::from_env()?
    };
    let entries = read_watch_log(log_path)?;

    let mut cache = MovieCache::load(&cache_path);
    let enriched = enrich_entries(entries, &client, &mut cache)?;
    let cache_warning = cache
        .save(&cache_path)
        .err()
        .map(|error| error.to_string());

    Ok(EnrichedHistory {
        entries: enriched,
        cache_path,
        cache_warning,
    })
}

pub fn default_log_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("GOO_LOG_PATH") {
        let path = PathBuf::from(path);
        if !path.as_os_str().is_empty() {
            return Some(path);
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let base = PathBuf::from(appdata);
            let vlc = base.join("vlc");
            if vlc.exists() {
                return Some(vlc.join(".goo_watch_log.txt"));
            }
            return Some(base.join(".goo_watch_log.txt"));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            let candidates = [
                home.join(".local/share/vlc"),
                home.join(".config/vlc"),
                home.clone(),
            ];
            for base in candidates {
                if base.exists() {
                    return Some(base.join(".goo_watch_log.txt"));
                }
            }
            return Some(home.join(".goo_watch_log.txt"));
        }
    }

    None
}

pub fn default_cache_path(log_path: &Path) -> PathBuf {
    log_path
        .parent()
        .map(|parent| parent.join(".goo_cache.json"))
        .unwrap_or_else(|| PathBuf::from(".goo_cache.json"))
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::MissingLogPath => write!(f, "log path is missing"),
            AppError::Io(error) => write!(f, "io error: {error}"),
            AppError::Tmdb(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        AppError::Io(error)
    }
}

impl From<TmdbError> for AppError {
    fn from(error: TmdbError) -> Self {
        AppError::Tmdb(error)
    }
}
