use log::{info, warn};
use tapo::{ApiClient, ColorLightHandler};
use crate::config::{DeviceDefinition, SupportedDevice};

pub struct Device {
    pub address: String,
    pub name: String,
    pub r#type: SupportedDevice,
    pub handler: DeviceHandler,
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
            name
        })
    }
}

pub enum DeviceHandler {
    ColorLight(ColorLightHandler)
}

