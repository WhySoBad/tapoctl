use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::process::exit;
use std::sync::Arc;
use colored::{Colorize, CustomColor};
use colorsys::Rgb;
use log::{error, info};
use serde::Serialize;
use serde_json::json;
use spinoff::Spinner;
use tapo::ApiClient;
use tokio::sync::Mutex;
use tonic::transport::Server;
use crate::cli::SpinnerOpt;
use crate::config::ServerConfig;
use crate::device;
use crate::device::Device;
use crate::tapo::server::rpc::{Color, EventRequest, EventResponse, EventType, InfoResponse, SessionStatus, UsageResponse};
use crate::tapo::server::rpc::tapo_server::TapoServer;
use crate::tapo::server::{rpc, TapoService};

pub mod server;
mod color;
mod state;

pub async fn start_server(port: Option<u16>, config: Option<ServerConfig>) {
    let Some(config) = config else {
        error!("Please specify a server config for setting up the server");
        exit(1);
    };

    let client = match ApiClient::new(&config.auth.username, &config.auth.password) {
        Ok(client) => client,
        Err(_) => {
            error!("Unable to create tapo api client");
            exit(1);
        }
    };

    let mut devices = HashMap::<String, Arc<Mutex<Device>>>::new();
    let (tx, rx) = tokio::sync::broadcast::channel(10);

    info!("Starting device login phase");

    let devices_async = config.devices.into_iter().map(|(name, definition)| {
        Device::new(name, definition, client.clone(), tx.clone())
    });

    futures::future::join_all(devices_async).await.into_iter()
        .flatten()
        .for_each(|dev| {
            devices.insert(dev.name.clone(), Arc::new(Mutex::new(dev)));
        });

    info!("Finished device login phase");

    let port = port.unwrap_or(config.port);

    let format = format!("0.0.0.0:{port}");
    let addr = match format.parse() {
        Ok(addr) => addr,
        Err(_) => {
            error!("'{format}' is not a valid socket address");
            exit(1)
        }
    };

    let svc = TapoServer::new(TapoService::new(devices, (tx, rx)));
    info!("Starting server at {format}");
    match Server::builder().add_service(svc).serve(addr).await {
        Ok(_) => info!("Stopped server"),
        Err(err) => {
            error!("Unable to serve at {format}. Reason: {err}");
            exit(1)
        }
    }
}

/// ugly solution to transform tonic colors to tapo colors since the map is crate public
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

pub fn transform_session_status(session_status: &device::SessionStatus) -> SessionStatus {
    match session_status { 
        device::SessionStatus::Authenticated => SessionStatus::Authenticated,
        device::SessionStatus::Failure => SessionStatus::Failure,
        device::SessionStatus::RepeatedFailure => SessionStatus::RepeatedFailure,
    }
}

pub fn create_event(event_type: EventType, body: impl Serialize) -> EventResponse {
    let mut bytes = vec![];
    serde_json::to_writer(&mut bytes, &body).unwrap_or_default();
    EventResponse { body: bytes, r#type: i32::from(event_type) }
}

pub trait TonicErrMap<R> {
    fn map_tonic_err(self, spinner: &mut Option<Spinner>, json: bool) -> R;
}

impl<R> TonicErrMap<R> for Result<R, tonic::Status> {
    fn map_tonic_err(self, spinner: &mut Option<Spinner>, json: bool) -> R {
        self.unwrap_or_else(|status| {
            let message = status.message();
            let code = status.code().to_string();
            if json {
                println!("{}", json!({ "message": message, "code": code }));
            } else if spinner.is_some() {
                spinner.fail(status.message());
            } else {
                error!("{}", status.message());
            }
            exit(1)
        })
    }
}


impl Display for InfoResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut lines = vec![];
        if let Some(on) = &self.device_on {
            let state = on.then_some("Turned on").unwrap_or("Turned off");
            lines.push(format!("{}: {state}", "State".bold()))
        }
        let overheated = if self.overheated { "Overheated" } else { "Normal" };
        lines.push(format!("{}: {overheated}", "Thermals".bold()));
        if let Some(on_time) = &self.on_time {
            lines.push(format!("{}: {}min", "Uptime".bold(), on_time / 60u64))
        }
        if let Some(temperature) = &self.temperature {
            if temperature > &0 {
                lines.push(format!("{}: {temperature}K", "Temperature".bold()))
            }
        }
        if let Some(color) = &self.color {
            let block = "  ".on_custom_color(CustomColor::new(u8::try_from(color.red).unwrap_or_default(), u8::try_from(color.green).unwrap_or_default(), u8::try_from(color.blue).unwrap_or_default()));
            let color = Rgb::new(color.red as f64, color.green as f64, color.blue as f64, None).to_hex_string();
            lines.push(format!("{}: {color} {block}", "Color".bold()));
        }
        if let Some(brightness) = &self.brightness {
            lines.push(format!("{}: {brightness}%", "Brightness".bold()))
        }
        if let Some((hue, saturation)) = &self.hue.zip(self.saturation) {
            lines.push(format!("{}: {hue}", "Hue".bold()));
            lines.push(format!("{}: {saturation}%", "Saturation".bold()))
        }
        if let Some(effect_id) = &self.dynamic_effect_id {
            lines.push(format!("{}: {effect_id}", "Effect".bold()))
        }

        f.write_str(lines.join("\n").as_str())
    }
}

impl Display for UsageResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut lines = vec![];
        if let Some(time) = &self.time_usage {
            lines.push("Uptime:".underline().bold().to_string());
            lines.push(format!("{}: {:.2}h", "Today".bold(), time.today as f32 / 60f32));
            lines.push(format!("{}: {:.2}h", "Week".bold(), time.week as f32 / 60f32));
            lines.push(format!("{}: {:.2}h", "Month".bold(), time.month as f32 / 60f32));
        }
        if let Some(power) = &self.power_usage {
            if !lines.is_empty() {
                lines.push(String::new());
            }
            lines.push("Power used:".underline().bold().to_string());
            lines.push(format!("{}: {:.3}kWh", "Today".bold(), power.today as f32 / 1000f32));
            lines.push(format!("{}: {:.3}kWh", "Week".bold(), power.week as f32 / 1000f32));
            lines.push(format!("{}: {:.3}kWh", "Month".bold(), power.month as f32 / 1000f32));
        }
        if let Some(saved) = &self.saved_power {
            if !lines.is_empty() {
                lines.push(String::new());
            }
            lines.push("Power saved:".underline().bold().to_string());
            lines.push(format!("{}: {:.3}kWh", "Today".bold(), saved.today as f32 / 1000f32));
            lines.push(format!("{}: {:.3}kWh", "Week".bold(), saved.week as f32 / 1000f32));
            lines.push(format!("{}: {:.3}kWh", "Month".bold(), saved.month as f32 / 1000f32));
        }
        f.write_str(lines.join("\n").as_str())
    }
}

impl Display for rpc::Device {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut lines = vec![];
        let status = match SessionStatus::try_from(self.status).unwrap_or_default() {
            SessionStatus::Authenticated => "Authenticated",
            SessionStatus::Failure => "Authentication failed",
            SessionStatus::RepeatedFailure => "Authentication failed multiple times",
        };
        lines.push(format!("{}: {}", "Type".bold(), self.r#type));
        lines.push(format!("{}: {}", "Session".bold(), status));
        lines.push(format!("{}: {}", "Address".bold(), self.address));
        f.write_str(lines.join("\n").as_str())
    }
}