use std::borrow::Cow;
use std::{env, fmt};

use futures_util::StreamExt;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use url::Url;

use crate::commands::CommandError;

pub enum GenerationError {
    NetworkError(reqwest::Error),
    GoogleError(Error),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerateContentRequest<'a> {
    contents: &'a [Content<'a>],
    safety_settings: &'static [SafetySetting],
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct Content<'a> {
    parts: &'a [Part],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Part {
    Text(String),
    InlineData(Blob),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Blob {
    pub mime_type: Cow<'static, str>,
    pub data: String,
}

#[derive(Serialize)]
pub struct SafetySetting {
    pub category: &'static str,
    pub threshold: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    max_output_tokens: u16,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentResponse {
    #[serde(default)]
    pub candidates: Vec<Candidate>,
    pub prompt_feedback: Option<PromptFeedback>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub content: Option<ContentResponse>,
    pub finish_reason: String,
    pub citation_metadata: Option<CitationMetadata>,
}

#[derive(Deserialize)]
pub struct ContentResponse {
    pub parts: Vec<PartResponse>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PartResponse {
    Text(String),
    InlineData,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationMetadata {
    pub citation_sources: Vec<CitationSource>,
}

#[derive(Deserialize)]
pub struct CitationSource {
    pub uri: Option<String>,
    pub license: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptFeedback {
    pub block_reason: Option<String>,
    pub safety_ratings: Option<Vec<SafetyRating>>,
}

#[derive(Debug, Deserialize)]
pub struct SafetyRating {
    pub category: String,
    #[serde(default)]
    pub blocked: bool,
}

#[derive(Deserialize)]
pub struct ContentFilter {
    pub reason: String,
    pub message: Option<String>,
}

#[derive(Deserialize)]
pub struct ErrorResponse {
    pub error: Error,
}

#[derive(Deserialize)]
pub struct Error {
    pub code: Option<u32>,
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Google error").unwrap();

        if let Some(code) = self.code {
            write!(f, " {code}").unwrap();
        }

        write!(f, ": {}", self.message).unwrap();

        Ok(())
    }
}

pub async fn stream_generate_content(
    http_client: reqwest::Client,
    tx: mpsc::UnboundedSender<Result<GenerateContentResponse, GenerationError>>,
    model: &str,
    parts: &[Part],
    max_output_tokens: u16,
) {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{model}:streamGenerateContent"
    );

    let response = http_client
        .post(
            Url::parse_with_params(&url, [("key", env::var("MAKERSUITE_API_KEY").unwrap())])
                .unwrap(),
        )
        .json(&GenerateContentRequest {
            contents: &[Content { parts }],
            #[rustfmt::skip]
            safety_settings: &[
                SafetySetting { category: "HARM_CATEGORY_HATE_SPEECH", threshold: "BLOCK_NONE" },
                SafetySetting { category: "HARM_CATEGORY_SEXUALLY_EXPLICIT", threshold: "BLOCK_NONE" },
                SafetySetting { category: "HARM_CATEGORY_DANGEROUS_CONTENT", threshold: "BLOCK_NONE" },
                SafetySetting { category: "HARM_CATEGORY_HARASSMENT", threshold: "BLOCK_NONE" },
            ],
            generation_config: GenerationConfig { max_output_tokens },
        })
        .send()
        .await;

    let response = match response {
        Ok(response) => response,
        Err(err) => {
            tx.send(Err(GenerationError::NetworkError(err))).unwrap();
            return;
        }
    };

    if response.status() != StatusCode::OK {
        let error_response = response.json::<ErrorResponse>().await;

        match error_response {
            Ok(error_response) => {
                tx.send(Err(GenerationError::GoogleError(error_response.error))).unwrap();
            }
            Err(err) => tx.send(Err(GenerationError::NetworkError(err))).unwrap(),
        }

        return;
    }

    let mut buffer = Vec::new();
    let mut stream = response.bytes_stream();

    while let Some(part) = stream.next().await {
        let part = match part {
            Ok(part) => part,
            Err(err) => {
                tx.send(Err(GenerationError::NetworkError(err))).unwrap();
                return;
            }
        };

        buffer.extend(&part);

        if let Some(stripped) = buffer.strip_prefix(b"[") {
            buffer = stripped.into();
        }

        if let Some(stripped) = buffer.strip_suffix(b"\n]") {
            buffer = stripped.into();
        }

        while let Some(index) = buffer.windows(4).position(|window| window == b"\n,\r\n") {
            let (first, rest) = buffer.split_at(index);
            tx.send(Ok(serde_json::from_str(&String::from_utf8_lossy(first)).unwrap())).unwrap();
            buffer = rest[4..].into();
        }
    }

    tx.send(Ok(serde_json::from_str(&String::from_utf8_lossy(&buffer)).unwrap())).unwrap();
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerateTextRequest<'a> {
    prompt: TextPrompt<'a>,
    safety_settings: &'a [SafetySetting],
    max_output_tokens: u16,
}

#[derive(Serialize)]
pub struct TextPrompt<'a> {
    text: &'a str,
}

#[derive(Deserialize)]
pub struct GenerateTextResponse {
    pub candidates: Option<Vec<TextCompletionResponse>>,
    pub filters: Option<Vec<ContentFilter>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextCompletionResponse {
    pub output: String,
    pub citation_metadata: Option<CitationMetadata>,
}

pub async fn generate_text(
    http_client: reqwest::Client,
    prompt: &str,
    max_output_tokens: u16,
) -> Result<Result<GenerateTextResponse, Error>, CommandError> {
    let response = http_client
        .post(
            Url::parse_with_params(
                concat!(
                    "https://generativelanguage.googleapis.com",
                    "/v1beta/models/text-bison-001:generateText"
                ),
                [("key", env::var("MAKERSUITE_API_KEY").unwrap())],
            )
            .unwrap(),
        )
        .json(&GenerateTextRequest {
            prompt: TextPrompt { text: prompt },
            max_output_tokens,
            safety_settings: &[
                SafetySetting { category: "HARM_CATEGORY_DEROGATORY", threshold: "BLOCK_NONE" },
                SafetySetting { category: "HARM_CATEGORY_TOXICITY", threshold: "BLOCK_NONE" },
                SafetySetting { category: "HARM_CATEGORY_VIOLENCE", threshold: "BLOCK_NONE" },
                SafetySetting { category: "HARM_CATEGORY_SEXUAL", threshold: "BLOCK_NONE" },
                SafetySetting { category: "HARM_CATEGORY_MEDICAL", threshold: "BLOCK_NONE" },
                SafetySetting { category: "HARM_CATEGORY_DANGEROUS", threshold: "BLOCK_NONE" },
            ],
        })
        .send()
        .await?;

    if response.status() == StatusCode::OK {
        Ok(Ok(response.json().await?))
    } else {
        Ok(Err(response.json::<ErrorResponse>().await?.error))
    }
}
