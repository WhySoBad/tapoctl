use std::time::{Duration, SystemTime};
use log::{info, warn};
use tapo::{ApiClient, ColorLightHandler, GenericDeviceHandler, LightHandler};
use tonic::Status;
use crate::config::{DeviceDefinition, SupportedDevice};

const SESSION_VALIDITY_MILLIS: u64 = 60 * 60 * 1000;

pub enum SessionStatus {
    Authenticated,
    Refreshing,
    Error,
}

pub struct Device {
    pub address: String,
    pub name: String,
    pub r#type: SupportedDevice,
    pub handler: DeviceHandler,
    pub session_start: SystemTime,
    pub session_status: SessionStatus
}

impl Device {
    pub async fn new(name: String, definition: DeviceDefinition, client: ApiClient) -> Option<Self> {
        let handler_opt = match definition.r#type {
            SupportedDevice::L530 => {
                client.l530(&definition.address).await.ok().map(DeviceHandler::ColorLight)
            },
            SupportedDevice::L630 => {
                client.l630(&definition.address).await.ok().map(DeviceHandler::ColorLight)
            },
            SupportedDevice::L900 => {
                client.l900(&definition.address).await.ok().map(DeviceHandler::ColorLight)
            },
            SupportedDevice::L510 => {
                client.l510(&definition.address).await.ok().map(DeviceHandler::Light)
            }
            SupportedDevice::L520 => {
                client.l520(&definition.address).await.ok().map(DeviceHandler::Light)
            }
            SupportedDevice::L610 => {
                client.l520(&definition.address).await.ok().map(DeviceHandler::Light)
            }
            SupportedDevice::Generic => {
                client.generic_device(&definition.address).await.ok().map(DeviceHandler::Generic)
            }
        };

        let handler = match handler_opt {
            Some(handler) => {
                info!("Logged into device '{name}'");
                handler
            },
            None => {
                warn!("Unable to log into device '{name}'");
                return None
            }
        };

        Some(Self {
            handler,
            r#type: definition.r#type,
            address: definition.address,
            name,
            session_start: SystemTime::now(),
            session_status: SessionStatus::Authenticated
        })
    }

    pub async fn try_refresh_session(&mut self) -> Result<(), Status> {
        let now = SystemTime::now();
        if now.duration_since(self.session_start).is_ok_and(|d| d.gt(&Duration::from_millis(SESSION_VALIDITY_MILLIS))) {
            self.session_status = SessionStatus::Refreshing;
            info!("Refreshing session for device '{}'", self.name);
            match &mut self.handler {
                DeviceHandler::ColorLight(handler) => {
                    handler.refresh_session().await.map_err(|err| Status::internal(err.to_string()))?;
                },
                DeviceHandler::Light(handler) => {
                    handler.refresh_session().await.map_err(|err| Status::internal(err.to_string()))?;
                },
                DeviceHandler::Generic(handler) => {
                    handler.refresh_session().await.map_err(|err| Status::internal(err.to_string()))?;
                },
            };
            self.session_status = SessionStatus::Authenticated;
            info!("Successfully refreshed session for device '{}'", self.name);
            self.session_start = now;
        }
        Ok(())
    }
}

pub enum DeviceHandler {
    ColorLight(ColorLightHandler),
    Light(LightHandler),
    Generic(GenericDeviceHandler)
}

