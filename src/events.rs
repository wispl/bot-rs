use anyhow::Result;
use tracing::info;

use poise::serenity_prelude as serenity;
use serenity::FullEvent as Event;

use crate::FrameworkContext;

pub async fn event_handler(
    ctx: FrameworkContext<'_>,
    event: &Event
) -> Result<()> {
    match event {
        Event::Ready { data_about_bot } => ready(ctx, data_about_bot).await,
        _ => Ok(()),
    }
}

async fn ready(
    _ctx: FrameworkContext<'_>,
    data: &serenity::Ready
) -> Result<()> {
    info!("Logged in as {}", data.user.name);
    Ok(())
}
