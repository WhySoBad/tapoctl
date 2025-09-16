use std::ops::{Deref, DerefMut};

use crate::device::{Device, DeviceHandler};
use crate::error::TapoErrorExt;

use super::server::rpc::{Empty, InfoJsonResponse, InfoResponse, PowerResponse, UsageResponse};
use super::{TapoDeviceExt, TapoDeviceHandlerExt};

pub mod color_light;
pub mod generic;
pub mod light;

macro_rules! call_device_handlers {
    ($device:ident, mut $handler:ident => $expr:expr) => {
        call_device_handlers!($device, $handler, $expr, deref_mut)
    };
    ($device:ident, $handler:ident => $expr:expr) => {
        call_device_handlers!($device, $handler, $expr, deref)
    };
    ($device:ident, $handler:ident, $expr:expr, $deref_type:ident) => {{
        let mut result = match $device.get_handler().await?.$deref_type() {
            DeviceHandler::ColorLight($handler) => $expr,
            DeviceHandler::Light($handler) => $expr,
            DeviceHandler::Generic($handler) => $expr,
        };
        if result.is_session_timeout() {
            log::warn!("Session for device '{}' expired, attempting refresh", $device.name);
            $device.refresh_session().await?;
            result = match $device.get_handler().await?.$deref_type() {
                DeviceHandler::ColorLight($handler) => $expr,
                DeviceHandler::Light($handler) => $expr,
                DeviceHandler::Generic($handler) => $expr,
            };
        }

        result
    }};
}

impl TapoDeviceExt for Device {
    async fn refresh_session(&self) -> Result<Empty, tapo::Error> {
        match self.get_handler_mut().await?.deref_mut() {
            DeviceHandler::ColorLight(handler) => {
                handler.refresh_session().await?;
            }
            DeviceHandler::Light(handler) => {
                handler.refresh_session().await?;
            }
            DeviceHandler::Generic(handler) => {
                handler.refresh_session().await?;
            }
        };
        Ok(Empty {})
    }

    async fn reset(&self) -> Result<Empty, tapo::Error> {
        let result = call_device_handlers! { self, handler => handler.reset().await };
        result.map(|_| Empty {})
    }

    async fn get_info(&self) -> Result<InfoResponse, tapo::Error> {
        call_device_handlers! { self, handler => handler.get_info().await }
    }

    async fn get_info_json(&self) -> Result<InfoJsonResponse, tapo::Error> {
        call_device_handlers! { self, handler => handler.get_info_json().await }
    }

    async fn get_usage(&self) -> Result<UsageResponse, tapo::Error> {
        call_device_handlers! { self, handler => handler.get_usage().await }
    }

    async fn on(&self) -> Result<PowerResponse, tapo::Error> {
        call_device_handlers! { self, handler => handler.power_on().await }
    }

    async fn off(&self) -> Result<PowerResponse, tapo::Error> {
        call_device_handlers! { self, handler => handler.power_off().await }
    }

    async fn set(
        &self,
        mut info: InfoResponse,
        power: Option<bool>,
        brightness: Option<u8>,
        temperature: Option<u16>,
        hue_saturation: Option<(u16, u8)>,
    ) -> Result<InfoResponse, tapo::Error> {
        let result = match self.get_handler().await?.deref() {
            DeviceHandler::ColorLight(handler) => {
                handler
                    .update(power, brightness, temperature, hue_saturation)
                    .await
            }
            DeviceHandler::Light(handler) => {
                info.hue = None;
                info.saturation = None;
                info.temperature = None;
                handler
                    .update(power, brightness, temperature, hue_saturation)
                    .await
            }
            DeviceHandler::Generic(handler) => {
                info.hue = None;
                info.saturation = None;
                info.temperature = None;
                info.brightness = None;
                handler
                    .update(power, brightness, temperature, hue_saturation)
                    .await
            }
        };

        result.map(|_| info)
    }
}
