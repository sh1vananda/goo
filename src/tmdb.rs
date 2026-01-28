use serde::Deserialize;

const TMDB_SEARCH_URL: &str = "https://api.themoviedb.org/3/search/movie";
const TMDB_IMAGE_BASE: &str = "https://image.tmdb.org/t/p/";
const TMDB_MOVIE_BASE: &str = "https://www.themoviedb.org/movie/";

pub const DEFAULT_POSTER_SIZE: &str = "w342";

#[derive(Debug, Clone)]
pub struct TmdbClient {
    api_key: String,
}

#[derive(Debug, Clone, Deserialize)]
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

    pub fn search_movie(&self, title: &str) -> Result<Vec<TmdbMovie>, TmdbError> {
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let response = ureq::get(TMDB_SEARCH_URL)
            .set("Accept", "application/json")
            .query("api_key", &self.api_key)
            .query("query", trimmed)
            .query("include_adult", "false")
            .call();

        let response = match response {
            Ok(value) => value,
            Err(ureq::Error::Status(code, res)) => {
                let body = res.into_string().unwrap_or_default();
                return Err(TmdbError::HttpStatus { code, body });
            }
            Err(err) => return Err(TmdbError::Request(err)),
        };

        let body = response.into_string()?;
        let parsed: TmdbSearchResponse = serde_json::from_str(&body)?;
        Ok(parsed.results)
    }

    pub fn best_match(&self, title: &str) -> Result<Option<TmdbMovie>, TmdbError> {
        Ok(self.search_movie(title)?.into_iter().next())
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
