use std::io::Write;
use std::sync::Arc;

use async_trait::async_trait;
use reqwest::StatusCode;
use tdlib::enums::{InputFile, InputMessageContent};
use tdlib::types::{InputFileLocal, InputMessageDocument};
use tempfile::NamedTempFile;

use super::CommandError::MissingArgument;
use super::{CommandResult, CommandTrait};
use crate::apis::cobalt;
use crate::utils::{donate_markup, Context};

#[derive(Default)]
pub struct CobaltDownload;

#[async_trait]
impl CommandTrait for CobaltDownload {
    fn name(&self) -> &'static str {
        "cobalt_download"
    }

    fn aliases(&self) -> &[&str] {
        &["cobalt", "download", "dl"]
    }

    async fn execute(&self, ctx: Arc<Context>, arguments: Option<String>) -> CommandResult {
        let media_url = arguments.ok_or(MissingArgument("URL to download"))?;

        let mut urls = cobalt::query(ctx.http_client.clone(), media_url.clone()).await??;

        let status_msg = ctx.reply("downloading…").await?;

        urls.truncate(4);
        let mut downloads = Vec::with_capacity(urls.len());

        for url in urls {
            match cobalt::download(ctx.http_client.clone(), url).await {
                Ok(download) if download.media.is_empty() => {
                    Err("≫ cobalt failed to download media. try again later.")?;
                }
                Ok(download) => downloads.push(download),
                Err(err) => {
                    Err(err.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR).to_string())?;
                }
            }
        }

        for download in downloads {
            let mut temp_file = NamedTempFile::new().unwrap();
            temp_file.write_all(&download.media).unwrap();

            ctx.reply_custom(
                InputMessageContent::InputMessageDocument(InputMessageDocument {
                    document: InputFile::Local(InputFileLocal {
                        path: temp_file.path().to_str().unwrap().into(),
                    }),
                    thumbnail: None,
                    disable_content_type_detection: false,
                    caption: None,
                }),
                Some(donate_markup("≫ cobalt", "https://boosty.to/wukko")),
            )
            .await?;
        }

        ctx.delete_message(status_msg.id).await.ok();

        Ok(())
    }
}
