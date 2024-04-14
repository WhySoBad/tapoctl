use std::cmp::min;
use std::time::{Duration, SystemTime};
use log::{debug, error, info, warn};
use tapo::{ApiClient, ColorLightHandler, GenericDeviceHandler, LightHandler};
use tonic::Status;
use crate::config::{DeviceDefinition, SupportedDevice};
use crate::tapo::server::{EventSender, rpc};
use crate::tapo::server::rpc::{EventType};
use crate::tapo::{create_event, transform_session_status};

const SESSION_VALIDITY_MILLIS: u64 = 60 * 60 * 1000; // 60 minutes
const SESSION_REFRESH_RETRIES: u8 = 10; // after 10 failed session refresh attempts the session status can be set to RepeatedFailure
const REPEATED_FAILURE_RETRY_MILLIS: u64 = 10 * 60 * 1000; // try to refresh as session which repeatedly failed to refresh after 10 minutes

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum SessionStatus {
    Authenticated,
    Failure,
    RepeatedFailure
}

pub struct Device {
    pub address: String,
    pub name: String,
    pub r#type: SupportedDevice,
    pub session_status: SessionStatus,
    client: ApiClient,
    next_session_action: SystemTime,
    handler: Option<DeviceHandler>,
    refresh_retires: u8,
    sender: EventSender,
}

impl Device {
    pub async fn new(name: String, definition: DeviceDefinition, client: ApiClient, sender: EventSender) -> Option<Self> {
        let handler = Self::acquire_handler(&definition.r#type, &definition.address, client.clone()).await;

        if handler.is_ok() {
            info!("Logged into device '{name}'")
        } else {
            warn!("Unable to log into device '{name}'. Retrying on next access...")
        }

        let next_session_action = if handler.is_ok() {
            SystemTime::now() + Duration::from_millis(SESSION_VALIDITY_MILLIS)
        } else {
            SystemTime::now()
        };

        Some(Self {
            refresh_retires: if handler.is_ok() { 0 } else { 1 },
            r#type: definition.r#type,
            address: definition.address,
            session_status: if handler.is_ok() { SessionStatus::Authenticated } else { SessionStatus::Failure },
            handler: handler.ok(),
            next_session_action,
            name,
            client,
            sender
        })
    }

    /// Try to get the device handler from the tapo api for a specific device
    async fn acquire_handler(device_type: &SupportedDevice, address: &String, client: ApiClient) -> Result<DeviceHandler, Status> {
        match device_type {
            SupportedDevice::L530 => {
                client.l530(address).await.map_err(|err| Status::internal(err.to_string())).map(DeviceHandler::ColorLight)
            },
            SupportedDevice::L630 => {
                client.l630(address).await.map_err(|err| Status::internal(err.to_string())).map(DeviceHandler::ColorLight)
            },
            SupportedDevice::L900 => {
                client.l900(address).await.map_err(|err| Status::internal(err.to_string())).map(DeviceHandler::ColorLight)
            },
            SupportedDevice::L510 => {
                client.l510(address).await.map_err(|err| Status::internal(err.to_string())).map(DeviceHandler::Light)
            }
            SupportedDevice::L520 => {
                client.l520(address).await.map_err(|err| Status::internal(err.to_string())).map(DeviceHandler::Light)
            }
            SupportedDevice::L610 => {
                client.l610(address).await.map_err(|err| Status::internal(err.to_string())).map(DeviceHandler::Light)
            }
            SupportedDevice::Generic => {
                client.generic_device(address).await.map_err(|err| Status::internal(err.to_string())).map(DeviceHandler::Generic)
            }
        }
    }

    /// Forcefully refresh the session for the device
    ///
    /// This method should only be called directly when a [`tapo::TapoResponseError::SessionTimeout`]
    /// was returned from an api call
    pub async fn refresh_session(&mut self) -> Result<(), Status> {
        info!("Attempting to refresh session for device '{}'", self.name);
        let now = SystemTime::now();
        let current = self.session_status.clone();

        let result = if let Some(handler) = &mut self.handler {
            let result = match handler {
                DeviceHandler::ColorLight(handler) => {
                    handler.refresh_session().await.map_err(|err| Status::internal(err.to_string())).err()
                },
                DeviceHandler::Light(handler) => {
                    handler.refresh_session().await.map_err(|err| Status::internal(err.to_string())).err()
                },
                DeviceHandler::Generic(handler) => {
                    handler.refresh_session().await.map_err(|err| Status::internal(err.to_string())).err()
                },
            };
            if let Some(error) = result {
                debug!("Session refresh failed for device '{}' with reason: {}", self.name, error);
                self.session_status = SessionStatus::Failure;
                self.refresh_retires = 1;
                Err(error)
            } else {
                debug!("Successfully refreshed session for device '{}'", self.name);
                self.next_session_action = now + Duration::from_millis(SESSION_VALIDITY_MILLIS);
                Ok(())
            }
        } else {
            debug!("Attempting initial session acquisition for device '{}'", self.name);
            match Self::acquire_handler(&self.r#type, &self.address, self.client.clone()).await {
                Ok(handler) => {
                    self.session_status = SessionStatus::Authenticated;
                    self.next_session_action = now + Duration::from_millis(SESSION_VALIDITY_MILLIS);
                    self.refresh_retires = 0;
                    self.handler = Some(handler);
                    debug!("Initial session acquisition succeeded for device '{}'. Next action is required at {:?}", self.name, self.next_session_action);
                    Ok(())
                },
                Err(status) => {
                    self.refresh_retires = min(self.refresh_retires + 1, SESSION_REFRESH_RETRIES);
                    if self.refresh_retires == SESSION_REFRESH_RETRIES {
                        self.session_status = SessionStatus::RepeatedFailure;
                        self.next_session_action = now + Duration::from_millis(REPEATED_FAILURE_RETRY_MILLIS)
                    }
                    debug!("Initial session acquisition failed for device '{}'. Next action is required at {:?}. Failures in row: {}", self.name, self.next_session_action, self.refresh_retires);
                    Err(status)
                }
            }
        };

        if current.ne(&self.session_status) {
            debug!("Session status changed: {:?}", self.session_status);
            let device = rpc::Device {
                name: self.name.clone(),
                status: i32::from(transform_session_status(&self.session_status)),
                address: self.address.clone(),
                r#type: format!("{:?}", self.r#type)
            };

            if let Err(err) = self.sender.send(create_event(EventType::DeviceAuthChange, device)) {
                error!("Error whilst sending new device auth state: {err}")
            }
        }

        result
    }

    /// Attempt to refresh the auth session for the device
    ///
    /// Should the session be expired or the previous refresh attempt failed a new attempt is started.
    /// After 10 failed refresh attempts the session state changes to [`SessionStatus::RepeatedFailure`] which only allows
    /// the next refresh attempt after 10 minutes
    pub async fn try_refresh_session(&mut self) -> Result<(), Status> {
        let now = SystemTime::now();

        debug!("Try session refresh: {:?} {:?}", now, self.next_session_action);

        if now.ge(&self.next_session_action) {
            self.refresh_session().await
        } else {
            Ok(())
        }
    }

    /// Access the current device handler
    ///
    /// Returns tonic status code should the handler be unavailable
    pub fn get_handler(&self) -> Result<&DeviceHandler, Status> {
        match &self.handler {
            Some(handler) => Ok(handler),
            None => Err(Status::unauthenticated(format!("The device '{}' is currently unauthenticated. Try again later or verify the configuration should the issue persist.", self.name)))
        }
    }
}

pub enum DeviceHandler {
    ColorLight(ColorLightHandler),
    Light(LightHandler),
    Generic(GenericDeviceHandler)
}

