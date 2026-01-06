#![warn(clippy::str_to_string)]

pub const MAIN_POSTING_CHANNEL_ID: u64 = 1457958995011698739;
pub const PUBLIC_CATEGORY_ID: u64 = 1456067853374586970;
pub const GUILD_ID: u64 = 1451378473858895884;
pub const FILE_UPLOAD_LIMIT: u32 = 95000000;

mod commands;
mod event_handlers;

use ::serenity::all::{Builder, ChannelId, CreateChannel, GuildId};
use poise::serenity_prelude as serenity;
use std::{env::var, sync::Arc};
use tokio::sync::RwLock;

// Types used by all command functions
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

// Custom user data passed to all command functions
pub struct Data {
    current_channel: Arc<RwLock<ChannelId>>,
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    // This is our custom error handler
    // They are many errors that can occur, so we only handle the ones we want to customize
    // and forward the rest to the default handler
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            println!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                println!("Error while handling error: {}", e)
            }
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // FrameworkOptions contains all of poise's configuration option in one struct
    // Every option can be omitted to use its default value
    let options = poise::FrameworkOptions {
        commands: vec![
            commands::send_to_channel(),
            commands::refresh_channel(),
            commands::change_status(),
        ],
        prefix_options: poise::PrefixFrameworkOptions {
            ..Default::default()
        },
        // The global error handler for all error cases that may occur
        on_error: |error| Box::pin(on_error(error)),
        // This code is run before every command
        pre_command: |ctx| {
            Box::pin(async move {
                println!("Executing command {}...", ctx.command().qualified_name);
            })
        },
        // This code is run after a command if it was successful (returned Ok)
        post_command: |ctx| {
            Box::pin(async move {
                println!("Executed command {}!", ctx.command().qualified_name);
            })
        },
        // Every command invocation must pass this check to continue execution
        command_check: Some(|ctx| {
            Box::pin(async move {
                if ctx.guild().is_none_or(|x| x.id != GUILD_ID) {
                    return Ok(false);
                }
                Ok(true)
            })
        }),
        // Enforce command checks even for owners (enforced by default)
        // Set to true to bypass checks, which is useful for testing
        skip_checks_for_owners: false,
        event_handler: |ctx, event, framework, data| {
            Box::pin(async move {
                match event {
                    serenity::FullEvent::ReactionAdd { add_reaction } => {
                        event_handlers::add_reaction(add_reaction, ctx, data).await
                    }
                    serenity::FullEvent::MessageUpdate {
                        old_if_available: _,
                        new: _,
                        event,
                    } => event_handlers::message_update(event, ctx, framework, data).await,
                    serenity::FullEvent::MessageDelete {
                        channel_id,
                        deleted_message_id,
                        guild_id: _,
                    } => {
                        event_handlers::message_delete(
                            channel_id,
                            deleted_message_id,
                            ctx,
                            framework,
                            data,
                        )
                        .await
                    }
                    _ => Ok(()),
                }
            })
        },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", _ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                let guild = ctx
                    .http
                    .clone()
                    .get_guild(GuildId::new(GUILD_ID))
                    .await
                    .unwrap();
                let guild_channels = guild.channels(ctx.http.clone()).await?;
                let leaks_channel_id = match guild_channels.iter().find(|x| {
                    x.1.name == "leaks" && x.1.parent_id == Some(ChannelId::new(PUBLIC_CATEGORY_ID))
                }) {
                    Some((channel_id, _)) => *channel_id,
                    None => {
                        let channel = CreateChannel::new("leaks")
                            .category(PUBLIC_CATEGORY_ID)
                            .kind(serenity::ChannelType::News);
                        let channel = channel.execute(ctx.http.clone(), guild.id).await?;
                        channel.id
                    }
                };

                Ok(Data {
                    current_channel: Arc::new(RwLock::new(leaks_channel_id)),
                })
            })
        })
        .options(options)
        .build();

    let token = var("DISCORD_TOKEN")
        .expect("Missing `DISCORD_TOKEN` env var, see README for more information.");
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap()
}
