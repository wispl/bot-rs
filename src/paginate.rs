use poise::serenity_prelude as serenity;

// Pagination sample copied from poise builtin with title and page footers
pub async fn paginate<U: Send + Sync + 'static, E>(
    ctx: poise::Context<'_, U, E>,
    title: &str,
    pages: &[String],
) -> Result<(), serenity::Error> {
    // Define some unique identifiers for the navigation buttons
    let ctx_id = ctx.id();
    let prev_button_id = format!("{}prev", ctx_id);
    let next_button_id = format!("{}next", ctx_id);
    let length = pages.len();

    // Send the embed with the first page as content
    let reply = {
        let components = serenity::CreateActionRow::Buttons(vec![
            serenity::CreateButton::new(&prev_button_id).label("←"),
            serenity::CreateButton::new(&next_button_id).label("→"),
        ]);

        poise::CreateReply::default()
            .embed(serenity::CreateEmbed::default()
                .title(title)
                .description(pages[0].clone())
                .footer(serenity::CreateEmbedFooter::new(
                    format!("page 1 out of {length}")
                ))
            )
            .components(vec![components])
    };

    ctx.send(reply).await?;

    // Loop through incoming interactions with the navigation buttons
    let shard = &ctx.serenity_context().shard;
    let mut current_page = 0;
    while let Some(press) =
        serenity::collector::ComponentInteractionCollector::new(shard.clone())
            // We defined our button IDs to start with `ctx_id`. If they don't, some other command's
            // button was pressed
            .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
            // Timeout when no navigation button has been pressed for 4 hours
            .timeout(std::time::Duration::from_secs(3600 * 4))
            .await
    {
        // Depending on which button was pressed, go to next or previous page
        if press.data.custom_id == next_button_id {
            current_page += 1;
            if current_page >= pages.len() {
                current_page = 0;
            }
        } else if press.data.custom_id == prev_button_id {
            current_page = current_page.checked_sub(1).unwrap_or(pages.len() - 1);
        } else {
            // This is an unrelated button interaction
            continue;
        }

        let footer = serenity::CreateEmbedFooter::new(
            format!("page {} out of {length}", current_page + 1)
        );

        let embed = serenity::CreateEmbed::new()
            .title(title)
            .description(pages[current_page].clone())
            .footer(footer);

        // Update the message with the new page contents
        press.create_response(
            ctx.http(),
            serenity::CreateInteractionResponse::UpdateMessage(
                serenity::CreateInteractionResponseMessage::new().embed(embed)
            ),
        )
        .await?;
    }

    Ok(())
}
