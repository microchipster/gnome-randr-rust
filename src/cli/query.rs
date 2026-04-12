use std::fmt::Write;

use gnome_randr::{display_config::resources::Resources, DisplayConfig};
use structopt::StructOpt;

use super::brightness;

#[derive(StructOpt)]
pub struct CommandOptions {
    #[structopt(
        value_name = "CONNECTOR",
        help = "Connector such as eDP-1 or HDMI-1",
        long_help = "Connector name reported by \"gnome-randr query\", such as \"eDP-1\" or \"HDMI-1\". Omit it to list every connected output, including the valid modes, scales, and current software brightness state for each one."
    )]
    pub connector: Option<String>,

    #[structopt(
        short,
        long,
        help = "Show one-line summaries instead of full details",
        long_help = "Show only the condensed view. With no connector this prints one summary block per output plus current software brightness state. With a connector it prints only that logical monitor summary and brightness state."
    )]
    pub summary: bool,
}

#[derive(Debug)]
pub enum Error {
    NotFound,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match &self {
                Error::NotFound => "fatal: unable to find output.",
            }
        )
    }
}

impl std::error::Error for Error {}

pub fn handle(
    opts: &CommandOptions,
    config: &DisplayConfig,
    proxy: &dbus::blocking::Proxy<&dbus::blocking::Connection>,
) -> Result<String, Box<dyn std::error::Error>> {
    let resources = Resources::get_resources(proxy)?;

    let format_brightness = |connector: &str| -> Result<String, Box<dyn std::error::Error>> {
        Ok(
            match brightness::load_current_brightness(connector, &resources, proxy)? {
                Some(current) => current.to_string(),
                None => "unknown".to_string(),
            },
        )
    };

    Ok(match &opts.connector {
        Some(connector) => {
            let (logical_monitor, physical_monitor) =
                config.search(connector).ok_or(Error::NotFound)?;
            let brightness = format_brightness(connector)?;

            if opts.summary {
                format!("{}software brightness: {}\n", logical_monitor, brightness)
            } else {
                format!(
                    "{}\n{}software brightness: {}\n",
                    logical_monitor, physical_monitor, brightness
                )
            }
        }
        None => {
            let mut output = if opts.summary {
                let mut s = String::new();
                config.format(&mut s, true)?;
                s
            } else {
                format!("{}", config)
            };

            if !output.ends_with('\n') {
                output.push('\n');
            }

            writeln!(&mut output, "software brightness:")?;
            for monitor in &config.monitors {
                writeln!(
                    &mut output,
                    "\t{}: {}",
                    monitor.connector,
                    format_brightness(&monitor.connector)?
                )?;
            }

            output
        }
    })
}
