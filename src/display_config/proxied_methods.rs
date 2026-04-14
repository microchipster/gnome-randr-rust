use std::convert::TryFrom;

use dbus::{
    arg::{self, PropMap},
    blocking::{Connection, Proxy},
};

use super::{
    logical_monitor::LogicalMonitor,
    physical_monitor::PhysicalMonitor,
    resources::{Crtc, Resources},
    DisplayConfig,
};

type Result<T> = std::prelude::rust_2021::Result<T, dbus::Error>;

#[derive(Debug, Clone)]
pub struct ApplyMonitor<'a> {
    pub connector: &'a str,
    pub mode_id: &'a str,
    pub properties: Vec<ApplyMonitorProperty>,
}

impl ApplyMonitor<'_> {
    pub fn serialize(&self) -> (&str, &str, PropMap) {
        let mut properties = PropMap::new();

        for property in &self.properties {
            match property {
                ApplyMonitorProperty::ColorMode(color_mode) => {
                    properties.insert(
                        "color-mode".to_string(),
                        arg::Variant(Box::new(color_mode.raw_value())),
                    );
                }
            }
        }

        (self.connector, self.mode_id, properties)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyMonitorProperty {
    ColorMode(ColorMode),
}

#[derive(Debug, Clone)]
pub struct ApplyConfig<'a> {
    pub x_pos: i32,
    pub y_pos: i32,
    pub scale: f64,
    pub transform: u32,
    pub primary: bool,
    pub monitors: Vec<ApplyMonitor<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerSaveMode {
    Unknown,
    On,
    Standby,
    Suspend,
    Off,
}

impl PowerSaveMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            PowerSaveMode::Unknown => "unknown",
            PowerSaveMode::On => "on",
            PowerSaveMode::Standby => "standby",
            PowerSaveMode::Suspend => "suspend",
            PowerSaveMode::Off => "off",
        }
    }

    pub const fn raw_value(self) -> i32 {
        match self {
            PowerSaveMode::Unknown => -1,
            PowerSaveMode::On => 0,
            PowerSaveMode::Standby => 1,
            PowerSaveMode::Suspend => 2,
            PowerSaveMode::Off => 3,
        }
    }

    pub fn from_raw(value: i32) -> Option<Self> {
        match value {
            -1 => Some(PowerSaveMode::Unknown),
            0 => Some(PowerSaveMode::On),
            1 => Some(PowerSaveMode::Standby),
            2 => Some(PowerSaveMode::Suspend),
            3 => Some(PowerSaveMode::Off),
            _ => None,
        }
    }
}

impl std::fmt::Display for PowerSaveMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for PowerSaveMode {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "on" => Ok(PowerSaveMode::On),
            "standby" => Ok(PowerSaveMode::Standby),
            "suspend" => Ok(PowerSaveMode::Suspend),
            "off" => Ok(PowerSaveMode::Off),
            "unknown" => Ok(PowerSaveMode::Unknown),
            _ => Err(format!("invalid power save mode: {}", value)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BacklightState {
    pub serial: u32,
    pub connectors: Vec<BacklightConnector>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BacklightConnector {
    pub connector: String,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LuminanceState {
    pub connector: String,
    pub color_mode: ColorMode,
    pub luminance: f64,
    pub default: f64,
    pub is_unset: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NativeDisplayState {
    pub power_save_mode: PowerSaveMode,
    pub panel_orientation_managed: bool,
    pub apply_monitors_config_allowed: Option<bool>,
    pub night_light_supported: Option<bool>,
    pub has_external_monitor: Option<bool>,
    pub backlight: Option<BacklightState>,
    pub luminance: Vec<LuminanceState>,
}

fn prop_string(properties: &PropMap, key: &str) -> Option<String> {
    properties
        .get(key)
        .and_then(|value| value.0.as_str())
        .map(str::to_owned)
}

fn prop_bool(properties: &PropMap, key: &str) -> Option<bool> {
    properties
        .get(key)
        .and_then(|value| value.0.as_u64())
        .map(|value| value != 0)
}

fn prop_f64(properties: &PropMap, key: &str) -> Option<f64> {
    properties.get(key).and_then(|value| value.0.as_f64())
}

fn prop_u32(properties: &PropMap, key: &str) -> Option<u32> {
    properties
        .get(key)
        .and_then(|value| value.0.as_u64())
        .and_then(|value| u32::try_from(value).ok())
}

fn parse_backlight_state(raw: (u32, Vec<PropMap>)) -> BacklightState {
    BacklightState {
        serial: raw.0,
        connectors: raw
            .1
            .into_iter()
            .filter_map(|entry| {
                Some(BacklightConnector {
                    connector: prop_string(&entry, "connector")?,
                    active: prop_bool(&entry, "active").unwrap_or(false),
                })
            })
            .collect(),
    }
}

fn parse_luminance_state(entries: Vec<PropMap>) -> Vec<LuminanceState> {
    entries
        .into_iter()
        .filter_map(|entry| {
            Some(LuminanceState {
                connector: prop_string(&entry, "connector")?,
                color_mode: ColorMode::from_raw(prop_u32(&entry, "color-mode")?)?,
                luminance: prop_f64(&entry, "luminance")?,
                default: prop_f64(&entry, "default")?,
                is_unset: prop_bool(&entry, "is-unset").unwrap_or(false),
            })
        })
        .collect()
}

impl ApplyConfig<'_> {
    pub fn from<'a>(
        logical_monitor: &LogicalMonitor,
        physical_monitor: &'a PhysicalMonitor,
    ) -> ApplyConfig<'a> {
        ApplyConfig {
            x_pos: logical_monitor.x,
            y_pos: logical_monitor.y,
            scale: logical_monitor.scale,
            transform: logical_monitor.transform.bits(),
            primary: logical_monitor.primary,
            monitors: vec![ApplyMonitor {
                connector: &physical_monitor.connector,
                mode_id: &physical_monitor
                    .modes
                    .iter()
                    .find(|mode| mode.known_properties.is_current)
                    .unwrap()
                    .id,
                properties: vec![],
            }],
        }
    }

    pub fn serialize(&self) -> (i32, i32, f64, u32, bool, Vec<(&str, &str, PropMap)>) {
        (
            self.x_pos,
            self.y_pos,
            self.scale,
            self.transform,
            self.primary,
            self.monitors
                .iter()
                .map(|monitor| monitor.serialize())
                .collect(),
        )
    }
}

impl DisplayConfig {
    pub fn apply_monitors_config_with_properties(
        &self,
        proxy: &Proxy<&Connection>,
        configs: Vec<ApplyConfig>,
        persistent: bool,
        properties: PropMap,
    ) -> Result<()> {
        use super::raw::OrgGnomeMutterDisplayConfig;

        let result = proxy.apply_monitors_config(
            self.serial,
            if persistent { 2 } else { 1 },
            configs.iter().map(|config| config.serialize()).collect(),
            properties,
        );

        if let Err(err) = &result {
            println!("{:?}", err);
        }
        result
    }

    pub fn apply_monitors_config(
        &self,
        proxy: &Proxy<&Connection>,
        configs: Vec<ApplyConfig>,
        persistent: bool,
    ) -> Result<()> {
        self.apply_monitors_config_with_properties(proxy, configs, persistent, PropMap::new())
    }

    pub fn get_current_state(proxy: &Proxy<&Connection>) -> Result<DisplayConfig> {
        use super::raw::OrgGnomeMutterDisplayConfig;

        let raw_output = proxy.get_current_state()?;
        Ok(DisplayConfig::from(raw_output))
    }

    pub fn native_display_state(proxy: &Proxy<&Connection>) -> Result<NativeDisplayState> {
        use super::raw::OrgFreedesktopDBusProperties;
        use super::raw::OrgGnomeMutterDisplayConfig;

        let power_save_mode = proxy
            .power_save_mode()
            .ok()
            .and_then(PowerSaveMode::from_raw)
            .unwrap_or(PowerSaveMode::Unknown);
        let properties = proxy.get_all("org.gnome.Mutter.DisplayConfig")?;
        let backlight = proxy
            .get::<(u32, Vec<PropMap>)>("org.gnome.Mutter.DisplayConfig", "Backlight")
            .ok()
            .map(parse_backlight_state);
        let luminance = proxy
            .get::<Vec<PropMap>>("org.gnome.Mutter.DisplayConfig", "Luminance")
            .ok()
            .map(parse_luminance_state)
            .unwrap_or_default();

        Ok(NativeDisplayState {
            power_save_mode,
            panel_orientation_managed: proxy.panel_orientation_managed().unwrap_or(false),
            apply_monitors_config_allowed: prop_bool(&properties, "ApplyMonitorsConfigAllowed"),
            night_light_supported: prop_bool(&properties, "NightLightSupported"),
            has_external_monitor: prop_bool(&properties, "HasExternalMonitor"),
            backlight,
            luminance,
        })
    }

    pub fn set_power_save_mode_native(
        proxy: &Proxy<&Connection>,
        mode: PowerSaveMode,
    ) -> Result<()> {
        use super::raw::OrgGnomeMutterDisplayConfig;
        proxy.set_power_save_mode(mode.raw_value())
    }

    pub fn set_backlight(
        proxy: &Proxy<&Connection>,
        serial: u32,
        connector: &str,
        value: i32,
    ) -> Result<()> {
        proxy.method_call(
            "org.gnome.Mutter.DisplayConfig",
            "SetBacklight",
            (serial, connector, value),
        )
    }

    pub fn set_luminance(
        proxy: &Proxy<&Connection>,
        connector: &str,
        color_mode: ColorMode,
        luminance: f64,
    ) -> Result<()> {
        proxy.method_call(
            "org.gnome.Mutter.DisplayConfig",
            "SetLuminance",
            (connector, color_mode.raw_value(), luminance),
        )
    }

    pub fn reset_luminance(
        proxy: &Proxy<&Connection>,
        connector: &str,
        color_mode: ColorMode,
    ) -> Result<()> {
        proxy.method_call(
            "org.gnome.Mutter.DisplayConfig",
            "ResetLuminance",
            (connector, color_mode.raw_value()),
        )
    }
}

#[derive(Debug, Clone)]
pub struct Gamma {
    pub red: Vec<u16>,
    pub green: Vec<u16>,
    pub blue: Vec<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrightnessFilter {
    Linear,
    Gamma,
    Filmic,
}

impl BrightnessFilter {
    pub const fn as_str(self) -> &'static str {
        match self {
            BrightnessFilter::Linear => "linear",
            BrightnessFilter::Gamma => "gamma",
            BrightnessFilter::Filmic => "filmic",
        }
    }
}

impl std::fmt::Display for BrightnessFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for BrightnessFilter {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "linear" => Ok(BrightnessFilter::Linear),
            "gamma" => Ok(BrightnessFilter::Gamma),
            "filmic" => Ok(BrightnessFilter::Filmic),
            _ => Err(format!("invalid brightness filter: {}", value)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Default,
    Bt2100,
}

impl ColorMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            ColorMode::Default => "default",
            ColorMode::Bt2100 => "bt2100",
        }
    }

    pub const fn raw_value(self) -> u32 {
        match self {
            ColorMode::Default => 0,
            ColorMode::Bt2100 => 1,
        }
    }

    pub fn from_raw(value: u32) -> Option<Self> {
        match value {
            0 => Some(ColorMode::Default),
            1 => Some(ColorMode::Bt2100),
            _ => None,
        }
    }
}

impl std::fmt::Display for ColorMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ColorMode {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "default" => Ok(ColorMode::Default),
            "bt2100" => Ok(ColorMode::Bt2100),
            _ => Err(format!("invalid color mode: {}", value)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GammaAdjustment {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
}

impl GammaAdjustment {
    pub const fn identity() -> GammaAdjustment {
        GammaAdjustment {
            red: 1.0,
            green: 1.0,
            blue: 1.0,
        }
    }

    pub fn is_identity(self) -> bool {
        (self.red - 1.0).abs() <= f64::EPSILON
            && (self.green - 1.0).abs() <= f64::EPSILON
            && (self.blue - 1.0).abs() <= f64::EPSILON
    }
}

impl Default for GammaAdjustment {
    fn default() -> Self {
        GammaAdjustment::identity()
    }
}

impl std::fmt::Display for GammaAdjustment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let format_component = |value: f64| {
            format!("{:.3}", value)
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        };

        if (self.red - self.green).abs() <= f64::EPSILON
            && (self.red - self.blue).abs() <= f64::EPSILON
        {
            write!(f, "{}", format_component(self.red))
        } else {
            write!(
                f,
                "{}:{}:{}",
                format_component(self.red),
                format_component(self.green),
                format_component(self.blue)
            )
        }
    }
}

impl std::str::FromStr for GammaAdjustment {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        let parts = value.split(':').collect::<Vec<_>>();
        let parse_component = |component: &str| -> std::result::Result<f64, String> {
            let parsed = component
                .parse::<f64>()
                .map_err(|_| format!("invalid gamma value: {}", component))?;
            if !parsed.is_finite() || parsed <= 0.0 {
                return Err(format!("invalid gamma value: {}", component));
            }
            Ok(parsed)
        };

        match parts.as_slice() {
            [red] => {
                let red = parse_component(red)?;
                Ok(GammaAdjustment {
                    red,
                    green: red,
                    blue: red,
                })
            }
            [red, green, blue] => Ok(GammaAdjustment {
                red: parse_component(red)?,
                green: parse_component(green)?,
                blue: parse_component(blue)?,
            }),
            _ => Err(format!(
                "invalid gamma value: {}. Use R or R:G:B, for example 1 or 1.1:1.0:0.9",
                value
            )),
        }
    }
}

impl Gamma {
    pub fn from(result: (Vec<u16>, Vec<u16>, Vec<u16>)) -> Gamma {
        Gamma {
            red: result.0,
            green: result.1,
            blue: result.2,
        }
    }

    pub fn scale_brightness(&self, brightness: f64) -> Gamma {
        self.apply_brightness(brightness, BrightnessFilter::Linear)
    }

    pub fn apply_brightness(&self, brightness: f64, filter: BrightnessFilter) -> Gamma {
        let max = f64::from(u16::MAX);
        let scale_channel = |channel: &[u16]| {
            channel
                .iter()
                .map(|value| {
                    if brightness == 0.0 || *value == 0 {
                        return 0;
                    }

                    let adjusted = match filter {
                        BrightnessFilter::Linear if brightness > 1.0 => {
                            (f64::from(*value) * brightness) / max
                        }
                        BrightnessFilter::Gamma if brightness > 1.0 => {
                            (f64::from(*value) / max).powf(brightness.recip())
                        }
                        BrightnessFilter::Filmic if brightness > 1.0 => {
                            let normalized = f64::from(*value) / max;
                            (normalized * brightness) / (1.0 + normalized * (brightness - 1.0))
                        }
                        _ => (f64::from(*value) * brightness) / max,
                    };

                    (adjusted * max).round().clamp(0.0, max) as u16
                })
                .collect()
        };

        Gamma {
            red: scale_channel(&self.red),
            green: scale_channel(&self.green),
            blue: scale_channel(&self.blue),
        }
    }

    pub fn apply_gamma_adjustment(&self, adjustment: GammaAdjustment) -> Gamma {
        let max = f64::from(u16::MAX);
        let adjust_channel = |channel: &[u16], gamma: f64| {
            channel
                .iter()
                .map(|value| {
                    if *value == 0 {
                        return 0;
                    }

                    let normalized = f64::from(*value) / max;
                    let adjusted = normalized.powf(gamma.recip());
                    (adjusted * max).round().clamp(0.0, max) as u16
                })
                .collect()
        };

        Gamma {
            red: adjust_channel(&self.red, adjustment.red),
            green: adjust_channel(&self.green, adjustment.green),
            blue: adjust_channel(&self.blue, adjustment.blue),
        }
    }

    pub fn apply_software_color(
        &self,
        brightness: f64,
        filter: BrightnessFilter,
        gamma_adjustment: GammaAdjustment,
    ) -> Gamma {
        self.apply_gamma_adjustment(gamma_adjustment)
            .apply_brightness(brightness, filter)
    }

    pub fn approx_eq(&self, other: &Gamma) -> bool {
        fn channels_match(left: &[u16], right: &[u16]) -> bool {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right.iter())
                    .all(|(left, right)| left.abs_diff(*right) <= 1)
        }

        channels_match(&self.red, &other.red)
            && channels_match(&self.green, &other.green)
            && channels_match(&self.blue, &other.blue)
    }

    pub fn is_identity(&self) -> bool {
        fn identity_channel(channel: &[u16]) -> bool {
            if channel.len() < 2 {
                return false;
            }

            let last_index = (channel.len() - 1) as f64;

            channel.iter().enumerate().all(|(index, value)| {
                let expected = ((index as f64 / last_index) * f64::from(u16::MAX)).round() as u16;
                value.abs_diff(expected) <= 1
            })
        }

        identity_channel(&self.red) && identity_channel(&self.green) && identity_channel(&self.blue)
    }
}

impl Resources {
    pub fn get_resources(proxy: &Proxy<&Connection>) -> Result<Resources> {
        use super::raw::OrgGnomeMutterDisplayConfig;

        let raw_output = proxy.get_resources()?;
        Ok(Resources::from(raw_output))
    }

    pub fn get_crtc_gamma(&self, proxy: &Proxy<&Connection>, crtc: &Crtc) -> Result<Gamma> {
        use super::raw::OrgGnomeMutterDisplayConfig;

        let result = proxy.get_crtc_gamma(self.serial, crtc.id)?;
        Ok(Gamma::from(result))
    }

    pub fn set_crtc_gamma(
        &self,
        proxy: &Proxy<&Connection>,
        crtc: &Crtc,
        gamma: Gamma,
    ) -> Result<()> {
        use super::raw::OrgGnomeMutterDisplayConfig;

        proxy.set_crtc_gamma(self.serial, crtc.id, gamma.red, gamma.green, gamma.blue)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ApplyMonitor, ApplyMonitorProperty, BrightnessFilter, ColorMode, Gamma, GammaAdjustment,
    };

    #[test]
    fn scaling_gamma_preserves_channel_shape() {
        let gamma = Gamma {
            red: vec![0, 5000, 10000],
            green: vec![0, 4000, 8000],
            blue: vec![0, 3000, 6000],
        };

        let scaled = gamma.scale_brightness(0.5);

        assert_eq!(scaled.red, vec![0, 2500, 5000]);
        assert_eq!(scaled.green, vec![0, 2000, 4000]);
        assert_eq!(scaled.blue, vec![0, 1500, 3000]);
    }

    #[test]
    fn scaling_gamma_clamps_values() {
        let gamma = Gamma {
            red: vec![60000],
            green: vec![65535],
            blue: vec![40000],
        };

        let scaled = gamma.scale_brightness(2.0);

        assert_eq!(scaled.red, vec![65535]);
        assert_eq!(scaled.green, vec![65535]);
        assert_eq!(scaled.blue, vec![65535]);
    }

    #[test]
    fn gamma_filter_brightens_midtones_without_clipping_white() {
        let gamma = Gamma {
            red: vec![0, 16384, 32768, 65535],
            green: vec![0, 16384, 32768, 65535],
            blue: vec![0, 16384, 32768, 65535],
        };

        let adjusted = gamma.apply_brightness(2.0, BrightnessFilter::Gamma);

        assert_eq!(adjusted.red[0], 0);
        assert!(adjusted.red[1] > gamma.red[1]);
        assert!(adjusted.red[2] > gamma.red[2]);
        assert_eq!(adjusted.red[3], 65535);
    }

    #[test]
    fn filmic_filter_rolls_off_highlights_more_than_gamma() {
        let gamma = Gamma {
            red: vec![0, 16384, 32768, 49152, 65535],
            green: vec![0, 16384, 32768, 49152, 65535],
            blue: vec![0, 16384, 32768, 49152, 65535],
        };

        let gamma_adjusted = gamma.apply_brightness(2.0, BrightnessFilter::Gamma);
        let filmic_adjusted = gamma.apply_brightness(2.0, BrightnessFilter::Filmic);

        assert!(filmic_adjusted.red[1] > gamma.red[1]);
        assert!(filmic_adjusted.red[2] > gamma.red[2]);
        assert!(filmic_adjusted.red[2] < gamma_adjusted.red[2]);
        assert!(filmic_adjusted.red[3] < gamma_adjusted.red[3]);
        assert_eq!(filmic_adjusted.red[4], 65535);
    }

    #[test]
    fn identity_gamma_is_detected() {
        let gamma = Gamma {
            red: vec![0, 32768, 65535],
            green: vec![0, 32768, 65535],
            blue: vec![0, 32768, 65535],
        };

        assert!(gamma.is_identity());
    }

    #[test]
    fn gamma_adjustment_is_identity_at_one() {
        let gamma = Gamma {
            red: vec![0, 1000, 2000],
            green: vec![0, 1500, 3000],
            blue: vec![0, 2000, 4000],
        };

        let adjusted = gamma.apply_gamma_adjustment(GammaAdjustment::identity());

        assert!(adjusted.approx_eq(&gamma));
    }

    #[test]
    fn gamma_adjustment_changes_channels_independently() {
        let gamma = Gamma {
            red: vec![0, 16384, 32768, 65535],
            green: vec![0, 16384, 32768, 65535],
            blue: vec![0, 16384, 32768, 65535],
        };

        let adjusted = gamma.apply_gamma_adjustment(GammaAdjustment {
            red: 2.0,
            green: 1.0,
            blue: 0.5,
        });

        assert!(adjusted.red[2] > gamma.red[2]);
        assert_eq!(adjusted.green, gamma.green);
        assert!(adjusted.blue[2] < gamma.blue[2]);
    }

    #[test]
    fn software_color_applies_gamma_before_brightness() {
        let gamma = Gamma {
            red: vec![0, 16384, 32768, 65535],
            green: vec![0, 16384, 32768, 65535],
            blue: vec![0, 16384, 32768, 65535],
        };
        let adjustment = GammaAdjustment {
            red: 2.0,
            green: 2.0,
            blue: 2.0,
        };

        let combined = gamma.apply_software_color(1.5, BrightnessFilter::Linear, adjustment);
        let sequential = gamma
            .apply_gamma_adjustment(adjustment)
            .apply_brightness(1.5, BrightnessFilter::Linear);

        assert!(combined.approx_eq(&sequential));
    }

    #[test]
    fn apply_monitor_serializes_color_mode_property() {
        let monitor = ApplyMonitor {
            connector: "eDP-1",
            mode_id: "1920x1080@60",
            properties: vec![ApplyMonitorProperty::ColorMode(ColorMode::Bt2100)],
        };

        let (_, _, properties) = monitor.serialize();
        let color_mode = properties
            .get("color-mode")
            .and_then(|value| value.0.as_u64())
            .unwrap();

        assert_eq!(color_mode, 1);
    }
}
