use std::fmt::Write;
use std::fs;
use std::time::Duration;

use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use tdlib::enums::File;
use tdlib::types::{FormattedText, Message};
use tdlib::{enums, functions};
use tokio::sync::mpsc;
use tokio::time::Instant;

use super::{CommandError, CommandResult, CommandTrait};
use crate::apis::makersuite::{
    self, Blob, Candidate, CitationSource, GenerateContentResponse, Part, PartResponse,
};
use crate::utilities::command_context::CommandContext;
use crate::utilities::convert_argument::{ConvertArgument, StringGreedyOrReply};
use crate::utilities::file_download::MEBIBYTE;
use crate::utilities::rate_limit::RateLimiter;
use crate::utilities::telegram_utils;

pub struct GoogleGemini;

#[async_trait]
impl CommandTrait for GoogleGemini {
    fn command_names(&self) -> &[&str] {
        &["gemini", "g"]
    }

    fn description(&self) -> Option<&'static str> {
        Some("ask Gemini Pro or Gemini Pro Vision")
    }

    fn rate_limit(&self) -> RateLimiter<i64> {
        RateLimiter::new(3, 45)
    }

    async fn execute(&self, ctx: &CommandContext, arguments: String) -> CommandResult {
        let prompt = Option::<StringGreedyOrReply>::convert(ctx, &arguments).await?.0;

        ctx.send_typing().await?;

        let mut model = "gemini-1.0-pro-latest";
        let mut parts = Vec::new();

        if let Some(prompt) = prompt {
            parts.push(Part::Text(prompt.0));
        }

        if let Some(message_image) =
            telegram_utils::get_message_or_reply_image(&ctx.message, ctx.client_id).await
        {
            if message_image.file.expected_size > 4 * MEBIBYTE {
                return Err(CommandError::Custom("the image cannot be larger than 4 MiB.".into()));
            }

            model = "gemini-1.0-pro-vision-latest";

            let File::File(file) =
                functions::download_file(message_image.file.id, 1, 0, 0, true, ctx.client_id)
                    .await?;

            let file = fs::read(file.local.path).unwrap();

            parts.push(Part::InlineData(Blob {
                mime_type: message_image.mime_type,
                data: STANDARD.encode(file),
            }));
        }

        if parts.is_empty() {
            return Err(CommandError::Custom("no prompt or image provided.".into()));
        }

        let http_client = ctx.bot_state.http_client.clone();
        let (tx, mut rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            makersuite::stream_generate_content(http_client, tx, model, &parts, 512).await;
        });

        let mut next_update = Instant::now() + Duration::from_secs(5);
        let mut changed_after_last_update = false;
        let mut progress = Option::<GenerationProgress>::None;
        let mut message = Option::<Message>::None;

        loop {
            let (update_message, finished) = if let Ok(response) =
                tokio::time::timeout_at(next_update, rx.recv()).await
            {
                match response {
                    Some(response) => {
                        let response = response?;

                        match progress.as_mut() {
                            Some(progress) => {
                                progress.update(response)?;
                                changed_after_last_update = true;
                            }
                            None => {
                                if let Some(candidate) = response.candidates.into_iter().next() {
                                    progress = Some(GenerationProgress::new(candidate));
                                    changed_after_last_update = true;
                                }
                            }
                        }

                        (false, false)
                    }
                    None => (true, true),
                }
            } else {
                next_update = Instant::now() + Duration::from_secs(5);
                (true, false)
            };

            if update_message && changed_after_last_update {
                let text = match progress.as_ref() {
                    Some(progress) => progress.format(finished),
                    None => {
                        continue;
                    }
                };

                let enums::FormattedText::FormattedText(formatted_text) =
                    functions::parse_markdown(
                        FormattedText { text, ..Default::default() },
                        ctx.client_id,
                    )
                    .await?;

                if let Some(message) = message.as_ref() {
                    ctx.edit_message_formatted_text(message.id, formatted_text).await?;
                } else {
                    let unsent_message = ctx.reply_formatted_text(formatted_text).await?;
                    message = Some(
                        ctx.bot_state.message_queue.wait_for_message(unsent_message.id).await?,
                    );
                }

                next_update = Instant::now() + Duration::from_secs(5);
                changed_after_last_update = false;
            }

            if finished {
                break;
            }
        }

        Ok(())
    }
}

pub struct GoogleGeminiFlash;

#[async_trait]
impl CommandTrait for GoogleGeminiFlash {
    fn command_names(&self) -> &[&str] {
        &["geminiflash", "flash", "f"]
    }

    fn description(&self) -> Option<&'static str> {
        Some("ask Gemini Flash")
    }

    fn rate_limit(&self) -> RateLimiter<i64> {
        RateLimiter::new(3, 45)
    }

    async fn execute(&self, ctx: &CommandContext, arguments: String) -> CommandResult {
        let prompt = Option::<StringGreedyOrReply>::convert(ctx, &arguments).await?.0;

        ctx.send_typing().await?;

        let mut model = "gemini-1.5-flash-latest";
        let mut parts = Vec::new();

        if let Some(prompt) = prompt {
            parts.push(Part::Text(prompt.0));
        }

        if let Some(media) =
            telegram_utils::get_message_or_reply_media(&ctx.message, ctx.client_id).await
        {
            // if file.expected_size > 10 * MEBIBYTE {
            //     return Err(CommandError::Custom("the media cannot be larger than 10 MiB.".into()));
            // }

            model = "gemini-1.0-pro-vision-latest";

            let File::File(file) =
                functions::download_file(media.file.id, 1, 0, 0, true, ctx.client_id)
                    .await?;

            let file = fs::read(file.local.path).unwrap();

            parts.push(Part::InlineData(Blob {
                mime_type: media.mime_type,
                data: STANDARD.encode(file),
            }));
        }

        if parts.is_empty() {
            return Err(CommandError::Custom("no prompt or media provided.".into()));
        }

        let http_client = ctx.bot_state.http_client.clone();
        let (tx, mut rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            makersuite::stream_generate_content(http_client, tx, model, &parts, 512).await;
        });

        let mut next_update = Instant::now() + Duration::from_secs(5);
        let mut changed_after_last_update = false;
        let mut progress = Option::<GenerationProgress>::None;
        let mut message = Option::<Message>::None;

        loop {
            let (update_message, finished) = if let Ok(response) =
                tokio::time::timeout_at(next_update, rx.recv()).await
            {
                match response {
                    Some(response) => {
                        let response = response?;

                        match progress.as_mut() {
                            Some(progress) => {
                                progress.update(response)?;
                                changed_after_last_update = true;
                            }
                            None => {
                                if let Some(candidate) = response.candidates.into_iter().next() {
                                    progress = Some(GenerationProgress::new(candidate));
                                    changed_after_last_update = true;
                                }
                            }
                        }

                        (false, false)
                    }
                    None => (true, true),
                }
            } else {
                next_update = Instant::now() + Duration::from_secs(5);
                (true, false)
            };

            if update_message && changed_after_last_update {
                let text = match progress.as_ref() {
                    Some(progress) => progress.format(finished),
                    None => {
                        continue;
                    }
                };

                let enums::FormattedText::FormattedText(formatted_text) =
                    functions::parse_markdown(
                        FormattedText { text, ..Default::default() },
                        ctx.client_id,
                    )
                    .await?;

                if let Some(message) = message.as_ref() {
                    ctx.edit_message_formatted_text(message.id, formatted_text).await?;
                } else {
                    let unsent_message = ctx.reply_formatted_text(formatted_text).await?;
                    message = Some(
                        ctx.bot_state.message_queue.wait_for_message(unsent_message.id).await?,
                    );
                }

                next_update = Instant::now() + Duration::from_secs(5);
                changed_after_last_update = false;
            }

            if finished {
                break;
            }
        }

        Ok(())
    }
}

pub struct GooglePalm;

#[async_trait]
impl CommandTrait for GooglePalm {
    fn command_names(&self) -> &[&str] {
        &["palm", "palm2", "p"]
    }

    fn description(&self) -> Option<&'static str> {
        Some("ask Google PaLM 2 (Legacy)")
    }

    fn rate_limit(&self) -> RateLimiter<i64> {
        RateLimiter::new(3, 45)
    }

    async fn execute(&self, ctx: &CommandContext, arguments: String) -> CommandResult {
        let StringGreedyOrReply(prompt) = ConvertArgument::convert(ctx, &arguments).await?.0;

        ctx.send_typing().await?;

        let response =
            makersuite::generate_text(ctx.bot_state.http_client.clone(), &prompt, 512).await?;

        let response = match response {
            Ok(response) => response,
            Err(response) => {
                return Err(CommandError::Custom(response.to_string()));
            }
        };

        if let Some(filters) = response.filters {
            let reasons = filters
                .into_iter()
                .map(|filter| {
                    if let Some(message) = filter.message {
                        format!("{}: {message}", filter.reason)
                    } else {
                        filter.reason
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");

            ctx.reply(format!("request blocked by Google: {reasons}.",)).await?;
            return Ok(());
        }

        let Some(candidate) =
            response.candidates.and_then(|candidates| candidates.into_iter().next())
        else {
            return Err(CommandError::Custom("no response generated.".into()));
        };

        if candidate.output.is_empty() {
            return Err(CommandError::Custom("no text generated.".into()));
        }

        let mut text = candidate.output;

        if let Some(citation_metadata) = candidate.citation_metadata {
            text.push_str("\n\n");
            text.push_str(&format_citations(&citation_metadata.citation_sources));
        }

        let enums::FormattedText::FormattedText(formatted_text) =
            functions::parse_markdown(FormattedText { text, ..Default::default() }, ctx.client_id)
                .await?;

        ctx.reply_formatted_text(formatted_text).await?;

        Ok(())
    }
}

struct GenerationProgress {
    parts: Vec<PartResponse>,
    finish_reason: String,
    citation_sources: Vec<CitationSource>,
}

impl GenerationProgress {
    fn new(candidate: Candidate) -> Self {
        Self {
            parts: candidate.content.map(|content| content.parts).unwrap_or_default(),
            finish_reason: candidate.finish_reason,
            citation_sources: candidate
                .citation_metadata
                .map(|citation_metadata| citation_metadata.citation_sources)
                .unwrap_or_default(),
        }
    }

    fn update(&mut self, response: GenerateContentResponse) -> Result<(), CommandError> {
        if let Some(prompt_feedback) = response.prompt_feedback {
            if let Some(block_reason) = &prompt_feedback.block_reason {
                if block_reason == "SAFETY" {
                    if let Some(safety_ratings) = &prompt_feedback.safety_ratings {
                        let reasons = safety_ratings
                            .iter()
                            .filter(|safety_rating| safety_rating.blocked)
                            .map(|safety_rating| safety_rating.category.as_str())
                            .collect::<Vec<_>>()
                            .join(", ");

                        return Err(CommandError::Custom(format!(
                            "request blocked by Google: {reasons}."
                        )));
                    }
                }

                return Err(CommandError::Custom("request blocked by Google.".into()));
            }
        }

        let Some(candidate) = response.candidates.into_iter().next() else {
            return Err(CommandError::Custom("no response generated.".into()));
        };

        if let Some(content) = candidate.content {
            self.parts.extend(content.parts);

            self.citation_sources = candidate
                .citation_metadata
                .map(|citation_metadata| citation_metadata.citation_sources)
                .unwrap_or_default();
        }

        self.finish_reason = candidate.finish_reason;

        Ok(())
    }

    fn format(&self, finished: bool) -> String {
        let mut text = self
            .parts
            .iter()
            .map(|part| match part {
                PartResponse::Text(text) => text.as_str(),
                PartResponse::InlineData => "[unsupported response part]",
            })
            .collect::<Vec<_>>()
            .concat();

        if !finished {
            text.push('…');
        }

        if self.finish_reason != "STOP" {
            write!(text, " [{}]", self.finish_reason).unwrap();
        }

        if !self.citation_sources.is_empty() {
            text.push_str("\n\n");
            text.push_str(&format_citations(&self.citation_sources));
        }

        text
    }
}

fn format_citations(citation_sources: &[CitationSource]) -> String {
    let mut text = String::new();

    for (i, source) in citation_sources.iter().enumerate() {
        if let Some(uri) = source.uri.as_ref() {
            write!(text, "\n[{}] ", i + 1).unwrap();

            if let Some(license) = source.license.as_ref() {
                if !license.is_empty() {
                    write!(text, "[{license}] ").unwrap();
                }
            }

            text.push_str(uri);
        }
    }

    text
}
