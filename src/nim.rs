use crate::tmdb::TmdbClient;
use serde::{Deserialize, Serialize};

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

    let system_prompt = format!(
        r#"You are a cinema curator specializing in psychologically devastating, formally audacious, and transgressive cinema. Your mandate is to surface films that explore existential dread, moral decay, and the grotesque with uncompromising vision.

### Guidelines:
- Favor cinema that is existentially harrowing, formally daring, or viscerally disturbing — think Haneke's cold cruelty, Żuławski's hysteria, Noé's sensory assault, Lanthimos' deadpan absurdism, Lynch's uncanny dread, Tsukamoto's body horror.
- Seek out: existential horror, body horror, transgressive drama, psychological thrillers with nihilistic undertones, and works that prioritize unease over entertainment.
- Avoid mainstream blockbusters, prestige Oscar bait, and any film that could be described as "uplifting" or "life-affirming."
- Be diverse across decades, countries, and languages — but always within the realm of dark, psychologically intense, and formally daring cinema. A cheerful film is never acceptable regardless of its origin.
- Surprise the user. Dig into cult oddities, forgotten gems, international rarities, and films that have slipped through the cracks of mainstream arthouse discourse. Avoid anything that appears on standard "best horror" or "best arthouse" lists.
- Prioritize films with a palpable sense of unease, moral ambiguity, and atmospheric dread — whether through stark realism, surrealism, or transgressive imagery.

### User's Watch History (ANALYZE for taste, but NEVER recommend any of these):
[{exclusions}]

Study the titles above with care. Identify recurring patterns in directors, themes, and stylistic signatures — look specifically for: clinical detachment, ritualistic behavior, the grotesque, the uncanny, bodily violation, societal decay, and the dissolution of the self. Use these throughlines to guide your picks toward films the user would love but has not yet discovered. You MUST NOT recommend any title from this list. This is a hard constraint.

### Output format:
- Return ONLY a valid JSON array with EXACTLY 8 objects. No other text.
- Schema: [{{"title": string, "year": number, "director": string, "genres": string[]}}]
- Genres must be specific and tonally precise (e.g. "Existential Dread", "Body Horror", "Transgressive Drama", "Psychological Collapse", "Arthouse Extreme", "Surrealist Horror", "Neo-Noir", "Nihilistic Realism").
- Every film must be real and verifiable."#,
        exclusions = exclusion_str,
    );

    let user_prompt =
        "Curate 8 films from the darkest, most formally daring corners of world cinema that I have not yet seen. Prioritize the obscure, the transgressive, and the genuinely unsettling. Return ONLY the JSON array."
            .to_string();

    let request_body = NimRequest {
        model: MIN_MODEL.to_string(),
        messages: vec![
            NimMessage {
                role: "system".to_string(),
                content: system_prompt,
            },
            NimMessage {
                role: "user".to_string(),
                content: user_prompt,
            },
        ],
        temperature: 1.0,
        top_p: 0.95,
        max_tokens: 4096,
        response_format: None,
    };

    let response = ureq::post(&format!("{}/chat/completions", NIM_API_BASE))
        .set("Authorization", &format!("Bearer {}", api_key))
        .set("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(30))
        .send_json(&request_body)
        .map_err(|e| format!("NIM request failed: {}", e))?;

    let nim_res: NimResponse = response
        .into_json()
        .map_err(|e| format!("Failed to parse NIM response: {}", e))?;
    let content = nim_res
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .ok_or("Empty NIM response")?;

    let clean_json = extract_json_array(&content)
        .ok_or_else(|| format!("No JSON array found in response: {}", content))?;

    let mut recommendations: Vec<Recommendation> =
        serde_json::from_str(&clean_json).map_err(|e| {
            format!(
                "Failed to parse recommendations: {}. Content: {}",
                e, clean_json
            )
        })?;

    // Hard filter: remove any recommendation that matches the exclusion list
    let excluded_lower: Vec<String> = exclusion_list
        .iter()
        .map(|t| t.trim().to_lowercase())
        .collect();
    recommendations.retain(|rec| {
        let title_lower = rec.title.trim().to_lowercase();
        !excluded_lower.iter().any(|ex| title_lower == *ex)
    });

    // Cap at 5
    recommendations.truncate(5);

    let mut enriched = Vec::new();
    for rec in recommendations {
        // Graceful TMDB: don't fail the batch if one lookup errors
        let movie = tmdb_client
            .best_match(&rec.title, Some(rec.year as i32))
            .ok()
            .flatten();
        enriched.push(EnrichedRecommendation {
            poster_url: movie.as_ref().and_then(|m| m.poster_url("w342")),
            tmdb_url: movie.map(|m| m.tmdb_url()),
            rec,
        });
    }

    Ok(enriched)
}

/// Extracts the first JSON array from a string, handling common AI quirks:
/// - `<think>...</think>` reasoning blocks
/// - Triple-backtick markdown fences (```json ... ```)
/// - Extra text before/after the array
fn extract_json_array(raw: &str) -> Option<String> {
    let mut text = raw.to_string();

    // Strip <think>...</think> blocks (some models emit reasoning)
    while let Some(start) = text.find("<think>") {
        if let Some(end) = text.find("</think>") {
            text = format!("{}{}", &text[..start], &text[end + "</think>".len()..]);
        } else {
            // Unclosed <think>, strip from start onwards
            text = text[..start].to_string();
            break;
        }
    }

    // Strip markdown code fences
    let text = text
        .replace("```json", "")
        .replace("```JSON", "")
        .replace("```", "");

    let text = text.trim();

    // Find the outermost [ ... ] pair
    let start = text.find('[')?;
    let mut depth = 0;
    for (i, ch) in text[start..].char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(text[start..=start + i].to_string());
                }
            }
            _ => {}
        }
    }

    None
}
