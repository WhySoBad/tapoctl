use std::cmp::{max, min};
use std::collections::HashMap;
use std::sync::Arc;
use futures::future::join_all;
use tokio::sync::{Mutex, MutexGuard};
use tonic::{Request, Response, Status};
use tonic::codegen::tokio_stream::wrappers::ReceiverStream;
use rpc::tapo_server::Tapo;
use crate::tapo::server::rpc::{DeviceRequest, DevicesResponse, Empty, EventRequest, EventResponse, InfoJsonResponse, InfoResponse, IntegerValueChange, PowerResponse, SetRequest, UsagePerPeriod, UsageResponse};
use crate::device;
use crate::device::Device;
use crate::tapo::{TapoErrMap, transform_color, transform_session_status};
use crate::tapo::color::{any_to_rgb, color_to_hst};
use crate::tapo::state::State;

pub mod rpc {
    tonic::include_proto!("tapo");
}

pub type EventSender = tokio::sync::broadcast::Sender<EventResponse>;
pub type EventReceiver = tokio::sync::broadcast::Receiver<EventResponse>;
pub type EventChannel = (EventSender, EventReceiver);

#[derive(Clone)]
pub struct TapoService {
    devices: Arc<HashMap<String, Arc<Mutex<Device>>>>,
    state: Arc<Mutex<State>>,
    channel: Arc<EventChannel>
}

impl TapoService {
    pub fn new(devices: HashMap<String, Arc<Mutex<Device>>>, channel: EventChannel) -> Self {
        Self {
            devices: Arc::new(devices),
            state: Arc::new(Mutex::new(State::new(channel.0.clone()))),
            channel: Arc::new(channel)
        }
    }

    async fn get_device_by_name(&self, name: &String) -> Result<Arc<Mutex<Device>>, Status> {
        match self.devices.get(name) {
            Some(dev) => Ok(dev.clone()),
            None => Err(Status::not_found(format!("Device '{name}' could not be found")))
        }
    }

    async fn get_state(&self) -> MutexGuard<'_, State> {
        self.state.lock().await
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

    type EventsStream = ReceiverStream<Result<EventResponse, Status>>;

    async fn events(&self, request: Request<EventRequest>) -> Result<Response<Self::EventsStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        let types = request.into_inner().types;
        let broadcast = self.channel.clone();
        let mut receiver = broadcast.1.resubscribe();

        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(event) => {
                        if types.contains(&event.r#type) || types.is_empty() {
                            match tx.send(Ok(event)).await {
                                Ok(_) => {},
                                // the stream was closed
                                Err(_) => return
                            }
                        }
                    },
                    Err(_) => {
                        tx.send(Err(Status::internal("Error whilst receiving event"))).await.unwrap();
                        return
                    },
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn reset(&self, request: Request<DeviceRequest>) -> Result<Response<Empty>, Status> {
        let inner = request.into_inner();
        let device = self.get_device_by_name(&inner.device).await?;
        let mut device = device.lock().await;
        device.try_refresh_session().await?;

        match device.get_handler()? {
            device::DeviceHandler::Light(handler) => {
                handler.device_reset().await.map_tapo_err(&mut device).await?;
            }
            device::DeviceHandler::ColorLight(handler) => {
               handler.device_reset().await.map_tapo_err(&mut device).await?;
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


        let response = match device.get_handler()? {
            device::DeviceHandler::Light(handler) => {
                let info = handler.get_device_info().await.map_tapo_err(&mut device).await?;
                InfoResponse {
                    brightness: Some(info.brightness as u32),
                    device_on: Some(info.device_on),
                    on_time: info.on_time,
                    name: device.name.clone(),
                    overheated: info.overheated,
                    ..InfoResponse::default()
                }
            }
            device::DeviceHandler::Generic(handler) => {
                let info = handler.get_device_info().await.map_tapo_err(&mut device).await?;
                InfoResponse {
                    device_on: info.device_on,
                    on_time: info.on_time,
                    name: device.name.clone(),
                    ..InfoResponse::default()
                }
            }
            device::DeviceHandler::ColorLight(handler) => {
                let info = handler.get_device_info().await.map_tapo_err(&mut device).await?;
                let brightness = Some(info.brightness as u32);
                let hue = info.hue.map(|v| v as u32);
                let saturation = info.saturation.map(|v| v as u32);
                let temperature = Some(info.color_temp as u32);
                InfoResponse {
                    brightness,
                    hue,
                    saturation,
                    temperature,
                    device_on: Some(info.device_on),
                    on_time: info.on_time,
                    dynamic_effect_id: info.dynamic_light_effect_id,
                    overheated: info.overheated,
                    name: device.name.clone(),
                    color: any_to_rgb(temperature, hue, saturation, brightness),
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

        let info = match device.get_handler()? {
            device::DeviceHandler::Light(handler) => {
                handler.get_device_info_json().await.map_tapo_err(&mut device).await?
            }
            device::DeviceHandler::Generic(handler) => {
                handler.get_device_info_json().await.map_tapo_err(&mut device).await?
            }
            device::DeviceHandler::ColorLight(handler) => {
                handler.get_device_info_json().await.map_tapo_err(&mut device).await?
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

        let usage = match device.get_handler()? {
            device::DeviceHandler::Light(handler) => {
                handler.get_device_usage().await.map_tapo_err(&mut device).await?
            }
            device::DeviceHandler::ColorLight(handler) => {
                handler.get_device_usage().await.map_tapo_err(&mut device).await?
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

        match device.get_handler()? {
            device::DeviceHandler::Light(handler) => {
                handler.on().await.map_tapo_err(&mut device).await?
            }
            device::DeviceHandler::Generic(handler) => {
                handler.on().await.map_tapo_err(&mut device).await?
            }
            device::DeviceHandler::ColorLight(handler) => {
                handler.on().await.map_tapo_err(&mut device).await?
            }
        }

        let mut info = self.get_state().await.get_info(&device).await?;
        info.device_on = Some(true);
        info.on_time = Some(0);
        self.get_state().await.update_info_optimistically(inner.device, info);

        Ok(Response::new(PowerResponse { device_on: true }))
    }

    async fn off(&self, request: Request<DeviceRequest>) -> Result<Response<PowerResponse>, Status> {
        let inner = request.into_inner();
        let device = self.get_device_by_name(&inner.device).await?;
        let mut device = device.lock().await;
        device.try_refresh_session().await?;

        match device.get_handler()? {
            device::DeviceHandler::Light(handler) => {
                handler.off().await.map_tapo_err(&mut device).await?
            }
            device::DeviceHandler::Generic(handler) => {
                handler.off().await.map_tapo_err(&mut device).await?
            }
            device::DeviceHandler::ColorLight(handler) => {
                handler.off().await.map_tapo_err(&mut device).await?
            }
        }

        let mut info = self.get_state().await.get_info(&device).await?;
        info.device_on = Some(false);
        info.on_time = Some(0);
        self.get_state().await.update_info_optimistically(inner.device, info);

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
        let temperature = inner.temperature.map(&mut check_for_relative);
        let brightness = inner.brightness.map(&mut check_for_relative);
        let (hue, saturation) = inner.hue_saturation.map(|hs| {
            (hs.hue.map(&mut check_for_relative), hs.saturation.map(check_for_relative))
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

        fn get_transformed_brightness(info: &InfoResponse, change: IntegerValueChange) -> u8 {
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
            min(max(current, 1), 100)
        }

        match device.get_handler()? {
            device::DeviceHandler::ColorLight(handler) => {
                let mut info = self.get_state().await.get_info_silent(&device).await?;

                let mut set = handler.set();
                if let Some(change) = brightness {
                    let current = get_transformed_brightness(&info, change);
                    set = set.brightness(current);
                    info.brightness = Some(current as u32);
                    info.device_on = Some(true);
                    info.on_time = info.on_time.or(Some(0));
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
                        if change_saturation.value.is_negative() {
                            current_saturation -= min(change, current_saturation - 1);
                        } else {
                            current_saturation += min(change, 100 - current_saturation);
                        }
                    }
                    current_hue = min(max(current_hue, 1), 360);
                    set = set.hue_saturation(current_hue, current_saturation);
                    info.hue = Some(current_hue as u32);
                    info.saturation = Some(current_saturation as u32);
                    info.device_on = Some(true);
                    info.temperature = Some(0);
                    info.on_time = info.on_time.or(Some(0));
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
                    info.hue = None;
                    info.saturation = None;
                    info.device_on = Some(true);
                    info.on_time = info.on_time.or(Some(0));
                }
                if let Some(color) = color {
                    set = set.color(color.clone());
                    info.device_on = Some(true);
                    let (hue, saturation, temperature) = color_to_hst(color);
                    if hue > 0 {
                        info.hue = Some(hue);
                        info.saturation = Some(saturation);
                    } else {
                        info.hue = None;
                        info.saturation = None;
                    }
                    info.temperature = Some(temperature);
                };
                if let Some(power) = inner.power {
                    if power {
                        set = set.on();
                        info.device_on = Some(true);
                        info.on_time = info.on_time.or(Some(0));
                    } else {
                        set = set.off();
                        info.device_on = Some(false);
                        info.on_time = None;
                    }
                }
                info.color = any_to_rgb(info.temperature, info.hue, info.saturation, info.brightness);
                set.send(handler).await.map_tapo_err(&mut device).await?;
                self.get_state().await.update_info_optimistically(device.name.clone(), info.clone());
                Ok(Response::new(info))
            }
            // LightDevice only supports brightness and power operations
            device::DeviceHandler::Light(handler) => {
                let mut info = self.get_state().await.get_info_silent(&device).await?;
                // since `map_tapo_err` needs mutable reference on device we can call it only once
                // therefore we store the result of
                let mut err: Option<tapo::Error> = None;

                if let Some(change) = brightness {
                    let current = get_transformed_brightness(&info.clone(), change);
                    match handler.set_brightness(current).await {
                        Ok(_) => {
                            info.brightness = Some(current as u32);
                            info.device_on = Some(true);
                            info.on_time = info.on_time.or(Some(0));
                        },
                        Err(error) => {
                            err = err.or(Some(error));
                        }
                    }
                };
                if let Some(power) = inner.power {
                    if power {
                        match handler.on().await {
                            Ok(_) => {
                                info.device_on = Some(true);
                                info.on_time = info.on_time.or(Some(0));
                            },
                            Err(error) => {
                                err = err.or(Some(error));
                            }
                        }
                    } else {
                        match handler.off().await {
                            Ok(_) => {
                                info.device_on = Some(false);
                                info.on_time = info.on_time.or(Some(0));
                            },
                            Err(error) => {
                                err = err.or(Some(error));
                            }
                        }
                    }
                }

                self.get_state().await.update_info_optimistically(device.name.clone(), info.clone());

                if let Some(err) = err {
                    Err(err).map_tapo_err(&mut device).await?;
                };

                Ok(Response::new(info))
            },
            // GenericDevice only supports power operations
            device::DeviceHandler::Generic(handler) => {
                let mut info = self.get_state().await.get_info_silent(&device).await?;
                if let Some(power) = inner.power {
                    if power {
                        handler.on().await.map_tapo_err(&mut device).await?;
                        info.device_on = Some(true);
                        info.on_time = info.on_time.or(Some(0));
                    } else {
                        handler.off().await.map_tapo_err(&mut device).await?;
                        info.device_on = Some(false);
                        info.on_time = None;
                    }
                }

                self.get_state().await.update_info_optimistically(device.name.clone(), info.clone());
                Ok(Response::new(info))
            },
        }
    }
}