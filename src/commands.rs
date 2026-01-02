use serenity::{
    all::{
        Attachment, Builder, Channel, ChannelId, CreateAttachment, CreateChannel, CreateMessage,
        GuildId,
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
