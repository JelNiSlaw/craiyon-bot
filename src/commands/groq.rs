use std::env;
use std::fmt::Write;

use async_trait::async_trait;
use tdlib::types::FormattedText;
use tdlib::{enums, functions};

use super::{CommandError, CommandResult, CommandTrait};
use crate::apis::openai::{self, Message};
use crate::utilities::command_context::CommandContext;
use crate::utilities::convert_argument::{ConvertArgument, StringGreedyOrReply};
use crate::utilities::rate_limit::RateLimiter;

pub struct Llama;

#[async_trait]
impl CommandTrait for Llama {
    fn command_names(&self) -> &[&str] {
        &["llama3", "llama", "groq"]
    }

    fn description(&self) -> Option<&'static str> {
        Some("ask Llama 3 70B")
    }

    fn rate_limit(&self) -> RateLimiter<i64> {
        RateLimiter::new(3, 60)
    }

    async fn execute(&self, ctx: &CommandContext, arguments: String) -> CommandResult {
        let StringGreedyOrReply(prompt) = StringGreedyOrReply::convert(ctx, &arguments).await?.0;

        ctx.send_typing().await?;

        let response = openai::chat_completion(
            ctx.bot_state.http_client.clone(),
            "https://api.groq.com/openai/v1",
            &env::var("GROQ_API_KEY").unwrap(),
            "llama3-70b-8192",
            &[Message { role: "user", content: &prompt }],
        )
        .await?
        .map_err(|err| CommandError::Custom(format!("error {}: {}", err.code, err.message)))?;

        let choice = response.choices.into_iter().next().unwrap();
        let mut text = choice.message.content;

        if choice.finish_reason != "stop" {
            write!(text, " [{}]", choice.finish_reason).unwrap();
        }

        let enums::FormattedText::FormattedText(formatted_text) =
            functions::parse_markdown(FormattedText { text, ..Default::default() }, ctx.client_id)
                .await?;

        ctx.reply_formatted_text(formatted_text).await?;

        Ok(())
    }
}

// TODO: Transcribe struct