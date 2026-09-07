use serde::{Deserialize, Serialize};

const TMDB_SEARCH_URL: &str = "https://api.themoviedb.org/3/search/movie";
const TMDB_IMAGE_BASE: &str = "https://image.tmdb.org/t/p/";
const TMDB_MOVIE_BASE: &str = "https://www.themoviedb.org/movie/";

pub const DEFAULT_POSTER_SIZE: &str = "w342";

#[derive(Debug, Clone)]
pub struct TmdbClient {
    api_key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TmdbMovie {
    pub id: u32,
    pub title: String,
    pub original_title: Option<String>,
    pub overview: Option<String>,
    pub release_date: Option<String>,
    pub poster_path: Option<String>,
}

#[derive(Debug)]
pub enum TmdbError {
    MissingApiKey,
    Request(ureq::Error),
    HttpStatus { code: u16, body: String },
    Io(std::io::Error),
    Parse(serde_json::Error),
}

#[derive(Debug, Deserialize)]
struct TmdbSearchResponse {
    results: Vec<TmdbMovie>,
}

impl TmdbClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
        }
    }

    pub fn from_env() -> Result<Self, TmdbError> {
        let key = std::env::var("TMDB_API_KEY").map_err(|_| TmdbError::MissingApiKey)?;
        if key.trim().is_empty() {
            return Err(TmdbError::MissingApiKey);
        }
        Ok(Self::new(key))
    }

    pub fn get_movie(&self, id: u32) -> Result<Option<TmdbMovie>, TmdbError> {
        let url = format!("https://api.themoviedb.org/3/movie/{}", id);
        for attempt in 0..2 {
            let request = ureq::get(&url)
                .set("Accept", "application/json")
                .set("User-Agent", "Goo/0.1.0")
                .query("api_key", &self.api_key)
                .timeout(std::time::Duration::from_secs(10));

            match request.call() {
                Ok(response) => {
                    let body = response.into_string()?;
                    let movie: TmdbMovie = serde_json::from_str(&body)?;
                    return Ok(Some(movie));
                }
                Err(ureq::Error::Status(404, _)) => return Ok(None),
                Err(ureq::Error::Status(code, res)) => {
                    let body = res.into_string().unwrap_or_default();
                    return Err(TmdbError::HttpStatus { code, body });
                }
                Err(_) if attempt == 0 => {
                    std::thread::sleep(std::time::Duration::from_millis(250));
                    continue;
                }
                Err(err) => return Err(TmdbError::Request(err)),
            }
        }
        Ok(None)
    }

    pub fn search_movie(
        &self,
        title: &str,
        year: Option<i32>,
    ) -> Result<Vec<TmdbMovie>, TmdbError> {
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        for attempt in 0..2 {
            let mut request = ureq::get(TMDB_SEARCH_URL)
                .set("Accept", "application/json")
                .set("User-Agent", "Goo/0.1.0")
                .query("api_key", &self.api_key)
                .query("query", trimmed)
                .query("include_adult", "false")
                .timeout(std::time::Duration::from_secs(10));

            if let Some(year) = year {
                request = request.query("year", &year.to_string());
            }

            match request.call() {
                Ok(response) => {
                    let body = response.into_string()?;
                    let parsed: TmdbSearchResponse = serde_json::from_str(&body)?;
                    return Ok(parsed.results);
                }
                Err(ureq::Error::Status(code, res)) => {
                    let body = res.into_string().unwrap_or_default();
                    return Err(TmdbError::HttpStatus { code, body });
                }
                Err(_) if attempt == 0 => {
                    std::thread::sleep(std::time::Duration::from_millis(250));
                    continue;
                }
                Err(err) => return Err(TmdbError::Request(err)),
            }
        }
        Ok(Vec::new())
    }

    pub fn best_match(
        &self,
        title: &str,
        year: Option<i32>,
    ) -> Result<Option<TmdbMovie>, TmdbError> {
        // 1. Strict title + year
        let results = self.search_movie(title, year)?;
        if let Some(first) = results.into_iter().next() {
            return Ok(Some(first));
        }

        // 2. Year fallback: try search without strict year constraint
        if year.is_some() {
            let fallback_results = self.search_movie(title, None)?;
            if let Some(first) = fallback_results.into_iter().next() {
                return Ok(Some(first));
            }
        }

        // 3. Punctuation fallback: try replacing hyphens/colons/underscores
        let cleaned_symbols = title.replace(['-', ':', ';', '_'], " ");
        let cleaned_symbols = cleaned_symbols.trim();
        if cleaned_symbols != title.trim() {
            let fallback_results = self.search_movie(cleaned_symbols, year)?;
            if let Some(first) = fallback_results.into_iter().next() {
                return Ok(Some(first));
            }
            if year.is_some() {
                let fallback_no_year = self.search_movie(cleaned_symbols, None)?;
                if let Some(first) = fallback_no_year.into_iter().next() {
                    return Ok(Some(first));
                }
            }
        }

        Ok(None)
    }
}

impl TmdbMovie {
    pub fn poster_url(&self, size: &str) -> Option<String> {
        let path = self.poster_path.as_deref()?.trim_start_matches('/');
        Some(format!("{TMDB_IMAGE_BASE}{size}/{path}"))
    }

    pub fn tmdb_url(&self) -> String {
        format!("{TMDB_MOVIE_BASE}{}", self.id)
    }
}

impl std::fmt::Display for TmdbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TmdbError::MissingApiKey => write!(f, "TMDB API key is missing"),
            TmdbError::Request(err) => write!(f, "TMDB request failed: {err}"),
            TmdbError::HttpStatus { code, body } => {
                write!(f, "TMDB returned status {code}: {body}")
            }
            TmdbError::Io(err) => write!(f, "TMDB response read failed: {err}"),
            TmdbError::Parse(err) => write!(f, "TMDB response parse failed: {err}"),
        }
    }
}

impl std::error::Error for TmdbError {}

impl From<std::io::Error> for TmdbError {
    fn from(err: std::io::Error) -> Self {
        TmdbError::Io(err)
    }
}

impl From<serde_json::Error> for TmdbError {
    fn from(err: serde_json::Error) -> Self {
        TmdbError::Parse(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_poster_url() {
        let movie = TmdbMovie {
            id: 1,
            title: "Test".to_string(),
            original_title: None,
            overview: None,
            release_date: None,
            poster_path: Some("/poster.png".to_string()),
        };
        let url = movie.poster_url(DEFAULT_POSTER_SIZE).expect("poster url");
        assert_eq!(url, "https://image.tmdb.org/t/p/w342/poster.png");
    }

    #[test]
    fn builds_tmdb_url() {
        let movie = TmdbMovie {
            id: 42,
            title: "Test".to_string(),
            original_title: None,
            overview: None,
            release_date: None,
            poster_path: None,
        };
        assert_eq!(movie.tmdb_url(), "https://www.themoviedb.org/movie/42");
    }
}
