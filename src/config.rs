use std::sync::OnceLock;

use serde::Deserialize;

use crate::{config_util, config_util::TomlConfig};

#[derive(Deserialize)]
pub struct BotConfig {
    pub discord_token: String,
    pub sent_channel_name: String,
    pub main_posting_channel_id: u64,
    pub public_category_id: u64,
    pub guild_id: u64,
    pub file_upload_limit: u32,
    pub prune_role: u64,
}

impl TomlConfig for BotConfig {
    const DEFAULT_TOML: &str = include_str!("../config.default.toml");
}

static CONFIG: OnceLock<BotConfig> = OnceLock::new();

pub fn get_config() -> &'static BotConfig {
    CONFIG.get_or_init(|| config_util::load_or_create("config.toml"))
}
