use poise::{serenity_prelude as serenity, CreateReply};
use serde::Deserialize;

use crate::{traits::ContextExt, Command, Context, Error};

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

#[derive(Debug, Deserialize)]
struct DictionaryResult {
    #[allow(dead_code)]
    word: String,
    phonetics: String,
    origin: String,

    meanings: Vec<Meaning>,
}

#[derive(Debug, Deserialize)]
struct Meaning {
    #[serde(rename = "camelCase")]
    part_of_speech: String,
    definitions: Vec<Definition>,
}

#[derive(Debug, Deserialize)]
struct Definition {
    definition: String,
    example: String,
    #[allow(dead_code)]
    synonyms: Vec<String>,
    #[allow(dead_code)]
    antonyms: Vec<String>,
}

#[poise::command(slash_command, category = "Others")]
pub async fn define(
    ctx: Context<'_>,
    #[description = "word to define"] word: String,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let reqwest = ctx.data().reqwest.clone();
    let url = format!("https://api.dictionaryapi.dev/api/v2/entries/en/{}", word);
    let json = reqwest
        .get(url)
        .send()
        .await?
        .json::<DictionaryResult>()
        .await;
    if let Ok(res) = json {
        let embed = serenity::CreateEmbed::default()
            .title(word)
            .field("origin", &res.origin, false)
            .field("phonetics", &res.phonetics, false)
            .fields(res.meanings.iter().take(3).map(|m| {
                (
                    &m.part_of_speech,
                    m.definitions
                        .iter()
                        .take(3)
                        .map(|d| format!("```•\n{}\n\t{}\n```", d.definition, d.example))
                        .collect::<Vec<String>>()
                        .join(""),
                    false,
                )
            }));
        ctx.send(poise::CreateReply::default().embed(embed)).await?;
    } else {
        ctx.say_ephemeral(format!("No definition found for {}", word))
            .await?;
    }

    Ok(())
}

pub fn commands() -> [Command; 3] {
    [ping(), uptime(), define()]
}
