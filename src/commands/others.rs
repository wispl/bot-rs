use crate::{Context, Data, Error};

#[poise::command(slash_command, category = "Other")]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Pong!").await?;
    Ok(())
}

#[poise::command(slash_command, category = "Other")]
pub async fn echo(
    ctx: Context<'_>,
    #[description = "message to say"] message: String,
) -> Result<(), Error> {
    ctx.say(message).await?;
    Ok(())
}

pub fn commands() -> [poise::Command<Data, Error>; 2] {
    [ping(), echo()]
}
