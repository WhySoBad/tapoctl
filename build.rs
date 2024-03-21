const SERDE_DERIVE: &str = "#[derive(serde::Deserialize, serde::Serialize)]";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .type_attribute("InfoResponse", SERDE_DERIVE)
        .type_attribute("InfoJsonResponse", SERDE_DERIVE)
        .type_attribute("UsagePerPeriod", SERDE_DERIVE)
        .type_attribute("DevicesResponse", SERDE_DERIVE)
        .type_attribute("UsageResponse", SERDE_DERIVE);
    tonic_build::compile_protos("proto/tapo.proto")?;
    Ok(())
}