use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use tdlib::functions;

use super::{CommandResult, CommandTrait};
use crate::utilities::command_context::CommandContext;

pub struct Ping;

#[async_trait]
impl CommandTrait for Ping {
    fn command_names(&self) -> &[&str] {
        &["ping"]
    }

    fn description(&self) -> Option<&'static str> {
        Some("check if the bot is online")
    }

    async fn execute(&self, ctx: Arc<CommandContext>, _: String) -> CommandResult {
        let start = Instant::now();
        functions::test_network(ctx.client_id).await?;
        let duration = start.elapsed();
        ctx.reply(format!("ping: {}ms", duration.as_millis())).await?;

        Ok(())
    }
}
