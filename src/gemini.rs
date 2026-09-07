use crate::nim::{EnrichedRecommendation, Recommendation};
use crate::tmdb::TmdbClient;
use serde::{Deserialize, Serialize};

const GEMINI_INTERACTIONS_URL: &str = "https://generativelanguage.googleapis.com/v1beta/interactions";
const GEMINI_MODEL: &str = "gemini-3.8-flash";

#[derive(Debug, Serialize)]
struct GeminiInteractionRequest {
    model: String,
    input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Debug, Serialize)]
struct GeminiGenerationConfig {
    temperature: f32,
    top_p: f32,
}

#[derive(Debug, Deserialize, Default)]
struct GeminiInteractionResponse {
    #[serde(default)]
    output_text: Option<String>,
    #[serde(default)]
    steps: Vec<InteractionStep>,
    #[serde(default)]
    candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Deserialize, Default)]
struct InteractionStep {
    #[serde(rename = "type")]
    step_type: Option<String>,
    #[serde(default)]
    content: Vec<InteractionContent>,
}

#[derive(Debug, Deserialize, Default)]
struct InteractionContent {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    content_type: Option<String>,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct GeminiCandidate {
    content: Option<GeminiCandidateContent>,
}

#[derive(Debug, Deserialize, Default)]
struct GeminiCandidateContent {
    #[serde(default)]
    parts: Vec<GeminiCandidatePart>,
}

#[derive(Debug, Deserialize, Default)]
struct GeminiCandidatePart {
    #[serde(default)]
    text: Option<String>,
}

pub fn get_recommendations(
    api_key: &str,
    exclusion_list: Vec<String>,
    tmdb_client: Option<&TmdbClient>,
) -> Result<Vec<EnrichedRecommendation>, String> {
    let exclusion_str = exclusion_list.join(", ");

    let prompt = format!(
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
- Return ONLY a valid JSON array with EXACTLY 5 objects. No other text.
- Schema: [{{"title": string, "year": number, "director": string, "genres": string[]}}]
- Genres must be specific and tonally precise (e.g. "Existential Dread", "Body Horror", "Transgressive Drama", "Psychological Collapse", "Arthouse Extreme", "Surrealist Horror", "Neo-Noir", "Nihilistic Realism").
- Every film must be real and verifiable.

Curate 5 films from the darkest, most formally daring corners of world cinema that I have not yet seen. Prioritize the obscure, the transgressive, and the genuinely unsettling. Return ONLY the JSON array."#,
        exclusions = exclusion_str,
    );

    let request_body = GeminiInteractionRequest {
        model: GEMINI_MODEL.to_string(),
        input: prompt.clone(),
        generation_config: Some(GeminiGenerationConfig {
            temperature: 1.0,
            top_p: 0.95,
        }),
    };

    let response = ureq::post(GEMINI_INTERACTIONS_URL)
        .set("x-goog-api-key", api_key)
        .set("Content-Type", "application/json")
        .set("Accept", "application/json")
        .timeout(std::time::Duration::from_secs(60))
        .send_json(&request_body);

    let content = match response {
        Ok(resp) => {
            let res: GeminiInteractionResponse = resp
                .into_json()
                .map_err(|e| format!("Failed to parse Gemini response: {}", e))?;
            extract_response_text(&res)
                .ok_or_else(|| "Gemini response did not contain text content".to_string())?
        }
        Err(ureq::Error::Status(400, resp)) => {
            let body = resp.into_string().unwrap_or_default();
            if body.contains("API_KEY_INVALID") || body.contains("API key not valid") {
                return Err("Gemini request failed: Invalid API key. Please check your Gemini API key in Settings or via GEMINI_API_KEY.".to_string());
            } else {
                return Err(format!("Gemini request failed: status code 400: {}", body));
            }
        }
        Err(ureq::Error::Status(401, _) | ureq::Error::Status(403, _)) => {
            return Err("Gemini request failed: status code 403 (Permission/Auth failed). Please verify your Gemini API key in Settings or via GEMINI_API_KEY.".to_string());
        }
        Err(ureq::Error::Status(429, _)) => {
            return Err("Gemini request failed: status code 429 (Rate limit or quota exceeded). Please wait a moment or check your Google AI Studio quota.".to_string());
        }
        Err(ureq::Error::Status(404, _)) => {
            // Fallback to generateContent API if interactions endpoint is not enabled
            fallback_generate_content(api_key, &prompt)?
        }
        Err(ureq::Error::Status(code, resp)) => {
            let body = resp.into_string().unwrap_or_default();
            return Err(format!("Gemini request failed: status code {}: {}", code, body));
        }
        Err(ureq::Error::Transport(err)) => {
            return Err(format!("Gemini transport error: {}", err));
        }
    };

    #[derive(Debug, Deserialize)]
    struct RecommendationsWrapper {
        recommendations: Vec<Recommendation>,
    }

    let clean_json = crate::nim::extract_json(&content)
        .ok_or_else(|| format!("No JSON structure found in Gemini response: {}", content))?;

    let mut recommendations: Vec<Recommendation> =
        if let Ok(wrapper) = serde_json::from_str::<RecommendationsWrapper>(&clean_json) {
            wrapper.recommendations
        } else {
            serde_json::from_str(&clean_json).map_err(|e| {
                format!(
                    "Failed to parse recommendations: {}. Content: {}",
                    e, clean_json
                )
            })?
        };

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
        let movie = tmdb_client.and_then(|client| {
            client
                .best_match(&rec.title, Some(rec.year as i32))
                .ok()
                .flatten()
        });
        enriched.push(EnrichedRecommendation {
            poster_url: movie.as_ref().and_then(|m| m.poster_url("w342")),
            tmdb_url: movie.map(|m| m.tmdb_url()),
            rec,
        });
    }

    Ok(enriched)
}

fn fallback_generate_content(api_key: &str, prompt: &str) -> Result<String, String> {
    #[derive(Serialize)]
    struct ContentPart<'a> {
        text: &'a str,
    }
    #[derive(Serialize)]
    struct ContentItem<'a> {
        parts: Vec<ContentPart<'a>>,
    }
    #[derive(Serialize)]
    struct GenerateContentRequest<'a> {
        contents: Vec<ContentItem<'a>>,
    }

    let body = GenerateContentRequest {
        contents: vec![ContentItem {
            parts: vec![ContentPart { text: prompt }],
        }],
    };

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
        api_key
    );

    let resp = ureq::post(&url)
        .set("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(60))
        .send_json(&body)
        .map_err(|e| format!("Gemini generateContent fallback failed: {}", e))?;

    let res: GeminiInteractionResponse = resp
        .into_json()
        .map_err(|e| format!("Failed to parse generateContent response: {}", e))?;

    extract_response_text(&res)
        .ok_or_else(|| "generateContent fallback did not contain text content".to_string())
}

fn extract_response_text(resp: &GeminiInteractionResponse) -> Option<String> {
    if let Some(text) = &resp.output_text {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    let mut step_texts = Vec::new();
    for step in &resp.steps {
        if step.step_type.as_deref() == Some("user_input") {
            continue;
        }
        for item in &step.content {
            if let Some(text) = &item.text {
                if !text.is_empty() {
                    step_texts.push(text.clone());
                }
            }
        }
    }
    if !step_texts.is_empty() {
        return Some(step_texts.join(""));
    }

    let mut cand_texts = Vec::new();
    for cand in &resp.candidates {
        if let Some(content) = &cand.content {
            for part in &content.parts {
                if let Some(text) = &part.text {
                    if !text.is_empty() {
                        cand_texts.push(text.clone());
                    }
                }
            }
        }
    }
    if !cand_texts.is_empty() {
        return Some(cand_texts.join(""));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_output_text() {
        let resp = GeminiInteractionResponse {
            output_text: Some("Direct text".to_string()),
            ..Default::default()
        };
        assert_eq!(extract_response_text(&resp), Some("Direct text".to_string()));
    }

    #[test]
    fn test_extract_steps_text() {
        let resp = GeminiInteractionResponse {
            steps: vec![
                InteractionStep {
                    step_type: Some("model_output".to_string()),
                    content: vec![
                        InteractionContent {
                            content_type: Some("text".to_string()),
                            text: Some(r#"[{"title":"Possession"}]"#.to_string()),
                        }
                    ],
                }
            ],
            ..Default::default()
        };
        assert_eq!(extract_response_text(&resp), Some(r#"[{"title":"Possession"}]"#.to_string()));
    }

    #[test]
    fn test_extract_candidates_text() {
        let resp = GeminiInteractionResponse {
            candidates: vec![
                GeminiCandidate {
                    content: Some(GeminiCandidateContent {
                        parts: vec![
                            GeminiCandidatePart {
                                text: Some(r#"[{"title":"Cure"}]"#.to_string()),
                            }
                        ],
                    }),
                }
            ],
            ..Default::default()
        };
        assert_eq!(extract_response_text(&resp), Some(r#"[{"title":"Cure"}]"#.to_string()));
    }
}
