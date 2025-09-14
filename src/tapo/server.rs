use crate::device::Device;
use crate::tapo::server::rpc::{
    DeviceRequest, DevicesResponse, Empty, EventRequest, EventResponse, InfoJsonResponse,
    InfoResponse, PowerResponse, SetRequest, UsageResponse,
};
use crate::tapo::state::State;
use crate::tapo::TapoRpcColorExt;
use futures::future::join_all;
use rpc::tapo_server::Tapo;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockWriteGuard};
use tonic::codegen::tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use super::{TapoDeviceExt, TapoSessionStatusExt};

pub mod rpc {
    tonic::include_proto!("tapo");
}

pub type EventSender = tokio::sync::broadcast::Sender<EventResponse>;
pub type EventReceiver = tokio::sync::broadcast::Receiver<EventResponse>;
pub type EventChannel = (EventSender, EventReceiver);

#[derive(Clone)]
pub struct TapoService {
    devices: Arc<HashMap<String, Device>>,
    state: Arc<RwLock<State>>,
    channel: Arc<EventChannel>,
}

impl TapoService {
    pub fn new(devices: HashMap<String, Arc<RwLock<Device>>>, channel: EventChannel) -> Self {
        Self {
            devices: Arc::new(devices),
            state: Arc::new(RwLock::new(State::new(channel.0.clone()))),
            channel: Arc::new(channel),
        }
    }

    async fn get_device_by_name(&self, name: &String) -> Result<Arc<RwLock<Device>>, Status> {
        match self.devices.get(name) {
            Some(dev) => Ok(dev.clone()),
            None => Err(Status::not_found(format!(
                "Device '{name}' could not be found"
            ))),
        }
    }

    async fn get_state_mut(&self) -> RwLockWriteGuard<'_, State> {
        self.state.write().await
    }
}

#[tonic::async_trait]
impl Tapo for TapoService {
    /// Get a list of all devices available on the server
    async fn devices(&self, _: Request<Empty>) -> Result<Response<DevicesResponse>, Status> {
        let map_async = self
            .devices
            .values()
            .map(|dev| dev.read())
            .collect::<Vec<_>>();
        let devices = join_all(map_async)
            .await
            .into_iter()
            .map(|dev| rpc::Device {
                name: dev.name.clone(),
                r#type: dev.device_type.to_string(),
                address: dev.address.clone(),
                status: dev.session_status.rpc().into(),
            })
            .collect::<Vec<_>>();

        Ok(Response::new(DevicesResponse { devices }))
    }

    type EventsStream = ReceiverStream<Result<EventResponse, Status>>;

    /// Subscribe to server events
    async fn events(
        &self,
        request: Request<EventRequest>,
    ) -> Result<Response<Self::EventsStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        let types = request.into_inner().types;
        let broadcast = self.channel.clone();
        let mut receiver = broadcast.1.resubscribe();

        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(event) => {
                        if (types.contains(&event.r#type) || types.is_empty())
                            && tx.send(Ok(event)).await.is_err()
                        {
                            return;
                        }
                    }
                    Err(_) => {
                        tx.send(Err(Status::internal("Error whilst receiving event")))
                            .await
                            .unwrap();
                        return;
                    }
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    /// Reset the device to it's factory defaults
    async fn reset(&self, request: Request<DeviceRequest>) -> Result<Response<Empty>, Status> {
        let inner = request.into_inner();
        let device = self.get_device_by_name(&inner.device).await?;
        let mut device = device.write().await;

        device.try_refresh_session().await?;
        device.reset().await
    }

    /// Get some selected information about the device
    async fn info(
        &self,
        request: Request<DeviceRequest>,
    ) -> Result<Response<InfoResponse>, Status> {
        let inner = request.into_inner();
        let device = self.get_device_by_name(&inner.device).await?;
        let mut device = device.write().await;

        device.try_refresh_session().await?;
        device.get_info().await
    }

    /// Get all raw json information about the device
    async fn info_json(
        &self,
        request: Request<DeviceRequest>,
    ) -> Result<Response<InfoJsonResponse>, Status> {
        let inner = request.into_inner();
        let device = self.get_device_by_name(&inner.device).await?;
        let mut device = device.write().await;

        device.try_refresh_session().await?;
        device.get_info_json().await
    }

    /// Get power and time usage of the device
    async fn usage(
        &self,
        request: Request<DeviceRequest>,
    ) -> Result<Response<UsageResponse>, Status> {
        let inner = request.into_inner();
        let device = self.get_device_by_name(&inner.device).await?;
        let mut device = device.write().await;

        device.try_refresh_session().await?;
        device.get_usage().await
    }

    /// Power the device on
    async fn on(&self, request: Request<DeviceRequest>) -> Result<Response<PowerResponse>, Status> {
        let inner = request.into_inner();
        let device = self.get_device_by_name(&inner.device).await?;
        let mut device = device.write().await;

        device.try_refresh_session().await?;
        let response = device.on().await?;

        let mut info = self.get_state_mut().await.get_info(&device).await?;
        info.device_on = Some(true);
        info.on_time = Some(0);
        self.get_state_mut()
            .await
            .update_info_optimistically(inner.device, info);

        Ok(response)
    }

    /// Power the device off
    async fn off(
        &self,
        request: Request<DeviceRequest>,
    ) -> Result<Response<PowerResponse>, Status> {
        let inner = request.into_inner();
        let device = self.get_device_by_name(&inner.device).await?;
        let mut device = device.write().await;

        device.try_refresh_session().await?;
        let response = device.off().await?;

        let mut info = self.get_state_mut().await.get_info(&device).await?;
        info.device_on = Some(false);
        info.on_time = Some(0);
        self.get_state_mut()
            .await
            .update_info_optimistically(inner.device, info);

        Ok(response)
    }

    /// Update one or more properties of a device in a single request
    async fn set(&self, request: Request<SetRequest>) -> Result<Response<InfoResponse>, Status> {
        let inner = request.into_inner();
        let device = self.get_device_by_name(&inner.device).await?;
        let mut device = device.write().await;
        device.try_refresh_session().await?;

        let mut info = self.get_state_mut().await.get_info_silent(&device).await?;

        let mut temperature = inner
            .temperature
            .map(|change| {
                let temperature = if change.absolute {
                    change.value as u16
                } else {
                    let updated = info.temperature() as i32 + change.value;
                    if updated.is_negative() {
                        2500u16
                    } else if updated >= u16::MAX.into() {
                        6500u16
                    } else {
                        updated as u16
                    }
                };
                info.temperature = Some(temperature as u32);
                temperature
            })
            .map(|value| value.clamp(2500, 6500));

        let brightness = inner
            .brightness
            .map(|change| {
                let brightness = if change.absolute {
                    change.value as u8
                } else {
                    let updated = info.brightness() as i32 + change.value;
                    if updated.is_negative() {
                        1u8
                    } else if updated >= u8::MAX.into() {
                        100u8
                    } else {
                        updated as u8
                    }
                };
                info.brightness = Some(brightness as u32);
                brightness
            })
            .map(|value| value.clamp(1, 100));

        let mut hue_saturation = inner
            .hue_saturation
            .map(|hs| {
                let saturation = hs
                    .saturation
                    .map(|change| {
                        let saturation = if change.absolute {
                            change.value as u8
                        } else {
                            let updated = info.saturation() as i32 + change.value;
                            if updated.is_negative() {
                                1u8
                            } else if updated >= u8::MAX.into() {
                                100u8
                            } else {
                                updated as u8
                            }
                        };
                        info.saturation = Some(saturation as u32);
                        saturation
                    })
                    .map(|value| value.clamp(1, 100));

                let hue = hs.hue.map(|change| {
                    let hue = if change.absolute {
                        change.value as u16
                    } else {
                        let updated = info.saturation() as i32 + change.value;
                        if updated.is_negative() {
                            (360 + (updated % 360)) as u16
                        } else {
                            (updated % 360) as u16
                        }
                    };
                    info.hue = Some(hue as u32);
                    hue
                });
                hue.zip(saturation)
            })
            .unwrap_or_default();

        let color = inner
            .color
            .and_then(|c| rpc::Color::try_from(c).map(|c| c.tapo_color()).ok());

        // the provided color always takes predecence over hue, saturation and
        // temperature arguments
        if let Some(color) = &color {
            let (h, s, t) = color.get_color_config();
            if h > 0 {
                temperature = None;
                hue_saturation = Some((h, s));
                info.hue = Some(h as u32);
                info.saturation = Some(s as u32);
                info.temperature = None;
            } else {
                temperature = Some(t);
                hue_saturation = None;
                info.hue = None;
                info.saturation = None;
                info.temperature = Some(t as u32);
            }
        }

        let power = inner.power;

        if color.is_some()
            || hue_saturation.is_some()
            || temperature.is_some()
            || brightness.is_some()
            || power.is_some_and(|v| v)
        {
            info.on_time = info.on_time.or(Some(0));
            info.device_on = Some(true);
        } else if power.is_some_and(|v| !v) {
            info.on_time = None;
            info.device_on = Some(false);
        }

        let response = device
            .set(info, power, brightness, temperature, hue_saturation)
            .await?;
        self.get_state_mut()
            .await
            .update_info_optimistically(device.name.clone(), response.get_ref().clone());
        Ok(response)
    }
}
