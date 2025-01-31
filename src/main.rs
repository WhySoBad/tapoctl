use std::collections::HashMap;
use std::path::Path;
use std::process::exit;
use std::str::FromStr;
use clap::{Parser, ValueEnum};
use clap_complete::{Generator, Shell};
use colored::Colorize;
use serde_json::{json, Value};
use spinoff::{Spinner, spinners};
use tonic::transport::Channel;
use crate::cli::{Cli, ClientCommand, Commands, ServerCommand, SpinnerOpt};
use crate::config::{ClientConfig, Config};
use crate::tapo::server::rpc::{DeviceRequest, HueSaturation, Empty, SetRequest, EventRequest, EventType, InfoResponse, Device};
use crate::tapo::server::rpc::tapo_client::TapoClient;
use crate::tapo::start_server;
use crate::tapo::TonicErrMap;

mod device;
mod config;
mod tapo;
mod cli;
mod completions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let cli: Cli = Cli::parse();
    let config = cli.config;
    let json = cli.json;

    match cli.command {

        Commands::Server(server_command) => {
            let server_config = match config {
                Config::Server(cfg) => Some(cfg),
                _ => None
            };
            match server_command {
                ServerCommand::Serve { port } => {
                    start_server(port, server_config).await;
                }
            }
        },
        Commands::Client(client_command) => {
            let client_config = match config {
                Config::Client(mut cfg) => {
                    cfg.address = cli.address.clone().unwrap_or(cfg.address.clone());
                    cfg.port = cli.port.unwrap_or(cfg.port);
                    cfg.secure = cli.secure.unwrap_or(cfg.secure);
                    Some(cfg)
                },
                _ => None,
            }.or(ClientConfig::from(cli.address, cli.port, cli.secure));

            let mut spinner = (!json).then(|| Spinner::new(spinners::Dots, "Preparing client...", None));
            let mut client = get_client(client_config, &mut spinner, json).await;
            spinner.update(spinners::Dots.into(), "Sending request...");

            match client_command {
                ClientCommand::Devices => {
                    let devices = client.devices(Empty {}).await.map_tonic_err(&mut spinner, json).into_inner();
                    completions::save_device_completions(&devices.devices);

                    if json {
                        println!("{}", json!(devices))
                    } else if devices.devices.is_empty() {
                        spinner.success("No devices registered")
                    } else {
                        spinner.success("Found devices:");
                        println!("{}", devices.devices.iter().map(|dev| {
                            let heading = format!("{}:", dev.name.bold().underline());
                            format!("{}\n{dev}", heading)
                        }).collect::<Vec<_>>().join("\n\n"));
                    }
                }

                ClientCommand::Set { device, color, brightness, temperature, hue_saturation, power } => {
                    let request = SetRequest {
                        color: color.map(|c| c as i32),
                        device,
                        brightness,
                        temperature,
                        power,
                        hue_saturation: {
                            let hue = hue_saturation.hue;
                            let saturation = hue_saturation.saturation;
                            if hue.is_some() && saturation.is_some() {
                                Some(HueSaturation { saturation, hue })
                            } else {
                                None
                            }
                        }
                    };

                    let state = client.set(request).await.map_tonic_err(&mut spinner, json).into_inner();
                    if json {
                        println!("{}", json!(state))
                    } else {
                        spinner.success("Updated device:");
                        println!("{state}");
                    }
                }
                ClientCommand::Info { device } => {
                    if json {
                        let json = client.info_json(DeviceRequest { device }).await.map_tonic_err(&mut spinner, json);
                        let value: HashMap<String, Value> = serde_json::from_slice(json.into_inner().data.as_slice()).unwrap();
                        println!("{}", json!(value));
                    } else {
                        let info = client.info(DeviceRequest { device }).await.map_tonic_err(&mut spinner, json).into_inner();
                        spinner.success("Device info:");
                        println!("{info}");
                    }
                }
                ClientCommand::Usage { device } => {
                    let usage = client.usage(DeviceRequest { device }).await.map_tonic_err(&mut spinner, json).into_inner();
                    if json {
                        println!("{}", json!(usage))
                    } else {
                        spinner.success("Device usage:");
                        println!("{usage}");
                    }
                }
                ClientCommand::On { device } => {
                    let result = client.on(DeviceRequest { device: device.clone() }).await.map_tonic_err(&mut spinner, json).into_inner();
                    if json {
                        println!("{}", json!(result))
                    } else {
                        spinner.success(format!("Device '{device}' is now turned on").as_str())
                    }
                }
                ClientCommand::Off { device } => {
                    let result = client.off(DeviceRequest { device: device.clone() }).await.map_tonic_err(&mut spinner, json).into_inner();
                    if json {
                        println!("{}", json!(result))
                    } else {
                        spinner.success(format!("Device '{device}' is now turned off").as_str())
                    }
                }
                ClientCommand::Reset { device } => {
                    client.reset(DeviceRequest { device }).await.map_tonic_err(&mut spinner, json);
                    if json {
                        println!("{}", json!({ "success": true }))
                    } else {
                        spinner.success("Restored factory defaults")
                    }
                }
                ClientCommand::Events { types } => {
                    let request = EventRequest { types: types.into_iter().map(i32::from).collect() };
                    let mut events  = client.events(request).await.map_tonic_err(&mut spinner, json).into_inner();
                    spinner.success("Subscribed to events");


                    while let Ok(Some(event)) = events.message().await {
                        if json {
                            let event_type = EventType::try_from(event.r#type).unwrap_or_default().as_str_name();
                            let body: HashMap<String, Value> = serde_json::from_slice(event.body.as_slice()).unwrap();
                            println!("{}", json!({ "type": event_type, "body": body }));
                            continue
                        }
                        match event.r#type.try_into() {
                            Ok(EventType::DeviceStateChange) => {
                                let body: InfoResponse = serde_json::from_slice(event.body.as_slice()).unwrap();
                                println!("{}\n{body}\n", format!("Device '{}' changed:", body.name).bold().underline());
                            },
                            Ok(EventType::DeviceAuthChange) => {
                                let body: Device = serde_json::from_slice(event.body.as_slice()).unwrap();
                                println!("{}\n{body}\n", format!("Auth changed for device '{}':", body.name).bold().underline());
                            }
                            Err(err) => {
                                println!("Error whilst decoding event type: {err}")
                            }
                        }
                    }

                    if !json {
                        println!("Finished subscription. Stream closed!")
                    }
                },
                ClientCommand::Completions { directory } => {
                    let path = Path::new(&directory);
                    if !path.exists() {
                        if let Err(err) = std::fs::create_dir_all(&directory) {
                            spinner.fail(format!("Failed to create completions directory at {directory}: {err}").as_str());
                            return Ok(());
                        };
                    } else if !path.is_dir() {
                        spinner.fail(format!("Unable to write completions to {directory}. File exists but is not a directory!").as_str());
                        return Ok(());
                    }

                    let devices = client.devices(Empty {}).await.map_tonic_err(&mut spinner, json).into_inner();
                    completions::save_device_completions(&devices.devices);
                    spinner.success("Loaded devices");

                    for shell in Shell::value_variants() {
                        let completions = completions::generate_completions(*shell, "tapoctl");
                        match std::fs::write(path.join(shell.file_name("tapoctl")), completions) {
                            Ok(_) => println!("Successfully created completions for {}", shell.to_string()),
                            Err(err) => println!("Error whilst writing completions file for {}: {err}", shell.to_string())
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

async fn get_client(config: Option<ClientConfig>, spinner: &mut Option<Spinner>, json: bool) -> TapoClient<Channel> {
    let (secure, host, port) = match config {
        Some(config) => (config.secure, config.address.clone(), config.port),
        None => (false, String::from("127.0.0.1"), 19191)
    };

    let secure = std::env::var("TAPO_SECURE").is_ok() || secure;
    let host = std::env::var("TAPO_HOST").unwrap_or(host);
    let port = std::env::var("TAPO_PORT").map(|p| u16::from_str(p.as_str()).unwrap_or(port)).unwrap_or(port);
    let protocol = if secure { "https" } else { "http" };

    let format = format!("{protocol}://{host}:{port}");
    TapoClient::connect(format.clone()).await.unwrap_or_else(|err| {
        if json {
            println!("{}", json!({ "code": "Unable to connect to grpc server", "message": err.to_string() }))
        } else {
            spinner.fail(format!("Unable to connect to server at {format}. Is it up and running?").as_str());
        }
        exit(1)
    })
}
