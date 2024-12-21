const SERDE_DERIVE: &str = "#[derive(serde::Deserialize, serde::Serialize)]";
const CLAP_ENUM: &str = "#[derive(clap::ValueEnum)]";
const VALIDATE_DERIVE: &str = "#[derive(validator::Validate)]";

const CUSTOM_VALIDATOR_LOCATION: &str = "crate::tapo::validation";
const VALIDATOR_REQUIRED: &str = "#[validate(required)]";

fn custom_validator(fn_name: &str) -> String {
    "#[validate(custom(function = \"".to_owned() + CUSTOM_VALIDATOR_LOCATION + "::" + fn_name + "\"))]"
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .type_attribute(".", SERDE_DERIVE)
        .type_attribute("tapo.Color", CLAP_ENUM)
        .type_attribute("tapo.EventType", CLAP_ENUM)
        .type_attribute("tapo.SetRequest", VALIDATE_DERIVE)
        .field_attribute("tapo.SetRequest.brightness", custom_validator("validate_brightness"))
        .field_attribute("tapo.SetRequest.temperature", custom_validator("validate_temperature"))
        .field_attribute("tapo.SetRequest.hue_saturation", custom_validator("validate_hue_saturation"))
        .field_attribute("tapo.SetRequest.hue_saturation.hue", VALIDATOR_REQUIRED)
        .field_attribute("tapo.SetRequest.hue_saturation.saturation", VALIDATOR_REQUIRED)
        .compile(&["proto/tapo.proto"], &["proto"])?;
    Ok(())
}