const SERDE_DERIVE: &str = "#[derive(serde::Deserialize, serde::Serialize)]";
const CLAP_ENUM: &str = "#[derive(clap::ValueEnum)]";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .type_attribute(".", SERDE_DERIVE)
        .type_attribute("tapo.Color", CLAP_ENUM)
        .type_attribute("tapo.EventType", CLAP_ENUM)
        .compile(&["proto/tapo.proto"], &["proto"])?;
    Ok(())
}