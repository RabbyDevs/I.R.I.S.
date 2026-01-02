use poise::ChoiceParameter;
use serenity::{
    all::{
        ActivityData, Attachment, Builder, Channel, ChannelId, CreateAttachment, CreateChannel,
        CreateMessage, GuildId, OnlineStatus,
    },
    futures::future::join_all,
};

use crate::{Context, Error, GUILD_ID, PUBLIC_CATEGORY_ID};

#[allow(clippy::too_many_arguments)]
/// Send an arbitrary message to a channel
///
/// Optional attachments included
#[poise::command(prefix_command, slash_command)]
pub async fn send_to_channel(
    ctx: Context<'_>,
    #[description = "Which channel to send the message to"] channel: Channel,
    #[description = "Message contents"] content: String,
    #[description = "(optional) attachment 1"] attachment1: Option<Attachment>,
    #[description = "(optional) attachment 2"] attachment2: Option<Attachment>,
    #[description = "(optional) attachment 3"] attachment3: Option<Attachment>,
    #[description = "(optional) attachment 4"] attachment4: Option<Attachment>,
    #[description = "(optional) attachment 5"] attachment5: Option<Attachment>,
) -> Result<(), Error> {
    let attachments = [
        attachment1,
        attachment2,
        attachment3,
        attachment4,
        attachment5,
    ];

    let files = join_all(attachments.iter().map(|op_attachment| async move {
        if let Some(attachment) = op_attachment
            && let Ok(file) = attachment.download().await
        {
            Some(CreateAttachment::bytes(file, attachment.filename.clone()))
        } else {
            None
        }
    }))
    .await
    .into_iter()
    .flatten()
    .collect::<Vec<CreateAttachment>>();

    let message = CreateMessage::new().content(content).add_files(files);
    message.execute(ctx.http(), (channel.id(), None)).await?;
    ctx.reply("Successfully sent!").await?;
    Ok(())
}

/// Refreshes the current leaks channel.
#[poise::command(prefix_command, slash_command)]
pub async fn refresh_channel(ctx: Context<'_>) -> Result<(), Error> {
    {
        let guild = ctx.http().get_guild(GuildId::new(GUILD_ID)).await.unwrap();
        let guild_channels = guild.channels(ctx.http()).await.unwrap();
        let mut current_channel = ctx.data().current_channel.write().await;
        *current_channel = match guild_channels.iter().find(|x| {
            x.1.name == "leaks" && x.1.parent_id == Some(ChannelId::new(PUBLIC_CATEGORY_ID))
        }) {
            Some((channel_id, _)) => *channel_id,
            None => {
                let channel = CreateChannel::new("leaks").category(PUBLIC_CATEGORY_ID);
                let channel = channel.execute(ctx.http(), guild.id).await.unwrap();
                channel.id
            }
        };
    }

    ctx.reply("Successfully refreshed!").await?;
    Ok(())
}

#[derive(Debug, Clone, Copy, ChoiceParameter)]
pub enum OnlineStatusChoice {
    Online,
    Idle,
    DoNotDisturb,
    Invisible,
    Offline,
}

#[derive(Debug, Clone, Copy, ChoiceParameter)]
pub enum ActivityTypeChoice {
    Playing,
    Streaming,
    Listening,
    Watching,
    Custom,
    Competing,
}

/// Set the bot's rich presence.
#[poise::command(prefix_command, slash_command)]
pub async fn change_status(
    ctx: Context<'_>,
    #[description = "What should the name of the status be?"] name: String,
    #[description = "Which online status do you want?"] status: OnlineStatusChoice,
    #[description = "Which activity type for rich presence do you want?"] kind: ActivityTypeChoice,
    #[description = "(optional, unless status is Custom) attachment 1"] state: Option<String>,
    #[description = "(optional, unless status is Streaming) attachment 1"] url: Option<String>,
) -> Result<(), Error> {
    ctx.serenity_context().reset_presence();
    ctx.serenity_context().set_presence(
        Some(match kind {
            ActivityTypeChoice::Playing => ActivityData::playing(name),
            ActivityTypeChoice::Streaming => {
                ActivityData::streaming(name, url.unwrap_or_default())?
            }
            ActivityTypeChoice::Listening => ActivityData::listening(name),
            ActivityTypeChoice::Watching => ActivityData::watching(name),
            ActivityTypeChoice::Custom => ActivityData::custom(state.unwrap_or_default()),
            ActivityTypeChoice::Competing => ActivityData::competing(name),
        }),
        match status {
            OnlineStatusChoice::Online => OnlineStatus::Online,
            OnlineStatusChoice::Idle => OnlineStatus::Idle,
            OnlineStatusChoice::DoNotDisturb => OnlineStatus::DoNotDisturb,
            OnlineStatusChoice::Invisible => OnlineStatus::Invisible,
            OnlineStatusChoice::Offline => OnlineStatus::Offline,
        },
    );

    ctx.reply("Successfully set!").await?;
    Ok(())
}
