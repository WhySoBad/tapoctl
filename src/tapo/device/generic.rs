use anyhow::anyhow;
use tapo::GenericDeviceHandler;

use crate::tapo::{
    server::rpc::{InfoJsonResponse, InfoResponse, PowerResponse, UsageResponse},
    TapoDeviceHandlerExt,
};

impl TapoDeviceHandlerExt for GenericDeviceHandler {
    async fn reset(&self) -> Result<(), tapo::Error> {
        Err(tapo::Error::Other(anyhow!("Cannot reset generic device")))
    }

    async fn get_info(&self) -> Result<InfoResponse, tapo::Error> {
        let info = self.get_device_info().await?;
        Ok(InfoResponse {
            device_on: info.device_on,
            on_time: info.on_time,
            ..InfoResponse::default()
        })
    }

    async fn get_info_json(&self) -> Result<InfoJsonResponse, tapo::Error> {
        let info = self.get_device_info_json().await?;
        let mut bytes = vec![];
        serde_json::to_writer(&mut bytes, &info).unwrap_or_default();

        Ok(InfoJsonResponse { data: bytes })
    }

    async fn get_usage(&self) -> Result<UsageResponse, tapo::Error> {
        Err(tapo::Error::Other(anyhow!(
            "Cannot get device usage for generic device"
        )))
    }

    async fn power_on(&self) -> Result<PowerResponse, tapo::Error> {
        self.on().await?;

        Ok(PowerResponse { device_on: true })
    }

    async fn power_off(&self) -> Result<PowerResponse, tapo::Error> {
        self.off().await?;

        Ok(PowerResponse { device_on: false })
    }

    async fn update(
        &self,
        power: Option<bool>,
        _brightness: Option<u8>,
        _temperature: Option<u16>,
        _hue_saturation: Option<(u16, u8)>,
    ) -> Result<(), tapo::Error> {
        if let Some(power_on) = power {
            if power_on {
                self.power_on().await?;
            } else {
                self.power_off().await?;
            }
        }

        Ok(())
    }
}
