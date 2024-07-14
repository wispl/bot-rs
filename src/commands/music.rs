use std::{
    sync::Arc,
    time::Duration,
};
use anyhow::Result;

use async_trait::async_trait;

use songbird::{
    // input::{YoutubeDl, AuxMetadata, Compose},
    input::{AuxMetadata, Compose},
    tracks::Track,
    EventContext,
    Event,
    TrackEvent,
    EventHandler
};

use poise::{
    serenity_prelude as serenity,
    CreateReply
};

use crate::{
    Context,
    Command,
    traits::ContextExt,
    paginate::paginate,
    audio::sources::RustYTDL,
};

struct TrackData {
    metadata: AuxMetadata,
    requester: String,
}

struct TrackEndNotifier {
    channel: serenity::ChannelId,
    http: Arc<serenity::Http>,
}

#[async_trait]
impl EventHandler for TrackEndNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(queue) = ctx {
            let (_state, track) = queue.first().unwrap();
            let data = track.data::<TrackData>();
            let embed = track_embed("Now Playing", &data);
            let reply = serenity::CreateMessage::new().add_embed(embed);
            self.channel.send_message(&self.http, reply).await.ok();
        }
        None
    }
}

pub fn commands() -> [Command; 9] {
    [play(), set_loop(), clear(), skip(), pause(), nowplaying(), resume(), leave(), queue()]
}

/// Play some music
#[poise::command(slash_command, category="Music", guild_only)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "url or term"] song: String
) -> Result<()> {
    ctx.defer().await?;

    let guild_id = ctx.guild_id().unwrap();
    let songbird = ctx.data().songbird.clone();

    let user_vc = ctx.guild().unwrap()
        .voice_states.get(&ctx.author().id)
        .and_then(|voice_state| voice_state.channel_id);

    let Some(user_vc) = user_vc else {
        ctx.say_ephemeral("You are not in a voice channel").await?;
        return Ok(());
    };

    // join the user's channel if we are currently not in one
    let mut joined = false;
    let handler_lock = if let Some(handler) = songbird.get(guild_id) { handler } else {
        joined = true;
        songbird.join(guild_id, user_vc).await?
    };

    let mut handler = handler_lock.lock().await;
    let bot_vc = handler.current_channel().unwrap();
    if  bot_vc != user_vc.into() {
        ctx.say_ephemeral("You are not in my voice channel").await?;
        return Ok(());
    }

    if joined {
        handler.add_global_event(
            Event::Track(TrackEvent::Play),
            TrackEndNotifier {
                channel: ctx.channel_id(),
                http: ctx.serenity_context().http.clone(),
            },
        );
    }

    // might want ytdl later for non youtube links
    // let mut input = if song.starts_with("https") {
    //     YoutubeDl::new(ctx.data().reqwest.clone(), song)
    // } else {
    //     YoutubeDl::new_search(ctx.data().reqwest.clone(), song)
    // };

    let mut input = if song.starts_with("https") {
        RustYTDL::url(
            ctx.data().reqwest.clone(),
            ctx.data().innertube.clone(),
            song
        )
    } else {
        let mut results = ctx.data().innertube.search(&song).await.unwrap();
        RustYTDL::url(
            ctx.data().reqwest.clone(),
            ctx.data().innertube.clone(),
            results.swap_remove(0)
        )
    };

    let data = Arc::new(TrackData {
        metadata: input.aux_metadata().await.unwrap(),
        requester: ctx.author().name.to_string(),
    });

    let len = handler.queue().current_queue().len();
    if len > 0 {
        let embed = track_embed("Enqueued", &data)
            .field("Position", format!("#{} in queue", len + 1), false);
        ctx.send(poise::CreateReply::default().embed(embed)).await?;
    } else {
        ctx.say("Track added".to_owned()).await?;
    }

    let track = Track::new_with_data(input.into(), data);
    handler.enqueue(track).await;
    Ok(())
}

/// Disconnect from the voice channel and clear the queue
#[poise::command(slash_command, category="Music", guild_only)]
pub async fn leave(ctx: Context<'_>) -> Result<()> {
    let guild_id = ctx.guild_id().unwrap();
    let songbird = ctx.data().songbird.clone();

    if songbird.get(guild_id).is_some() {
        songbird.remove(guild_id).await?;
        ctx.say("Leaving the channel").await?;
    } else {
        ctx.say_ephemeral("Not in a voice channel").await?;
    }
    Ok(())
}

/// Gets information of the currect track
#[poise::command(slash_command, category="Music", guild_only)]
pub async fn nowplaying(ctx: Context<'_>) -> Result<()> {
    let guild_id = ctx.guild_id().unwrap();
    let songbird = ctx.data().songbird.clone();

    let Some(handler_lock) = songbird.get(guild_id) else {
        ctx.say_ephemeral("Not in a voice channel").await?;
        return Ok(());
    };

    let handler = handler_lock.lock().await;
    let current = handler.queue().current();

    if let Some(track) = current {
        let data = track.data::<TrackData>();
        let metadata = &data.metadata;

        let Ok(trackstate) = track.get_info().await else {
            ctx.say_ephemeral("Oops something went wrong...").await?;
            return Ok(())
        };

        let duration = metadata.duration.unwrap_or(Duration::new(0, 0));
        let position = trackstate.position;
        let left = duration - position;
        let progress = format!(
            "[{}/{}]\n{}",
            duration_hhmmss(&position),
            duration_hhmmss(&duration),
            progress_bar(&position, &duration, 18)
        );

        let footer = serenity::CreateEmbedFooter::new(
            format!("{} left in track", duration_hhmmss(&left))
        );

        let embed = track_embed("Now Playing", &data)
            .field("Progress", progress, false)
            .footer(footer);
        ctx.send(CreateReply::default().embed(embed)).await?;
    } else {
        ctx.say_ephemeral("Nothing is playing right now").await?;
    }
    Ok(())
}

/// Show all tracks in the queue
#[poise::command(slash_command, category="Music", guild_only)]
pub async fn queue(ctx: Context<'_>) -> Result<()> {
    let guild_id = ctx.guild_id().unwrap();
    let songbird = ctx.data().songbird.clone();

    let Some(handler_lock) = songbird.get(guild_id) else {
        ctx.say_ephemeral("Not in a voice channel").await?;
        return Ok(());
    };

    let handler = handler_lock.lock().await;
    let queue = handler.queue().current_queue();
    drop(handler);

    if queue.is_empty() {
        ctx.say_ephemeral("Nothing is in the queue").await?;
        return Ok(());
    }

    let pagecount = (queue.len() as f32 / 10.0).ceil() as usize;
    let mut pages: Vec<String> = Vec::with_capacity(pagecount);
    for (i, track) in queue.iter().enumerate() {
        let metadata = &track.data::<TrackData>().metadata;
        let title = metadata.title.clone().unwrap_or("~~~~".to_owned());
        if i % 10 == 0 {
            pages.push(format!("{i}. {title}"));
        } else {
            let idx = pages.len() - 1;
            pages[idx] += &format!("\n{i}. {title}");
        }
    }

    paginate(ctx, "Currently Playing", &pages).await?;
    Ok(())
}

/// Clear all tracks in the queue
#[poise::command(slash_command, category="Music", guild_only)]
pub async fn clear(ctx: Context<'_>) -> Result<()> {
    let guild_id = ctx.guild_id().unwrap();
    let songbird = ctx.data().songbird.clone();

    if let Some(handler_lock) = songbird.get(guild_id) {
        let handler = handler_lock.lock().await;
        handler.queue().stop();
        ctx.say("Cleared queue").await?;
    } else {
        ctx.say_ephemeral("Not in a voice channel").await?;
    }
    Ok(())
}

/// Skip the current playing track
#[poise::command(slash_command, category="Music", guild_only)]
pub async fn skip(ctx: Context<'_>) -> Result<()> {
    let guild_id = ctx.guild_id().unwrap();
    let songbird = ctx.data().songbird.clone();

    if let Some(handler_lock) = songbird.get(guild_id) {
        let handler = handler_lock.lock().await;
        handler.queue().skip()?;
        ctx.say("Skipped track").await?;
    } else {
        ctx.say_ephemeral("Not in a voice channel").await?;
    }
    Ok(())
}

/// Pause the queue
#[poise::command(slash_command, category="Music", guild_only)]
pub async fn pause(ctx: Context<'_>) -> Result<()> {
    let guild_id = ctx.guild_id().unwrap();
    let songbird = ctx.data().songbird.clone();

    if let Some(handler_lock) = songbird.get(guild_id) {
        let handler = handler_lock.lock().await;
        handler.queue().pause()?;
        ctx.say("Paused queue").await?;
    } else {
        ctx.say_ephemeral("Not in a voice channel").await?;
    };
    Ok(())
}

/// Resume the queue
#[poise::command(slash_command, category="Music", guild_only)]
pub async fn resume(ctx: Context<'_>) -> Result<()> {
    let guild_id = ctx.guild_id().unwrap();
    let songbird = ctx.data().songbird.clone();

    if let Some(handler_lock) = songbird.get(guild_id) {
        let handler = handler_lock.lock().await;
        handler.queue().resume()?;
        ctx.say("Resumed queue").await?;
    } else {
        ctx.say_ephemeral("Not in a voice channel").await?;
    }
    Ok(())
}

#[derive(Debug, poise::ChoiceParameter)]
pub enum LoopState {
    NoLoop,
    Loop,
}

/// Set the loop state of the queue
#[poise::command(slash_command, category="Music", guild_only)]
pub async fn set_loop(
    ctx: Context<'_>,
    #[description = "new loop state"] state: LoopState
) -> Result<()> {
    let guild_id = ctx.guild_id().unwrap();
    let songbird = ctx.data().songbird.clone();

    if let Some(handler_lock) = songbird.get(guild_id) {
        let handler = handler_lock.lock().await;

        if let Some(current) = handler.queue().current() {
            match state {
                LoopState::Loop => {
                    current.enable_loop()?;
                    ctx.say("Enabled loop").await?;
                },
                LoopState::NoLoop => {
                    current.disable_loop()?;
                    ctx.say("Disabled loop").await?;
                },
            }
        } else {
            ctx.say_ephemeral("Nothing is playing").await?;
        };
    } else {
        ctx.say_ephemeral("Not in a voice channel").await?;
    };

    Ok(())
}

fn track_embed<'a>(
    header: &'a str,
    data: &'a TrackData
) -> serenity::CreateEmbed<'a> {
    let metadata = &data.metadata;
    let requester = &data.requester;

    let title = metadata.title.as_deref().unwrap_or("No Title");
    let channel = metadata.channel.as_deref().unwrap_or("No Channel");
    let link = metadata.source_url.as_deref().unwrap_or("");

    let duration = metadata.duration.unwrap_or(Duration::new(0, 0));
    let footer = serenity::CreateEmbedFooter::new(
        format!("Duration: {}", duration_hhmmss(&duration))
    );

    serenity::CreateEmbed::default()
        .title(header)
        .description(format!("### {title}"))
        .field("Link", format!("[click me]({link})"), true)
        .field("Channel", channel, true)
        .field("Requester", requester, true)
        .footer(footer)
}

fn duration_hhmmss(duration: &Duration) -> String {
    let secs = duration.as_secs();
    let seconds = secs % 60;
    let minutes = (secs / 60) % 60;
    let hours = (secs / 60) / 60;
    format!("{hours:0>2}:{minutes:0>2}:{seconds:0>2}")
}

fn progress_bar(
    current: &Duration,
    end: &Duration,
    bar_length: usize
) -> String {
    let percentage = current.as_secs() as f64 / end.as_secs() as f64;
    let left = (bar_length as f64 * percentage).floor() as usize;
    let right = bar_length - left;
    format!("**[{}{}]**", "#".repeat(left), "-".repeat(right))
}
