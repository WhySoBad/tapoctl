use tapo::ColorLightHandler;

use crate::tapo::{
    color::any_to_rgb,
    server::rpc::{InfoJsonResponse, InfoResponse, PowerResponse, UsagePerPeriod, UsageResponse},
    TapoDeviceHandlerExt,
};

impl TapoDeviceHandlerExt for ColorLightHandler {
    async fn reset(&self) -> Result<(), tapo::Error> {
        self.device_reset().await
    }

    async fn get_info(&self) -> Result<InfoResponse, tapo::Error> {
        let info = self.get_device_info().await?;
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
        temperature: Option<u16>,
        hue_saturation: Option<(u16, u8)>,
    ) -> Result<(), tapo::Error> {
        if let Some(brightness) = brightness {
            self.set_brightness(brightness).await?;
        }
        if let Some(temperature) = temperature {
            self.set_color_temperature(temperature).await?;
        }
        if let Some((hue, saturation)) = hue_saturation {
            self.set_hue_saturation(hue, saturation).await?;
        }

        if let Some(power_on) = power {
            if power_on && brightness.is_none() && temperature.is_none() && hue_saturation.is_none()
            {
                self.power_on().await?;
            } else if !power_on {
                self.power_off().await?;
            }
        }

        Ok(())
    }
}
