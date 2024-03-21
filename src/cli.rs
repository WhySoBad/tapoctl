use std::fmt::format;
use clap::{Args, Parser, Subcommand, ValueEnum};
use clap::builder::PossibleValue;
use crate::tapo::server::rpc::Color;

#[derive(Parser, Debug)]
#[command(name = "tapoctl")]
#[command(about = "A cli and server for interacting locally with your tplink tapo lamps from the command line")]
#[command(version, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List all registered devices
    Devices,
    /// Start the grpc server
    Serve {
        #[arg(value_parser = clap::value_parser!(u16).range(1..=65535), default_value_t = 19191)]
        port: u16
    },
    /// Update properties of a device
    Set {
        /// Device which should be updated
        device: String,

        /// New brightness value
        #[arg(value_parser = clap::value_parser!(u8).range(1..=100), long, short)]
        brightness: Option<u8>,

        #[command(flatten)]
        hue_saturation: HueSaturation,

        /// New color temperature
        #[arg(value_parser = clap::value_parser!(u16).range(2500..=6500), long, short)]
        temperature: Option<u16>,

        /// Use predefined google home color (as PascalCase name)
        #[arg(long, short, value_parser = valid_color)]
        color: Option<Color>,

        /// Turn device on or off
        #[arg(long, short)]
        power: Option<bool>
    },
    /// Print information about a device
    Info {
        /// Device for which the info should be fetched
        device: String,

        /// Return the full device info json from the tapo api
        #[arg(default_value_t = false, long, short)]
        json: bool
    },
    /// Print usage information about a device
    Usage {
        /// Device to get the usage for
        device: String,
    },
    /// Turn device on
    On {
        /// Device which should be turned on
        device: String
    },
    /// Turn device off
    Off {
        /// Device which should be turned off
        device: String
    },
    /// Reset a device to factory defaults
    Reset {
        /// Device which should be reset
        device: String
    }
}

#[derive(Args, Clone, Debug)]
#[group(multiple = true, requires_all = ["hue", "saturation"])]
pub struct HueSaturation {
    /// New hue value
    #[arg(value_parser = clap::value_parser!(u16).range(1..=360), long, short_alias = 'u')]
    pub hue: Option<u16>,

    /// New saturation value
    #[arg(value_parser = clap::value_parser!(u8).range(1..=100), long, short)]
    pub saturation: Option<u8>,
}


fn valid_color(s: &str) -> Result<Color, String> {
    match Color::from_str_name(s) {
        Some(c) => Ok(c),
        None => Err(format!("'{s}' is not a valid PascalCase google home color"))
    }
}