use tapo::LightHandler;

use crate::tapo::{
    server::rpc::{InfoJsonResponse, InfoResponse, PowerResponse, UsagePerPeriod, UsageResponse},
    TapoDeviceHandlerExt,
};

impl TapoDeviceHandlerExt for LightHandler {
    async fn reset(&self) -> Result<(), tapo::Error> {
        self.device_reset().await
    }

    async fn get_info(&self) -> Result<InfoResponse, tapo::Error> {
        let info = self.get_device_info().await?;
        Ok(InfoResponse {
            brightness: Some(info.brightness as u32),
            device_on: Some(info.device_on),
            on_time: info.on_time,
            overheated: info.overheated,
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
        let usage = self.get_device_usage().await?;

        let power_usage = UsagePerPeriod {
            today: usage.power_usage.today,
            week: usage.power_usage.past7,
            month: usage.power_usage.past30,
        };

        let time_usage = UsagePerPeriod {
            today: usage.time_usage.today,
            week: usage.time_usage.past7,
            month: usage.time_usage.past30,
        };

        let saved_power = UsagePerPeriod {
            today: usage.saved_power.today,
            week: usage.saved_power.past7,
            month: usage.saved_power.past30,
        };

        Ok(UsageResponse {
            power_usage: Some(power_usage),
            time_usage: Some(time_usage),
            saved_power: Some(saved_power),
        })
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
        brightness: Option<u8>,
        _temperature: Option<u16>,
        _hue_saturation: Option<(u16, u8)>,
    ) -> Result<(), tapo::Error> {
        if let Some(brightness) = brightness {
            self.set_brightness(brightness).await?;
            // if power is true at the same time we can ignore it since changing the brightness
            // turns the lamp on anyways
            if power.is_some_and(|v| v) {
                return Ok(());
            }
        }
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
