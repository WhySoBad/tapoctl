const SERDE_DERIVE: &str = "#[derive(serde::Deserialize, serde::Serialize)]";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .type_attribute(".", SERDE_DERIVE)
        .type_attribute("tapo.Color", "#[derive(clap::ValueEnum)]")
        .compile(&["proto/tapo.proto"], &["proto"])?;
    Ok(())
}