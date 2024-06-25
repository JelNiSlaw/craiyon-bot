use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tempfile::TempPath;
use tokio::fs::File;

use crate::commands::CommandError;
use crate::utilities::api_utils::DetectServerError;

#[derive(Deserialize)]
pub struct TranscriptionResponse {
    pub text: String,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'static str,
    messages: &'a [Message<'a>],
    max_tokens: u16,
}

#[derive(Serialize)]
pub struct Message<'a> {
    pub role: &'static str,
    pub content: &'a str,
}

#[derive(Deserialize)]
pub struct ChatCompletion {
    pub choices: Vec<Choice>,
}

#[derive(Deserialize)]
pub struct Choice {
    pub message: MessageResponse,
    pub finish_reason: String,
}

#[derive(Deserialize)]
pub struct MessageResponse {
    pub content: String,
}

#[derive(Deserialize)]
pub struct ErrorResponse {
    pub error: Error,
}

#[derive(Deserialize)]
pub struct Error {
    pub code: String,
    pub message: String,
}

pub async fn chat_completion(
    http_client: reqwest::Client,
    base_url: &str,
    api_key: &str,
    model: &'static str,
    messages: &[Message<'_>],
) -> Result<Result<ChatCompletion, Error>, CommandError> {
    let response = http_client
        .post(format!("{base_url}/chat/completions"))
        .bearer_auth(api_key)
        .json(&ChatRequest { model, messages, max_tokens: 256 })
        .send()
        .await?
        .server_error()?;

    if response.status() == StatusCode::OK {
        let response = response.json::<ChatCompletion>().await?;
        Ok(Ok(response))
    } else {
        let response = response.json::<ErrorResponse>().await?;
        Ok(Err(response.error))
    }
}

pub async fn transcription(
    http_client: reqwest::Client,
    base_url: &str,
    api_key: &str,
    audio: TempPath,
) -> Result<Result<TranscriptionResponse, Error>, CommandError> {
    // Create multipart request
    let part = reqwest::multipart::Part::bytes(tokio::fs::read(audio).await.unwrap())
        .file_name("audio.ogg") // TODO: infer from audio path
        .mime_str("audio/ogg")
        .unwrap();
    let form = reqwest::multipart::Form::new()
        .text("model", "whisper-large-v3")
        .text("response_format", "json")
        .part("file", part);

    let response = http_client
        .post(format!("{base_url}/audio/transcriptions"))
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await?
        .server_error()?;

    if response.status() == StatusCode::OK {
        let response = response.json::<TranscriptionResponse>().await?;
        Ok(Ok(response))
    } else {
        let response = response.json::<ErrorResponse>().await?;
        Ok(Err(response.error))
    }
}
