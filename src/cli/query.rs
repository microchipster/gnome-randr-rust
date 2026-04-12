use std::fmt::Write;

use gnome_randr::{display_config::resources::Resources, DisplayConfig};
use serde::Serialize;
use structopt::StructOpt;

use super::brightness;
use super::brightness::{CurrentBrightness, CurrentBrightnessState};

const JSON_SCHEMA_VERSION: u32 = 1;

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
        conflicts_with = "json",
        help = "Show one-line summaries instead of full details",
        long_help = "Show only the condensed view. With no connector this prints one summary block per output plus current software brightness state. With a connector it prints only that logical monitor summary and brightness state."
    )]
    pub summary: bool,

    #[structopt(
        long,
        help = "Print structured JSON instead of text",
        long_help = "Print structured JSON using the documented schema in README.md. This includes logical monitors, physical monitors, modes, and software brightness status for scripts.",
        conflicts_with = "summary"
    )]
    pub json: bool,
}

#[derive(Serialize)]
struct QueryJson {
    schema_version: u32,
    serial: u32,
    layout_mode: String,
    supports_mirroring: bool,
    supports_changing_layout_mode: bool,
    global_scale_required: bool,
    renderer: Option<String>,
    logical_monitors: Vec<LogicalMonitorJson>,
    monitors: Vec<PhysicalMonitorJson>,
}

#[derive(Serialize)]
struct LogicalMonitorJson {
    x: i32,
    y: i32,
    scale: f64,
    rotation: String,
    primary: bool,
    monitors: Vec<AssociatedMonitorJson>,
}

#[derive(Serialize)]
struct AssociatedMonitorJson {
    connector: String,
    vendor: String,
    product: String,
    serial: String,
}

#[derive(Serialize)]
struct PhysicalMonitorJson {
    connector: String,
    vendor: String,
    product: String,
    serial: String,
    display_name: Option<String>,
    is_builtin: Option<bool>,
    width_mm: Option<i64>,
    height_mm: Option<i64>,
    modes: Vec<ModeJson>,
    software_brightness: SoftwareBrightnessJson,
}

#[derive(Serialize)]
struct ModeJson {
    id: String,
    width: i32,
    height: i32,
    refresh_rate: f64,
    preferred_scale: f64,
    supported_scales: Vec<f64>,
    is_current: bool,
    is_preferred: bool,
}

#[derive(Serialize)]
struct SoftwareBrightnessJson {
    state: String,
    brightness: Option<f64>,
    filter: Option<String>,
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

fn property_string(properties: &dbus::arg::PropMap, key: &str) -> Option<String> {
    properties
        .get(key)
        .and_then(|value| value.0.as_str())
        .map(str::to_owned)
}

fn property_bool(properties: &dbus::arg::PropMap, key: &str) -> Option<bool> {
    match properties.get(key).and_then(|value| value.0.as_u64()) {
        Some(1) => Some(true),
        Some(0) => Some(false),
        _ => None,
    }
}

fn property_i64(properties: &dbus::arg::PropMap, key: &str) -> Option<i64> {
    properties.get(key).and_then(|value| {
        value
            .0
            .as_i64()
            .or_else(|| value.0.as_u64().map(|value| value as i64))
    })
}

fn software_brightness_json(current: &CurrentBrightness) -> SoftwareBrightnessJson {
    SoftwareBrightnessJson {
        state: match current.state {
            CurrentBrightnessState::Managed => "managed",
            CurrentBrightnessState::Identity => "identity",
            CurrentBrightnessState::Unknown => "unknown",
        }
        .to_string(),
        brightness: current.brightness,
        filter: current.filter.map(|filter| filter.to_string()),
    }
}

fn build_json<F>(
    opts: &CommandOptions,
    config: &DisplayConfig,
    brightness_for: F,
) -> Result<String, Box<dyn std::error::Error>>
where
    F: Fn(&str) -> Result<CurrentBrightness, Box<dyn std::error::Error>>,
{
    let (logical_monitors, physical_monitors): (Vec<_>, Vec<_>) = match &opts.connector {
        Some(connector) => {
            let (logical_monitor, physical_monitor) =
                config.search(connector).ok_or(Error::NotFound)?;
            (vec![logical_monitor], vec![physical_monitor])
        }
        None => (
            config.logical_monitors.iter().collect(),
            config.monitors.iter().collect(),
        ),
    };

    let json = QueryJson {
        schema_version: JSON_SCHEMA_VERSION,
        serial: config.serial,
        layout_mode: config.known_properties.layout_mode.to_string(),
        supports_mirroring: config.known_properties.supports_mirroring,
        supports_changing_layout_mode: config.known_properties.supports_changing_layout_mode,
        global_scale_required: config.known_properties.global_scale_required,
        renderer: property_string(&config.properties, "renderer"),
        logical_monitors: logical_monitors
            .into_iter()
            .map(|monitor| LogicalMonitorJson {
                x: monitor.x,
                y: monitor.y,
                scale: monitor.scale,
                rotation: monitor.transform.to_string(),
                primary: monitor.primary,
                monitors: monitor
                    .monitors
                    .iter()
                    .map(|monitor| AssociatedMonitorJson {
                        connector: monitor.connector.clone(),
                        vendor: monitor.vendor.clone(),
                        product: monitor.product.clone(),
                        serial: monitor.serial.clone(),
                    })
                    .collect(),
            })
            .collect(),
        monitors: physical_monitors
            .into_iter()
            .map(|monitor| {
                Ok(PhysicalMonitorJson {
                    connector: monitor.connector.clone(),
                    vendor: monitor.vendor.clone(),
                    product: monitor.product.clone(),
                    serial: monitor.serial.clone(),
                    display_name: property_string(&monitor.properties, "display-name"),
                    is_builtin: property_bool(&monitor.properties, "is-builtin"),
                    width_mm: property_i64(&monitor.properties, "width-mm"),
                    height_mm: property_i64(&monitor.properties, "height-mm"),
                    modes: monitor
                        .modes
                        .iter()
                        .map(|mode| ModeJson {
                            id: mode.id.clone(),
                            width: mode.width,
                            height: mode.height,
                            refresh_rate: mode.refresh_rate,
                            preferred_scale: mode.preferred_scale,
                            supported_scales: mode.supported_scales.clone(),
                            is_current: mode.known_properties.is_current,
                            is_preferred: mode.known_properties.is_preferred,
                        })
                        .collect(),
                    software_brightness: software_brightness_json(&brightness_for(
                        &monitor.connector,
                    )?),
                })
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?,
    };

    Ok(serde_json::to_string_pretty(&json)?)
}

fn build_text<F>(
    opts: &CommandOptions,
    config: &DisplayConfig,
    brightness_for: F,
) -> Result<String, Box<dyn std::error::Error>>
where
    F: Fn(&str) -> Result<CurrentBrightness, Box<dyn std::error::Error>>,
{
    let format_brightness = |connector: &str| -> Result<String, Box<dyn std::error::Error>> {
        Ok(brightness_for(connector)?.to_string())
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

pub fn handle(
    opts: &CommandOptions,
    config: &DisplayConfig,
    proxy: &dbus::blocking::Proxy<&dbus::blocking::Connection>,
) -> Result<String, Box<dyn std::error::Error>> {
    let resources = Resources::get_resources(proxy)?;

    let brightness_for =
        |connector: &str| brightness::load_current_brightness(connector, &resources, proxy);

    if opts.json {
        build_json(opts, config, brightness_for)
    } else {
        build_text(opts, config, brightness_for)
    }
}

#[cfg(test)]
mod tests {
    use super::{build_json, CommandOptions};
    use crate::cli::brightness::{CurrentBrightness, CurrentBrightnessState};
    use gnome_randr::display_config::{
        logical_monitor::{LogicalMonitor, Monitor, Transform},
        physical_monitor::{KnownModeProperties, Mode, PhysicalMonitor},
        DisplayConfig, KnownProperties, LayoutMode,
    };
    use serde_json::Value;

    fn sample_config() -> DisplayConfig {
        let mut properties = dbus::arg::PropMap::new();
        properties.insert(
            "renderer".to_string(),
            dbus::arg::Variant(Box::new("native".to_string())),
        );

        let mut monitor_properties = dbus::arg::PropMap::new();
        monitor_properties.insert(
            "display-name".to_string(),
            dbus::arg::Variant(Box::new("Built-in display".to_string())),
        );
        monitor_properties.insert("is-builtin".to_string(), dbus::arg::Variant(Box::new(1u32)));
        monitor_properties.insert("width-mm".to_string(), dbus::arg::Variant(Box::new(300i32)));
        monitor_properties.insert(
            "height-mm".to_string(),
            dbus::arg::Variant(Box::new(190i32)),
        );

        DisplayConfig {
            serial: 7,
            monitors: vec![PhysicalMonitor {
                connector: "eDP-1".to_string(),
                vendor: "BOE".to_string(),
                product: "0x07c9".to_string(),
                serial: "0x00000000".to_string(),
                modes: vec![Mode {
                    id: "1920x1080@60".to_string(),
                    width: 1920,
                    height: 1080,
                    refresh_rate: 60.0,
                    preferred_scale: 1.0,
                    supported_scales: vec![1.0, 2.0],
                    known_properties: KnownModeProperties {
                        is_current: true,
                        is_preferred: true,
                    },
                    properties: dbus::arg::PropMap::new(),
                }],
                properties: monitor_properties,
            }],
            logical_monitors: vec![LogicalMonitor {
                x: 0,
                y: 0,
                scale: 1.0,
                transform: Transform::NORMAL,
                primary: true,
                monitors: vec![Monitor {
                    connector: "eDP-1".to_string(),
                    vendor: "BOE".to_string(),
                    product: "0x07c9".to_string(),
                    serial: "0x00000000".to_string(),
                }],
                properties: dbus::arg::PropMap::new(),
            }],
            known_properties: KnownProperties {
                supports_mirroring: true,
                layout_mode: LayoutMode::Physical,
                supports_changing_layout_mode: false,
                global_scale_required: false,
            },
            properties,
        }
    }

    #[test]
    fn json_output_uses_documented_schema() {
        let output = build_json(
            &CommandOptions {
                connector: None,
                summary: false,
                json: true,
            },
            &sample_config(),
            |_connector| Ok(CurrentBrightness::managed(1.5, "filmic".parse().unwrap())),
        )
        .unwrap();

        let value: Value = serde_json::from_str(&output).unwrap();

        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["renderer"], "native");
        assert_eq!(value["logical_monitors"][0]["rotation"], "normal");
        assert_eq!(value["monitors"][0]["connector"], "eDP-1");
        assert_eq!(value["monitors"][0]["display_name"], "Built-in display");
        assert_eq!(
            value["monitors"][0]["software_brightness"]["state"],
            "managed"
        );
        assert_eq!(
            value["monitors"][0]["software_brightness"]["brightness"],
            1.5
        );
        assert_eq!(
            value["monitors"][0]["software_brightness"]["filter"],
            "filmic"
        );
    }

    #[test]
    fn json_output_marks_unknown_brightness() {
        let output = build_json(
            &CommandOptions {
                connector: Some("eDP-1".to_string()),
                summary: false,
                json: true,
            },
            &sample_config(),
            |_connector| {
                Ok(CurrentBrightness {
                    state: CurrentBrightnessState::Unknown,
                    brightness: None,
                    filter: None,
                })
            },
        )
        .unwrap();

        let value: Value = serde_json::from_str(&output).unwrap();

        assert_eq!(value["logical_monitors"].as_array().unwrap().len(), 1);
        assert_eq!(value["monitors"].as_array().unwrap().len(), 1);
        assert_eq!(
            value["monitors"][0]["software_brightness"]["state"],
            "unknown"
        );
        assert!(value["monitors"][0]["software_brightness"]["brightness"].is_null());
        assert!(value["monitors"][0]["software_brightness"]["filter"].is_null());
    }
}
