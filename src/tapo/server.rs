use std::collections::HashMap;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use rpc::tapo_server::Tapo;
use crate::tapo::server::rpc::{DeviceRequest, DevicesResponse, EmptyRequest, EmptyResponse, InfoJsonResponse, InfoResponse, SetRequest, UsagePerPeriod, UsageResponse};
use crate::device;
use crate::device::Device;
use crate::tapo::transform_color;

pub mod rpc {
    tonic::include_proto!("tapo");
}

#[derive(Clone)]
pub struct TapoService {
    devices: Arc<HashMap<String, Device>>
}

impl TapoService {
    pub fn new(devices: HashMap<String, Device>) -> Self {
        Self { devices: Arc::new(devices) }
    }

    fn get_device_by_name(&self, name: &String) -> Result<&Device, Status> {
        match self.devices.get(name) {
            Some(dev) => Ok(dev),
            None => Err(Status::not_found(format!("Device '{name}' could not be found")))
        }
    }
}

#[tonic::async_trait]
impl Tapo for TapoService {
    async fn devices(&self, _: Request<EmptyRequest>) -> Result<Response<DevicesResponse>, Status> {
        let response = DevicesResponse {
            devices: self.devices.values().map(|dev| rpc::Device { name: dev.name.clone(), address: dev.address.clone(), r#type: format!("{:?}", dev.r#type) }).collect()
        };

        Ok(Response::new(response))
    }

    async fn reset(&self, request: Request<DeviceRequest>) -> Result<Response<EmptyResponse>, Status> {
        let inner = request.into_inner();
        let device = self.get_device_by_name(&inner.device)?;

        match &device.handler {
            device::DeviceHandler::ColorLight(handler) => {
               handler.device_reset().await.map_err(|err| Status::internal(err.to_string()))?;
            },
        }

        Ok(Response::new(EmptyResponse {}))
    }

    async fn info(&self, request: Request<DeviceRequest>) -> Result<Response<InfoResponse>, Status> {
        let inner = request.into_inner();
        let device = self.get_device_by_name(&inner.device)?;

        let response;

        match &device.handler {
            device::DeviceHandler::ColorLight(handler) => {
                let info = handler.get_device_info().await.map_err(|err| Status::internal(err.to_string()))?;
                response = InfoResponse {
                    brightness: Some(info.brightness as u32),
                    device_on: Some(info.device_on),
                    hue: info.hue.map(|v| v as u32),
                    on_time: info.on_time,
                    dynamic_effect_id: info.dynamic_light_effect_id,
                    overheated: info.overheated,
                    saturation: info.saturation.map(|v| v as u32),
                    temperature: Some(info.color_temp as u32)
                }
            }
        }

        Ok(Response::new(response))
    }

    async fn info_json(&self, request: Request<DeviceRequest>) -> Result<Response<InfoJsonResponse>, Status> {
        let inner = request.into_inner();
        let device = self.get_device_by_name(&inner.device)?;

        let response;

        match &device.handler {
            device::DeviceHandler::ColorLight(handler) => {
                let info = handler.get_device_info_json().await.map_err(|err| Status::internal(err.to_string()))?;
                response = InfoJsonResponse {
                    data: info.to_string().into_bytes()
                }
            }
        }

        Ok(Response::new(response))
    }

    async fn usage(&self, request: Request<DeviceRequest>) -> Result<Response<UsageResponse>, Status> {
        let inner = request.into_inner();
        let device = self.get_device_by_name(&inner.device)?;

        let response;

        match &device.handler {
            device::DeviceHandler::ColorLight(handler) => {
                let usage = handler.get_device_usage().await.map_err(|err| Status::internal(err.to_string()))?;

                let power_usage = UsagePerPeriod {
                    today: usage.power_usage.today,
                    week: usage.power_usage.past7,
                    month: usage.power_usage.past30
                };

                let time_usage = UsagePerPeriod {
                    today: usage.time_usage.today,
                    week: usage.time_usage.past7,
                    month: usage.time_usage.past30
                };
                
                let saved_power = UsagePerPeriod {
                    today: usage.saved_power.today,
                    week: usage.saved_power.past7,
                    month: usage.saved_power.past30
                };

                response = UsageResponse {
                    power_usage: Some(power_usage),
                    time_usage: Some(time_usage),
                    saved_power: Some(saved_power)
                }
            }
        }
        
        Ok(Response::new(response))
    }

    async fn on(&self, request: Request<DeviceRequest>) -> Result<Response<EmptyResponse>, Status> {
        let inner = request.into_inner();
        let device = self.get_device_by_name(&inner.device)?;

        match &device.handler {
            device::DeviceHandler::ColorLight(handler) => {
                handler.on().await.map_err(|err| Status::internal(err.to_string()))?
            }
        }

        Ok(Response::new(EmptyResponse {}))
    }

    async fn off(&self, request: Request<DeviceRequest>) -> Result<Response<EmptyResponse>, Status> {
        let inner = request.into_inner();
        let device = self.get_device_by_name(&inner.device)?;

        match &device.handler {
            device::DeviceHandler::ColorLight(handler) => {
                handler.off().await.map_err(|err| Status::internal(err.to_string()))?
            }
        }

        Ok(Response::new(EmptyResponse {}))
    }

    async fn set(&self, request: Request<SetRequest>) -> Result<Response<EmptyResponse>, Status> {
        let inner = request.into_inner();
        let device = self.get_device_by_name(&inner.device)?;
        let color = inner.color.map(|_| transform_color(inner.color()));

        let hue_saturation: Option<Result<(u16, u8), Status>> = inner.hue_saturation.map(|hs| {
            let hue = u16::try_from(hs.hue).map_err(|_| Status::invalid_argument("Hue has to be unsigned 16-bit integer"))?;
            let saturation = u8::try_from(hs.saturation).map_err(|_| Status::invalid_argument("Saturation has to be unsigned 8-bit integer"))?;
            Ok((hue, saturation))
        });

        let hue_saturation = match hue_saturation {
            Some(res) => Some(res?),
            None => None
        };

        if let Some((hue, saturation)) = hue_saturation {
            if !(1..=360).contains(&hue) {
                Err(Status::invalid_argument("Hue has to be in range 1 to 360"))?

            }

            if !(1..=100).contains(&saturation) {
                Err(Status::invalid_argument("Saturation has to be in range 1 to 100"))?
            }
        }

        // let temperature = u16::try_from(inner.temperature).map_err(|_| Status::invalid_argument("Temperature has to be unsigned 16-bit integer"))?

        // if !(2500..=6500).contains(&temperature) {
        //     Err(Status::invalid_argument("Temperature has to be in range 2500 to 6500"))?
        // }
        // let brightness = u8::try_from(inner.brightness).map_err(|_| Status::invalid_argument("Brightness has to be unsigned 8-bit integer"))?;

        // if !(1..=100).contains(&brightness) {
        //    Err(Status::invalid_argument("Brightness has to be in range 1 to 100"))?
        // }
        todo!()
    }
}