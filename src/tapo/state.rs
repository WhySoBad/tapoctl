use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use log::{error, info};
use tonic::Status;
use crate::device::{Device, DeviceHandler};
use crate::tapo::color::any_to_rgb;
use crate::tapo::create_event;
use crate::tapo::server::EventSender;
use crate::tapo::server::rpc::{EventType, InfoResponse};

const INFO_VALIDITY_MILLIS: u64 = 30 * 1000; // update device info after 30 seconds

#[derive(Clone)]
pub struct State {
    info: HashMap<String, DeviceInfo>,
    sender: EventSender,
}

#[derive(Clone)]
pub struct DeviceInfo {
    response: InfoResponse,
    created: SystemTime
}

impl State {
    pub fn new(sender: EventSender) -> Self {
        State { info: HashMap::new(), sender }
    }

    /// Manually populate the cached state information for a device
    ///
    /// This is mainly used for the `set` endpoint where the new state is known without having to fetch it again
    pub fn update_info_optimistically(&mut self, device: String, info: InfoResponse) {
        if let Some(current) = self.info.get(&device) {
            // nothing to do: the info is already up-to-date
            if current.response.eq(&info) { return; }
        }

        info!("Sending new device state event 2");
        let event = create_event(EventType::DeviceStateChange, &info);

        let device_info = DeviceInfo {
            created: SystemTime::now(),
            response: info
        };
        self.info.insert(device, device_info);

        if let Err(err) = self.sender.send(event) {
            error!("Error whilst sending new device state: {err}")
        }
    }

    /// Refresh the cached state information for a device
    ///
    /// When `send_state` is set to `true` the refreshed info is sent as an update event to
    /// all subscribed clients. It should be set to `false` when the refresh is coming from
    /// a request which updates the state afterwards optimistically
    pub async fn refresh_info(&mut self, device: &Device, send_state: bool) -> Result<InfoResponse, Status> {
        let info = match device.get_handler()? {
            DeviceHandler::Light(handler) => {
                let info = handler.get_device_info().await.map_err(|err| Status::internal(err.to_string()))?;
                InfoResponse {
                    brightness: Some(info.brightness as u32),
                    device_on: Some(info.device_on),
                    on_time: info.on_time,
                    overheated: info.overheated,
                    name: device.name.clone(),
                    ..InfoResponse::default()
                }
            }
            DeviceHandler::Generic(handler) => {
                let info = handler.get_device_info().await.map_err(|err| Status::internal(err.to_string()))?;
                InfoResponse {
                    device_on: info.device_on,
                    on_time: info.on_time,
                    overheated: info.overheated,
                    name: device.name.clone(),
                    ..InfoResponse::default()
                }
            }
            DeviceHandler::ColorLight(handler) => {
                let info = handler.get_device_info().await.map_err(|err| Status::internal(err.to_string()))?;
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
                    color: any_to_rgb(temperature, hue, saturation, brightness),
                    name: device.name.clone()
                }
            }
        };

        if send_state {
            info!("Sending new device state event");
            match self.sender.send(create_event(EventType::DeviceStateChange, &info)) {
                Ok(_) => {},
                Err(err) => error!("Error whilst sending new device state: {err}")
            }
        }

        Ok(info)
    }

    /// Get the current state for a device
    ///
    /// The state may be cached and have a maximum age of [`INFO_VALIDITY_SECS`]. Should the state
    /// exceed the cache period it gets renewed automatically
    pub async fn get_info(&mut self, device: &Device) -> Result<InfoResponse, Status> {
        let info = self.info.get(&device.name);

        let now = SystemTime::now();
        if let Some(info) = info {
            if info.created + Duration::from_millis(INFO_VALIDITY_MILLIS) < now {
                // info is still valid
                let mut copy = info.response.clone();
                copy.on_time = copy.on_time.map(|time| time + now.duration_since(info.created).unwrap_or_default().as_secs());
                return Ok(copy)
            };
        };

        // get refreshed device info from device handler
        let response = self.refresh_info(device, true).await?;
        self.info.insert(device.name.clone(), DeviceInfo { response: response.clone(), created: now });
        Ok(response)
    }

    /// Get the current state for a device silently
    ///
    /// It's the same as [`self.get_info`] but it doesn't send an update state event
    /// should an expired state needs to be re-fetched. It should only be used when
    /// the state is updated optimistically later on to ensure the clients have the
    /// correct device states
    pub async fn get_info_silent(&mut self, device: &Device) -> Result<InfoResponse, Status> {
        let info = self.info.get(&device.name);

        let now = SystemTime::now();
        if let Some(info) = info {
            if info.created + Duration::from_millis(INFO_VALIDITY_MILLIS) < now {
                // info is still valid
                let mut copy = info.response.clone();
                copy.on_time = copy.on_time.map(|time| time + now.duration_since(info.created).unwrap_or_default().as_secs());
                return Ok(copy)
            };
        };

        // get refreshed device info from device handler without sending an update event
        let response = self.refresh_info(device, false).await?;
        self.info.insert(device.name.clone(), DeviceInfo { response: response.clone(), created: now });
        Ok(response)
    }
}