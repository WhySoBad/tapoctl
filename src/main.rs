#![feature(duration_constructors)]

use std::collections::HashMap;
use std::process::exit;
use std::str::FromStr;
use clap::Parser;
use colored::Colorize;
use log::error;
use serde_json::{json, Value};
use spinoff::{Spinner, spinners};
use tonic::transport::Channel;
use crate::cli::{Cli, Commands, SpinnerOpt};
use crate::config::{ClientConfig, Config};
use crate::tapo::server::rpc::{DeviceRequest, HueSaturation, Empty, SetRequest};
use crate::tapo::server::rpc::tapo_client::TapoClient;
use crate::tapo::start_server;
use crate::tapo::TonicErrMap;

mod device;
mod config;
mod tapo;
mod cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let cli = Cli::parse();
    let config = cli.config;
    let json = cli.json;

    let client_config = match &config {
        Config::Client(cfg) => Some(cfg),
        _ => None,
    };

    let server_config = match &config {
        Config::Server(cfg) => Some(cfg),
        _ => None
    };

    if let Commands::Serve { port } = cli.command {
        start_server(port, server_config).await;
    } else {
        let mut spinner = Some(&mut Spinner::new(spinners::Dots, "Preparing client...", None));
        let mut client = get_client(client_config).await;
        if let Some(mut spinner) = spinner {
            // TODO: Create custom update method
            spinner.update(spinners::Dots, "Sending request...", None);
        }
        match cli.command {
            Commands::Devices => {
                let devices = client.devices(Empty {}).await.map_tonic_err(spinner).into_inner();
                if json {
                    println!("{}", json!(devices))
                } else {
                    spinner.success("Found devices:");
                    devices.devices.iter().for_each(|dev| {
                        println!("{}: type {} at {}", dev.name.bold(), dev.r#type, dev.address)
                    });
                }
            }

            Commands::Set { device, color, brightness, temperature, hue_saturation, power } => {
                let request = SetRequest {
                    color: color.map(|c| c as i32),
                    device,
                    brightness,
                    temperature,
                    power,
                    hue_saturation: {
                        let hue = hue_saturation.hue;
                        let saturation = hue_saturation.saturation;
                        if hue.is_some() && saturation.is_some() {
                            Some(HueSaturation { saturation, hue })
                        } else {
                            None
                        }
                    }
                };

                let state = client.set(request).await.map_tonic_err(spinner).into_inner();
                if json {
                    println!("{}", json!(state))
                } else {
                    spinner.success("Updated device:");
                    todo!("Create a nice print format")
                }
            }
            Commands::Info { device } => {
                if json {
                    let json = client.info_json(DeviceRequest { device }).await.map_tonic_err(spinner);
                    let value: HashMap<String, serde_json::Value> = serde_json::from_slice(json.into_inner().data.as_slice()).unwrap();
                    println!("{}", json!(value));
                } else {
                    let info = client.info(DeviceRequest { device }).await.map_tonic_err(spinner);
                    println!("{:#?}", info.into_inner());
                    todo!("Create a nice print format")
                }
            }
            Commands::Usage { device } => {
                let usage = client.usage(DeviceRequest { device }).await.map_tonic_err(spinner).into_inner();
                if json {
                    println!("{}", json!(usage))
                } else {
                    todo!("Create a nice print format")
                }
            }
            Commands::On { device } => {
                let result = client.on(DeviceRequest { device: device.clone() }).await.map_tonic_err(spinner).into_inner();
                if json {
                    println!("{}", json!(result))
                } else {
                    println!("Device '{device}' is now turned on")
                }
            }
            Commands::Off { device } => {
                let result = client.off(DeviceRequest { device: device.clone() }).await.map_tonic_err(spinner).into_inner();
                if json {
                    println!("{}", json!(result))
                } else {
                    println!("Device '{device}' is now turned off")
                }
            }
            Commands::Reset { device } => {
                client.reset(DeviceRequest { device }).await.map_tonic_err(spinner);
            }
            _ => {
                unreachable!()
            }
        }

    }

    Ok(())
}

async fn get_client(config: Option<&ClientConfig>) -> TapoClient<Channel> {
    let (secure, host, port) = match config {
        Some(config) => (config.secure, config.address.clone(), config.port),
        None => (false, String::from("127.0.0.1"), 19191)
    };

    let secure = std::env::var("TAPO_SECURE").is_ok() || secure;
    let host = std::env::var("TAPO_HOST").unwrap_or(host);
    let port = std::env::var("TAPO_PORT").map(|p| u16::from_str(p.as_str()).unwrap_or(port)).unwrap_or(port);
    let protocol = secure.then_some("https").unwrap_or("http");

    let format = format!("{protocol}://{host}:{port}");
    TapoClient::connect(format.clone()).await.unwrap_or_else(|_| {
        error!("Unable to connect to server at {format}. Is it up and running?");
        exit(1)
    })
}
