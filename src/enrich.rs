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
    pub release_year: Option<i32>,
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
        let mut cache: Self = serde_json::from_str(&content).unwrap_or_default();
        // Evict negative entries on load so unmatched/poisoned entries are re-queried
        cache.entries.retain(|_, v| v.is_some());
        cache
    }

    pub fn set_entry(&mut self, title: &str, year: Option<i32>, movie: TmdbMovie) {
        let key = cache_key(title, year);
        self.entries.insert(key, Some(movie));
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let data = serde_json::to_string(self)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        std::fs::write(path, data)
    }
}

pub fn enrich_entries(
    entries: Vec<WatchEntry>,
    client: Option<&TmdbClient>,
    cache: &mut MovieCache,
) -> Result<Vec<EnrichedEntry>, TmdbError> {
    let mut enriched = Vec::with_capacity(entries.len());
    for entry in entries {
        let key = cache_key(&entry.cleaned_title, entry.release_year);
        let movie = if key.is_empty() {
            None
        } else if let Some(cached) = cache.entries.get(&key) {
            cached.clone()
        } else if let Some(client) = client {
            match client.best_match(&entry.cleaned_title, entry.release_year) {
                Ok(Some(movie)) => {
                    cache.entries.insert(key, Some(movie.clone()));
                    Some(movie)
                }
                Ok(None) => {
                    // Confirmed absent on TMDB: cache negative result
                    cache.entries.insert(key, None);
                    None
                }
                Err(_) => {
                    // Network failure, timeout, or rate-limit: do NOT poison cache with None
                    None
                }
            }
        } else {
            None
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
            release_year: entry.release_year,
            movie,
            tmdb_url,
            poster_url,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enriches_without_tmdb_client() {
        let entries = vec![WatchEntry {
            watched_at: Some("2026-01-01T00:00:00Z".to_string()),
            raw_title: "Possession.1981.mkv".to_string(),
            cleaned_title: "Possession".to_string(),
            release_year: Some(1981),
        }];
        let mut cache = MovieCache::default();
        let enriched = enrich_entries(entries, None, &mut cache).expect("enrich success");
        assert_eq!(enriched.len(), 1);
        assert_eq!(enriched[0].cleaned_title, "Possession");
        assert_eq!(enriched[0].poster_url, None);
        assert_eq!(enriched[0].tmdb_url, None);
        // Cache must NOT be polluted with negative entries when client is absent
        assert!(cache.entries.is_empty());
    }
}
