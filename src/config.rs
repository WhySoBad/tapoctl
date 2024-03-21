use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use anyhow::Context;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub auth: Authentication,
    pub devices: HashMap<String, DeviceDefinition>
}

#[derive(Deserialize, Debug)]
pub struct Authentication {
    pub username: String,
    pub password: String
}

#[derive(Deserialize, Debug)]
pub struct DeviceDefinition {
    pub r#type: SupportedDevice,
    pub address: String
}

#[derive(Deserialize, Debug)]
pub enum SupportedDevice {
    L530,
    L630,
    L900
}

const CONFIG_PATH: &str = "config.toml";

impl Config {
    pub fn new(alternative_path: Option<String>) -> anyhow::Result<Self> {
        let path = PathBuf::from(alternative_path.unwrap_or(CONFIG_PATH.to_string()));
        let content = fs::read(&path).context(format!("Missing configuration file at '{}'", path.to_string_lossy()))?;
        toml::from_str(String::from_utf8(content).context("Invalid utf8 config file")?.as_str()).context("Config file doesn't match config definition")
    }
}