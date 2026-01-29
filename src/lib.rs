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
    pub release_year: Option<i32>,
}

struct Cleaners {
    bracketed: Regex,
    audio_channels: Regex,
    fluff: Regex,
    separators: Regex,
    whitespace: Regex,
}

fn cleaners() -> &'static Cleaners {
    static CLEANERS: OnceLock<Cleaners> = OnceLock::new();
    CLEANERS.get_or_init(|| Cleaners {
        bracketed: Regex::new(r"(?i)[\[\(\{].*?[\]\)\}]").expect("valid bracket regex"),
        audio_channels: Regex::new(
            r"(?i)\b(?:aac|ac3|eac3|ddp|dts|truehd|atmos|flac|opus|mp3|mp2)[\s._-]*\d\.\d\b",
        )
        .expect("valid audio channel regex"),
        fluff: Regex::new(
            r"(?i)\b(480p|720p|1080p|2160p|4k|8k|x264|x265|h264|h265|hevc|aac\d*\.?\d*|ac3|dts|truehd|atmos|bluray|brrip|webrip|web-dl|hdr|hdr10|hdr10\+|dvdrip|remux|proper|repack|extended|uncut|10bit|8bit|yify|rarbg|yts|mx|etrg|pahe|tigole|qxr|joy|sparks)\b",
        )
        .expect("valid fluff regex"),
        separators: Regex::new(r"[._-]+").expect("valid separator regex"),
        whitespace: Regex::new(r"\s+").expect("valid whitespace regex"),
    })
}

pub fn read_watch_log(path: &Path) -> std::io::Result<Vec<WatchEntry>> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };
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
    let (cleaned, release_year) = clean_title_and_year(&title_source);

    Some(WatchEntry {
        watched_at: watched_at.map(|value| value.to_string()),
        raw_title: title_source,
        cleaned_title: cleaned,
        release_year,
    })
}

pub fn clean_title(raw: &str) -> String {
    let (cleaned, _) = clean_title_and_year(raw);
    cleaned
}

fn clean_title_and_year(raw: &str) -> (String, Option<i32>) {
    let cleaners = cleaners();
    let mut value = raw.trim().to_string();
    
    value = cleaners.bracketed.replace_all(&value, " ").to_string();
    
    // Process fluff BEFORE separators
    value = cleaners.audio_channels.replace_all(&value, " ").to_string();
    value = cleaners.fluff.replace_all(&value, " ").to_string();
    value = cleaners.separators.replace_all(&value, " ").to_string();
    value = cleaners.whitespace.replace_all(&value, " ").to_string();
    
    let tokens: Vec<&str> = value.split_whitespace().collect();
    if tokens.is_empty() {
        return (String::new(), None);
    }

    let current_year = current_year();
    let mut keep = vec![true; tokens.len()];
    let mut year_positions = Vec::new();
    for (idx, token) in tokens.iter().enumerate() {
        if is_year_token(token, current_year) {
            year_positions.push(idx);
        }
    }

    let mut release_year = None;
    if !year_positions.is_empty() {
        let last_idx = *year_positions.last().unwrap();
        if year_positions.len() == 1 {
            if last_idx != 0 {
                keep[last_idx] = false;
                release_year = tokens[last_idx].parse::<i32>().ok();
            }
        } else {
            keep[last_idx] = false;
            release_year = tokens[last_idx].parse::<i32>().ok();
        }
    }

    let mut cleaned = Vec::new();
    for (idx, token) in tokens.iter().enumerate() {
        if keep[idx] {
            cleaned.push(*token);
        }
    }

    (cleaned.join(" "), release_year)
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

fn is_year_token(token: &str, current_year: i32) -> bool {
    if token.len() != 4 {
        return false;
    }
    let Ok(value) = token.parse::<i32>() else {
        return false;
    };
    value >= 1900 && value <= current_year + 1
}

fn current_year() -> i32 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let days = seconds / 86_400;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    year as i32
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
        assert_eq!(entry.cleaned_title, "Alien");
    }

    #[test]
    fn ignores_blank_lines() {
        assert!(parse_log_line("   ").is_none());
    }
}
