use crate::config::{DeviceDefinition, SupportedDevice};
use anyhow::anyhow;
use log::{info, warn};
use tapo::{ApiClient, ColorLightHandler, GenericDeviceHandler, LightHandler};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tonic::Status;

pub struct Device {
    pub address: String,
    pub name: String,
    pub device_type: SupportedDevice,
    handler: Option<RwLock<DeviceHandler>>,
}

impl Device {
    pub async fn new(
        name: String,
        definition: DeviceDefinition,
        client: ApiClient,
    ) -> Option<Self> {
        let handler =
            Self::acquire_handler(&definition.device_type, &definition.address, client).await;

        if let Err(err) = &handler {
            warn!("Unable to log into device '{name}': {err}. Retrying on next access...")
        } else {
            info!("Logged into device {name}");
        }

        Some(Self {
            device_type: definition.device_type,
            address: definition.address,
            handler: handler.ok(),
            name,
        })
    }

    /// Try to get the device handler from the tapo api for a specific device
    async fn acquire_handler(
        device_type: &SupportedDevice,
        address: &String,
        client: ApiClient,
    ) -> Result<RwLock<DeviceHandler>, Status> {
        let handler = match device_type {
            SupportedDevice::L530 => client
                .l530(address)
                .await
                .map_err(|err| Status::internal(err.to_string()))
                .map(DeviceHandler::ColorLight),
            SupportedDevice::L630 => client
                .l630(address)
                .await
                .map_err(|err| Status::internal(err.to_string()))
                .map(DeviceHandler::ColorLight),
            SupportedDevice::L510 => client
                .l510(address)
                .await
                .map_err(|err| Status::internal(err.to_string()))
                .map(DeviceHandler::Light),
            SupportedDevice::L520 => client
                .l520(address)
                .await
                .map_err(|err| Status::internal(err.to_string()))
                .map(DeviceHandler::Light),
            SupportedDevice::L610 => client
                .l610(address)
                .await
                .map_err(|err| Status::internal(err.to_string()))
                .map(DeviceHandler::Light),
            SupportedDevice::Generic => client
                .generic_device(address)
                .await
                .map_err(|err| Status::internal(err.to_string()))
                .map(DeviceHandler::Generic),
        }?;

        Ok(RwLock::new(handler))
    }

    /// Access the current device handler
    ///
    /// Returns tonic status code should the handler be unavailable
    pub async fn get_handler(&self) -> Result<RwLockReadGuard<'_, DeviceHandler>, tapo::Error> {
        match &self.handler {
            Some(handler) => Ok(handler.read().await),
            None => Err(tapo::Error::Other(anyhow!(
                "The device '{}' is current unauthenticated",
                self.name
            ))),
        }
    }

    /// Access a mutable reference of the current device handler
    ///
    /// Returns tonic status code should the handler be unavailable
    pub async fn get_handler_mut(
        &self,
    ) -> Result<RwLockWriteGuard<'_, DeviceHandler>, tapo::Error> {
        match &self.handler {
            Some(handler) => Ok(handler.write().await),
            None => Err(tapo::Error::Other(anyhow!(
                "The device '{}' is current unauthenticated",
                self.name
            ))),
        }
    }
}

pub enum DeviceHandler {
    ColorLight(ColorLightHandler),
    Light(LightHandler),
    Generic(GenericDeviceHandler),
}
