use std::{env, sync::Arc};

use anyhow::{Error, Result};
use poise::serenity_prelude as serenity;
use tracing::{error, warn};

use yinfo::{ClientConfig, ClientType, Innertube};

use crate::traits::ContextExt;

mod audio;
mod commands;

mod events;
mod paginate;
mod traits;

type Context<'a> = poise::Context<'a, Data, Error>;
type Command = poise::Command<Data, Error>;
type FrameworkContext<'a> = poise::FrameworkContext<'a, Data, Error>;

pub struct Data {
    start_time: std::time::SystemTime,
    reqwest: reqwest::Client,
    songbird: Arc<songbird::Songbird>,
    innertube: Arc<Innertube>,
}

#[tokio::main]
async fn main() {
    let start_time = std::time::SystemTime::now();
    let token = env::var("TOKEN").expect("Missing Token");
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    if let Err(why) = tracing::subscriber::set_global_default(subscriber) {
        eprint!("Could not set up logger {why:?}");
    }

    let reqwest = reqwest::Client::new();
    let http = Arc::new(serenity::HttpBuilder::new(&token).build());
    let config = yinfo::Config {
        configs: vec![
            ClientConfig::new(ClientType::Web),
            ClientConfig::new(ClientType::Android),
            ClientConfig::new(ClientType::WebCreator),
        ],
        retry_limit: 1,
        http: reqwest.clone(),
    };
    let innertube = Arc::new(Innertube::new(config).unwrap());

    let data = Arc::new(Data {
        start_time,
        reqwest,
        songbird: songbird::Songbird::serenity_from_config(
            songbird::Config::default().use_softclip(false),
        ),
        innertube,
    });

    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::GUILD_VOICE_STATES;

    let options = poise::FrameworkOptions {
        commands: commands::commands(),
        command_check: Some(|ctx| Box::pin(async move { Ok(!ctx.author().bot()) })),
        on_error: |error| {
            Box::pin(async move {
                let result = on_error(error).await;
                result.unwrap_or_else(|e| error!("during error handling {:?}", e));
            })
        },
        event_handler: |ctx, event| Box::pin(events::event_handler(ctx, event)),
        ..poise::FrameworkOptions::default()
    };

    let mut client = serenity::ClientBuilder::new_with_http(http, intents)
        .voice_manager::<songbird::Songbird>(data.songbird.clone())
        .framework(poise::Framework::new(options))
        .data(data as _)
        .await
        .unwrap();

    if let Err(why) = client.start().await {
        warn!("during bot startup: {why:?}");
    }
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) -> Result<()> {
    match error {
        poise::FrameworkError::Command { error, ctx, .. } => {
            warn!("during command execution: {:?}", error);
            ctx.say_ephemeral("An error occured during command execution")
                .await?;
        }
        error => poise::builtins::on_error(error).await?,
    }
    Ok(())
}
