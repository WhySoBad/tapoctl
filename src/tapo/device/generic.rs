use tapo::GenericDeviceHandler;

use crate::tapo::{server::rpc::{InfoJsonResponse, InfoResponse, PowerResponse, UsageResponse}, TapoDeviceHandlerExt, TapoErrMap};

impl TapoDeviceHandlerExt for GenericDeviceHandler {
    async fn reset(&self, _device: &crate::device::Device) -> Result<(), tonic::Status> {
        Err(tonic::Status::unimplemented("Reset API is not supported by this device type"))
    }

    async fn get_info(&self, device: &crate::device::Device) -> Result<InfoResponse, tonic::Status> {
        let info = self.get_device_info().await.map_tapo_err(device).await?;
        Ok(InfoResponse {
                device_on: info.device_on,
                on_time: info.on_time,
                name: device.name.clone(),
                ..InfoResponse::default()
        })
    }

    async fn get_info_json(&self, device: &crate::device::Device) -> Result<InfoJsonResponse, tonic::Status> {
        let info = self.get_device_info_json().await.map_tapo_err(device).await?;
        let mut bytes = vec![];
        serde_json::to_writer(&mut bytes, &info).unwrap_or_default();

        Ok(InfoJsonResponse { data: bytes })
    }

    async fn get_usage(&self, _device: &crate::device::Device) -> Result<UsageResponse, tonic::Status> {
        Err(tonic::Status::unimplemented("Device usage API is not supported by this device type"))
    }

    async fn power_on(&self, device: &crate::device::Device) -> Result<PowerResponse, tonic::Status> {
        self.on().await.map_tapo_err(device).await?;

        Ok(PowerResponse { device_on: true })
    }

    async fn power_off(&self, device: &crate::device::Device) -> Result<PowerResponse, tonic::Status> {
        self.off().await.map_tapo_err(device).await?;

        Ok(PowerResponse { device_on: false })
    }

    async fn update(
        &self,
        device: &crate::device::Device,
        power: Option<bool>,
        _brightness: Option<u8>,
        _temperature: Option<u16>,
        _hue_saturation: Option<(u16, u8)>
    ) -> Result<(), tonic::Status> {
        if let Some(power_on) = power {
            if power_on { self.power_on(device).await?; }
            else { self.power_off(device).await?; }
        }

        Ok(())
    }
}