use clap::{Args, Parser, Subcommand};
use spinoff::Spinner;
use spinoff::spinners::SpinnerFrames;
use crate::config::Config;
use crate::tapo::server::rpc::{Color, EventType, IntegerValueChange};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Path to the configuration file which should be used
    #[arg(long, short, value_parser = parse_config, default_value_t = Config::None, global = true)]
    pub config: Config,

    /// Address for client to connect to gRPC server [default: config or 127.0.0.1]
    #[arg(long, short, global = true)]
    pub address: Option<String>,

    /// Port for client to connect to gRPC server [default: config or 19191]
    #[arg(long, short = 'n', value_parser = clap::value_parser!(u16).range(1..=65535), global = true)]
    pub port: Option<u16>,

    /// Boolean whether to connect to the gRPC using https [default: config or false]
    #[arg(long, short = 'i', global = true)]
    pub secure: Option<bool>,

    /// Print result (if any) as json
    #[arg(long, short, default_value_t = false, global = true)]
    pub json: bool
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(flatten)]
    Client(ClientCommand),
    #[command(flatten)]
    Server(ServerCommand)
}

#[derive(Subcommand, Debug)]
pub enum ServerCommand {
    /// Start the grpc server
    Serve {
        #[arg(value_parser = clap::value_parser!(u16).range(1..=65535))]
        port: Option<u16>
    },
}

#[derive(Subcommand, Debug)]
pub enum ClientCommand {
    /// List all registered devices
    Devices,
    /// Subscribe to device events
    Events {
        /// Event types to subscribe to
        /// When nothing specified all events are subscribed
        types: Vec<EventType>
    },
    /// Update properties of a device
    Set {
        /// Device which should be updated
        device: String,

        /// Brightness value between 1 and 100
        #[arg(value_parser = parse_100_value, allow_negative_numbers = true, long, short)]
        brightness: Option<IntegerValueChange>,

        #[command(flatten)]
        hue_saturation: HueSaturation,

        /// Color temperature in kelvin between 2500 and 6500
        #[arg(value_parser = parse_kelvin_value, allow_negative_numbers = true, long, short)]
        temperature: Option<IntegerValueChange>,

        /// Use predefined google home color
        #[arg(long, short = 'o', value_enum)]
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
    },
    #[clap(hide = true)]
    /// Create shell completions
    Completions {
        directory: String
    }
}

#[derive(Args, Clone, Debug)]
#[group(multiple = true, requires_all = ["hue", "saturation"])]
pub struct HueSaturation {
    /// Hue value between 1 and 360
    #[arg(value_parser = parse_360_value, long, short = 'u', allow_negative_numbers = true)]
    pub hue: Option<IntegerValueChange>,

    /// Saturation value between 1 and 100
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

fn parse_kelvin_value(s: &str) -> Result<IntegerValueChange, String> {
    let int = s.parse().map_err(|_| format!("'{s}' is not a valid integer"))?;
    let relative = s.starts_with('+') || s.starts_with('-');
    if !relative && !(2500..=6500).contains(&int) {
        Err(format!("'{int}' is not in range 2500 to 6500"))?;
    }
    Ok(IntegerValueChange {
        absolute: !relative,
        value: int
    })
}

fn parse_config(s: &str) -> Result<Config, String> {
    Ok(Config::new(Some(s.to_string())))
}

pub trait SpinnerOpt<'a> {
    fn success(&mut self, message: impl Into<&'a str>);

    fn fail(&mut self, message: impl Into<&'a str>);

    fn update(&mut self, spinner_type: SpinnerFrames, message: impl Into<&'a str>);
}

impl<'a> SpinnerOpt<'a> for Option<Spinner> {
    fn success(&mut self, message: impl Into<&'a str>) {
        if let Some(spinner) = self {
            spinner.success(message.into())
        }
    }

    fn fail(&mut self, message: impl Into<&'a str>) {
        if let Some(spinner) = self {
            spinner.fail(message.into())
        }
    }

    fn update(&mut self, spinner_type: SpinnerFrames, message: impl Into<&'a str>) {
        if let Some(spinner) = self {
            spinner.update(spinner_type, message.into().to_string(), None)
        }
    }
}