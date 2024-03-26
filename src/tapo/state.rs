use std::collections::HashMap;
use std::time::SystemTime;
use tonic::Status;
use crate::device::{Device, DeviceHandler};
use crate::tapo::color::any_to_rgb;
use crate::tapo::server::rpc::InfoResponse;

const INFO_VALIDITY_SECS: u64 = 30;

#[derive(Clone)]
pub struct State {
    info: HashMap<String, DeviceInfo>,
}

#[derive(Clone)]
pub struct DeviceInfo {
    response: InfoResponse,
    created: SystemTime
}

impl State {
    pub fn new() -> Self {
        State { info: HashMap::new() }
    }

    pub fn update_info_optimistically(&mut self, device: String, info: InfoResponse) {
        let device_info = DeviceInfo {
            created: SystemTime::now(),
            response: info
        };
        self.info.insert(device, device_info);
    }

    pub async fn refresh_info(&mut self, device: &Device) -> Result<InfoResponse, Status> {
        match &device.handler {
            DeviceHandler::Light(handler) => {
                let info = handler.get_device_info().await.map_err(|err| Status::internal(err.to_string()))?;
                Ok(InfoResponse {
                    brightness: Some(info.brightness as u32),
                    device_on: Some(info.device_on),
                    on_time: info.on_time,
                    overheated: info.overheated,
                    ..InfoResponse::default()
                })
            }
            DeviceHandler::Generic(handler) => {
                let info = handler.get_device_info().await.map_err(|err| Status::internal(err.to_string()))?;
                Ok(InfoResponse {
                    device_on: info.device_on,
                    on_time: info.on_time,
                    overheated: info.overheated,
                    ..InfoResponse::default()
                })
            }
            DeviceHandler::ColorLight(handler) => {
                let info = handler.get_device_info().await.map_err(|err| Status::internal(err.to_string()))?;
                let brightness = Some(info.brightness as u32);
                let hue = info.hue.map(|v| v as u32);
                let saturation = info.saturation.map(|v| v as u32);
                let temperature = Some(info.color_temp as u32);
                Ok(InfoResponse {
                    brightness,
                    hue,
                    saturation,
                    temperature,
                    device_on: Some(info.device_on),
                    on_time: info.on_time,
                    dynamic_effect_id: info.dynamic_light_effect_id,
                    overheated: info.overheated,
                    color: any_to_rgb(temperature, hue, saturation, brightness),
                })
            }
        }
    }

    pub async fn get_info(&mut self, device: &Device) -> Result<InfoResponse, Status> {
        let info = self.info.get(&device.name);

        let now = SystemTime::now();
        if let Some(info) = info {
            if now.duration_since(info.created).is_ok_and(|dur| dur.as_secs() < INFO_VALIDITY_SECS) {
                // info is still valid
                let mut copy = info.response.clone();
                copy.on_time = copy.on_time.map(|time| time + now.duration_since(info.created).unwrap_or_default().as_secs());
                return Ok(copy)
            };
        };

        // get refreshed device info from device handler
        let response = self.refresh_info(device).await?;
        self.info.insert(device.name.clone(), DeviceInfo { response: response.clone(), created: now });
        Ok(response)
    }
}