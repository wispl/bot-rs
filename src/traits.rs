use std::borrow::Cow;

use anyhow::Result;

use crate::Context;

pub trait ContextExt<'ctx> {
    async fn say_ephemeral(
        &'ctx self,
        message: impl Into<Cow<'ctx, str>>,
    ) -> Result<poise::ReplyHandle<'ctx>>;
}

impl<'ctx> ContextExt<'ctx> for Context<'ctx> {
    async fn say_ephemeral(
        &'ctx self,
        message: impl Into<Cow<'ctx, str>>,
    ) -> Result<poise::ReplyHandle<'ctx>> {
        let reply = poise::CreateReply::default()
            .content(message)
            .ephemeral(true);
        let handle = self.send(reply).await?;
        Ok(handle)
    }
}
