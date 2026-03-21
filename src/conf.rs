use std::{env, fs};

use anyhow::{Context, Result};
use serde::Deserialize;

pub struct Config {
    pub url: String,
    pub list_id: String,
    pub api_key: String,
}

#[derive(Deserialize)]
struct ConfigFile {
    url: String,
    list_id: String,
    api_key_path: String,
}

pub fn get_config() -> Result<Config> {
    let xdg_config_dir = env::var("XDG_CONFIG_HOME");
    let home_config_dir = env::var("HOME").map(|h| format!("{h}/.config"));
    let user_config_dir = env::var("USER").map(|u| format!("/home{u}/.config"));
    let config_dir = xdg_config_dir
        .or(home_config_dir.or(user_config_dir))
        .context("Failed to determine config dir")?;

    let karatui_conf = fs::read_to_string(format!("{config_dir}/karatui/karatui.toml"))
        .context("Failed to read karatui config")?;

    let parsed_conf: ConfigFile =
        toml::from_str(&karatui_conf).context("Failed to parse karatui config")?;

    let api_key = fs::read_to_string(parsed_conf.api_key_path)
        .map(|k| k.replace("\n", ""))
        .context("Failed to read API key")?;

    Ok(Config {
        url: parsed_conf.url,
        list_id: parsed_conf.list_id,
        api_key,
    })
}
