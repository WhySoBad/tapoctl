use std::collections::HashMap;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;
use log::{error, info};
use serde::Serialize;
use serde_json::json;
use server::rpc::{Empty, InfoJsonResponse, PowerResponse, UsageResponse};
use spinoff::Spinner;
use tapo::ApiClient;
use tokio::sync::RwLock;
use tonic::transport::Server;
use tonic::Response;
use crate::cli::SpinnerOpt;
use crate::config::ServerConfig;
use crate::device::Device;
use crate::tapo::server::rpc::{EventResponse, EventType, InfoResponse, SessionStatus};
use crate::tapo::server::rpc::tapo_server::TapoServer;
use crate::tapo::server::{rpc, TapoService};

pub mod server;
mod color;
mod state;
mod print;
mod device;
mod validation;

pub async fn start_server(port: Option<u16>, config: Option<ServerConfig>) {
    let Some(config) = config else {
        error!("Please specify a server config for setting up the server");
        exit(1);
    };

    let mut devices = HashMap::<String, Arc<RwLock<Device>>>::new();
    let (tx, rx) = tokio::sync::broadcast::channel(10);

    info!("Starting device login phase");

    let devices_async = config.devices.into_iter().map(|(name, definition)| {
        // give every device its own client for more parallelism since it seems as if sharing the same client
        // causes blocking when sending requests for multiple devices in a short period of time
        let client = ApiClient::new(&config.auth.username, &config.auth.password).with_timeout(Duration::from_millis(config.timeout as u64));
        Device::new(name, definition, client, tx.clone())
    });

    futures::future::join_all(devices_async).await.into_iter()
        .flatten()
        .for_each(|dev| {
            devices.insert(dev.name.clone(), Arc::new(RwLock::new(dev)));
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

pub fn create_event(event_type: EventType, body: impl Serialize) -> EventResponse {
    let mut bytes = vec![];
    serde_json::to_writer(&mut bytes, &body).unwrap_or_default();
    EventResponse { body: bytes, r#type: i32::from(event_type) }
}

pub trait TapoRpcColorExt {
    /// Get the tapo library color representation of the color
    fn tapo_color(&self) -> tapo::requests::Color;
}

pub trait TapoSessionStatusExt {
    /// Get the rpc representation of the session status
    fn rpc(&self) -> rpc::SessionStatus;
}

impl TapoSessionStatusExt for crate::device::SessionStatus {
    fn rpc(&self) -> rpc::SessionStatus {
        match self {
            crate::device::SessionStatus::Authenticated => SessionStatus::Authenticated,
            crate::device::SessionStatus::Failure => SessionStatus::Failure,
            crate::device::SessionStatus::RepeatedFailure => SessionStatus::RepeatedFailure,
        }
    }
}

pub trait TapoDeviceExt {
    /// Reset the device to factory defaults
    async fn reset(&self) -> Result<Response<Empty>, tonic::Status>;

    /// Get some information about the device
    async fn get_info(&self) -> Result<Response<InfoResponse>, tonic::Status>;

    /// Get all information as raw json about the device
    async fn get_info_json(&self) -> Result<Response<InfoJsonResponse>, tonic::Status>;

    /// Get the power and energy usage of the device
    async fn get_usage(&self) -> Result<Response<UsageResponse>, tonic::Status>;

    /// Power the device on
    async fn on(&self) -> Result<Response<PowerResponse>, tonic::Status>;

    /// Power the device off
    async fn off(&self) -> Result<Response<PowerResponse>, tonic::Status>;

    /// Set multiple properties of the device at once
    async fn set(
        &self,
        info: InfoResponse,
        power: Option<bool>,
        brightness: Option<u8>,
        temperature: Option<u16>,
        hue_saturation: Option<(u16, u8)>
    ) -> Result<Response<InfoResponse>, tonic::Status>;
}

pub trait TapoDeviceHandlerExt {
    /// Reset the device to factory defaults
    async fn reset(&self, device: &Device) -> Result<(), tonic::Status>;

    /// Get some information about the device
    async fn get_info(&self, device: &Device) -> Result<InfoResponse, tonic::Status>;

    /// Get all information as raw json about the device
    async fn get_info_json(&self, device: &Device) -> Result<InfoJsonResponse, tonic::Status>;

    /// Get the power and energy usage of the device
    async fn get_usage(&self, device: &Device) -> Result<UsageResponse, tonic::Status>;

    /// Power the device on
    async fn power_on(&self, device: &Device) -> Result<PowerResponse, tonic::Status>;

    /// Power the device off
    async fn power_off(&self, device: &Device) -> Result<PowerResponse, tonic::Status>;

    /// Set multiple properties of the device at once
    async fn update(
        &self,
        device: &Device,
        power: Option<bool>,
        brightness: Option<u8>,
        temperature: Option<u16>,
        hue_saturation: Option<(u16, u8)>
    ) -> Result<(), tonic::Status>;
}

pub trait TonicErrMap<R> {
    /// Map a result to a result containing a tonic error
    fn map_tonic_err(self, spinner: &mut Option<Spinner>, json: bool) -> R;
}

impl<R> TonicErrMap<R> for Result<R, tonic::Status> {
    fn map_tonic_err(self, spinner: &mut Option<Spinner>, json: bool) -> R {
        self.unwrap_or_else(|status| {
            let message = status.message();
            let code = status.code().to_string();
            if json { println!("{}", json!({ "message": message, "code": code })); }
            else if spinner.is_some() { spinner.fail(status.message()); }
            else { error!("{}", status.message()); }
            exit(1)
        })
    }
}

pub trait TapoErrMap<R> {
    async fn map_tapo_err(self, device: &Device) -> Result<R, tonic::Status>;
}

impl<R> TapoErrMap<R> for Result<R, tapo::Error> {
    async fn map_tapo_err(self, _device: &Device) -> Result<R, tonic::Status> {
        self.map_err(|err| {
            match err {
                tapo::Error::Tapo(tapo_response_error) => tonic::Status::internal(format!("{tapo_response_error:?}")),
                tapo::Error::Validation { field: _field, message } => tonic::Status::invalid_argument(message),
                tapo::Error::Serde(error) => tonic::Status::internal(error.to_string()),
                tapo::Error::Http(error) => tonic::Status::internal(error.to_string()),
                tapo::Error::DeviceNotFound => tonic::Status::not_found(err.to_string()),
                _ => tonic::Status::unknown(err.to_string()),
            }
        })
    }
}