use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::exit;
use anyhow::Context;
use log::{debug, error};
use serde::Deserialize;

const CONFIG_PATH: &str = "tapoctl/config.toml";

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum Config {
    Server(ServerConfig),
    Client(ClientConfig),
    None
}

impl ToString for Config {
    fn to_string(&self) -> String {
        // we return the CONFIG_PATH here since the `ToString` trait is used by
        // clap for the default preview and we parse the config path directly
        // to a config
        dirs::config_dir().unwrap_or_default().join(CONFIG_PATH).to_str().unwrap_or_default().to_string()
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct ClientConfig {
    #[serde(default = "default_address")]
    pub address: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub secure: bool
}

#[derive(Deserialize, Debug, Clone)]
pub struct ServerConfig {
    pub auth: Authentication,
    pub devices: HashMap<String, DeviceDefinition>,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_timeout")]
    pub timeout: u32
}

#[derive(Deserialize, Debug, Clone)]
pub struct Authentication {
    pub username: String,
    pub password: String
}

#[derive(Deserialize, Debug, Clone)]
pub struct DeviceDefinition {
    pub r#type: SupportedDevice,
    pub address: String
}

#[derive(Deserialize, Debug, Clone)]
pub enum SupportedDevice {
    L530,
    L630,
    L900,
    L510,
    L520,
    L610,
    Generic
}

impl ClientConfig {
    pub fn from(address: Option<String>, port: Option<u16>, secure: Option<bool>) -> Option<Self> {
        if address.is_some() || port.is_some() || secure.is_some() {
            Some(Self { port: port.unwrap_or(default_port()), address: address.unwrap_or(default_address()), secure: secure.unwrap_or_default() })
        } else {
            None
        }
    }
}

impl Config {
    pub fn new(alternative_path: Option<String>) -> Self {
        let path = match &alternative_path {
            Some(path) => PathBuf::from(path),
            None => dirs::config_dir().unwrap_or_default().join(CONFIG_PATH)
        };

        let content = match fs::read(&path).context(format!("Missing configuration file at '{}'", path.to_string_lossy())) {
            Ok(content) => content,
            Err(err) => {
                debug!("Unable to read config file at {path:?}: {err}");
                return Config::None
            }
        };

        let utf8 = match String::from_utf8(content) {
            Ok(utf8) => utf8,
            Err(_) => {
                error!("Invalid UTF-8 config file at {path:?}");
                exit(1)
            }
        };

        toml::from_str(utf8.as_str()).context("Config file doesn't match config definition").unwrap_or_else(|err| {
            error!("Error whilst reading config file: {err}");
            Config::None
        })
    }
}

fn default_address() -> String {
    String::from("127.0.0.1")
}

fn default_port() -> u16 {
    19191
}

fn default_timeout() -> u32 { 10000 }