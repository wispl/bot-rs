use crate::{Context, Data, Error};

use poise::CreateReply;

#[poise::command(slash_command, category = "Others")]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    let before = std::time::SystemTime::now();
    let msg = ctx.say("Pinging!").await?;
    let elapsed = before.elapsed()?.as_millis();
    msg.edit(
        ctx,
        CreateReply::default().content(format!("Pong! {}ms!", elapsed)),
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command, category = "Others")]
pub async fn uptime(ctx: Context<'_>) -> Result<(), Error> {
    let start_time = ctx.data().start_time;
    let now = std::time::SystemTime::now();
    let elapsed = now.duration_since(start_time)?.as_secs();
    ctx.say(format!("Running for {} seconds!", elapsed)).await?;
    Ok(())
}

pub fn commands() -> [poise::Command<Data, Error>; 2] {
    [ping(), uptime()]
}
