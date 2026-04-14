use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use gnome_randr::{
    display_config::{
        physical_monitor::{Mode, PhysicalMonitor},
        proxied_methods::{
            ApplyConfig, ApplyMonitor, ApplyMonitorProperty, BrightnessFilter, ColorMode,
            GammaAdjustment,
        },
        resources::Resources,
    },
    DisplayConfig,
};
use serde::Deserialize;
use structopt::StructOpt;

use super::brightness;

const MIN_SUPPORTED_SCHEMA_VERSION: u32 = 4;
const MAX_SUPPORTED_SCHEMA_VERSION: u32 = 5;

#[derive(StructOpt)]
pub struct CommandOptions {
    #[structopt(
        value_name = "FILE",
        help = "Profile file generated from `gnome-randr query --json`"
    )]
    file: PathBuf,

    #[structopt(long, help = "Persist the applied profile to disk")]
    persistent: bool,

    #[structopt(
        long,
        help = "Preview the profile without applying it",
        long_help = "Preview the profile without applying it. This validates the JSON schema, resolves hardware by vendor/product/serial identity, matches modes on current hardware, and shows the software color state that would be restored."
    )]
    dry_run: bool,
}

#[derive(Debug)]
enum Error {
    ReadFile {
        path: PathBuf,
        message: String,
    },
    ParseProfile {
        path: PathBuf,
        message: String,
    },
    UnsupportedSchemaVersion(u32),
    UnsupportedLayoutMode {
        profile: String,
        current: String,
    },
    DuplicateCurrentMonitorIdentity(MonitorIdentity),
    DuplicateProfileMonitorIdentity(MonitorIdentity),
    MissingProfileMonitor(MonitorIdentity),
    MissingCurrentMonitor(MonitorIdentity),
    ProfileMonitorDisabled(MonitorIdentity),
    DuplicateActiveMonitor(MonitorIdentity),
    MissingCurrentMode(MonitorIdentity),
    ModeUnavailable {
        identity: MonitorIdentity,
        requested: String,
    },
    InvalidRotation(String),
    InvalidReflection(String),
    InvalidColorMode(String),
    ColorModeUnavailable {
        identity: MonitorIdentity,
        requested: ColorMode,
        supported: Vec<ColorMode>,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ReadFile { path, message } => {
                write!(f, "fatal: failed to read profile {}: {}", path.display(), message)
            }
            Error::ParseProfile { path, message } => write!(
                f,
                "fatal: failed to parse profile {}: {}",
                path.display(),
                message
            ),
            Error::UnsupportedSchemaVersion(version) => write!(
                f,
                "fatal: unsupported profile schema version {}. Supported versions are {} through {}.",
                version, MIN_SUPPORTED_SCHEMA_VERSION, MAX_SUPPORTED_SCHEMA_VERSION
            ),
            Error::UnsupportedLayoutMode { profile, current } => write!(
                f,
                "fatal: profile layout_mode {} does not match current layout_mode {}. Changing layout_mode is not implemented.",
                profile, current
            ),
            Error::DuplicateCurrentMonitorIdentity(identity) => write!(
                f,
                "fatal: current hardware has duplicate monitor identity {}.",
                identity
            ),
            Error::DuplicateProfileMonitorIdentity(identity) => write!(
                f,
                "fatal: profile contains duplicate monitor identity {}.",
                identity
            ),
            Error::MissingProfileMonitor(identity) => write!(
                f,
                "fatal: profile logical monitor references {}, but no matching monitor entry exists in the profile.",
                identity
            ),
            Error::MissingCurrentMonitor(identity) => write!(
                f,
                "fatal: profile requires monitor {}, but it is not connected on current hardware.",
                identity
            ),
            Error::ProfileMonitorDisabled(identity) => write!(
                f,
                "fatal: profile marks {} as disabled but also uses it in an active logical monitor.",
                identity
            ),
            Error::DuplicateActiveMonitor(identity) => write!(
                f,
                "fatal: profile uses monitor {} in more than one active logical monitor.",
                identity
            ),
            Error::MissingCurrentMode(identity) => write!(
                f,
                "fatal: profile monitor {} does not have a current mode marked in the saved JSON.",
                identity
            ),
            Error::ModeUnavailable { identity, requested } => write!(
                f,
                "fatal: current hardware for {} does not provide a compatible mode for profile mode {}.",
                identity, requested
            ),
            Error::InvalidRotation(value) => write!(
                f,
                "fatal: profile uses unsupported rotation {}. Expected normal, right, inverted, or left.",
                value
            ),
            Error::InvalidReflection(value) => write!(
                f,
                "fatal: profile uses unsupported reflection {}. Expected normal, x, y, or xy.",
                value
            ),
            Error::InvalidColorMode(value) => {
                write!(f, "fatal: profile uses unsupported color mode {}.", value)
            }
            Error::ColorModeUnavailable {
                identity,
                requested,
                supported,
            } => write!(
                f,
                "fatal: {} does not support color mode {}. Supported color modes are {}.",
                identity,
                requested,
                supported
                    .iter()
                    .map(|mode| mode.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct MonitorIdentity {
    vendor: String,
    product: String,
    serial: String,
}

impl std::fmt::Display for MonitorIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.vendor, self.product, self.serial)
    }
}

#[derive(Debug, Deserialize)]
struct Profile {
    schema_version: u32,
    layout_mode: String,
    logical_monitors: Vec<ProfileLogicalMonitor>,
    monitors: Vec<ProfileMonitor>,
}

#[derive(Debug, Deserialize)]
struct ProfileLogicalMonitor {
    x: i32,
    y: i32,
    scale: f64,
    rotation: String,
    #[serde(default = "default_reflection")]
    reflection: String,
    primary: bool,
    monitors: Vec<ProfileAssociatedMonitor>,
}

#[derive(Debug, Deserialize)]
struct ProfileAssociatedMonitor {
    #[allow(dead_code)]
    connector: String,
    vendor: String,
    product: String,
    serial: String,
}

#[derive(Debug, Deserialize)]
struct ProfileMonitor {
    #[allow(dead_code)]
    connector: String,
    enabled: bool,
    vendor: String,
    product: String,
    serial: String,
    color_mode: Option<String>,
    modes: Vec<ProfileMode>,
    software_brightness: Option<ProfileSoftwareBrightness>,
    software_gamma: Option<ProfileSoftwareGamma>,
}

#[derive(Debug, Deserialize)]
struct ProfileMode {
    id: String,
    width: i32,
    height: i32,
    refresh_rate: f64,
    is_current: bool,
}

#[derive(Debug, Deserialize)]
struct ProfileSoftwareBrightness {
    state: String,
    brightness: Option<f64>,
    filter: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProfileSoftwareGamma {
    state: String,
    red: Option<f64>,
    green: Option<f64>,
    blue: Option<f64>,
}

#[derive(Debug)]
struct DesiredSoftwareColor {
    brightness: Option<f64>,
    filter: BrightnessFilter,
    gamma_adjustment: Option<GammaAdjustment>,
}

fn default_reflection() -> String {
    "normal".to_string()
}

fn identity_from_parts(vendor: &str, product: &str, serial: &str) -> MonitorIdentity {
    MonitorIdentity {
        vendor: vendor.to_string(),
        product: product.to_string(),
        serial: serial.to_string(),
    }
}

fn associated_identity(monitor: &ProfileAssociatedMonitor) -> MonitorIdentity {
    identity_from_parts(&monitor.vendor, &monitor.product, &monitor.serial)
}

fn profile_identity(monitor: &ProfileMonitor) -> MonitorIdentity {
    identity_from_parts(&monitor.vendor, &monitor.product, &monitor.serial)
}

fn current_identity(monitor: &PhysicalMonitor) -> MonitorIdentity {
    identity_from_parts(&monitor.vendor, &monitor.product, &monitor.serial)
}

fn parse_profile(path: &Path) -> Result<Profile, Error> {
    let contents = fs::read_to_string(path).map_err(|error| Error::ReadFile {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let profile =
        serde_json::from_str::<Profile>(&contents).map_err(|error| Error::ParseProfile {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

    if !(MIN_SUPPORTED_SCHEMA_VERSION..=MAX_SUPPORTED_SCHEMA_VERSION)
        .contains(&profile.schema_version)
    {
        return Err(Error::UnsupportedSchemaVersion(profile.schema_version));
    }

    Ok(profile)
}

fn profile_monitor_map<'a>(
    profile: &'a Profile,
) -> Result<HashMap<MonitorIdentity, &'a ProfileMonitor>, Error> {
    let mut map = HashMap::new();
    for monitor in &profile.monitors {
        let identity = profile_identity(monitor);
        if map.insert(identity.clone(), monitor).is_some() {
            return Err(Error::DuplicateProfileMonitorIdentity(identity));
        }
    }
    Ok(map)
}

fn current_monitor_map<'a>(
    config: &'a DisplayConfig,
) -> Result<HashMap<MonitorIdentity, &'a PhysicalMonitor>, Error> {
    let mut map = HashMap::new();
    for monitor in &config.monitors {
        let identity = current_identity(monitor);
        if map.insert(identity.clone(), monitor).is_some() {
            return Err(Error::DuplicateCurrentMonitorIdentity(identity));
        }
    }
    Ok(map)
}

fn current_profile_mode_by_identity<'a>(
    profile_monitor: &'a ProfileMonitor,
    identity: &MonitorIdentity,
) -> Result<&'a ProfileMode, Error> {
    profile_monitor
        .modes
        .iter()
        .find(|mode| mode.is_current)
        .ok_or_else(|| Error::MissingCurrentMode(identity.clone()))
}

fn match_mode<'a>(
    current_monitor: &'a PhysicalMonitor,
    profile_monitor: &ProfileMonitor,
    identity: &MonitorIdentity,
) -> Result<&'a Mode, Error> {
    let profile_mode = current_profile_mode_by_identity(profile_monitor, identity)?;

    if let Some(mode) = current_monitor
        .modes
        .iter()
        .find(|mode| mode.id == profile_mode.id)
    {
        return Ok(mode);
    }

    current_monitor
        .modes
        .iter()
        .filter(|mode| mode.width == profile_mode.width && mode.height == profile_mode.height)
        .min_by(|left, right| {
            let left_delta = (left.refresh_rate - profile_mode.refresh_rate).abs();
            let right_delta = (right.refresh_rate - profile_mode.refresh_rate).abs();
            left_delta
                .partial_cmp(&right_delta)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .ok_or_else(|| Error::ModeUnavailable {
            identity: identity.clone(),
            requested: profile_mode.id.clone(),
        })
}

fn rotation_from_profile(value: &str) -> Result<u32, Error> {
    match value {
        "normal" => Ok(0),
        "right" => Ok(1),
        "inverted" => Ok(2),
        "left" => Ok(3),
        _ => Err(Error::InvalidRotation(value.to_string())),
    }
}

fn rotate_180(transform: u32) -> u32 {
    (transform + 2) & 3
}

fn transform_from_profile(rotation: &str, reflection: &str) -> Result<u32, Error> {
    let rotation = rotation_from_profile(rotation)?;
    match reflection {
        "normal" => Ok(rotation),
        "y" => Ok(rotation | 4),
        "x" => Ok(rotate_180(rotation) | 4),
        "xy" => Ok(rotate_180(rotation)),
        _ => Err(Error::InvalidReflection(reflection.to_string())),
    }
}

fn rotation_from_transform(transform: u32) -> &'static str {
    match transform & 3 {
        0 => "normal",
        1 => "right",
        2 => "inverted",
        _ => "left",
    }
}

fn reflection_from_transform(transform: u32) -> &'static str {
    if transform & 4 == 0 {
        return "normal";
    }

    match transform & 3 {
        2 | 3 => "x",
        _ => "y",
    }
}

fn property_u32(properties: &dbus::arg::PropMap, key: &str) -> Option<u32> {
    properties
        .get(key)
        .and_then(|value| value.0.as_u64())
        .map(|value| value as u32)
}

fn supported_color_modes(monitor: &PhysicalMonitor) -> Vec<ColorMode> {
    monitor
        .properties
        .get("supported-color-modes")
        .and_then(|value| value.0.as_iter())
        .map(|iter| {
            iter.filter_map(|value| {
                value
                    .as_u64()
                    .and_then(|raw| ColorMode::from_raw(raw as u32))
            })
            .collect()
        })
        .unwrap_or_default()
}

fn resolve_color_mode(
    profile_monitor: &ProfileMonitor,
    current_monitor: &PhysicalMonitor,
    identity: &MonitorIdentity,
) -> Result<Option<ColorMode>, Error> {
    let requested = match &profile_monitor.color_mode {
        Some(value) => Some(
            value
                .parse::<ColorMode>()
                .map_err(|_| Error::InvalidColorMode(value.clone()))?,
        ),
        None => {
            property_u32(&current_monitor.properties, "color-mode").and_then(ColorMode::from_raw)
        }
    };

    match requested {
        Some(requested) => {
            let supported = supported_color_modes(current_monitor);
            if supported.is_empty() || supported.iter().any(|mode| *mode == requested) {
                Ok(Some(requested))
            } else {
                Err(Error::ColorModeUnavailable {
                    identity: identity.clone(),
                    requested,
                    supported,
                })
            }
        }
        None => Ok(None),
    }
}

fn desired_software_color(
    profile_monitor: &ProfileMonitor,
) -> Result<Option<DesiredSoftwareColor>, Error> {
    let brightness = profile_monitor.software_brightness.as_ref();
    let gamma = profile_monitor.software_gamma.as_ref();

    let state = brightness
        .map(|value| value.state.as_str())
        .or_else(|| gamma.map(|value| value.state.as_str()));

    match state {
        Some("managed") | Some("identity") => {
            let brightness_value = brightness.and_then(|value| value.brightness).or(Some(1.0));
            let filter = brightness
                .and_then(|value| value.filter.as_deref())
                .unwrap_or("linear")
                .parse::<BrightnessFilter>()
                .map_err(|message| Error::ParseProfile {
                    path: PathBuf::from("<profile>"),
                    message,
                })?;
            let gamma_adjustment = GammaAdjustment {
                red: gamma.and_then(|value| value.red).unwrap_or(1.0),
                green: gamma.and_then(|value| value.green).unwrap_or(1.0),
                blue: gamma.and_then(|value| value.blue).unwrap_or(1.0),
            };
            Ok(Some(DesiredSoftwareColor {
                brightness: brightness_value,
                filter,
                gamma_adjustment: Some(gamma_adjustment),
            }))
        }
        Some("unknown") | None => Ok(None),
        Some(other) => Err(Error::ParseProfile {
            path: PathBuf::from("<profile>"),
            message: format!("unsupported software color state {}", other),
        }),
    }
}

fn build_apply_configs<'a>(
    profile: &'a Profile,
    config: &'a DisplayConfig,
) -> Result<
    (
        Vec<ApplyConfig<'a>>,
        Vec<(&'a PhysicalMonitor, Option<DesiredSoftwareColor>)>,
    ),
    Error,
> {
    let current_layout_mode = config.known_properties.layout_mode.to_string();
    if profile.layout_mode != current_layout_mode {
        return Err(Error::UnsupportedLayoutMode {
            profile: profile.layout_mode.clone(),
            current: current_layout_mode,
        });
    }

    let profile_monitors = profile_monitor_map(profile)?;
    let current_monitors = current_monitor_map(config)?;
    let mut used_identities = HashSet::new();
    let mut configs = Vec::new();
    let mut software_color = Vec::new();

    for logical_monitor in &profile.logical_monitors {
        let mut monitors = Vec::new();
        for associated in &logical_monitor.monitors {
            let identity = associated_identity(associated);
            if !used_identities.insert(identity.clone()) {
                return Err(Error::DuplicateActiveMonitor(identity));
            }

            let profile_monitor = profile_monitors
                .get(&identity)
                .copied()
                .ok_or_else(|| Error::MissingProfileMonitor(identity.clone()))?;
            if !profile_monitor.enabled {
                return Err(Error::ProfileMonitorDisabled(identity));
            }

            let current_monitor = current_monitors
                .get(&identity)
                .copied()
                .ok_or_else(|| Error::MissingCurrentMonitor(identity.clone()))?;
            let mode = match_mode(current_monitor, profile_monitor, &identity)?;
            let mut properties = Vec::new();
            if let Some(color_mode) =
                resolve_color_mode(profile_monitor, current_monitor, &identity)?
            {
                properties.push(ApplyMonitorProperty::ColorMode(color_mode));
            }

            monitors.push(ApplyMonitor {
                connector: &current_monitor.connector,
                mode_id: &mode.id,
                properties,
            });
            software_color.push((current_monitor, desired_software_color(profile_monitor)?));
        }

        configs.push(ApplyConfig {
            x_pos: logical_monitor.x,
            y_pos: logical_monitor.y,
            scale: logical_monitor.scale,
            transform: transform_from_profile(
                &logical_monitor.rotation,
                &logical_monitor.reflection,
            )?,
            primary: logical_monitor.primary,
            monitors,
        });
    }

    Ok((configs, software_color))
}

fn print_preview(
    configs: &[ApplyConfig<'_>],
    software_color: &[(&PhysicalMonitor, Option<DesiredSoftwareColor>)],
) {
    println!("applying saved profile");
    for config in configs {
        println!(
            "logical monitor at {},{} scale {} rotation {} reflection {} primary={}",
            config.x_pos,
            config.y_pos,
            config.scale,
            rotation_from_transform(config.transform),
            reflection_from_transform(config.transform),
            config.primary
        );
        for monitor in &config.monitors {
            let color_mode = monitor
                .properties
                .iter()
                .find_map(|property| match property {
                    ApplyMonitorProperty::ColorMode(color_mode) => Some(color_mode.to_string()),
                });

            match color_mode {
                Some(color_mode) => println!(
                    "  {} mode {} color_mode {}",
                    monitor.connector, monitor.mode_id, color_mode
                ),
                None => println!("  {} mode {}", monitor.connector, monitor.mode_id),
            }
        }
    }

    for (monitor, desired) in software_color {
        if let Some(desired) = desired {
            if desired.brightness.unwrap_or(1.0) != 1.0
                || desired.filter != BrightnessFilter::Linear
            {
                println!(
                    "setting software brightness on {} to {} using {} filter",
                    monitor.connector,
                    desired.brightness.unwrap_or(1.0),
                    desired.filter
                );
            }
            if desired
                .gamma_adjustment
                .unwrap_or_else(GammaAdjustment::identity)
                .is_identity()
            {
                continue;
            }
            println!(
                "setting software gamma on {} to {}",
                monitor.connector,
                desired
                    .gamma_adjustment
                    .unwrap_or_else(GammaAdjustment::identity)
            );
        }
    }
}

pub fn handle(
    opts: &CommandOptions,
    config: &DisplayConfig,
    proxy: &dbus::blocking::Proxy<&dbus::blocking::Connection>,
) -> Result<(), Box<dyn std::error::Error>> {
    let profile = parse_profile(&opts.file)?;
    let (configs, software_color) = build_apply_configs(&profile, config)?;

    if opts.dry_run {
        print_preview(&configs, &software_color);
        println!("dry run: no changes made.");
        return Ok(());
    }

    config.apply_monitors_config(proxy, configs, opts.persistent)?;

    if software_color.iter().any(|(_, desired)| desired.is_some()) {
        let resources = Resources::get_resources(proxy)?;
        for (monitor, desired) in &software_color {
            if let Some(desired) = desired {
                brightness::apply_color(
                    &monitor.connector,
                    desired.brightness,
                    desired.filter,
                    desired.gamma_adjustment,
                    false,
                    &resources,
                    proxy,
                )?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{build_apply_configs, parse_profile, ColorMode, Error, Profile};
    use gnome_randr::{
        display_config::{
            logical_monitor::{LogicalMonitor, Monitor, Transform},
            physical_monitor::{KnownModeProperties, Mode, PhysicalMonitor},
            KnownProperties, LayoutMode,
        },
        DisplayConfig,
    };

    fn display_config() -> DisplayConfig {
        let mut hdmi_props = dbus::arg::PropMap::new();
        hdmi_props.insert(
            "supported-color-modes".to_string(),
            dbus::arg::Variant(Box::new(vec![0u32, 1u32])),
        );
        hdmi_props.insert("color-mode".to_string(), dbus::arg::Variant(Box::new(1u32)));

        DisplayConfig {
            serial: 1,
            monitors: vec![
                PhysicalMonitor {
                    connector: "HDMI-9".to_string(),
                    vendor: "Dell".to_string(),
                    product: "U2720Q".to_string(),
                    serial: "123".to_string(),
                    modes: vec![
                        Mode {
                            id: "2560x1440@60".to_string(),
                            width: 2560,
                            height: 1440,
                            refresh_rate: 60.0,
                            preferred_scale: 1.0,
                            supported_scales: vec![1.0],
                            known_properties: KnownModeProperties {
                                is_current: true,
                                is_preferred: true,
                            },
                            properties: Default::default(),
                        },
                        Mode {
                            id: "2560x1440@59.94".to_string(),
                            width: 2560,
                            height: 1440,
                            refresh_rate: 59.94,
                            preferred_scale: 1.0,
                            supported_scales: vec![1.0],
                            known_properties: KnownModeProperties {
                                is_current: false,
                                is_preferred: false,
                            },
                            properties: Default::default(),
                        },
                    ],
                    properties: hdmi_props,
                },
                PhysicalMonitor {
                    connector: "eDP-1".to_string(),
                    vendor: "BOE".to_string(),
                    product: "Panel".to_string(),
                    serial: "abc".to_string(),
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
            ],
            logical_monitors: vec![LogicalMonitor {
                x: 0,
                y: 0,
                scale: 1.0,
                transform: Transform::NORMAL,
                primary: true,
                monitors: vec![Monitor {
                    connector: "HDMI-9".to_string(),
                    vendor: "Dell".to_string(),
                    product: "U2720Q".to_string(),
                    serial: "123".to_string(),
                }],
                properties: Default::default(),
            }],
            known_properties: KnownProperties {
                supports_mirroring: true,
                layout_mode: LayoutMode::Physical,
                supports_changing_layout_mode: false,
                global_scale_required: false,
            },
            properties: Default::default(),
        }
    }

    fn profile() -> Profile {
        serde_json::from_str(
            r#"{
                "schema_version": 5,
                "layout_mode": "physical",
                "logical_monitors": [
                    {
                        "x": 100,
                        "y": 200,
                        "scale": 1.0,
                        "rotation": "normal",
                        "reflection": "x",
                        "primary": true,
                        "monitors": [
                            {
                                "connector": "HDMI-1",
                                "vendor": "Dell",
                                "product": "U2720Q",
                                "serial": "123"
                            }
                        ]
                    }
                ],
                "monitors": [
                    {
                        "connector": "HDMI-1",
                        "enabled": true,
                        "vendor": "Dell",
                        "product": "U2720Q",
                        "serial": "123",
                        "color_mode": "bt2100",
                        "modes": [
                            {"id": "2560x1440@59.94", "width": 2560, "height": 1440, "refresh_rate": 59.94, "is_current": true}
                        ],
                        "software_brightness": {"state": "managed", "brightness": 1.25, "filter": "filmic"},
                        "software_gamma": {"state": "managed", "red": 1.1, "green": 1.0, "blue": 0.9}
                    }
                ]
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn build_apply_configs_matches_monitor_by_identity() {
        let profile = profile();
        let config = display_config();
        let (configs, software_color) = build_apply_configs(&profile, &config).unwrap();

        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].x_pos, 100);
        assert_eq!(configs[0].y_pos, 200);
        assert_eq!(configs[0].transform, 6);
        assert_eq!(configs[0].monitors[0].connector, "HDMI-9");
        assert_eq!(configs[0].monitors[0].mode_id, "2560x1440@59.94");
        assert_eq!(software_color.len(), 1);
    }

    #[test]
    fn build_apply_configs_rejects_missing_hardware() {
        let mut profile = profile();
        profile.logical_monitors[0].monitors[0].serial = "missing".to_string();
        profile.monitors[0].serial = "missing".to_string();

        match build_apply_configs(&profile, &display_config()).unwrap_err() {
            Error::MissingCurrentMonitor(identity) => assert_eq!(identity.serial, "missing"),
            error => panic!("unexpected error: {:?}", error),
        }
    }

    #[test]
    fn build_apply_configs_validates_color_mode_support() {
        let mut config = display_config();
        config.monitors[0].properties.insert(
            "supported-color-modes".to_string(),
            dbus::arg::Variant(Box::new(vec![0u32])),
        );

        match build_apply_configs(&profile(), &config).unwrap_err() {
            Error::ColorModeUnavailable {
                requested,
                supported,
                ..
            } => {
                assert_eq!(requested, ColorMode::Bt2100);
                assert_eq!(supported, vec![ColorMode::Default]);
            }
            error => panic!("unexpected error: {:?}", error),
        }
    }

    #[test]
    fn parse_profile_rejects_unsupported_schema_version() {
        let path = std::env::temp_dir().join("gnome-randr-profile-invalid.json");
        fs::write(&path, r#"{"schema_version": 3, "layout_mode": "physical", "logical_monitors": [], "monitors": []}"#).unwrap();

        match parse_profile(&path).unwrap_err() {
            Error::UnsupportedSchemaVersion(3) => {}
            error => panic!("unexpected error: {:?}", error),
        }

        let _ = fs::remove_file(path);
    }
}
