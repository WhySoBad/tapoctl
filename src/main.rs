use std::collections::HashMap;
use std::process::exit;
use std::str::FromStr;
use ::tapo::ApiClient;
use anyhow::Context;
use clap::Parser;
use log::{error, info};
use tonic::transport::{Channel, Server};
use crate::cli::{Cli, Commands};
use crate::config::Config;
use crate::tapo::server::rpc::{Color, DeviceRequest, EmptyRequest, EmptyResponse, HueSaturation, SetRequest, UsageResponse};
use crate::tapo::server::rpc::tapo_client::TapoClient;
use crate::tapo::server::rpc::tapo_server::TapoServer;
use crate::device::Device;
use crate::tapo::server::rpc;
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
    match cli.command {
        Commands::Devices => {
            let mut client = get_client().await;
            let devices = client.devices(EmptyRequest {}).await.map_tonic_err();
            println!("{:?}", devices.into_inner());
        }
        Commands::Serve { port } => {
            start_server(port).await;
        }
        Commands::Set { device, color, brightness, temperature, hue_saturation, power } => {
            let mut client = get_client().await;
            let request = SetRequest {
                color: color.map(|c| c as i32),
                device,
                brightness: brightness.map(|v| v as u32),
                temperature: temperature.map(|v| v as u32),
                hue_saturation: {
                    let hue = hue_saturation.hue.map(|v| v as u32);
                    let saturation = hue_saturation.saturation.map(|v| v as u32);
                    if hue.is_some() && saturation.is_some() {
                        Some(HueSaturation {
                            saturation: saturation.unwrap_or_default(),
                            hue: hue.unwrap_or_default()
                        })
                    } else {
                        None
                    }
                },
                power
            };
            
            let response = client.set(request).await.map_tonic_err();
            println!("{:#?}", response.into_inner());
        }
        Commands::Info { device, json } => {
            let mut client = get_client().await;
            if json {
                let json = client.info_json(DeviceRequest { device }).await.map_tonic_err();
                println!("{:#?}", json.into_inner().data);
            } else {
                let info = client.info(DeviceRequest { device }).await.map_tonic_err();
                println!("{:#?}", info.into_inner());
            }
        }
        Commands::Usage { device } => {
            let mut client = get_client().await;
            let usage = client.usage(DeviceRequest { device }).await.map_tonic_err();
            println!("{:?}", usage.into_inner());
        }
        Commands::On { device } => {
            let mut client = get_client().await;
            client.on(DeviceRequest { device }).await.map_tonic_err();
        }
        Commands::Off { device } => {
            let mut client = get_client().await;
            client.off(DeviceRequest { device }).await.map_tonic_err();
        }
        Commands::Reset { device } => {
            let mut client = get_client().await;
            client.reset(DeviceRequest { device }).await.map_tonic_err();
        }
    }
    Ok(())
}

async fn get_client() -> TapoClient<Channel> {
    // TODO: Add option for custom host and port via cli arguments
    // additionally add cli configuration file at ~/.config/tapoctl.toml which persists those default values
    let port = u16::from_str(std::env::var("TAPO_PORT").unwrap_or(String::from("19191")).as_str()).unwrap_or(19191);
    let format = format!("http://127.0.0.1:{port}");
    TapoClient::connect(format.clone()).await.unwrap_or_else(|_| {
        error!("Unable to connect to server at {format}. Is it up and running?");
        exit(1)
    })
}
