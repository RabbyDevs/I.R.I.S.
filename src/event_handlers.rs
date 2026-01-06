use poise::FrameworkContext;
use serenity::{
    all::{
        Builder, ChannelId, Context, CreateAttachment, CreateMessage, EditAttachments, EditMessage,
        Message, MessageId, MessageReference, MessageUpdateEvent, Reaction, ReactionType,
        StickerItem,
    },
    futures::{StreamExt, future::join_all},
};

use crate::{Data, Error, FILE_UPLOAD_LIMIT, MAIN_POSTING_CHANNEL_ID};

pub struct CloneMessage {
    pub reference: Option<MessageReference>,
    pub content: String,
    pub files: Vec<CreateAttachment>,
    pub stickers: Vec<StickerItem>,
}

pub async fn clone_message(msg: &Message) -> CloneMessage {
    let mut clone = CloneMessage {
        reference: None,
        content: String::new(),
        files: vec![],
        stickers: vec![],
    };
    if let Some(reference) = msg.message_reference.clone() {
        clone.reference = Some(reference);
        return clone;
    }

    let content_base = msg.content.clone();

    let results = join_all(msg.attachments.iter().map(|attachment| async move {
        if attachment.size > FILE_UPLOAD_LIMIT {
            return (None, Some(attachment.url.clone()), 0);
        }

        match attachment.download().await {
            Ok(file) => (
                Some(CreateAttachment::bytes(file, attachment.filename.clone())),
                Some(attachment.url.clone()),
                attachment.size,
            ),
            Err(_) => (None, None, 0),
        }
    }))
    .await;

    let mut content = content_base;
    let mut files = Vec::new();
    let mut uploaded_bytes = 0u32;

    for (file, url, bytes) in results {
        uploaded_bytes += bytes;
        if let Some(url) = url {
            if let Some(file) = file {
                if uploaded_bytes > FILE_UPLOAD_LIMIT {
                    content.push_str(&format!("\n{}", url));
                } else {
                    files.push(file);
                }
            } else {
                content.push_str(&format!("\n{}", url));
            }
        }
    }

    clone.content = content;
    clone.files = files;
    clone.stickers = msg.sticker_items.clone();
    clone
}

pub async fn add_reaction(
    add_reaction: &Reaction,
    ctx: &Context,
    data: &Data,
) -> Result<(), Error> {
    if add_reaction.channel_id != MAIN_POSTING_CHANNEL_ID
        || add_reaction.emoji != ReactionType::Unicode(String::from("✅"))
        || add_reaction.message_author_id != add_reaction.user_id
    {
        return Ok(());
    }
    let leaks_channel_id = *data.current_channel.read().await;
    let msg = add_reaction.message(ctx.http.clone()).await?;

    let clone = clone_message(&msg).await;

    let mut create_msg = CreateMessage::new()
        .content(clone.content)
        .add_files(clone.files)
        .sticker_ids(clone.stickers.iter().map(|x| x.id).collect::<Vec<_>>());
    if let Some(reference) = clone.reference {
        create_msg = create_msg.reference_message(reference);
    }
    let final_msg = create_msg
        .execute(ctx.http.clone(), (leaks_channel_id, None))
        .await?
        .crosspost(ctx.http.clone())
        .await?;

    msg.reply(ctx.http.clone(), format!("{}:{}", final_msg.id, msg.id))
        .await?;
    Ok(())
}

pub async fn message_update(
    event: &MessageUpdateEvent,
    ctx: &Context,
    framework: FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    let leaks_channel_id = *data.current_channel.read().await;
    if event.channel_id != MAIN_POSTING_CHANNEL_ID {
        return Ok(());
    }

    let new_message = ChannelId::new(MAIN_POSTING_CHANNEL_ID)
        .message(ctx.http.clone(), event.id)
        .await?;

    let mut messages = ChannelId::new(MAIN_POSTING_CHANNEL_ID)
        .messages_iter(&ctx)
        .boxed();

    while let Some(message_result) = messages.next().await {
        let message = message_result?;
        if message.author.id == framework.bot_id
            && let Some(reference) = message.referenced_message
            && reference.id == new_message.id
            && let Some((first, _)) = message.content.split_once(':')
            && let Ok(parsed) = first.parse::<u64>()
        {
            let mut msg = leaks_channel_id.message(ctx.http.clone(), parsed).await?;

            let clone_message = clone_message(&new_message).await;
            let mut edit_attachments = EditAttachments::new();

            for file in clone_message.files {
                edit_attachments = edit_attachments.add(file);
            }

            let edit_message = EditMessage::new()
                .attachments(edit_attachments)
                .content(clone_message.content);
            msg.edit(ctx.http.clone(), edit_message).await?;
        }
    }
    Ok(())
}

pub async fn message_delete(
    channel_id: &ChannelId,
    deleted_message_id: &MessageId,
    ctx: &Context,
    framework: FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    let leaks_channel_id = *data.current_channel.read().await;
    if *channel_id != MAIN_POSTING_CHANNEL_ID {
        return Ok(());
    }

    let mut messages = ChannelId::new(MAIN_POSTING_CHANNEL_ID)
        .messages_iter(&ctx)
        .boxed();

    while let Some(message_result) = messages.next().await {
        let message = message_result?;
        if message.author.id == framework.bot_id
            && let Some((first, second)) = message.content.split_once(':')
            && let Ok(first_parsed) = first.parse::<u64>()
            && let Ok(second_parsed) = second.parse::<u64>()
            && second_parsed == deleted_message_id.get()
        {
            let msg = leaks_channel_id
                .message(ctx.http.clone(), first_parsed)
                .await?;

            msg.delete(ctx.http.clone()).await?;
        }
    }
    Ok(())
}
