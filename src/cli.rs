use clap::{Args, Parser, Subcommand};
use spinoff::Spinner;
use spinoff::spinners::SpinnerFrames;
use crate::config::Config;
use crate::tapo::server::rpc::{Color, IntegerValueChange};

#[derive(Parser, Debug)]
#[command(name = "tapoctl")]
#[command(about = "A cli and server for interacting locally with your tplink tapo lamps from the command line")]
#[command(version, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Path to the configuration file which should be used
    #[arg(long, short, value_parser = parse_config, default_value_t = Config::None)]
    pub config: Config,

    /// Print result (if any) as json
    #[arg(long, short, default_value_t = false)]
    pub json: bool
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List all registered devices
    Devices,
    /// Start the grpc server
    Serve {
        #[arg(value_parser = clap::value_parser!(u16).range(1..=65535))]
        port: Option<u16>
    },
    /// Update properties of a device
    Set {
        /// Device which should be updated
        device: String,

        /// New brightness value
        #[arg(value_parser = parse_100_value, allow_negative_numbers = true, long, short)]
        brightness: Option<IntegerValueChange>,

        #[command(flatten)]
        hue_saturation: HueSaturation,

        /// New color temperature
        #[arg(value_parser = parse_100_value, allow_negative_numbers = true, long, short)]
        temperature: Option<IntegerValueChange>,

        /// Use predefined google home color
        #[arg(long, short, value_enum)]
        color: Option<Color>,

        /// Turn device on or off
        #[arg(long, short)]
        power: Option<bool>,
    },
    /// Print information about a device
    Info {
        /// Device for which the info should be fetched
        device: String,
    },
    /// Print usage information about a device
    Usage {
        /// Device to get the usage for
        device: String,
    },
    /// Turn device on
    On {
        /// Device which should be turned on
        device: String,
    },
    /// Turn device off
    Off {
        /// Device which should be turned off
        device: String,
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
    #[arg(value_parser = parse_360_value, long, short_alias = 'u', allow_negative_numbers = true)]
    pub hue: Option<IntegerValueChange>,

    /// New saturation value
    #[arg(value_parser = parse_100_value, long, short, allow_negative_numbers = true)]
    pub saturation: Option<IntegerValueChange>,
}

fn parse_360_value(s: &str) -> Result<IntegerValueChange, String> {
    let int = s.parse().map_err(|_| format!("'{s}' is not a valid integer"))?;
    let relative = s.starts_with('+') || s.starts_with('-');
    if !relative && !(1..=360).contains(&int) {
        Err(format!("'{int}' is not in range 1 to 360"))?;
    }
    Ok(IntegerValueChange {
        absolute: !relative,
        value: int
    })
}

fn parse_100_value(s: &str) -> Result<IntegerValueChange, String> {
    let int = s.parse().map_err(|_| format!("'{s}' is not a valid integer"))?;
    let relative = s.starts_with('+') || s.starts_with('-');
    if !relative && !(1..=100).contains(&int) {
        Err(format!("'{int}' is not in range 1 to 100"))?;
    }
    Ok(IntegerValueChange {
        absolute: !relative,
        value: int
    })
}

fn parse_config(s: &str) -> Result<Config, String> {
    Ok(Config::new(Some(s.to_string())))
}

pub trait SpinnerOpt {
    fn success(&mut self, message: &str);

    fn fail(&mut self, message: &str);

    fn update(&mut self, spinner_type: SpinnerFrames, message: String);
}

impl SpinnerOpt for Option<Spinner> {
    fn success(&mut self, message: &str) {
        if let Some(spinner) = self {
            spinner.success(message)
        }
    }

    fn fail(&mut self, message: &str) {
        if let Some(spinner) = self {
            spinner.fail(message)
        }
    }

    fn update(&mut self, spinner_type: SpinnerFrames, message: String) {
        if let Some(spinner) = self {
            spinner.update(spinner_type, message, None);
        }
    }
}