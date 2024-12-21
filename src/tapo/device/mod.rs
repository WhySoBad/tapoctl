use tonic::Response;

use crate::device::{Device, DeviceHandler};

use super::server::rpc::{Empty, InfoJsonResponse, InfoResponse, PowerResponse, UsageResponse};
use super::{TapoDeviceExt, TapoDeviceHandlerExt};

pub mod color_light;
pub mod light;
pub mod generic;

impl TapoDeviceExt for Device {
    async fn reset(&self) -> Result<Response<Empty>, tonic::Status> {
        match self.get_handler()? {
            DeviceHandler::ColorLight(handler) => handler.reset(self).await,
            DeviceHandler::Light(handler) => handler.reset(self).await,
            DeviceHandler::Generic(handler) => handler.reset(self).await,
        }.map(|_| Response::new(Empty {}))
    }

    async fn get_info(&self) -> Result<Response<InfoResponse>, tonic::Status> {
        match self.get_handler()? {
            DeviceHandler::ColorLight(handler) => handler.get_info(self).await,
            DeviceHandler::Light(handler) => handler.get_info(self).await,
            DeviceHandler::Generic(handler) => handler.get_info(self).await,
        }.map(Response::new)
    }

    async fn get_info_json(&self) -> Result<Response<InfoJsonResponse>, tonic::Status> {
        match self.get_handler()? {
            DeviceHandler::ColorLight(handler) => handler.get_info_json(self).await,
            DeviceHandler::Light(handler) => handler.get_info_json(self).await,
            DeviceHandler::Generic(handler) => handler.get_info_json(self).await,
        }.map(Response::new)
    }

    async fn get_usage(&self) -> Result<Response<UsageResponse>, tonic::Status> {
        match self.get_handler()? {
            DeviceHandler::ColorLight(handler) => handler.get_usage(self).await,
            DeviceHandler::Light(handler) => handler.get_usage(self).await,
            DeviceHandler::Generic(handler) => handler.get_usage(self).await,
        }.map(Response::new)
    }

    async fn on(&self) -> Result<Response<PowerResponse>, tonic::Status> {
        match self.get_handler()? {
            DeviceHandler::ColorLight(handler) => handler.power_on(self).await,
            DeviceHandler::Light(handler) => handler.power_on(self).await,
            DeviceHandler::Generic(handler) => handler.power_on(self).await,
        }.map(Response::new)
    }

    async fn off(&self) -> Result<Response<PowerResponse>, tonic::Status> {
        match self.get_handler()? {
            DeviceHandler::ColorLight(handler) => handler.power_off(self).await,
            DeviceHandler::Light(handler) => handler.power_off(self).await,
            DeviceHandler::Generic(handler) => handler.power_off(self).await,
        }.map(Response::new)
    }

    async fn set(
        &self,
        mut info: InfoResponse,
        power: Option<bool>,
        brightness: Option<u8>,
        temperature: Option<u16>,
        hue_saturation: Option<(u16, u8)>
    ) -> Result<Response<InfoResponse>, tonic::Status> {
        match self.get_handler()? {
            DeviceHandler::ColorLight(handler) =>
                handler.update(self, power, brightness, temperature, hue_saturation).await,
            DeviceHandler::Light(handler) => {
                info.hue = None;
                info.saturation = None;
                info.temperature = None;
                handler.update(self, power, brightness, temperature, hue_saturation).await
            },
            DeviceHandler::Generic(handler) => {
                info.hue = None;
                info.saturation = None;
                info.temperature = None;
                info.brightness = None;
                handler.update(self, power, brightness, temperature, hue_saturation).await
            }
        }?;

        Ok(Response::new(info))
    }
}