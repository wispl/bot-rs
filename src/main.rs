use std::{
    sync::Arc,
    env,
};

use anyhow::{Result, Error};
use tracing::{warn, error};
use poise::serenity_prelude as serenity;

use yinfo::{
    clients::{ClientConfig, ClientType},
    innertube::Innertube,
};

use crate::traits::ContextExt;

mod audio;
mod commands;

mod paginate;
mod events;
mod traits;

type Context<'a> = poise::Context<'a, Data, Error>;
type Command = poise::Command<Data, Error>;
type FrameworkContext<'a> = poise::FrameworkContext<'a, Data, Error>;

pub struct Data {
    reqwest: reqwest::Client,
    songbird: Arc<songbird::Songbird>,
    innertube: Arc<Innertube>
}

#[tokio::main]
async fn main() {
    let token = env::var("TOKEN").expect("Missing Token");
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    if let Err(why) = tracing::subscriber::set_global_default(subscriber) {
        eprint!("Could not set up logger {why:?}");
    }

    let reqwest = reqwest::Client::new();
    let config = ClientConfig::new(ClientType::Web);
    let innertube = Arc::new(Innertube::new(reqwest.clone(), config).unwrap());

    let data = Arc::new(Data {
        reqwest,
        songbird: songbird::Songbird::serenity_from_config(
            songbird::Config::default().use_softclip(false)
        ),
        innertube,
    });

    let intents = serenity::GatewayIntents::non_privileged()
                  | serenity::GatewayIntents::GUILD_VOICE_STATES;

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

    let mut client = serenity::Client::builder(&token, intents)
        .voice_manager::<songbird::Songbird>(data.songbird.clone())
        .framework(poise::Framework::new(options))
        .data(data as _)
        .await
        .unwrap();

    if let Err(why) = client.start().await {
        warn!("during bot startup: {why:?}")
    }
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) -> Result<()>{
    match error {
        poise::FrameworkError::Command { error, ctx, .. } => {
            warn!("during command execution: {:?}", error);
            ctx.say_ephemeral("An error occured during command execution").await?;
        }
        error => poise::builtins::on_error(error).await?,
    }
    Ok(())
}
