use crate::{Context, Error, Data};

use poise::builtins::register_application_commands;

#[poise::command(slash_command, guild_only, owners_only, category = "Admin")]
pub async fn sync(ctx: Context<'_>) -> Result<(), Error> {
    register_application_commands(ctx, false).await?;
    Ok(())
}

#[poise::command(slash_command, owners_only, category = "Admin")]
pub async fn sync_global(ctx: Context<'_>) -> Result<(), Error> {
    register_application_commands(ctx, true).await?;
    Ok(())
}

pub fn commands() -> [poise::Command<Data, Error>; 2] {
    [sync(), sync_global()]
}
