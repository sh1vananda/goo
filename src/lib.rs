use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub mod tmdb;
pub mod enrich;
pub mod app;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchEntry {
    pub watched_at: Option<String>,
    pub raw_title: String,
    pub cleaned_title: String,
}

struct Cleaners {
    bracketed: Regex,
    fluff: Regex,
    separators: Regex,
    whitespace: Regex,
}

fn cleaners() -> &'static Cleaners {
    static CLEANERS: OnceLock<Cleaners> = OnceLock::new();
    CLEANERS.get_or_init(|| Cleaners {
        bracketed: Regex::new(r"(?i)[\[\(\{].*?[\]\)\}]").expect("valid bracket regex"),
        fluff: Regex::new(
            r"(?i)\b(480p|720p|1080p|2160p|4k|8k|x264|x265|h264|h265|hevc|aac\d*\.?\d*|ac3|dts|truehd|atmos|bluray|brrip|webrip|web-dl|hdr|hdr10|hdr10\+|dvdrip|remux|proper|repack|extended|uncut|10bit|8bit|yify|rarbg|yts|mx|etrg|pahe|tigole|qxr|joy|sparks)\b",
        )
        .expect("valid fluff regex"),
        separators: Regex::new(r"[._-]+").expect("valid separator regex"),
        whitespace: Regex::new(r"\s+").expect("valid whitespace regex"),
    })
}

pub fn read_watch_log(path: &Path) -> std::io::Result<Vec<WatchEntry>> {
    let content = std::fs::read_to_string(path)?;
    let mut entries = Vec::new();
    for line in content.lines() {
        if let Some(entry) = parse_log_line(line) {
            entries.push(entry);
        }
    }
    Ok(entries)
}

pub fn parse_log_line(line: &str) -> Option<WatchEntry> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (watched_at, raw) = split_log_line(trimmed);
    let title_source = extract_title(raw);
    let cleaned = clean_title(&title_source);

    Some(WatchEntry {
        watched_at: watched_at.map(|value| value.to_string()),
        raw_title: title_source,
        cleaned_title: cleaned,
    })
}

pub fn clean_title(raw: &str) -> String {
    let cleaners = cleaners();
    let mut value = raw.trim().to_string();
    
    // Remove bracketed content first
    value = cleaners.bracketed.replace_all(&value, " ").to_string();
    
    // Remove quality/codec fluff BEFORE replacing separators
    // This catches patterns like "AAC5.1" before the dot becomes a space
    value = cleaners.fluff.replace_all(&value, " ").to_string();
    
    // Now replace separators with spaces
    value = cleaners.separators.replace_all(&value, " ").to_string();
    
    // Normalize whitespace
    value = cleaners.whitespace.replace_all(&value, " ").to_string();
    
    // Remove 4-digit years (1900-2099)
    value = Regex::new(r"\b(19|20)\d{2}\b")
        .unwrap()
        .replace_all(&value, "")
        .to_string();
    
    // Remove any remaining single word that's just a number
    value = Regex::new(r"\b\d+\b")
        .unwrap()
        .replace_all(&value, "")
        .to_string();
    
    // Final whitespace cleanup
    value = cleaners.whitespace.replace_all(&value, " ").to_string();
    
    value.trim().to_string()
}

fn split_log_line(line: &str) -> (Option<&str>, &str) {
    if let Some((left, right)) = line.split_once('|') {
        return (Some(left.trim()), right.trim());
    }

    if let Some((left, right)) = line.split_once('\t') {
        return (Some(left.trim()), right.trim());
    }

    (None, line)
}

fn extract_title(raw: &str) -> String {
    let trimmed = raw.trim();
    let without_prefix = trimmed.strip_prefix("file:///").unwrap_or(trimmed);
    let path = Path::new(without_prefix);
    if let Some(stem) = path.file_stem().and_then(|value| value.to_str()) {
        return stem.to_string();
    }
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_common_fluff() {
        let cleaned = clean_title("Dune.2021.1080p.BluRay.x264.DTS.mkv");
        assert_eq!(cleaned, "Dune");
    }

    #[test]
    fn removes_bracketed_segments() {
        let cleaned = clean_title("The.Matrix.(1999).2160p.HDR.Remux.mkv");
        assert_eq!(cleaned, "The Matrix");
    }

    #[test]
    fn cleans_amores_perros() {
        let cleaned = clean_title("Amores.Perros.2000.1080p.BluRay.x264.AAC5.1");
        assert_eq!(cleaned, "Amores Perros");
    }

    #[test]
    fn parses_pipe_delimited_log_lines() {
        let entry = parse_log_line("2025-01-01T10:00:00Z|C:\\Movies\\Blade.Runner.2049.1080p.mkv")
            .expect("entry");
        assert_eq!(entry.watched_at.as_deref(), Some("2025-01-01T10:00:00Z"));
        assert_eq!(entry.raw_title, "Blade.Runner.2049.1080p");
        assert_eq!(entry.cleaned_title, "Blade Runner 2049");
    }

    #[test]
    fn parses_tab_delimited_log_lines() {
        let entry = parse_log_line("2025-01-01T10:00:00Z\tAlien.1979.720p.mkv")
            .expect("entry");
        assert_eq!(entry.watched_at.as_deref(), Some("2025-01-01T10:00:00Z"));
        assert_eq!(entry.cleaned_title, "Alien 1979");
    }

    #[test]
    fn ignores_blank_lines() {
        assert!(parse_log_line("   ").is_none());
    }
}
