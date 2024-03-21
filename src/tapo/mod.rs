use std::collections::HashMap;
use std::process::exit;
use anyhow::Context;
use log::{error, info};
use tapo::ApiClient;
use tonic::transport::Server;
use crate::config::Config;
use crate::device::Device;
use crate::tapo::server::rpc::Color;
use crate::tapo::server::rpc::tapo_server::TapoServer;
use crate::tapo::server::TapoService;

pub mod server;

pub async fn start_server(port: u16) {
    let config = match Config::new(None) {
        Ok(config) => config,
        Err(err) => {
            error!("Error whilst reading config: {} ({})", err.to_string(), err.root_cause());
            exit(1)
        }
    };

    let client = match ApiClient::new(&config.auth.username, &config.auth.password) {
        Ok(client) => client,
        Err(_) => {
            error!("Unable to create tapo api client");
            exit(1);
        }
    };

    let mut devices = HashMap::<String, Device>::new();

    info!("Starting device login phase");

    let devices_async = config.devices.into_iter().map(|(name, definition)| {
        Device::new(name, definition, client.clone())
    });

    futures::future::join_all(devices_async).await.into_iter()
        .flatten()
        .for_each(|dev| {
            devices.insert(dev.name.clone(), dev);
        });

    info!("Finished device login phase");

    let format = format!("127.0.0.1:{port}");
    let addr = match format.parse() {
        Ok(addr) => addr,
        Err(_) => {
            error!("'{format}' is not a valid socket address");
            exit(1)
        }
    };

    let svc = TapoServer::new(TapoService::new(devices));
    info!("Starting server at {format}");
    match Server::builder().add_service(svc).serve(addr).await {
        Ok(_) => {
            info!("Stopped server")
        },
        Err(err) => {
            error!("Unable to serve at {format}. Reason: {err}");
            exit(1)
        }
    }
}

/// ugly solution to transform tonic colors to tapo colors
fn transform_color(color: Color) -> tapo::requests::Color {
    match color {
        Color::CoolWhite => tapo::requests::Color::CoolWhite,
        Color::Daylight => tapo::requests::Color::Daylight,
        Color::Ivory => tapo::requests::Color::Ivory,
        Color::WarmWhite => tapo::requests::Color::WarmWhite,
        Color::Incandescent => tapo::requests::Color::Incandescent,
        Color::Candlelight => tapo::requests::Color::Candlelight,
        Color::Snow => tapo::requests::Color::Snow,
        Color::GhostWhite => tapo::requests::Color::GhostWhite,
        Color::AliceBlue => tapo::requests::Color::AliceBlue,
        Color::LightGoldenrod => tapo::requests::Color::LightGoldenrod,
        Color::LemonChiffon => tapo::requests::Color::LemonChiffon,
        Color::AntiqueWhite => tapo::requests::Color::AntiqueWhite,
        Color::Gold => tapo::requests::Color::Gold,
        Color::Peru => tapo::requests::Color::Peru,
        Color::Chocolate => tapo::requests::Color::Chocolate,
        Color::SandyBrown => tapo::requests::Color::SandyBrown,
        Color::Coral => tapo::requests::Color::Coral,
        Color::Pumpkin => tapo::requests::Color::Pumpkin,
        Color::Tomato => tapo::requests::Color::Tomato,
        Color::Vermilion => tapo::requests::Color::Vermilion,
        Color::OrangeRed => tapo::requests::Color::OrangeRed,
        Color::Pink => tapo::requests::Color::Pink,
        Color::Crimson => tapo::requests::Color::Crimson,
        Color::DarkRed => tapo::requests::Color::DarkRed,
        Color::HotPink => tapo::requests::Color::HotPink,
        Color::Smitten => tapo::requests::Color::Smitten,
        Color::MediumPurple => tapo::requests::Color::MediumPurple,
        Color::BlueViolet => tapo::requests::Color::BlueViolet,
        Color::Indigo => tapo::requests::Color::Indigo,
        Color::LightSkyBlue => tapo::requests::Color::LightSkyBlue,
        Color::CornflowerBlue => tapo::requests::Color::CornflowerBlue,
        Color::Ultramarine => tapo::requests::Color::Ultramarine,
        Color::DeepSkyBlue => tapo::requests::Color::DeepSkyBlue,
        Color::Azure => tapo::requests::Color::Azure,
        Color::NavyBlue => tapo::requests::Color::NavyBlue,
        Color::LightTurquoise => tapo::requests::Color::LightTurquoise,
        Color::Aquamarine => tapo::requests::Color::Aquamarine,
        Color::Turquoise => tapo::requests::Color::Turquoise,
        Color::LightGreen => tapo::requests::Color::LightGreen,
        Color::Lime => tapo::requests::Color::Lime,
        Color::ForestGreen => tapo::requests::Color::ForestGreen,
    }
}

pub trait TonicErrMap<R> {
    fn map_tonic_err(self) -> R;
}

impl<R> TonicErrMap<R> for Result<R, tonic::Status> {
    fn map_tonic_err(self) -> R {
        self.unwrap_or_else(|status| {
            error!("{}", status.message());
            exit(1)
        })
    }
}