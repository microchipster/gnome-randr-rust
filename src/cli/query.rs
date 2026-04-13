use std::{
    collections::BTreeMap,
    convert::TryFrom,
    fmt::{self, Write},
    ptr,
};

use dbus::arg::{ArgType, PropMap, RefArg};
use gnome_randr::{
    display_config::{
        logical_monitor::{LogicalMonitor, Transform},
        physical_monitor::{Mode, PhysicalMonitor},
        proxied_methods::ColorMode,
        resources::Resources,
        LayoutMode,
    },
    DisplayConfig,
};
use serde::Serialize;
use serde_json::Value;
use structopt::StructOpt;

use super::brightness;
use super::brightness::{CurrentColor, CurrentColorState};
use super::common::{format_refresh, format_scale};

const JSON_SCHEMA_VERSION: u32 = 5;
const DISPLAY_PROPERTY_KEYS: [&str; 1] = ["renderer"];
const MONITOR_PROPERTY_KEYS: [&str; 4] = ["display-name", "is-builtin", "width-mm", "height-mm"];

type PropertyJson = BTreeMap<String, Value>;

#[derive(StructOpt)]
pub struct CommandOptions {
    #[structopt(
        value_name = "CONNECTOR",
        help = "Connector such as eDP-1 or HDMI-1",
        long_help = "Connector name reported by \"gnome-randr query\", such as \"eDP-1\" or \"HDMI-1\". Omit it to inspect every connected output and logical monitor."
    )]
    pub connector: Option<String>,

    #[structopt(
        short,
        long,
        help = "Show one-line summaries instead of the default sections",
        long_help = "Show only the condensed text view. With no connector this prints one logical-monitor summary block per active monitor plus per-output enabled state, typed reflection/color-mode state, underscanning visibility, and current software brightness/gamma state. With a connector it prints that logical monitor summary plus the connector's typed monitor state and managed software color state."
    )]
    pub summary: bool,

    #[structopt(
        long,
        help = "Print structured JSON instead of text",
        long_help = "Print structured JSON using the documented schema in README.md. This includes logical monitors, typed rotation/reflection fields, physical monitors, typed color-mode and underscanning visibility, software brightness, software gamma, and raw D-Bus property maps for scripts."
    )]
    pub json: bool,

    #[structopt(
        long = "properties",
        alias = "prop",
        help = "Show raw Mutter property maps in text output",
        long_help = "Include raw Mutter property maps in the text UI. `--prop` is accepted as a short alias. This surfaces values such as underscanning or color-mode-related state using the same property names exposed in JSON output."
    )]
    pub properties: bool,

    #[structopt(
        long,
        help = "Show a more detailed inspection view",
        long_help = "Show a more detailed text inspection view with explicit field names that match the JSON schema, plus raw property maps where available."
    )]
    pub verbose: bool,

    #[structopt(
        long = "listmonitors",
        help = "List logical monitors in an xrandr-style view",
        long_help = "List logical monitors in a concise xrandr-style view showing geometry, primary status, and associated connectors."
    )]
    pub list_monitors: bool,

    #[structopt(
        long = "listactivemonitors",
        help = "List active logical monitors only",
        long_help = "List active logical monitors only. With the current Mutter DisplayConfig API this usually matches --listmonitors because the query surface is already active-monitor oriented."
    )]
    pub list_active_monitors: bool,
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
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    properties: PropertyJson,
    logical_monitors: Vec<LogicalMonitorJson>,
    monitors: Vec<PhysicalMonitorJson>,
}

#[derive(Serialize)]
struct LogicalMonitorJson {
    x: i32,
    y: i32,
    scale: f64,
    rotation: String,
    reflection: String,
    primary: bool,
    monitors: Vec<AssociatedMonitorJson>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    properties: PropertyJson,
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
    enabled: bool,
    vendor: String,
    product: String,
    serial: String,
    display_name: Option<String>,
    is_builtin: Option<bool>,
    width_mm: Option<i64>,
    height_mm: Option<i64>,
    color_mode: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    supported_color_modes: Vec<String>,
    is_underscanning: Option<bool>,
    modes: Vec<ModeJson>,
    software_brightness: SoftwareBrightnessJson,
    software_gamma: SoftwareGammaJson,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    properties: PropertyJson,
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
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    properties: PropertyJson,
}

#[derive(Serialize)]
struct SoftwareBrightnessJson {
    state: String,
    brightness: Option<f64>,
    filter: Option<String>,
}

#[derive(Serialize)]
struct SoftwareGammaJson {
    state: String,
    red: Option<f64>,
    green: Option<f64>,
    blue: Option<f64>,
}

type SelectedLogicalMonitor<'a> = (usize, &'a LogicalMonitor);

struct SelectedMonitors<'a> {
    logical_monitors: Vec<SelectedLogicalMonitor<'a>>,
    physical_monitors: Vec<&'a PhysicalMonitor>,
}

#[derive(Clone, Copy, Debug)]
enum TextView {
    Default,
    Summary,
    Verbose,
    ListMonitors,
    ListActiveMonitors,
}

#[derive(Clone, Copy, Debug)]
enum QueryView {
    Json,
    Text(TextView),
}

#[derive(Debug)]
pub enum Error {
    NotFound,
    ConflictingOptions {
        option: &'static str,
        conflicting: &'static str,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotFound => write!(f, "fatal: unable to find output."),
            Error::ConflictingOptions {
                option,
                conflicting,
            } => write!(f, "fatal: {} cannot be used with {}.", option, conflicting),
        }
    }
}

impl std::error::Error for Error {}

fn validate_options(opts: &CommandOptions) -> Result<QueryView, Error> {
    let views = [
        (
            opts.summary,
            "--summary",
            QueryView::Text(TextView::Summary),
        ),
        (opts.json, "--json", QueryView::Json),
        (
            opts.verbose,
            "--verbose",
            QueryView::Text(TextView::Verbose),
        ),
        (
            opts.list_monitors,
            "--listmonitors",
            QueryView::Text(TextView::ListMonitors),
        ),
        (
            opts.list_active_monitors,
            "--listactivemonitors",
            QueryView::Text(TextView::ListActiveMonitors),
        ),
    ];

    let mut active = views.iter().filter(|(enabled, _, _)| *enabled);
    if let Some((_, option, _)) = active.next() {
        if let Some((_, conflicting, _)) = active.next() {
            return Err(Error::ConflictingOptions {
                option,
                conflicting,
            });
        }
    }

    if opts.properties
        && (opts.summary || opts.json || opts.list_monitors || opts.list_active_monitors)
    {
        let conflicting = if opts.summary {
            "--summary"
        } else if opts.json {
            "--json"
        } else if opts.list_monitors {
            "--listmonitors"
        } else {
            "--listactivemonitors"
        };

        return Err(Error::ConflictingOptions {
            option: "--properties",
            conflicting,
        });
    }

    Ok(views
        .iter()
        .find(|(enabled, _, _)| *enabled)
        .map(|(_, _, view)| *view)
        .unwrap_or(QueryView::Text(TextView::Default)))
}

fn property_string(properties: &PropMap, key: &str) -> Option<String> {
    properties
        .get(key)
        .and_then(|value| value.0.as_str())
        .map(str::to_owned)
}

fn property_bool(properties: &PropMap, key: &str) -> Option<bool> {
    match properties
        .get(key)
        .and_then(|value| match value.0.arg_type() {
            ArgType::Boolean => value.0.as_u64().map(|flag| flag != 0),
            _ => value.0.as_u64().map(|flag| flag != 0),
        }) {
        Some(value) => Some(value),
        None => None,
    }
}

fn property_i64(properties: &PropMap, key: &str) -> Option<i64> {
    properties.get(key).and_then(|value| {
        value
            .0
            .as_i64()
            .or_else(|| value.0.as_u64().map(|value| value as i64))
    })
}

fn property_u32(properties: &PropMap, key: &str) -> Option<u32> {
    properties
        .get(key)
        .and_then(|value| value.0.as_u64())
        .and_then(|value| u32::try_from(value).ok())
}

fn color_mode(properties: &PropMap) -> Option<ColorMode> {
    property_u32(properties, "color-mode").and_then(ColorMode::from_raw)
}

fn supported_color_modes(properties: &PropMap) -> Vec<ColorMode> {
    properties
        .get("supported-color-modes")
        .and_then(|value| value.0.as_iter())
        .map(|iter| {
            iter.filter_map(|entry| {
                entry
                    .as_u64()
                    .and_then(|value| u32::try_from(value).ok())
                    .and_then(ColorMode::from_raw)
            })
            .collect()
        })
        .unwrap_or_default()
}

fn rotation_from_transform(transform: Transform) -> &'static str {
    match transform.bits() & Transform::R270.bits() {
        bits if bits == Transform::R90.bits() => "right",
        bits if bits == Transform::R180.bits() => "inverted",
        bits if bits == Transform::R270.bits() => "left",
        _ => "normal",
    }
}

fn reflection_from_transform(transform: Transform) -> &'static str {
    if transform.bits() & Transform::FLIPPED.bits() == 0 {
        return "normal";
    }

    match transform.bits() & Transform::R270.bits() {
        bits if bits == Transform::R180.bits() || bits == Transform::R270.bits() => "x",
        _ => "y",
    }
}

fn format_color_mode(color_mode: Option<ColorMode>) -> String {
    color_mode
        .map(|color_mode| color_mode.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn format_supported_color_modes(modes: &[ColorMode]) -> String {
    format!(
        "[{}]",
        modes
            .iter()
            .map(|mode| mode.to_string())
            .collect::<Vec<String>>()
            .join(", ")
    )
}

fn property_json_value(value: &dyn RefArg) -> Value {
    match value.arg_type() {
        ArgType::Boolean => Value::Bool(value.as_u64().unwrap_or(0) != 0),
        ArgType::Byte | ArgType::Int16 | ArgType::Int32 | ArgType::Int64 | ArgType::UnixFd => {
            Value::from(value.as_i64().unwrap_or_default())
        }
        ArgType::UInt16 | ArgType::UInt32 | ArgType::UInt64 => {
            Value::from(value.as_u64().unwrap_or_default())
        }
        ArgType::Double => serde_json::Number::from_f64(value.as_f64().unwrap_or_default())
            .map(Value::Number)
            .unwrap_or(Value::Null),
        ArgType::String | ArgType::ObjectPath | ArgType::Signature => {
            Value::String(value.as_str().unwrap_or_default().to_string())
        }
        ArgType::Array | ArgType::Struct | ArgType::DictEntry => Value::Array(
            value
                .as_iter()
                .map(|iter| iter.map(property_json_value).collect())
                .unwrap_or_default(),
        ),
        ArgType::Variant => value
            .as_iter()
            .and_then(|mut iter| iter.next().map(property_json_value))
            .unwrap_or(Value::Null),
        ArgType::Invalid => Value::Null,
    }
}

fn filtered_properties_json(properties: &PropMap, excluded: &[&str]) -> PropertyJson {
    properties
        .iter()
        .filter(|(key, _)| !excluded.iter().any(|candidate| key == candidate))
        .map(|(key, value)| (key.clone(), property_json_value(value.0.as_ref())))
        .collect()
}

fn software_brightness_json(current: &CurrentColor) -> SoftwareBrightnessJson {
    SoftwareBrightnessJson {
        state: match current.state {
            CurrentColorState::Managed => "managed",
            CurrentColorState::Identity => "identity",
            CurrentColorState::Unknown => "unknown",
        }
        .to_string(),
        brightness: current.brightness,
        filter: current.filter.map(|filter| filter.to_string()),
    }
}

fn software_gamma_json(current: &CurrentColor) -> SoftwareGammaJson {
    SoftwareGammaJson {
        state: match current.state {
            CurrentColorState::Managed => "managed",
            CurrentColorState::Identity => "identity",
            CurrentColorState::Unknown => "unknown",
        }
        .to_string(),
        red: current.gamma_adjustment.map(|gamma| gamma.red),
        green: current.gamma_adjustment.map(|gamma| gamma.green),
        blue: current.gamma_adjustment.map(|gamma| gamma.blue),
    }
}

fn selected_monitors<'a>(
    opts: &CommandOptions,
    config: &'a DisplayConfig,
) -> Result<SelectedMonitors<'a>, Error> {
    match &opts.connector {
        Some(connector) => {
            let physical_monitor = config.physical_monitor(connector).ok_or(Error::NotFound)?;
            let logical_monitors = config
                .logical_monitor_for_connector(connector)
                .map(|logical_monitor| {
                    vec![(
                        config
                            .logical_monitor_index_for_connector(connector)
                            .unwrap_or_else(|| {
                                config
                                    .logical_monitors
                                    .iter()
                                    .position(|candidate| ptr::eq(candidate, logical_monitor))
                                    .unwrap_or(0)
                            }),
                        logical_monitor,
                    )]
                })
                .unwrap_or_default();
            Ok(SelectedMonitors {
                logical_monitors,
                physical_monitors: vec![physical_monitor],
            })
        }
        None => Ok(SelectedMonitors {
            logical_monitors: config.logical_monitors.iter().enumerate().collect(),
            physical_monitors: config.monitors.iter().collect(),
        }),
    }
}

fn monitor_enabled(config: &DisplayConfig, connector: &str) -> bool {
    config.logical_monitor_for_connector(connector).is_some()
}

fn current_mode(monitor: &PhysicalMonitor) -> Option<&Mode> {
    monitor
        .modes
        .iter()
        .find(|mode| mode.known_properties.is_current)
}

fn preferred_mode(monitor: &PhysicalMonitor) -> Option<&Mode> {
    monitor
        .modes
        .iter()
        .find(|mode| mode.known_properties.is_preferred)
}

fn current_mode_geometry(
    logical_monitor: &LogicalMonitor,
    config: &DisplayConfig,
) -> (i32, i32, i64, i64) {
    let mut width_px = 0;
    let mut height_px = 0;
    let mut width_mm = 0;
    let mut height_mm = 0;

    for associated in &logical_monitor.monitors {
        if let Some(monitor) = config.physical_monitor(&associated.connector) {
            if let Some(mode) = current_mode(monitor).or_else(|| preferred_mode(monitor)) {
                width_px = width_px.max(mode.width);
                height_px = height_px.max(mode.height);
            }

            width_mm = width_mm.max(property_i64(&monitor.properties, "width-mm").unwrap_or(0));
            height_mm = height_mm.max(property_i64(&monitor.properties, "height-mm").unwrap_or(0));
        }
    }

    let scale = match config.known_properties.layout_mode {
        LayoutMode::Logical if logical_monitor.scale > 0.0 => logical_monitor.scale,
        _ => 1.0,
    };

    let mut width = ((width_px as f64) / scale).round() as i32;
    let mut height = ((height_px as f64) / scale).round() as i32;
    let rotated = logical_monitor.transform.bits() & Transform::R90.bits() != 0;
    if rotated {
        std::mem::swap(&mut width, &mut height);
        std::mem::swap(&mut width_mm, &mut height_mm);
    }

    (width, height, width_mm, height_mm)
}

fn format_property_value(value: &Value) -> String {
    serde_json::to_string(value).expect("serializing JSON value")
}

fn write_properties(output: &mut String, indent: usize, properties: &PropertyJson) -> fmt::Result {
    if properties.is_empty() {
        return Ok(());
    }

    writeln!(output, "{:indent$}properties:", "", indent = indent)?;
    for (key, value) in properties {
        writeln!(
            output,
            "{:indent$}{}: {}",
            "",
            key,
            format_property_value(value),
            indent = indent + 2
        )?;
    }

    Ok(())
}

fn write_display_section(
    output: &mut String,
    config: &DisplayConfig,
    show_properties: bool,
) -> fmt::Result {
    writeln!(output, "display:")?;
    writeln!(output, "  serial: {}", config.serial)?;
    writeln!(
        output,
        "  layout_mode: {}",
        config.known_properties.layout_mode
    )?;
    writeln!(
        output,
        "  supports_mirroring: {}",
        config.known_properties.supports_mirroring
    )?;
    writeln!(
        output,
        "  supports_changing_layout_mode: {}",
        config.known_properties.supports_changing_layout_mode
    )?;
    writeln!(
        output,
        "  global_scale_required: {}",
        config.known_properties.global_scale_required
    )?;

    if let Some(renderer) = property_string(&config.properties, "renderer") {
        writeln!(output, "  renderer: {}", renderer)?;
    }

    if show_properties {
        write_properties(
            output,
            2,
            &filtered_properties_json(&config.properties, &DISPLAY_PROPERTY_KEYS),
        )?;
    }

    Ok(())
}

fn logical_monitor_summary(logical_monitor: &LogicalMonitor) -> String {
    format!(
        "x={} y={} scale={} rotation={} reflection={} primary={} connectors={}",
        logical_monitor.x,
        logical_monitor.y,
        format_scale(logical_monitor.scale),
        rotation_from_transform(logical_monitor.transform),
        reflection_from_transform(logical_monitor.transform),
        if logical_monitor.primary { "yes" } else { "no" },
        logical_monitor
            .monitors
            .iter()
            .map(|monitor| monitor.connector.clone())
            .collect::<Vec<String>>()
            .join(",")
    )
}

fn write_logical_monitors(
    output: &mut String,
    logical_monitors: &[SelectedLogicalMonitor<'_>],
    verbose: bool,
    show_properties: bool,
) -> fmt::Result {
    writeln!(output, "logical monitors:")?;

    for (index, logical_monitor) in logical_monitors {
        if verbose {
            writeln!(output, "  {}:", index)?;
            writeln!(output, "    x: {}", logical_monitor.x)?;
            writeln!(output, "    y: {}", logical_monitor.y)?;
            writeln!(output, "    scale: {}", format_scale(logical_monitor.scale))?;
            writeln!(
                output,
                "    rotation: {}",
                rotation_from_transform(logical_monitor.transform)
            )?;
            writeln!(
                output,
                "    reflection: {}",
                reflection_from_transform(logical_monitor.transform)
            )?;
            writeln!(output, "    primary: {}", logical_monitor.primary)?;
            writeln!(output, "    monitors:")?;
            for monitor in &logical_monitor.monitors {
                writeln!(output, "      - connector: {}", monitor.connector)?;
                writeln!(output, "        vendor: {}", monitor.vendor)?;
                writeln!(output, "        product: {}", monitor.product)?;
                writeln!(output, "        serial: {}", monitor.serial)?;
            }
            if show_properties {
                write_properties(
                    output,
                    4,
                    &filtered_properties_json(&logical_monitor.properties, &[]),
                )?;
            }
        } else {
            writeln!(
                output,
                "  {}: {}",
                index,
                logical_monitor_summary(logical_monitor)
            )?;
            if show_properties {
                write_properties(
                    output,
                    4,
                    &filtered_properties_json(&logical_monitor.properties, &[]),
                )?;
            }
        }
    }

    Ok(())
}

fn write_mode_table(output: &mut String, modes: &[Mode]) -> fmt::Result {
    for mode in modes {
        writeln!(output, "      {}", mode)?;
    }

    Ok(())
}

fn write_verbose_modes(output: &mut String, modes: &[Mode], show_properties: bool) -> fmt::Result {
    writeln!(output, "    modes:")?;
    for mode in modes {
        writeln!(output, "      - id: {}", mode.id)?;
        writeln!(output, "        width: {}", mode.width)?;
        writeln!(output, "        height: {}", mode.height)?;
        writeln!(
            output,
            "        refresh_rate: {}",
            format_refresh(mode.refresh_rate)
        )?;
        writeln!(
            output,
            "        preferred_scale: {}",
            format_scale(mode.preferred_scale)
        )?;
        writeln!(
            output,
            "        supported_scales: [{}]",
            mode.supported_scales
                .iter()
                .map(|scale| format_scale(*scale))
                .collect::<Vec<String>>()
                .join(", ")
        )?;
        writeln!(
            output,
            "        is_current: {}",
            mode.known_properties.is_current
        )?;
        writeln!(
            output,
            "        is_preferred: {}",
            mode.known_properties.is_preferred
        )?;
        if show_properties {
            write_properties(output, 8, &filtered_properties_json(&mode.properties, &[]))?;
        }
    }

    Ok(())
}

fn write_monitors<F>(
    output: &mut String,
    config: &DisplayConfig,
    monitors: &[&PhysicalMonitor],
    color_for: &F,
    verbose: bool,
    show_properties: bool,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: Fn(&str) -> Result<CurrentColor, Box<dyn std::error::Error>>,
{
    writeln!(output, "monitors:")?;

    for monitor in monitors {
        writeln!(output, "  {}:", monitor.connector)?;
        writeln!(output, "    connector: {}", monitor.connector)?;
        writeln!(
            output,
            "    enabled: {}",
            monitor_enabled(config, &monitor.connector)
        )?;
        writeln!(output, "    vendor: {}", monitor.vendor)?;
        writeln!(output, "    product: {}", monitor.product)?;
        writeln!(output, "    serial: {}", monitor.serial)?;
        if let Some(display_name) = property_string(&monitor.properties, "display-name") {
            writeln!(output, "    display_name: {}", display_name)?;
        }
        writeln!(
            output,
            "    is_builtin: {}",
            property_bool(&monitor.properties, "is-builtin")
                .map(|value| value.to_string())
                .unwrap_or_else(|| "null".to_string())
        )?;
        writeln!(
            output,
            "    width_mm: {}",
            property_i64(&monitor.properties, "width-mm")
                .map(|value| value.to_string())
                .unwrap_or_else(|| "null".to_string())
        )?;
        writeln!(
            output,
            "    height_mm: {}",
            property_i64(&monitor.properties, "height-mm")
                .map(|value| value.to_string())
                .unwrap_or_else(|| "null".to_string())
        )?;
        writeln!(
            output,
            "    color_mode: {}",
            format_color_mode(color_mode(&monitor.properties))
        )?;
        writeln!(
            output,
            "    supported_color_modes: {}",
            format_supported_color_modes(&supported_color_modes(&monitor.properties))
        )?;
        writeln!(
            output,
            "    is_underscanning: {}",
            property_bool(&monitor.properties, "is-underscanning")
                .map(|value| value.to_string())
                .unwrap_or_else(|| "null".to_string())
        )?;
        if let Some(mode) = current_mode(monitor) {
            writeln!(output, "    current_mode: {}", mode.id)?;
        }
        if let Some(mode) = preferred_mode(monitor) {
            writeln!(output, "    preferred_mode: {}", mode.id)?;
        }
        writeln!(
            output,
            "    software_brightness: {}",
            color_for(&monitor.connector)?.brightness_display()
        )?;
        writeln!(
            output,
            "    software_gamma: {}",
            color_for(&monitor.connector)?.gamma_display()
        )?;

        if verbose {
            write_verbose_modes(output, &monitor.modes, show_properties)?;
        } else {
            writeln!(output, "    modes:")?;
            write_mode_table(output, &monitor.modes)?;
            if show_properties {
                for mode in &monitor.modes {
                    let properties = filtered_properties_json(&mode.properties, &[]);
                    if properties.is_empty() {
                        continue;
                    }

                    writeln!(output, "    mode_properties[{}]:", mode.id)?;
                    write_properties(output, 6, &properties)?;
                }
            }
        }

        if show_properties {
            write_properties(
                output,
                4,
                &filtered_properties_json(&monitor.properties, &MONITOR_PROPERTY_KEYS),
            )?;
        }
    }

    Ok(())
}

fn build_list_monitors(
    config: &DisplayConfig,
    logical_monitors: &[SelectedLogicalMonitor<'_>],
) -> Result<String, Box<dyn std::error::Error>> {
    let mut output = String::new();
    writeln!(&mut output, "Monitors: {}", logical_monitors.len())?;

    for (index, logical_monitor) in logical_monitors {
        let name = logical_monitor
            .monitors
            .iter()
            .map(|monitor| monitor.connector.clone())
            .collect::<Vec<String>>()
            .join("+");
        let connectors = logical_monitor
            .monitors
            .iter()
            .map(|monitor| monitor.connector.clone())
            .collect::<Vec<String>>()
            .join(" ");
        let (width, height, width_mm, height_mm) = current_mode_geometry(logical_monitor, config);
        writeln!(
            &mut output,
            " {}: +{}{} {}/{}x{}/{}+{}+{}  {}",
            index,
            if logical_monitor.primary { "*" } else { "" },
            name,
            width,
            width_mm,
            height,
            height_mm,
            logical_monitor.x,
            logical_monitor.y,
            connectors
        )?;
    }

    Ok(output)
}

fn build_json<F>(
    opts: &CommandOptions,
    config: &DisplayConfig,
    color_for: &F,
) -> Result<String, Box<dyn std::error::Error>>
where
    F: Fn(&str) -> Result<CurrentColor, Box<dyn std::error::Error>>,
{
    let selected = selected_monitors(opts, config)?;

    let json = QueryJson {
        schema_version: JSON_SCHEMA_VERSION,
        serial: config.serial,
        layout_mode: config.known_properties.layout_mode.to_string(),
        supports_mirroring: config.known_properties.supports_mirroring,
        supports_changing_layout_mode: config.known_properties.supports_changing_layout_mode,
        global_scale_required: config.known_properties.global_scale_required,
        renderer: property_string(&config.properties, "renderer"),
        properties: filtered_properties_json(&config.properties, &DISPLAY_PROPERTY_KEYS),
        logical_monitors: selected
            .logical_monitors
            .into_iter()
            .map(|(_, monitor)| LogicalMonitorJson {
                x: monitor.x,
                y: monitor.y,
                scale: monitor.scale,
                rotation: rotation_from_transform(monitor.transform).to_string(),
                reflection: reflection_from_transform(monitor.transform).to_string(),
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
                properties: filtered_properties_json(&monitor.properties, &[]),
            })
            .collect(),
        monitors: selected
            .physical_monitors
            .into_iter()
            .map(|monitor| {
                Ok(PhysicalMonitorJson {
                    connector: monitor.connector.clone(),
                    enabled: monitor_enabled(config, &monitor.connector),
                    vendor: monitor.vendor.clone(),
                    product: monitor.product.clone(),
                    serial: monitor.serial.clone(),
                    display_name: property_string(&monitor.properties, "display-name"),
                    is_builtin: property_bool(&monitor.properties, "is-builtin"),
                    width_mm: property_i64(&monitor.properties, "width-mm"),
                    height_mm: property_i64(&monitor.properties, "height-mm"),
                    color_mode: color_mode(&monitor.properties).map(|mode| mode.to_string()),
                    supported_color_modes: supported_color_modes(&monitor.properties)
                        .into_iter()
                        .map(|mode| mode.to_string())
                        .collect(),
                    is_underscanning: property_bool(&monitor.properties, "is-underscanning"),
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
                            properties: filtered_properties_json(&mode.properties, &[]),
                        })
                        .collect(),
                    software_brightness: software_brightness_json(&color_for(&monitor.connector)?),
                    software_gamma: software_gamma_json(&color_for(&monitor.connector)?),
                    properties: filtered_properties_json(
                        &monitor.properties,
                        &MONITOR_PROPERTY_KEYS,
                    ),
                })
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?,
    };

    Ok(serde_json::to_string_pretty(&json)?)
}

fn build_summary_text<F>(
    opts: &CommandOptions,
    config: &DisplayConfig,
    color_for: &F,
) -> Result<String, Box<dyn std::error::Error>>
where
    F: Fn(&str) -> Result<CurrentColor, Box<dyn std::error::Error>>,
{
    let format_brightness = |connector: &str| -> Result<String, Box<dyn std::error::Error>> {
        Ok(color_for(connector)?.brightness_display())
    };
    let format_gamma = |connector: &str| -> Result<String, Box<dyn std::error::Error>> {
        Ok(color_for(connector)?.gamma_display())
    };
    let format_monitor_summary =
        |monitor: &PhysicalMonitor| -> Result<String, Box<dyn std::error::Error>> {
            Ok(format!(
            "enabled={}, color_mode={}, supported_color_modes={}, is_underscanning={}, software brightness={}, software gamma={}",
            monitor_enabled(config, &monitor.connector),
            format_color_mode(color_mode(&monitor.properties)),
            format_supported_color_modes(&supported_color_modes(&monitor.properties)),
            property_bool(&monitor.properties, "is-underscanning")
                .map(|value| value.to_string())
                .unwrap_or_else(|| "null".to_string()),
            format_brightness(&monitor.connector)?,
            format_gamma(&monitor.connector)?,
            ))
        };

    Ok(match &opts.connector {
        Some(connector) => {
            let physical_monitor = config.physical_monitor(connector).ok_or(Error::NotFound)?;
            let mut output = String::new();
            if let Some(logical_monitor) = config.logical_monitor_for_connector(connector) {
                writeln!(&mut output, "{}", logical_monitor_summary(logical_monitor))?;
            }
            writeln!(&mut output, "connector: {}", physical_monitor.connector)?;
            writeln!(&mut output, "{}", format_monitor_summary(physical_monitor)?)?;
            output
        }
        None => {
            let mut output = String::new();
            for (_, logical_monitor) in config.logical_monitors.iter().enumerate() {
                writeln!(&mut output, "{}", logical_monitor_summary(logical_monitor))?;
            }

            writeln!(&mut output, "outputs:")?;
            for monitor in &config.monitors {
                writeln!(
                    &mut output,
                    "\t{}: {}",
                    monitor.connector,
                    format_monitor_summary(monitor)?
                )?;
            }

            output
        }
    })
}

fn build_structured_text<F>(
    opts: &CommandOptions,
    config: &DisplayConfig,
    color_for: &F,
    verbose: bool,
    show_properties: bool,
) -> Result<String, Box<dyn std::error::Error>>
where
    F: Fn(&str) -> Result<CurrentColor, Box<dyn std::error::Error>>,
{
    let selected = selected_monitors(opts, config)?;
    let mut output = String::new();

    if opts.connector.is_none() || verbose {
        write_display_section(&mut output, config, show_properties)?;
        output.push('\n');
    }

    if opts.connector.is_none() || !selected.logical_monitors.is_empty() || verbose {
        write_logical_monitors(
            &mut output,
            &selected.logical_monitors,
            verbose,
            show_properties,
        )?;
        output.push('\n');
    }

    write_monitors(
        &mut output,
        config,
        &selected.physical_monitors,
        color_for,
        verbose,
        show_properties,
    )?;

    Ok(output)
}

fn build_text<F>(
    opts: &CommandOptions,
    config: &DisplayConfig,
    color_for: &F,
    view: TextView,
) -> Result<String, Box<dyn std::error::Error>>
where
    F: Fn(&str) -> Result<CurrentColor, Box<dyn std::error::Error>>,
{
    match view {
        TextView::Default => build_structured_text(opts, config, color_for, false, opts.properties),
        TextView::Summary => build_summary_text(opts, config, color_for),
        TextView::Verbose => build_structured_text(opts, config, color_for, true, true),
        TextView::ListMonitors | TextView::ListActiveMonitors => {
            let selected = selected_monitors(opts, config)?;
            build_list_monitors(config, &selected.logical_monitors)
        }
    }
}

pub fn handle(
    opts: &CommandOptions,
    config: &DisplayConfig,
    proxy: &dbus::blocking::Proxy<&dbus::blocking::Connection>,
) -> Result<String, Box<dyn std::error::Error>> {
    let view = validate_options(opts)?;
    let resources = Resources::get_resources(proxy)?;

    let color_for =
        |connector: &str| match brightness::load_current_color(connector, &resources, proxy) {
            Ok(current) => Ok(current),
            Err(error)
                if error
                    .downcast_ref::<brightness::Error>()
                    .map(|error| {
                        matches!(
                            error,
                            brightness::Error::OutputDisabled | brightness::Error::CrtcNotFound
                        )
                    })
                    .unwrap_or(false) =>
            {
                Ok(CurrentColor::unknown())
            }
            Err(error) => Err(error),
        };

    match view {
        QueryView::Json => build_json(opts, config, &color_for),
        QueryView::Text(view) => build_text(opts, config, &color_for, view),
    }
}

#[cfg(test)]
mod tests {
    use super::{build_json, build_text, validate_options, CommandOptions, QueryView, TextView};
    use crate::cli::brightness::{CurrentColor, CurrentColorState};
    use gnome_randr::display_config::proxied_methods::{BrightnessFilter, GammaAdjustment};
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
        properties.insert(
            "compositor-capabilities".to_string(),
            dbus::arg::Variant(Box::new(vec!["gamma".to_string(), "clone".to_string()])),
        );

        let mut monitor_properties = dbus::arg::PropMap::new();
        monitor_properties.insert(
            "display-name".to_string(),
            dbus::arg::Variant(Box::new("Built-in display".to_string())),
        );
        monitor_properties.insert("is-builtin".to_string(), dbus::arg::Variant(Box::new(true)));
        monitor_properties.insert(
            "is-underscanning".to_string(),
            dbus::arg::Variant(Box::new(false)),
        );
        monitor_properties.insert(
            "supported-color-modes".to_string(),
            dbus::arg::Variant(Box::new(vec![0u32, 1u32])),
        );
        monitor_properties.insert("color-mode".to_string(), dbus::arg::Variant(Box::new(1u32)));
        monitor_properties.insert("width-mm".to_string(), dbus::arg::Variant(Box::new(300i32)));
        monitor_properties.insert(
            "height-mm".to_string(),
            dbus::arg::Variant(Box::new(190i32)),
        );

        let mut mode_properties = dbus::arg::PropMap::new();
        mode_properties.insert(
            "color-space".to_string(),
            dbus::arg::Variant(Box::new("srgb".to_string())),
        );

        let mut logical_properties = dbus::arg::PropMap::new();
        logical_properties.insert(
            "presentation".to_string(),
            dbus::arg::Variant(Box::new(false)),
        );

        DisplayConfig {
            serial: 7,
            monitors: vec![
                PhysicalMonitor {
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
                        properties: mode_properties,
                    }],
                    properties: monitor_properties,
                },
                PhysicalMonitor {
                    connector: "HDMI-1".to_string(),
                    vendor: "Dell".to_string(),
                    product: "U2720Q".to_string(),
                    serial: "0x11111111".to_string(),
                    modes: vec![Mode {
                        id: "2560x1440@60".to_string(),
                        width: 2560,
                        height: 1440,
                        refresh_rate: 60.0,
                        preferred_scale: 1.0,
                        supported_scales: vec![1.0],
                        known_properties: KnownModeProperties {
                            is_current: false,
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
                    vendor: "BOE".to_string(),
                    product: "0x07c9".to_string(),
                    serial: "0x00000000".to_string(),
                }],
                properties: logical_properties,
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

    fn opts() -> CommandOptions {
        CommandOptions {
            connector: None,
            summary: false,
            json: false,
            properties: false,
            verbose: false,
            list_monitors: false,
            list_active_monitors: false,
        }
    }

    #[test]
    fn json_output_uses_documented_schema() {
        let output = build_json(
            &CommandOptions {
                json: true,
                ..opts()
            },
            &sample_config(),
            &|_connector| {
                Ok(CurrentColor::managed(
                    1.5,
                    "filmic".parse().unwrap(),
                    GammaAdjustment {
                        red: 1.1,
                        green: 1.0,
                        blue: 0.9,
                    },
                ))
            },
        )
        .unwrap();

        let value: Value = serde_json::from_str(&output).unwrap();

        assert_eq!(value["schema_version"], 5);
        assert_eq!(value["renderer"], "native");
        assert_eq!(value["properties"]["compositor-capabilities"][0], "gamma");
        assert_eq!(value["logical_monitors"][0]["rotation"], "normal");
        assert_eq!(value["logical_monitors"][0]["reflection"], "normal");
        assert_eq!(
            value["logical_monitors"][0]["properties"]["presentation"],
            false
        );
        assert_eq!(value["monitors"][0]["connector"], "eDP-1");
        assert_eq!(value["monitors"][0]["enabled"], true);
        assert_eq!(value["monitors"][0]["display_name"], "Built-in display");
        assert_eq!(value["monitors"][0]["color_mode"], "bt2100");
        assert_eq!(value["monitors"][0]["supported_color_modes"][0], "default");
        assert_eq!(value["monitors"][0]["supported_color_modes"][1], "bt2100");
        assert_eq!(value["monitors"][0]["is_underscanning"], false);
        assert_eq!(
            value["monitors"][0]["properties"]["is-underscanning"],
            false
        );
        assert_eq!(
            value["monitors"][0]["modes"][0]["properties"]["color-space"],
            "srgb"
        );
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
        assert_eq!(value["monitors"][0]["software_gamma"]["red"], 1.1);
        assert_eq!(value["monitors"][0]["software_gamma"]["green"], 1.0);
        assert_eq!(value["monitors"][0]["software_gamma"]["blue"], 0.9);
        assert_eq!(value["monitors"][1]["connector"], "HDMI-1");
        assert_eq!(value["monitors"][1]["enabled"], false);
    }

    #[test]
    fn json_output_marks_unknown_brightness() {
        let output = build_json(
            &CommandOptions {
                connector: Some("eDP-1".to_string()),
                json: true,
                ..opts()
            },
            &sample_config(),
            &|_connector| {
                Ok(CurrentColor {
                    state: CurrentColorState::Unknown,
                    brightness: None,
                    filter: None,
                    gamma_adjustment: None,
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
        assert!(value["monitors"][0]["software_gamma"]["red"].is_null());
        assert!(value["monitors"][0]["software_gamma"]["green"].is_null());
        assert!(value["monitors"][0]["software_gamma"]["blue"].is_null());
    }

    #[test]
    fn json_output_keeps_disabled_connector_visible() {
        let output = build_json(
            &CommandOptions {
                connector: Some("HDMI-1".to_string()),
                json: true,
                ..opts()
            },
            &sample_config(),
            &|_connector| Ok(CurrentColor::unknown()),
        )
        .unwrap();

        let value: Value = serde_json::from_str(&output).unwrap();

        assert_eq!(value["logical_monitors"].as_array().unwrap().len(), 0);
        assert_eq!(value["monitors"].as_array().unwrap().len(), 1);
        assert_eq!(value["monitors"][0]["connector"], "HDMI-1");
        assert_eq!(value["monitors"][0]["enabled"], false);
    }

    #[test]
    fn properties_text_includes_raw_property_sections() {
        let output = build_text(
            &CommandOptions {
                properties: true,
                ..opts()
            },
            &sample_config(),
            &|_connector| Ok(CurrentColor::identity()),
            TextView::Default,
        )
        .unwrap();

        assert!(output.contains("properties:"));
        assert!(output.contains("is-underscanning: false"));
        assert!(output.contains("supported-color-modes: [0,1]"));
        assert!(output.contains("mode_properties[1920x1080@60]:"));
        assert!(output.contains("color-space: \"srgb\""));
        assert!(output.contains("HDMI-1:"));
        assert!(output.contains("enabled: false"));
    }

    #[test]
    fn verbose_text_uses_json_style_field_names() {
        let output = build_text(
            &CommandOptions {
                verbose: true,
                connector: Some("eDP-1".to_string()),
                ..opts()
            },
            &sample_config(),
            &|_connector| {
                Ok(CurrentColor::managed(
                    1.25,
                    BrightnessFilter::Gamma,
                    GammaAdjustment {
                        red: 1.2,
                        green: 1.1,
                        blue: 1.0,
                    },
                ))
            },
            TextView::Verbose,
        )
        .unwrap();

        assert!(output.contains("display:"));
        assert!(output.contains("logical monitors:"));
        assert!(output.contains("display_name: Built-in display"));
        assert!(output.contains("reflection: normal"));
        assert!(output.contains("color_mode: bt2100"));
        assert!(output.contains("supported_color_modes: [default, bt2100]"));
        assert!(output.contains("is_underscanning: false"));
        assert!(output.contains("software_brightness: 1.25 (gamma)"));
        assert!(output.contains("software_gamma: 1.2:1.1:1"));
        assert!(output.contains("refresh_rate: 60"));
        assert!(output.contains("is_current: true"));
    }

    #[test]
    fn disabled_connector_text_query_stays_queryable() {
        let output = build_text(
            &CommandOptions {
                connector: Some("HDMI-1".to_string()),
                ..opts()
            },
            &sample_config(),
            &|_connector| Ok(CurrentColor::unknown()),
            TextView::Default,
        )
        .unwrap();

        assert!(!output.contains("logical monitors:"));
        assert!(output.contains("connector: HDMI-1"));
        assert!(output.contains("enabled: false"));
        assert!(output.contains("software_brightness: unknown"));
        assert!(output.contains("software_gamma: unknown"));
    }

    #[test]
    fn summary_output_reports_enabled_state() {
        let output = build_text(
            &CommandOptions {
                summary: true,
                ..opts()
            },
            &sample_config(),
            &|connector| {
                if connector == "eDP-1" {
                    Ok(CurrentColor::identity())
                } else {
                    Ok(CurrentColor::unknown())
                }
            },
            TextView::Summary,
        )
        .unwrap();

        assert!(output.contains("outputs:"));
        assert!(output.contains(
            "eDP-1: enabled=true, color_mode=bt2100, supported_color_modes=[default, bt2100], is_underscanning=false, software brightness=1 (linear), software gamma=1"
        ));
        assert!(output.contains(
            "HDMI-1: enabled=false, color_mode=null, supported_color_modes=[], is_underscanning=null, software brightness=unknown, software gamma=unknown"
        ));
    }

    #[test]
    fn listmonitors_view_matches_xrandr_style_shape() {
        let output = build_text(
            &CommandOptions {
                list_monitors: true,
                ..opts()
            },
            &sample_config(),
            &|_connector| Ok(CurrentColor::identity()),
            TextView::ListMonitors,
        )
        .unwrap();

        assert!(output.starts_with("Monitors: 1\n"));
        assert!(output.contains(" 0: +*eDP-1 1920/300x1080/190+0+0  eDP-1"));
    }

    #[test]
    fn validate_options_rejects_conflicting_views() {
        let error = validate_options(&CommandOptions {
            summary: true,
            json: true,
            ..opts()
        })
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "fatal: --summary cannot be used with --json."
        );

        let error = validate_options(&CommandOptions {
            properties: true,
            list_monitors: true,
            ..opts()
        })
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "fatal: --properties cannot be used with --listmonitors."
        );
    }

    #[test]
    fn validate_options_accepts_verbose_with_properties() {
        let view = validate_options(&CommandOptions {
            verbose: true,
            properties: true,
            ..opts()
        })
        .unwrap();

        assert!(matches!(view, QueryView::Text(TextView::Verbose)));
    }
}
