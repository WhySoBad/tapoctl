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
use crate::tapo::server::rpc::{Color, DeviceRequest, EmptyRequest, EmptyResponse};
use crate::tapo::server::rpc::tapo_client::TapoClient;
use crate::tapo::server::rpc::tapo_server::TapoServer;
use crate::device::Device;
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
        Commands::Set { device, color, brightness, temperature, hue_saturation } => {
            todo!()
        }
        Commands::Info { device, json } => {
            todo!()
        }
        Commands::Usage { device } => {
            todo!()
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
    let port = u16::from_str(std::env::var("TAPO_PORT").unwrap_or(String::from("19191")).as_str()).unwrap_or(19191);
    let format = format!("http://127.0.0.1:{port}");
    TapoClient::connect(format.clone()).await.unwrap_or_else(|_| {
        error!("Unable to connect to server at {format}. Is it up and running?");
        exit(1)
    })
}
