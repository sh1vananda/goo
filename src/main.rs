use std::env;
use std::path::Path;

fn main() {
    let Some(path) = env::args().nth(1) else {
        eprintln!("Usage: goo <log-path>");
        return;
    };

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
