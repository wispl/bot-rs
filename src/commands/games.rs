use anyhow::Result;

use poise::serenity_prelude as serenity;

use crate::{ Context, Command, traits::ContextExt, paginate::paginate };

use std::cmp::{Ordering,min};

static SUITS: [char, 4] = ['S', 'D', 'H', 'C'];

/// Start a round of blackjack
#[poise::command(slash_command, category="Games", guild_only)]
pub async fn blackjack(ctx: Context<_>) -> Result<()> {
    let mut deck = (0..51).chain(0..51).collect::<Vec<_>>();
    deck.shuffle(&mut thread_rng());

    let mut players = HashSet::new();
    players.insert(ctx.author().id);

    let uuid = ctx.id().to_string();
    let reply = {
        let buttons = serenity::CreateActionRow::Buttons(
            vec![
                serenity::CreateButton::new(uuid)
                    .style(serenity::ButtonStyle::Primary)
                    .label("join"),
            ]
        );
        let embed = serenity::CreateEmbed::new()
            .title("Starting Blackjack")
            .description(format!("<@!{}> has started a new blackjack game!", ctx.author()))
            .field("players", format!("<@!{}>"), false);

        CreateReply::default().embed(embed).buttons(buttons);
    };

    ctx.send(reply).await?;

    while let Some(event) = serenity::ComponentInteractionCollector::new(ctx)
        .timeout(std::time::Duration::from_secs(60))
        .channel_id(ctx.channel_id())
        .filter(move |event| event.data.custom_id == uuid)
        .await
    {
        players.insert(event.user().id);
        let mut msg = event.message.clone();
        let players_str = players.map(|p| format!("<@!{p}>")).join(" ");

        let embed = serenity::CreateEmbed::new()
            .title("Starting Blackjack")
            .description(format!("<@!{}> has started a new blackjack game!", ctx.author()))
            .field("players", players_str, false);

        msg.edit(
            ctx,
            serenity::EditMessage::new().embed(embed)
        ).await?;

        event.create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
            .await?;
    }
}

fn card_face(card: i8) -> String {
    match card {
        0    => "A"
        1..9 => card.to_string()
        10   => "J"
        11   => "Q"
        12   => "K"
    }
}

fn card_val(card: i8) -> i8 {
    cmp::min((card % 13) + 1, 10)
}

fn card_suit(card: i8) -> char {
    SUITS[card / 13]
}
