const SERDE_DERIVE: &str = "#[derive(serde::Deserialize, serde::Serialize)]";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .type_attribute("tapo.InfoResponse", SERDE_DERIVE)
        .type_attribute("tapo.InfoJsonResponse", SERDE_DERIVE)
        .type_attribute("tapo.UsagePerPeriod", SERDE_DERIVE)
        .type_attribute("tapo.DevicesResponse", SERDE_DERIVE)
        .type_attribute("tapo.PowerResponse", SERDE_DERIVE)
        .type_attribute("tapo.Device", SERDE_DERIVE)
        .type_attribute("tapo.UsageResponse", SERDE_DERIVE)
        .compile(&["proto/tapo.proto"], &["proto"])?;
    Ok(())
}