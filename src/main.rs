use std::env;
use std::path::{Path, PathBuf};

const USAGE: &str = "Usage:\n  goo <log-path>\n  goo enrich <log-path> [cache-path]";

fn main() {
    let mut args = env::args().skip(1);
    let Some(first) = args.next() else {
        eprintln!("{USAGE}");
        return;
    };

    if first == "enrich" {
        run_enrich(args);
    } else {
        run_clean(first);
    }
}

fn run_clean(path: String) {
    match goo::read_watch_log(Path::new(&path)) {
        Ok(entries) => {
            for entry in entries {
                if let Some(watched_at) = entry.watched_at.as_deref() {
                    println!("{watched_at}\t{}", entry.cleaned_title);
                } else {
                    println!("{}", entry.cleaned_title);
                }
            }
        }
        Err(error) => {
            eprintln!("Failed to read log: {error}");
        }
    }
}

fn run_enrich(mut args: impl Iterator<Item = String>) {
    let Some(log_path) = args.next() else {
        eprintln!("{USAGE}");
        return;
    };

    let log_path = PathBuf::from(log_path);
    let cache_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_cache_path(&log_path));

    let client = match goo::tmdb::TmdbClient::from_env() {
        Ok(client) => client,
        Err(error) => {
            eprintln!("TMDB error: {error}. Set TMDB_API_KEY to continue.");
            return;
        }
    };

    let entries = match goo::read_watch_log(&log_path) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("Failed to read log: {error}");
            return;
        }
    };

    let mut cache = goo::enrich::MovieCache::load(&cache_path);
    let enriched = match goo::enrich::enrich_entries(entries, &client, &mut cache) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("TMDB lookup failed: {error}");
            return;
        }
    };

    if let Err(error) = cache.save(&cache_path) {
        eprintln!("Failed to save cache: {error}");
    }

    match serde_json::to_string(&enriched) {
        Ok(payload) => println!("{payload}"),
        Err(error) => eprintln!("Failed to serialize output: {error}"),
    }
}

fn default_cache_path(log_path: &Path) -> PathBuf {
    log_path
        .parent()
        .map(|parent| parent.join(".goo_cache.json"))
        .unwrap_or_else(|| PathBuf::from(".goo_cache.json"))
}
