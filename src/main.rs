use std::env;
use std::path::PathBuf;

const USAGE: &str = "Usage:\n  goo [log-path]\n  goo enrich [log-path] [cache-path]";

fn main() {
    let mut args = env::args().skip(1);
    let first = args.next();

    match first.as_deref() {
        Some("enrich") => run_enrich(args),
        Some(path) => run_clean(Some(path.to_string())),
        None => run_clean(None),
    }
}

fn run_clean(path: Option<String>) {
    let log_path = match resolve_log_path(path) {
        Some(path) => path,
        None => {
            eprintln!("{USAGE}\nLog path not found. Set GOO_LOG_PATH or pass a path.");
            return;
        }
    };

    match goo::read_watch_log(&log_path) {
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
    let log_path = resolve_log_path(args.next());
    let Some(log_path) = log_path else {
        eprintln!("{USAGE}\nLog path not found. Set GOO_LOG_PATH or pass a path.");
        return;
    };
    let cache_path = args.next().map(PathBuf::from);

    let history = match goo::app::load_enriched_history(&log_path, cache_path.as_deref()) {
        Ok(history) => history,
        Err(goo::app::AppError::Tmdb(error)) => {
            eprintln!("TMDB error: {error}. Set TMDB_API_KEY to continue.");
            return;
        }
        Err(error) => {
            eprintln!("Failed to enrich log: {error}");
            return;
        }
    };

    if let Some(warning) = history.cache_warning.as_deref() {
        eprintln!("Cache warning: {warning}");
    }

    match serde_json::to_string(&history.entries) {
        Ok(payload) => println!("{payload}"),
        Err(error) => eprintln!("Failed to serialize output: {error}"),
    }
}

fn resolve_log_path(arg: Option<String>) -> Option<PathBuf> {
    match arg {
        Some(value) => Some(PathBuf::from(value)),
        None => goo::app::default_log_path(),
    }
}
