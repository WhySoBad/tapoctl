use std::fmt::{Display, Formatter};

use colored::{Colorize, CustomColor};
use colorsys::Rgb;

use super::server::rpc::{self, InfoResponse, UsageResponse};

impl Display for InfoResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut lines = vec![];
        if let Some(on) = &self.device_on {
            let state = on.then_some("Turned on").unwrap_or("Turned off");
            lines.push(format!("{}: {state}", "State".bold()))
        }
        let overheated = if self.overheated {
            "Overheated"
        } else {
            "Normal"
        };
        lines.push(format!("{}: {overheated}", "Thermals".bold()));
        if let Some(on_time) = &self.on_time {
            lines.push(format!("{}: {}min", "Uptime".bold(), on_time / 60u64))
        }
        if let Some(temperature) = &self.temperature {
            if temperature > &0 {
                lines.push(format!("{}: {temperature}K", "Temperature".bold()))
            }
        }
        if let Some(color) = &self.color {
            let block = "  ".on_custom_color(CustomColor::new(
                u8::try_from(color.red).unwrap_or_default(),
                u8::try_from(color.green).unwrap_or_default(),
                u8::try_from(color.blue).unwrap_or_default(),
            ));
            let color = Rgb::new(
                color.red as f64,
                color.green as f64,
                color.blue as f64,
                None,
            )
            .to_hex_string();
            lines.push(format!("{}: {color} {block}", "Color".bold()));
        }
        if let Some(brightness) = &self.brightness {
            lines.push(format!("{}: {brightness}%", "Brightness".bold()))
        }
        if let Some((hue, saturation)) = &self.hue.zip(self.saturation) {
            lines.push(format!("{}: {hue}", "Hue".bold()));
            lines.push(format!("{}: {saturation}%", "Saturation".bold()))
        }
        if let Some(effect_id) = &self.dynamic_effect_id {
            lines.push(format!("{}: {effect_id}", "Effect".bold()))
        }

        f.write_str(lines.join("\n").as_str())
    }
}

impl Display for UsageResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut lines = vec![];
        if let Some(time) = &self.time_usage {
            let convert_time = |t| format!("{:.2}h", t as f32 / 60f32).into();
            lines.push("Uptime:".underline().bold().to_string());
            lines.push(format!(
                "{}: {}",
                "Today".bold(),
                time.today.map_or("No information".dimmed(), convert_time)
            ));
            lines.push(format!(
                "{}: {:.2}h",
                "Week".bold(),
                time.week.map_or("No information".dimmed(), convert_time)
            ));
            lines.push(format!(
                "{}: {:.2}h",
                "Month".bold(),
                time.month.map_or("No information".dimmed(), convert_time)
            ));
        }
        if let Some(power) = &self.power_usage {
            if !lines.is_empty() {
                lines.push(String::new());
            }
            let convert_power = |p: u64| format!("{:.3}kWh", p as f32 / 1000f32).into();
            lines.push("Power used:".underline().bold().to_string());
            lines.push(format!(
                "{}: {}",
                "Today".bold(),
                power.today.map_or("No information".dimmed(), convert_power)
            ));
            lines.push(format!(
                "{}: {:.3}kWh",
                "Week".bold(),
                power.week.map_or("No information".dimmed(), convert_power)
            ));
            lines.push(format!(
                "{}: {:.3}kWh",
                "Month".bold(),
                power.month.map_or("No information".dimmed(), convert_power)
            ));
        }
        if let Some(saved) = &self.saved_power {
            if !lines.is_empty() {
                lines.push(String::new());
            }
            let convert_power = |p: u64| format!("{:.3}kWh", p as f32 / 1000f32).into();
            lines.push("Power saved:".underline().bold().to_string());
            lines.push(format!(
                "{}: {}",
                "Today".bold(),
                saved.today.map_or("No information".dimmed(), convert_power)
            ));
            lines.push(format!(
                "{}: {:.3}kWh",
                "Week".bold(),
                saved.week.map_or("No information".dimmed(), convert_power)
            ));
            lines.push(format!(
                "{}: {:.3}kWh",
                "Month".bold(),
                saved.month.map_or("No information".dimmed(), convert_power)
            ));
        }
        f.write_str(lines.join("\n").as_str())
    }
}

impl Display for rpc::Device {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut lines = vec![];
        let status = match rpc::SessionStatus::try_from(self.status).unwrap_or_default() {
            rpc::SessionStatus::Authenticated => "Authenticated",
            rpc::SessionStatus::Failure => "Authentication failed",
            rpc::SessionStatus::RepeatedFailure => "Authentication failed multiple times",
        };
        lines.push(format!("{}: {}", "Type".bold(), self.r#type));
        lines.push(format!("{}: {}", "Session".bold(), status));
        lines.push(format!("{}: {}", "Address".bold(), self.address));
        f.write_str(lines.join("\n").as_str())
    }
}
