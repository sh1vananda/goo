use crate::tmdb::{TmdbClient, TmdbError, TmdbMovie, DEFAULT_POSTER_SIZE};
use crate::WatchEntry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct EnrichedEntry {
    pub watched_at: Option<String>,
    pub raw_title: String,
    pub cleaned_title: String,
    pub movie: Option<TmdbMovie>,
    pub tmdb_url: Option<String>,
    pub poster_url: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MovieCache {
    entries: HashMap<String, Option<TmdbMovie>>,
}

impl MovieCache {
    pub fn load(path: &Path) -> Self {
        let Ok(content) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        serde_json::from_str(&content).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let data = serde_json::to_string(self)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        std::fs::write(path, data)
    }
}

pub fn enrich_entries(
    entries: Vec<WatchEntry>,
    client: &TmdbClient,
    cache: &mut MovieCache,
) -> Result<Vec<EnrichedEntry>, TmdbError> {
    let mut enriched = Vec::with_capacity(entries.len());
    for entry in entries {
        let key = cache_key(&entry.cleaned_title, entry.release_year);
        let movie = if key.is_empty() {
            None
        } else if let Some(cached) = cache.entries.get(&key) {
            cached.clone()
        } else {
            let fetched = client.best_match(&entry.cleaned_title, entry.release_year)?;
            cache.entries.insert(key, fetched.clone());
            fetched
        };

        enriched.push(EnrichedEntry::from_watch(entry, movie));
    }
    Ok(enriched)
}

fn cache_key(title: &str, year: Option<i32>) -> String {
    let mut key = title.trim().to_lowercase();
    if let Some(year) = year {
        key.push('|');
        key.push_str(&year.to_string());
    }
    key
}

impl EnrichedEntry {
    fn from_watch(entry: WatchEntry, movie: Option<TmdbMovie>) -> Self {
        let tmdb_url = movie.as_ref().map(|item| item.tmdb_url());
        let poster_url = movie
            .as_ref()
            .and_then(|item| item.poster_url(DEFAULT_POSTER_SIZE));
        Self {
            watched_at: entry.watched_at,
            raw_title: entry.raw_title,
            cleaned_title: entry.cleaned_title,
            movie,
            tmdb_url,
            poster_url,
        }
    }
}
