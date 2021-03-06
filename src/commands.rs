use std::error::Error;
use std::io::Cursor;

use image::{ImageFormat, ImageOutputFormat};
use reqwest::StatusCode;
use teloxide::prelude::*;
use teloxide::types::{InputFile, MessageEntity, ParseMode, User};
use teloxide::utils::markdown;

use crate::utils::CollageOptions;
use crate::{craiyon, openai, urbandictionary, utils};

pub async fn generate(
    bot: Bot,
    message: Message,
    prompt: String,
    http_client: reqwest::Client,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if prompt.chars().count() > 1024 {
        bot.send_message(message.chat.id, "This prompt is too long.")
            .reply_to_message_id(message.id)
            .send()
            .await
            .ok();
        return Ok(());
    };

    let info_msg = match bot
        .send_message(message.chat.id, format!("Generating {prompt}…"))
        .reply_to_message_id(message.id)
        .send()
        .await
    {
        Ok(message) => message,
        Err(_) => return Ok(()), // this usually means that the original message was deleted
    };

    log::info!(
        "Generating {prompt:?} for {user_name} ({user_id}) in {chat_name} ({chat_id})",
        user_name = message
            .from()
            .map_or_else(|| "-".to_string(), User::full_name),
        user_id = message.from().map(|u| u.id.0).unwrap_or_default(),
        chat_name = message.chat.title().unwrap_or("-"),
        chat_id = message.chat.id
    );

    match craiyon::generate(http_client, prompt.clone()).await {
        Ok(result) => {
            let images = result
                .images
                .iter()
                .flat_map(|image| image::load_from_memory_with_format(image, ImageFormat::Jpeg))
                .collect::<Vec<_>>();
            let image_size = {
                let image = images.first().unwrap();
                (image.width(), image.height())
            };
            let image = utils::image_collage(
                images,
                CollageOptions {
                    image_count: (3, 3),
                    image_size,
                    gap: 8,
                },
            );

            let mut buffer = Cursor::new(Vec::new());
            image.write_to(&mut buffer, ImageOutputFormat::Png).unwrap();

            bot.send_photo(message.chat.id, InputFile::memory(buffer.into_inner()))
                .caption(format!(
                    "Generated from prompt: *{}* in {}\\.",
                    markdown::escape(&prompt),
                    utils::format_duration(result.duration)
                ))
                .parse_mode(ParseMode::MarkdownV2)
                .reply_to_message_id(message.id)
                .allow_sending_without_reply(true)
                .send()
                .await?;
        }
        Err(err) => {
            bot.send_message(
                message.chat.id,
                format!(
                    "zjebalo sie: {}",
                    err.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
                ),
            )
            .reply_to_message_id(message.id)
            .allow_sending_without_reply(true)
            .send()
            .await?;
        }
    };

    bot.delete_message(message.chat.id, info_msg.id)
        .send()
        .await
        .ok();

    Ok(())
}

pub async fn urbandictionary(
    bot: Bot,
    message: Message,
    term: String,
    http_client: reqwest::Client,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let response = if let Ok(Some(definition)) = urbandictionary::define(http_client, term).await {
        definition.to_string()
    } else {
        "There are no definitions for this word\\. \
            Be the first to [define it](https://www.urbandictionary.com/add.php)\\!"
            .to_string()
    };

    bot.send_message(message.chat.id, response)
        .parse_mode(ParseMode::MarkdownV2)
        .disable_web_page_preview(true)
        .reply_to_message_id(message.id)
        .send()
        .await?;

    Ok(())
}

pub async fn gpt3_code(
    bot: Bot,
    message: Message,
    prompt: String,
    http_client: reqwest::Client,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let (text, entities) = match openai::complete_code(
        http_client,
        openai::Config {
            prompt,
            max_tokens: 256,
            temperature: Some(0.),
            stop: None,
        },
    )
    .await
    {
        Ok(text) => {
            let len = text.chars().count();
            (text, Vec::from([MessageEntity::pre(None, 0, len)]))
        }
        Err(_) => ("zjebalo sie".to_string(), Vec::new()),
    };
    bot.send_message(message.chat.id, text)
        .entities(entities)
        .reply_to_message_id(message.id)
        .send()
        .await?;

    Ok(())
}

pub async fn charinfo(
    bot: Bot,
    message: Message,
    chars: String,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut lines = chars
        .chars()
        .into_iter()
        .map(|c| {
            if c.is_ascii_whitespace() {
                String::new()
            } else {
                format!(
                    "`{}` `U\\+{:04X}`",
                    markdown::escape(&c.to_string()),
                    c as u32
                )
            }
        })
        .collect::<Vec<_>>();

    if lines.len() > 10 {
        lines.truncate(10);
        lines.push(String::from('…'));
    }

    bot.send_message(message.chat.id, lines.join("\n"))
        .parse_mode(ParseMode::MarkdownV2)
        .send()
        .await?;
    Ok(())
}
