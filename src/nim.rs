use serde::{Deserialize, Serialize};
use crate::tmdb::TmdbClient;

const NIM_API_BASE: &str = "https://integrate.api.nvidia.com/v1";
const MIN_MODEL: &str = "qwen/qwen3-next-80b-a3b-instruct";

#[derive(Debug, Serialize, Deserialize)]
pub struct Recommendation {
    pub title: String,
    pub year: u32,
    pub director: String,
    pub genres: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnrichedRecommendation {
    #[serde(flatten)]
    pub rec: Recommendation,
    pub poster_url: Option<String>,
    pub tmdb_url: Option<String>,
}

#[derive(Debug, Serialize)]
struct NimRequest {
    model: String,
    messages: Vec<NimMessage>,
    temperature: f32,
    top_p: f32,
    max_tokens: u32,
    #[serde(rename = "response_format")]
    response_format: Option<ResponseFormat>,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    r_type: String,
}

#[derive(Debug, Serialize)]
struct NimMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct NimResponse {
    choices: Vec<NimChoice>,
}

#[derive(Debug, Deserialize)]
struct NimChoice {
    message: NimMessageContent,
}

#[derive(Debug, Deserialize)]
struct NimMessageContent {
    content: String,
}

pub fn get_recommendations(
    api_key: &str,
    exclusion_list: Vec<String>,
    tmdb_client: &TmdbClient,
) -> Result<Vec<EnrichedRecommendation>, String> {
    let exclusion_str = exclusion_list.join(", ");
    
    let system_prompt = r#"You are an elitist, hyper-literate cinema curator. Your objective is to recommend films that possess profound psychological depth and formalist ambition.

### Core Philosophy:
- Prioritize "Auteur Cinema" (directors with singular visual/thematic language).
- Reject "Mainstream Slop" (predictable narrative arcs, commercial safety).
- Value Complexity over Catharsis.

### Output Constraints:
- Return ONLY a valid JSON array containing EXACTLY 5 objects.
- Schema: [{"title": string, "year": number, "director": string, "genres": string[]}].
- Ensure genres are accurate but concise (e.g., "Psychological Horror", "Neo-Noir")."#;

    let user_prompt = format!(
        "STRICTLY EXCLUDE these titles from your suggestions: [{}].\n\nCurate 5 niche recommendations based on my archive philosophy. Return as a JSON array.",
        exclusion_str
    );

    let request_body = NimRequest {
        model: MIN_MODEL.to_string(),
        messages: vec![
            NimMessage { role: "system".to_string(), content: system_prompt.to_string() },
            NimMessage { role: "user".to_string(), content: user_prompt },
        ],
        temperature: 0.7, // Slightly higher for more variety
        top_p: 0.7,
        max_tokens: 4096,
        response_format: None, // Explicitly letting the prompt handle it to avoid single-object enforcement
    };

    let response = ureq::post(&format!("{}/chat/completions", NIM_API_BASE))
        .set("Authorization", &format!("Bearer {}", api_key))
        .set("Content-Type", "application/json")
        .send_json(&request_body)
        .map_err(|e| format!("NIM request failed: {}", e))?;

    let nim_res: NimResponse = response.into_json().map_err(|e| format!("Failed to parse NIM response: {}", e))?;
    let content = nim_res.choices.first()
        .map(|c| c.message.content.clone())
        .ok_or("Empty NIM response")?;

    // AI might wrap JSON in backticks, clean it
    let clean_json = content.trim_start_matches("```json").trim_end_matches("```").trim();
    
    let recommendations: Vec<Recommendation> = if clean_json.starts_with('[') {
        serde_json::from_str(clean_json)
            .map_err(|e| format!("Failed to parse array: {}. Content: {}", e, clean_json))?
    } else {
        let single: Recommendation = serde_json::from_str(clean_json)
            .map_err(|e| format!("Failed to parse object: {}. Content: {}", e, clean_json))?;
        vec![single]
    };

    let mut enriched = Vec::new();
    for rec in recommendations {
        let movie = tmdb_client.best_match(&rec.title, Some(rec.year as i32)).ok().flatten();
        enriched.push(EnrichedRecommendation {
            poster_url: movie.as_ref().and_then(|m| m.poster_url("w342")),
            tmdb_url: movie.map(|m| m.tmdb_url()),
            rec,
        });
    }

    Ok(enriched)
}
