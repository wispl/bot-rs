use poise::{builtins::register_application_commands, serenity_prelude as serenity};

use crate::{traits::ContextExt, Context, Data, Error};

// TODO: move to a more permanent solution using button listener
/// Create limited-time buttons where users can choose roles for themselves
#[poise::command(
    slash_command,
    guild_only,
    category = "Admin",
    required_permissions = "MANAGE_ROLES",
    required_bot_permissions = "MANAGE_ROLES"
)]
pub async fn self_role(
    ctx: Context<'_>,
    #[description = "description for the embed"] description: String,
    role: serenity::Role,
    role1: Option<serenity::Role>,
    role2: Option<serenity::Role>,
    role3: Option<serenity::Role>,
    role4: Option<serenity::Role>,
) -> Result<(), Error> {
    static DANGEROUS_PERMISSIONS: serenity::model::permissions::Permissions =
        serenity::model::permissions::PRESET_GENERAL.complement();

    let roles: Vec<serenity::Role> = vec![Some(role), role1, role2, role3, role4]
        .into_iter()
        .flatten()
        .collect();
    for role in &roles {
        if role.permissions.intersects(DANGEROUS_PERMISSIONS) {
            let intersection = role.permissions.intersection(DANGEROUS_PERMISSIONS);
            ctx.say_ephemeral(format!(
                "'{}' has dangerous permissions:\n* {}",
                role.name,
                intersection.get_permission_names().join("\n* ")
            ))
            .await?;
            return Ok(());
        }
    }

    let id = ctx.id();
    let embed = serenity::CreateEmbed::default()
        .title("Choose your roles")
        .description(description)
        .footer(serenity::CreateEmbedFooter::new("Expires in 1 day!"));

    let components = serenity::CreateActionRow::Buttons(
        roles
            .iter()
            .map(|role| {
                serenity::CreateButton::new(format!("{id}:{}", role.id)).label(role.name.clone())
            })
            .collect(),
    );

    let reply = poise::CreateReply::default()
        .embed(embed)
        .components(vec![components]);

    ctx.send(reply).await?;

    let shard = &ctx.serenity_context().shard;
    while let Some(interaction) = serenity::ComponentInteractionCollector::new(shard.clone())
        .timeout(std::time::Duration::from_secs(3600 * 24))
        .channel_id(ctx.channel_id())
        .filter(move |i| i.data.custom_id.starts_with(&id.to_string()))
        .await
    {
        let index = interaction.data.custom_id.find(':').unwrap() + 1;
        let role_id = interaction.data.custom_id[index..].parse::<u64>().unwrap();

        let response = serenity::CreateInteractionResponseMessage::new()
            .content(format!("Selected <@&{role_id}>"))
            .ephemeral(true);

        interaction
            .create_response(
                ctx.http(),
                serenity::CreateInteractionResponse::Message(response),
            )
            .await?;

        let member = interaction.member.unwrap();
        member
            .add_role(ctx.http(), serenity::RoleId::new(role_id), None)
            .await?;
    }

    Ok(())
}

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

pub fn commands() -> [poise::Command<Data, Error>; 3] {
    [self_role(), sync(), sync_global()]
}
