use tapo::ColorLightHandler;

use crate::{device::Device, tapo::{color::any_to_rgb, server::rpc::{InfoJsonResponse, InfoResponse, PowerResponse, UsagePerPeriod, UsageResponse}, TapoDeviceHandlerExt, TapoErrMap}};

impl TapoDeviceHandlerExt for ColorLightHandler {
    async fn reset(&self, device: &Device) -> Result<(), tonic::Status> {
        self.device_reset().await.map_tapo_err(device).await
    }

    async fn get_info(&self, device: &Device) -> Result<crate::tapo::server::rpc::InfoResponse, tonic::Status> {
        let info = self.get_device_info().await.map_tapo_err(device).await?;
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
            name: device.name.clone(),
            color: any_to_rgb(temperature, hue, saturation, brightness),
        })
    }

    async fn get_info_json(&self, device: &Device) -> Result<crate::tapo::server::rpc::InfoJsonResponse, tonic::Status> {
        let info = self.get_device_info_json().await.map_tapo_err(device).await?;
        let mut bytes = vec![];
        serde_json::to_writer(&mut bytes, &info).unwrap_or_default();

        Ok(InfoJsonResponse { data: bytes })
    }

    async fn get_usage(&self, device: &Device) -> Result<crate::tapo::server::rpc::UsageResponse, tonic::Status> {
        let usage = self.get_device_usage().await.map_tapo_err(device).await?;

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

        Ok(UsageResponse {
            power_usage: Some(power_usage),
            time_usage: Some(time_usage),
            saved_power: Some(saved_power)
        })
    }

    async fn power_on(&self, device: &Device) -> Result<crate::tapo::server::rpc::PowerResponse, tonic::Status> {
        self.on().await.map_tapo_err(device).await?;

        Ok(PowerResponse { device_on: true })
    }

    async fn power_off(&self, device: &Device) -> Result<crate::tapo::server::rpc::PowerResponse, tonic::Status> {
        self.off().await.map_tapo_err(device).await?;

        Ok(PowerResponse { device_on: false })
    }

    async fn update(
        &self,
        device: &crate::device::Device,
        power: Option<bool>,
        brightness: Option<u8>,
        temperature: Option<u16>,
        hue_saturation: Option<(u16, u8)>
    ) -> Result<(), tonic::Status> {

        if let Some(brightness) = brightness {
            self.set_brightness(brightness).await.map_tapo_err(device).await?;
        }
        if let Some(temperature) = temperature {
            self.set_color_temperature(temperature).await.map_tapo_err(device).await?;
        }
        if let Some((hue, saturation)) = hue_saturation {
            self.set_hue_saturation(hue, saturation).await.map_tapo_err(device).await?;
        }

        if let Some(power_on) = power {
            if power_on && brightness.is_none() && temperature.is_none() && hue_saturation.is_none() {
                self.power_on(device).await?;
            } else if !power_on { self.power_off(device).await?; }
        }

        Ok(())
    }
}