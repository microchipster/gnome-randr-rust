use std::{
    collections::HashSet,
    env,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use gnome_randr::display_config::{
    logical_monitor::Transform,
    physical_monitor::{Mode, PhysicalMonitor},
    proxied_methods::{ApplyConfig, ApplyMonitor, ApplyMonitorProperty},
    DisplayConfig, LayoutMode,
};

#[derive(Debug)]
pub enum Error {
    MissingConfigHome,
    InvalidPath(PathBuf),
    MissingPhysicalMonitor { connector: String },
    MissingMode { connector: String, mode_id: String },
    Io { path: PathBuf, message: String },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::MissingConfigHome => write!(
                f,
                "fatal: unable to determine a configuration directory for monitors.xml. Set XDG_CONFIG_HOME or HOME."
            ),
            Error::InvalidPath(path) => write!(
                f,
                "fatal: monitors.xml path {} does not have a parent directory.",
                path.display()
            ),
            Error::MissingPhysicalMonitor { connector } => write!(
                f,
                "fatal: monitors.xml writer could not find physical monitor {} in the current config.",
                connector
            ),
            Error::MissingMode { connector, mode_id } => write!(
                f,
                "fatal: monitors.xml writer could not find mode {} for {}.",
                mode_id, connector
            ),
            Error::Io { path, message } => write!(
                f,
                "fatal: failed to write monitors.xml at {}: {}",
                path.display(),
                message
            ),
        }
    }
}

impl std::error::Error for Error {}

pub fn default_monitors_xml_path() -> Result<PathBuf, Error> {
    if let Some(config_home) = env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(config_home).join("monitors.xml"));
    }

    if let Some(home) = env::var_os("HOME") {
        return Ok(PathBuf::from(home).join(".config").join("monitors.xml"));
    }

    Err(Error::MissingConfigHome)
}

pub fn write_monitors_xml(
    config: &DisplayConfig,
    layout_mode: Option<LayoutMode>,
    configs: &[ApplyConfig<'_>],
) -> Result<PathBuf, Error> {
    let path = default_monitors_xml_path()?;
    let xml = render_monitors_xml(config, layout_mode, configs)?;
    let parent = path
        .parent()
        .ok_or_else(|| Error::InvalidPath(path.clone()))?;

    fs::create_dir_all(parent).map_err(|error| Error::Io {
        path: parent.to_path_buf(),
        message: error.to_string(),
    })?;

    let temp_path = path.with_extension("tmp");
    fs::write(&temp_path, xml).map_err(|error| Error::Io {
        path: temp_path.clone(),
        message: error.to_string(),
    })?;

    fs::rename(&temp_path, &path).map_err(|error| Error::Io {
        path: path.clone(),
        message: error.to_string(),
    })?;

    Ok(path)
}

pub(crate) fn render_monitors_xml(
    config: &DisplayConfig,
    layout_mode: Option<LayoutMode>,
    configs: &[ApplyConfig<'_>],
) -> Result<String, Error> {
    let mut xml = String::new();
    let layout_mode = layout_mode.unwrap_or(config.known_properties.layout_mode);
    let active_connectors = active_connectors(configs);

    writeln!(&mut xml, "<monitors version=\"2\">").unwrap();
    writeln!(&mut xml, "  <configuration>").unwrap();
    writeln!(&mut xml, "    <layoutmode>{}</layoutmode>", layout_mode).unwrap();

    for logical_monitor in configs {
        writeln!(&mut xml, "    <logicalmonitor>").unwrap();
        writeln!(&mut xml, "      <x>{}</x>", logical_monitor.x_pos).unwrap();
        writeln!(&mut xml, "      <y>{}</y>", logical_monitor.y_pos).unwrap();
        writeln!(
            &mut xml,
            "      <scale>{}</scale>",
            format_scale(logical_monitor.scale)
        )
        .unwrap();

        if logical_monitor.primary {
            writeln!(&mut xml, "      <primary>yes</primary>").unwrap();
        }

        let (rotation, flipped) = transform_to_xml(logical_monitor.transform);
        if rotation != "normal" || flipped != "no" {
            writeln!(&mut xml, "      <transform>").unwrap();
            writeln!(&mut xml, "        <rotation>{}</rotation>", rotation).unwrap();
            writeln!(&mut xml, "        <flipped>{}</flipped>", flipped).unwrap();
            writeln!(&mut xml, "      </transform>").unwrap();
        }

        for monitor in &logical_monitor.monitors {
            let physical_monitor = config.physical_monitor(monitor.connector).ok_or_else(|| {
                Error::MissingPhysicalMonitor {
                    connector: monitor.connector.to_string(),
                }
            })?;
            let mode = physical_monitor
                .modes
                .iter()
                .find(|mode| mode.id == monitor.mode_id)
                .ok_or_else(|| Error::MissingMode {
                    connector: monitor.connector.to_string(),
                    mode_id: monitor.mode_id.to_string(),
                })?;

            write_monitor(&mut xml, physical_monitor, mode, monitor);
        }

        writeln!(&mut xml, "    </logicalmonitor>").unwrap();
    }

    for monitor in config
        .monitors
        .iter()
        .filter(|monitor| !active_connectors.contains(monitor.connector.as_str()))
    {
        writeln!(&mut xml, "    <disabled>").unwrap();
        writeln!(&mut xml, "      <monitorspec>").unwrap();
        writeln!(
            &mut xml,
            "        <connector>{}</connector>",
            escape_xml(&monitor.connector)
        )
        .unwrap();
        writeln!(
            &mut xml,
            "        <vendor>{}</vendor>",
            escape_xml(&monitor.vendor)
        )
        .unwrap();
        writeln!(
            &mut xml,
            "        <product>{}</product>",
            escape_xml(&monitor.product)
        )
        .unwrap();
        writeln!(
            &mut xml,
            "        <serial>{}</serial>",
            escape_xml(&monitor.serial)
        )
        .unwrap();
        writeln!(&mut xml, "      </monitorspec>").unwrap();
        writeln!(&mut xml, "    </disabled>").unwrap();
    }

    writeln!(&mut xml, "  </configuration>").unwrap();
    writeln!(&mut xml, "</monitors>").unwrap();

    Ok(xml)
}

fn active_connectors<'a>(configs: &'a [ApplyConfig<'a>]) -> HashSet<&'a str> {
    let mut connectors = HashSet::new();
    for config in configs {
        for monitor in &config.monitors {
            connectors.insert(monitor.connector);
        }
    }
    connectors
}

fn write_monitor(
    xml: &mut String,
    physical_monitor: &PhysicalMonitor,
    mode: &Mode,
    monitor: &ApplyMonitor<'_>,
) {
    writeln!(xml, "      <monitor>").unwrap();
    writeln!(xml, "        <monitorspec>").unwrap();
    writeln!(
        xml,
        "          <connector>{}</connector>",
        escape_xml(&physical_monitor.connector)
    )
    .unwrap();
    writeln!(
        xml,
        "          <vendor>{}</vendor>",
        escape_xml(&physical_monitor.vendor)
    )
    .unwrap();
    writeln!(
        xml,
        "          <product>{}</product>",
        escape_xml(&physical_monitor.product)
    )
    .unwrap();
    writeln!(
        xml,
        "          <serial>{}</serial>",
        escape_xml(&physical_monitor.serial)
    )
    .unwrap();
    writeln!(xml, "        </monitorspec>").unwrap();
    writeln!(xml, "        <mode>").unwrap();
    writeln!(xml, "          <width>{}</width>", mode.width).unwrap();
    writeln!(xml, "          <height>{}</height>", mode.height).unwrap();
    writeln!(
        xml,
        "          <rate>{}</rate>",
        format_refresh(mode.refresh_rate)
    )
    .unwrap();
    writeln!(xml, "        </mode>").unwrap();

    for property in &monitor.properties {
        match property {
            ApplyMonitorProperty::ColorMode(color_mode) => {
                writeln!(
                    xml,
                    "        <color-mode>{}</color-mode>",
                    color_mode.as_str()
                )
                .unwrap();
            }
            ApplyMonitorProperty::RgbRange(rgb_range) => {
                writeln!(xml, "        <rgbrange>{}</rgbrange>", rgb_range.as_str()).unwrap();
            }
        }
    }

    writeln!(xml, "      </monitor>").unwrap();
}

fn transform_to_xml(transform: u32) -> (&'static str, &'static str) {
    let transform = Transform::from_bits_truncate(transform);

    let rotation = if transform.contains(Transform::R270) {
        "left"
    } else if transform.contains(Transform::R180) {
        "inverted"
    } else if transform.contains(Transform::R90) {
        "right"
    } else {
        "normal"
    };
    let flipped = if transform.contains(Transform::FLIPPED) {
        "yes"
    } else {
        "no"
    };

    (rotation, flipped)
}

fn escape_xml(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());

    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(ch),
        }
    }

    escaped
}

fn format_scale(scale: f64) -> String {
    let formatted = format!("{:.2}", scale);
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn format_refresh(refresh: f64) -> String {
    let formatted = format!("{:.3}", refresh);
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::render_monitors_xml;
    use gnome_randr::display_config::{
        logical_monitor::{LogicalMonitor, Monitor, Transform},
        physical_monitor::{KnownModeProperties, Mode, PhysicalMonitor},
        proxied_methods::{ApplyConfig, ApplyMonitor, ApplyMonitorProperty, ColorMode, RgbRange},
        DisplayConfig, KnownProperties, LayoutMode,
    };

    fn display_config() -> DisplayConfig {
        DisplayConfig {
            serial: 1,
            monitors: vec![
                PhysicalMonitor {
                    connector: "eDP-1".to_string(),
                    vendor: "AUO".to_string(),
                    product: "0x2992".to_string(),
                    serial: "0x00000000".to_string(),
                    modes: vec![Mode {
                        id: "1920x1080@60".to_string(),
                        width: 1920,
                        height: 1080,
                        refresh_rate: 60.0,
                        preferred_scale: 1.0,
                        supported_scales: vec![1.0],
                        known_properties: KnownModeProperties {
                            is_current: true,
                            is_preferred: true,
                        },
                        properties: Default::default(),
                    }],
                    properties: Default::default(),
                },
                PhysicalMonitor {
                    connector: "HDMI-1".to_string(),
                    vendor: "GSM".to_string(),
                    product: "0x4b77".to_string(),
                    serial: "0x000084ff".to_string(),
                    modes: vec![Mode {
                        id: "1440x900@59.887".to_string(),
                        width: 1440,
                        height: 900,
                        refresh_rate: 59.887,
                        preferred_scale: 1.0,
                        supported_scales: vec![1.0],
                        known_properties: KnownModeProperties {
                            is_current: true,
                            is_preferred: true,
                        },
                        properties: Default::default(),
                    }],
                    properties: Default::default(),
                },
            ],
            logical_monitors: vec![LogicalMonitor {
                x: 0,
                y: 0,
                scale: 1.0,
                transform: Transform::NORMAL,
                primary: true,
                monitors: vec![Monitor {
                    connector: "eDP-1".to_string(),
                    vendor: "AUO".to_string(),
                    product: "0x2992".to_string(),
                    serial: "0x00000000".to_string(),
                }],
                properties: Default::default(),
            }],
            known_properties: KnownProperties {
                supports_mirroring: true,
                layout_mode: LayoutMode::Logical,
                supports_changing_layout_mode: true,
                global_scale_required: false,
            },
            properties: Default::default(),
        }
    }

    fn configs() -> Vec<ApplyConfig<'static>> {
        vec![ApplyConfig {
            x_pos: 0,
            y_pos: 0,
            scale: 1.0,
            transform: Transform::NORMAL.bits(),
            primary: true,
            monitors: vec![ApplyMonitor {
                connector: "eDP-1",
                mode_id: "1920x1080@60",
                properties: vec![
                    ApplyMonitorProperty::ColorMode(ColorMode::SdrNative),
                    ApplyMonitorProperty::RgbRange(RgbRange::Limited),
                ],
            }],
        }]
    }

    #[test]
    fn renders_monitors_xml() {
        let xml =
            render_monitors_xml(&display_config(), Some(LayoutMode::Physical), &configs()).unwrap();

        assert!(xml.contains("<monitors version=\"2\">"));
        assert!(xml.contains("<layoutmode>physical</layoutmode>"));
        assert!(xml.contains("<primary>yes</primary>"));
        assert!(xml.contains("<color-mode>sdr-native</color-mode>"));
        assert!(xml.contains("<rgbrange>limited</rgbrange>"));
        assert!(xml.contains("<disabled>"));
        assert!(xml.contains("<connector>HDMI-1</connector>"));
    }
}
