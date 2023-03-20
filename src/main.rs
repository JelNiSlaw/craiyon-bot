#![warn(clippy::pedantic)]

use bot::Bot;
use utilities::logchamp;

mod apis;
mod bot;
mod commands;
mod utilities;

#[tokio::main]
async fn main() {
    logchamp::init();
    dotenv::dotenv().ok();

    let mut bot = Bot::new();

    // bot.add_command(commands::start::Start);
    bot.add_command(commands::generate::Generate);
    bot.add_command(commands::stablehorde::StableHorde::stable_diffusion_2());
    bot.add_command(commands::stablehorde::StableHorde::stable_diffusion());
    bot.add_command(commands::stablehorde::StableHorde::waifu_diffusion());
    bot.add_command(commands::stablehorde::StableHorde::furry_diffusion());
    // bot.add_command(commands::different_dimension_me::DifferentDimensionMe);
    // bot.add_command(commands::translate::Translate);
    // bot.add_command(commands::badtranslate::BadTranslate);
    bot.add_command(commands::trollslate::Trollslate);
    bot.add_command(commands::urbandictionary::UrbanDictionary);
    bot.add_command(commands::screenshot::Screenshot);
    bot.add_command(commands::cobalt_download::CobaltDownload);
    bot.add_command(commands::charinfo::CharInfo);
    bot.add_command(commands::radio_poligon::RadioPoligon);
    bot.add_command(commands::autocomplete::Autocomplete);
    bot.add_command(commands::kiwifarms::KiwiFarms);
    bot.add_command(commands::startit_joke::StartitJoke);
    bot.add_command(commands::kebab::Kebab);
    bot.add_command(commands::ping::Ping);
    bot.add_command(commands::delete::Delete);
    bot.add_command(commands::sex::Sex);

    bot.run().await;
}
