use std::cmp::{max, min};
use std::collections::HashMap;
use std::sync::Arc;
use futures::future::join_all;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};
use rpc::tapo_server::Tapo;
use crate::tapo::server::rpc::{DeviceRequest, DevicesResponse, Empty, InfoJsonResponse, InfoResponse, IntegerValueChange, PowerResponse, SetRequest, UsagePerPeriod, UsageResponse};
use crate::device;
use crate::device::Device;
use crate::tapo::{transform_color, transform_session_status};

pub mod rpc {
    tonic::include_proto!("tapo");
}

#[derive(Clone)]
pub struct TapoService {
    devices: Arc<HashMap<String, Arc<Mutex<Device>>>>
}

impl TapoService {
    pub fn new(devices: HashMap<String, Arc<Mutex<Device>>>) -> Self {
        Self { devices: Arc::new(devices) }
    }

    async fn get_device_by_name(&self, name: &String) -> Result<Arc<Mutex<Device>>, Status> {
        match self.devices.get(name) {
            Some(dev) => Ok(dev.clone()),
            None => Err(Status::not_found(format!("Device '{name}' could not be found")))
        }
    }
}

#[tonic::async_trait]
impl Tapo for TapoService {
    async fn devices(&self, _: Request<Empty>) -> Result<Response<DevicesResponse>, Status> {

        let map_async = self.devices.values().map(|dev| dev.lock()).collect::<Vec<_>>();
        let devices = join_all(map_async).await.into_iter()
            .map(|dev| {
                rpc::Device {
                    name: dev.name.clone(),
                    r#type: format!("{:?}", dev.r#type),
                    address: dev.address.clone(),
                    status: i32::from(transform_session_status(&dev.session_status))
                }
            })
            .collect::<Vec<_>>();

        let response = DevicesResponse {
            devices
        };

        Ok(Response::new(response))
    }

    async fn reset(&self, request: Request<DeviceRequest>) -> Result<Response<Empty>, Status> {
        let inner = request.into_inner();
        let device = self.get_device_by_name(&inner.device).await?;
        let mut device = device.lock().await;
        device.try_refresh_session().await?;

        match &device.handler {
            device::DeviceHandler::Light(handler) => {
                handler.device_reset().await.map_err(|err| Status::internal(err.to_string()))?;
            }
            device::DeviceHandler::ColorLight(handler) => {
               handler.device_reset().await.map_err(|err| Status::internal(err.to_string()))?;
            },
            _ => {
                return Err(Status::unimplemented("Reset API is not supported by this device type"))
            }
        }

        Ok(Response::new(Empty {}))
    }

    async fn info(&self, request: Request<DeviceRequest>) -> Result<Response<InfoResponse>, Status> {
        let inner = request.into_inner();
        let device = self.get_device_by_name(&inner.device).await?;
        let mut device = device.lock().await;
        device.try_refresh_session().await?;


        let response = match &device.handler {
            device::DeviceHandler::Light(handler) => {
                let info = handler.get_device_info().await.map_err(|err| Status::internal(err.to_string()))?;
                InfoResponse {
                    brightness: Some(info.brightness as u32),
                    device_on: Some(info.device_on),
                    on_time: info.on_time,
                    overheated: info.overheated,
                    ..InfoResponse::default()
                }
            }
            device::DeviceHandler::Generic(handler) => {
                let info = handler.get_device_info().await.map_err(|err| Status::internal(err.to_string()))?;
                InfoResponse {
                    device_on: info.device_on,
                    on_time: info.on_time,
                    overheated: info.overheated,
                    ..InfoResponse::default()
                }
            }
            device::DeviceHandler::ColorLight(handler) => {
                let info = handler.get_device_info().await.map_err(|err| Status::internal(err.to_string()))?;
                InfoResponse {
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
        };

        Ok(Response::new(response))
    }

    async fn info_json(&self, request: Request<DeviceRequest>) -> Result<Response<InfoJsonResponse>, Status> {
        let inner = request.into_inner();
        let device = self.get_device_by_name(&inner.device).await?;
        let mut device = device.lock().await;
        device.try_refresh_session().await?;

        let info = match &device.handler {
            device::DeviceHandler::Light(handler) => {
                handler.get_device_info_json().await.map_err(|err| Status::internal(err.to_string()))?
            }
            device::DeviceHandler::Generic(handler) => {
                handler.get_device_info_json().await.map_err(|err| Status::internal(err.to_string()))?
            }
            device::DeviceHandler::ColorLight(handler) => {
                handler.get_device_info_json().await.map_err(|err| Status::internal(err.to_string()))?
            }
        };

        let mut bytes = vec![];
        serde_json::to_writer(&mut bytes, &info).unwrap_or_default();

        let response = InfoJsonResponse { data: bytes };

        Ok(Response::new(response))
    }

    async fn usage(&self, request: Request<DeviceRequest>) -> Result<Response<UsageResponse>, Status> {
        let inner = request.into_inner();
        let device = self.get_device_by_name(&inner.device).await?;
        let mut device = device.lock().await;
        device.try_refresh_session().await?;

        let usage = match &device.handler {
            device::DeviceHandler::Light(handler) => {
                handler.get_device_usage().await.map_err(|err| Status::internal(err.to_string()))?
            }
            device::DeviceHandler::ColorLight(handler) => {
                handler.get_device_usage().await.map_err(|err| Status::internal(err.to_string()))?
            },
            device::DeviceHandler::Generic(_) => {
                return Err(Status::unimplemented("Device usage API is not supported by this device type"))
            }
        };

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

        let response = UsageResponse {
            power_usage: Some(power_usage),
            time_usage: Some(time_usage),
            saved_power: Some(saved_power)
        };
        
        Ok(Response::new(response))
    }

    async fn on(&self, request: Request<DeviceRequest>) -> Result<Response<PowerResponse>, Status> {
        let inner = request.into_inner();
        let device = self.get_device_by_name(&inner.device).await?;
        let mut device = device.lock().await;
        device.try_refresh_session().await?;

        match &device.handler {
            device::DeviceHandler::Light(handler) => {
                handler.on().await.map_err(|err| Status::internal(err.to_string()))?
            }
            device::DeviceHandler::Generic(handler) => {
                handler.on().await.map_err(|err| Status::internal(err.to_string()))?
            }
            device::DeviceHandler::ColorLight(handler) => {
                handler.on().await.map_err(|err| Status::internal(err.to_string()))?
            }
        }

        Ok(Response::new(PowerResponse { device_on: true }))
    }

    async fn off(&self, request: Request<DeviceRequest>) -> Result<Response<PowerResponse>, Status> {
        let inner = request.into_inner();
        let device = self.get_device_by_name(&inner.device).await?;
        let mut device = device.lock().await;
        device.try_refresh_session().await?;

        match &device.handler {
            device::DeviceHandler::Light(handler) => {
                handler.on().await.map_err(|err| Status::internal(err.to_string()))?
            }
            device::DeviceHandler::Generic(handler) => {
                handler.on().await.map_err(|err| Status::internal(err.to_string()))?
            }
            device::DeviceHandler::ColorLight(handler) => {
                handler.off().await.map_err(|err| Status::internal(err.to_string()))?
            }
        }

        Ok(Response::new(PowerResponse { device_on: false }))
    }

    async fn set(&self, request: Request<SetRequest>) -> Result<Response<InfoResponse>, Status> {
        let inner = request.into_inner();
        let device = self.get_device_by_name(&inner.device).await?;
        let mut device = device.lock().await;
        device.try_refresh_session().await?;

        let mut has_relative_change = false;
        let mut check_for_relative = |v: IntegerValueChange| {
            has_relative_change = !v.absolute || has_relative_change;
            v
        };

        let color = inner.color.map(|_| transform_color(inner.color()));
        let temperature = inner.temperature.map(|v| check_for_relative(v));
        let brightness = inner.brightness.map(|v| check_for_relative(v));
        let (hue, saturation) = inner.hue_saturation.map(|hs| {
            (hs.hue.map(|v| check_for_relative(v)), hs.saturation.map(|v| check_for_relative(v)))
        }).unwrap_or((None, None));

        if let Some(change) = &temperature {
            if change.absolute && !(2500..=6500).contains(&change.value) {
                Err(Status::invalid_argument("Temperature has to be in range 2500 to 6500"))?
            }
        }

        if let Some(change) = &hue {
            if change.absolute && !(1..=360).contains(&change.value) {
                Err(Status::invalid_argument("Hue has to be in range 1 to 360"))?
            }
        }

        if let Some(change) = &saturation {
            if change.absolute && !(1..=100).contains(&change.value) {
                Err(Status::invalid_argument("Saturation has to be in range 1 to 100"))?
            }
        }

        if let Some(change) = &brightness {
            if change.absolute && !(1..=100).contains(&change.value) {
                Err(Status::invalid_argument("Brightness has to be in range 1 to 100"))?
            }
        }

        match &device.handler {
            device::DeviceHandler::ColorLight(handler) => {
                let info = handler.get_device_info().await.map_err(|err| Status::internal(err.to_string()))?;
                let mut info = InfoResponse {
                    brightness: Some(info.brightness as u32),
                    device_on: Some(info.device_on),
                    hue: info.hue.map(|v| v as u32),
                    on_time: info.on_time,
                    dynamic_effect_id: info.dynamic_light_effect_id,
                    overheated: info.overheated,
                    saturation: info.saturation.map(|v| v as u32),
                    temperature: Some(info.color_temp as u32)
                };

                let mut set = handler.set();
                if let Some(change) = brightness {
                    let mut current = u8::try_from(info.brightness.unwrap_or_default()).unwrap_or_default();
                    if change.absolute {
                        current = u8::try_from(change.value).unwrap_or_default();
                    } else {
                        let change_abs = u8::try_from(max(min(change.value.abs(), 100), 1)).unwrap_or_default();
                        if change.value.is_negative() {
                            current -= change_abs;
                        } else {
                            current += change_abs;
                        }
                    }
                    current = min(max(current, 1), 100);
                    set = set.brightness(current);
                    info.brightness = Some(current as u32);
                    info.device_on = Some(true);
                };
                if let Some((change_hue, change_saturation)) = hue.zip(saturation) {
                    let mut current_hue = u16::try_from(info.hue.unwrap_or_default()).unwrap_or_default();
                    let mut current_saturation = u8::try_from(info.saturation.unwrap_or_default()).unwrap_or_default();
                    if change_hue.absolute {
                        current_hue = u16::try_from(change_hue.value).unwrap_or_default();
                    } else {
                        let change = u16::try_from((change_hue.value % 360).abs()).unwrap_or_default();
                        if change_hue.value.is_negative() {
                            current_hue -= change;
                        } else {
                            current_hue += change;
                        }
                        current_hue %= 360;
                        if current_hue == 0 {
                            current_hue = 360;
                        }
                    }
                    if change_saturation.absolute {
                        current_saturation = u8::try_from(change_saturation.value).unwrap_or_default();
                    } else {
                        let change = u8::try_from(max(min(change_saturation.value.abs(), 100), 1)).unwrap_or_default();
                        println!("{change} {current_saturation}");
                        if change_saturation.value.is_negative() {
                            current_saturation -= min(change, current_saturation - 1);
                        } else {
                            current_saturation += min(change, 100 - current_saturation);
                        }
                        println!("updated: {current_saturation}")
                    }
                    current_hue = min(max(current_hue, 1), 360);
                    set = set.hue_saturation(current_hue, current_saturation);
                    info.hue = Some(current_hue as u32);
                    info.saturation = Some(current_saturation as u32);
                    info.device_on = Some(true);
                };
                if let Some(change) = temperature {
                    let mut current = u16::try_from(info.temperature.unwrap_or_default()).unwrap_or_default();
                    if change.absolute {
                        current = u16::try_from(change.value).unwrap_or_default();
                    } else {
                        let change_abs = u16::try_from(max(min(change.value.abs(), 6500), 2500)).unwrap_or_default();
                        if change.value.is_negative() {
                            current -= change_abs;
                        } else {
                            current += change_abs;
                        }
                    }
                    current = min(max(current, 2500), 6500);
                    set = set.color_temperature(current);
                    info.temperature = Some(current as u32);
                    info.device_on = Some(true);
                }
                if let Some(color) = color {
                    set = set.color(color);
                    info.device_on = Some(true);
                    // TODO: Update info after color change
                };
                if let Some(power) = inner.power {
                    if power {
                        set = set.on();
                        info.device_on = Some(true);
                    } else {
                        set = set.off();
                        info.device_on = Some(false);
                    }
                }
                set.send(handler).await.map_err(|err| Status::internal(err.to_string()))?;
                Ok(Response::new(info))
            }
            device::DeviceHandler::Light(_handler) => {
                Err(Status::unimplemented("Set API is not yet implemented for this device type"))?;
                todo!("Send request for every sub-category to simulate set-api like behaviour")
            }
            _ => {
                Err(Status::unimplemented("Set API is not supported by this device type"))
            }
        }
    }
}