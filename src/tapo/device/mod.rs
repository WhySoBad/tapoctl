use std::ops::{Deref, DerefMut};

use tonic::Response;

use crate::device::{Device, DeviceHandler};
use crate::error::TapoErrorExt;
use crate::tapo::TapoErrMap;

use super::server::rpc::{Empty, InfoJsonResponse, InfoResponse, PowerResponse, UsageResponse};
use super::{TapoDeviceExt, TapoDeviceHandlerExt};

pub mod color_light;
pub mod generic;
pub mod light;

macro_rules! handler_fn {
    ($function_name:ident on $device:ident) => {
        handler_fn!($device, $function_name, deref)
    };
    (mutable $function_name:ident on $device:ident) => {
        handler_fn!($device, $function_name, deref_mut)
    };
    ($device:ident, $function_name:ident, $deref_kind:ident) => {
        match $device.get_handler_mut()?.$deref_kind() {
            DeviceHandler::ColorLight(handler) => handler.$function_name().await,
            DeviceHandler::Light(handler) => handler.$function_name().await,
            DeviceHandler::Generic(handler) => handler.$function_name().await,
        }
    };
}

impl TapoDeviceExt for Device {
    async fn refresh_session(&self) -> Result<Response<Empty>, tonic::Status> {
        let result = match self.get_handler_mut()?.deref_mut() {
            DeviceHandler::ColorLight(handler) => handler.refresh_session().await.err(),
            DeviceHandler::Light(handler) => handler.refresh_session().await.err(),
            DeviceHandler::Generic(handler) => handler.refresh_session().await.err(),
        };

        if let Some(err) = result {
            Err(tonic::Status::internal(err.to_string()))
        } else {
            Ok(Response::new(Empty {}))
        }
    }

    async fn reset(&self) -> Result<Response<Empty>, tonic::Status> {
        let mut result = handler_fn!(reset on self);
        if result.is_session_timeout() {
            self.refresh_session().await?;
            result = handler_fn!(reset on self);
        }

        result
            .map_tapo_err(self)
            .await
            .map(|_| Response::new(Empty {}))
    }

    async fn get_info(&self) -> Result<Response<InfoResponse>, tonic::Status> {
        let mut result = handler_fn!(get_info on self);
        if result.is_session_timeout() {
            self.refresh_session().await?;
            result = handler_fn!(get_info on self);
        }

        result.map_tapo_err(self).await.map(Response::new)
    }

    async fn get_info_json(&self) -> Result<Response<InfoJsonResponse>, tonic::Status> {
        let mut result = handler_fn!(get_info_json on self);
        if result.is_session_timeout() {
            self.refresh_session().await?;
            result = handler_fn!(get_info_json on self);
        }

        result.map_tapo_err(self).await.map(Response::new)
    }

    async fn get_usage(&self) -> Result<Response<UsageResponse>, tonic::Status> {
        let mut result = handler_fn!(get_usage on self);
        if result.is_session_timeout() {
            self.refresh_session().await?;
            result = handler_fn!(get_usage on self);
        }

        result.map_tapo_err(self).await.map(Response::new)
    }

    async fn on(&self) -> Result<Response<PowerResponse>, tonic::Status> {
        let mut result = handler_fn!(power_on on self);
        if result.is_session_timeout() {
            self.refresh_session().await?;
            result = handler_fn!(power_on on self);
        }

        result.map_tapo_err(self).await.map(Response::new)
    }

    async fn off(&self) -> Result<Response<PowerResponse>, tonic::Status> {
        let mut result = handler_fn!(power_off on self);
        if result.is_session_timeout() {
            self.refresh_session().await?;
            result = handler_fn!(power_off on self);
        }

        result.map_tapo_err(self).await.map(Response::new)
    }

    async fn set(
        &self,
        mut info: InfoResponse,
        power: Option<bool>,
        brightness: Option<u8>,
        temperature: Option<u16>,
        hue_saturation: Option<(u16, u8)>,
    ) -> Result<Response<InfoResponse>, tonic::Status> {
        let result = match self.get_handler()?.deref() {
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

        match result {
            Ok(_) => Ok(Response::new(info)),
            Err(tapo::Error::Tapo(TapoResponseError))
        }

        todo!("Use result");
        Ok(Response::new(info))
    }
}
