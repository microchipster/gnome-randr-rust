mod planner;

use std::cmp::Ordering;

use dbus::arg::{PropMap, Variant};

use gnome_randr::display_config::proxied_methods::{
    BacklightState, BrightnessFilter, ColorMode, GammaAdjustment, NativeDisplayState, PowerSaveMode,
};
use gnome_randr::{
    display_config::{
        physical_monitor::Mode,
        physical_monitor::PhysicalMonitor,
        resources::{Output, Resources},
        LayoutMode,
    },
    DisplayConfig,
};
use structopt::StructOpt;

use self::planner::{MonitorPlanner, RelativePlacement};
use super::{
    brightness,
    common::{
        format_scale, match_supported_scale, parse_position, parse_resolution, resolve_connector,
    },
};
use gnome_randr::display_config::logical_monitor::Transform;

const REFLECT_VALUES: &[&str] = &["normal", "x", "y", "xy"];
const COLOR_MODE_VALUES: &[&str] = &["default", "bt2100"];
const LAYOUT_MODE_VALUES: &[&str] = &["logical", "physical", "global-ui-logical"];
const POWER_SAVE_VALUES: &[&str] = &["on", "standby", "suspend", "off"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    x: i32,
    y: i32,
}

impl std::str::FromStr for Position {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (x, y) = parse_position(s)
            .ok_or_else(|| "position must be X,Y or XxY, for example 0,0 or 1920x0".to_string())?;
        Ok(Position { x, y })
    }
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{},{}", self.x, self.y)
    }
}

#[derive(Clone, Copy)]
struct RelativePlacementRequest<'a> {
    placement: RelativePlacement,
    reference: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reflection {
    Normal,
    X,
    Y,
    XY,
}

impl std::str::FromStr for Reflection {
    type Err = std::fmt::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "normal" => Ok(Reflection::Normal),
            "x" => Ok(Reflection::X),
            "y" => Ok(Reflection::Y),
            "xy" => Ok(Reflection::XY),
            _ => Err(std::fmt::Error),
        }
    }
}

impl std::fmt::Display for Reflection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Reflection::Normal => "normal",
                Reflection::X => "x",
                Reflection::Y => "y",
                Reflection::XY => "xy",
            }
        )
    }
}

#[derive(Debug)]
struct CloneRequest<'a> {
    reference: &'a str,
    mode: &'a Mode,
}

#[derive(Clone, Copy)]
pub enum Rotation {
    Normal,
    Left,
    Right,
    Inverted,
}

impl std::str::FromStr for Rotation {
    type Err = std::fmt::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "normal" => Ok(Rotation::Normal),
            "left" => Ok(Rotation::Left),
            "right" => Ok(Rotation::Right),
            "inverted" => Ok(Rotation::Inverted),
            _ => Err(std::fmt::Error),
        }
    }
}

impl std::fmt::Display for Rotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Rotation::Normal => "normal",
                Rotation::Left => "left",
                Rotation::Right => "right",
                Rotation::Inverted => "inverted",
            }
        )
    }
}

#[derive(StructOpt)]
pub struct ActionOptions {
    #[structopt(
        short,
        long = "rotate",
        value_name = "ROTATION",
        possible_values = &["normal", "left", "right", "inverted"],
        help = "Rotation: normal, left, right, or inverted",
        long_help = "Rotate the output. Valid values are \"normal\", \"left\", \"right\", and \"inverted\". \"right\" is clockwise and \"left\" is counter-clockwise."
    )]
    pub rotation: Option<Rotation>,

    #[structopt(
        long,
        value_name = "REFLECTION",
        possible_values = REFLECT_VALUES,
        help = "Reflection: normal, x, y, or xy",
        long_help = "Reflect the output using xrandr-style names: normal, x, y, or xy. This maps onto Mutter's flipped transform model and composes with rotation through the existing transform bits."
    )]
    pub reflect: Option<Reflection>,

    #[structopt(
        short,
        long,
        value_name = "MODE",
        help = "Mode id like 1920x1080@59.999, or resolution like 1920x1080",
        long_help = "Mode selection for this output. You can pass a full mode id from \"gnome-randr query CONNECTOR\", for example \"1920x1080@59.999\", or a resolution such as \"1920x1080\". When you pass only a resolution, gnome-randr chooses the preferred or current mode for that resolution unless you also add --refresh."
    )]
    pub mode: Option<String>,

    #[structopt(
        long,
        help = "Use the preferred mode for this output",
        long_help = "Select the preferred mode reported by \"gnome-randr query CONNECTOR\" for this output. This conflicts with --mode, --auto, and --refresh."
    )]
    pub preferred: bool,

    #[structopt(
        long = "auto",
        help = "Use the preferred mode, or current/best fallback",
        long_help = "Select the preferred mode for this output when one is advertised. If no preferred mode is available, keep the current mode or fall back to the best available mode. This conflicts with --mode, --preferred, and --refresh."
    )]
    pub auto_mode: bool,

    #[structopt(
        long = "refresh",
        alias = "rate",
        value_name = "HZ",
        help = "Nearest refresh for --mode <WxH> or the current resolution",
        long_help = "Select the nearest refresh rate in hertz for the requested resolution. Use this with \"--mode 1920x1080\" to avoid spelling the full mode id, or by itself to pick a different refresh for the current resolution. \"--rate\" is accepted as an alias. This conflicts with --preferred and --auto."
    )]
    pub refresh: Option<f64>,

    #[structopt(
        long,
        help = "Make this output the primary monitor",
        long_help = "Make this output the primary logical monitor. If another monitor is currently primary, it will be cleared. This conflicts with --noprimary."
    )]
    pub primary: bool,

    #[structopt(
        long,
        help = "Clear primary state from this output",
        long_help = "Clear the primary flag from this output's logical monitor. This does not automatically assign another output as primary. This conflicts with --primary."
    )]
    pub noprimary: bool,

    #[structopt(
        long,
        help = "Disable this output",
        long_help = "Disable this output by removing it from the applied logical-monitor layout. This is a real planner-level output disable, not a fake mode id. This conflicts with mode, preferred, auto, refresh, same-as mirroring, layout-mode, power-save, rotation, position, scale, primary, noprimary, backlight, luminance, reset-luminance, and software color options."
    )]
    pub off: bool,

    #[structopt(
        long = "layout-mode",
        value_name = "MODE",
        possible_values = LAYOUT_MODE_VALUES,
        parse(try_from_str = parse_layout_mode),
        help = "Global layout mode: logical, physical, or global-ui-logical",
        long_help = "Set Mutter's global layout mode. This is a Wayland-native control for the whole display configuration and does not require a connector."
    )]
    pub layout_mode: Option<LayoutMode>,

    #[structopt(
        long = "power-save",
        value_name = "MODE",
        possible_values = POWER_SAVE_VALUES,
        parse(try_from_str = parse_power_save_mode),
        help = "Global power-save mode: on, standby, suspend, or off",
        long_help = "Set Mutter's global power-save mode. This is a Wayland-native control for the whole display configuration and does not require a connector."
    )]
    pub power_save: Option<PowerSaveMode>,

    #[structopt(
        long,
        alias = "pos",
        value_name = "X,Y",
        help = "Absolute position such as 0,0 or 1920x0",
        long_help = "Absolute top-left position for this outputs logical monitor. Use a simple coordinate pair such as 0,0 or 1920x0. --pos is accepted as an alias. This conflicts with the relative placement flags."
    )]
    pub position: Option<Position>,

    #[structopt(
        long = "color-mode",
        value_name = "MODE",
        possible_values = COLOR_MODE_VALUES,
        parse(try_from_str = parse_color_mode),
        help = "Monitor color mode: default or bt2100",
        long_help = "Set a known Mutter color mode on this output. Current Mutter/GNOME builds expose color modes such as default and bt2100 when the monitor supports them. This is an explicit Mutter-native property control, not generic xrandr-style property plumbing."
    )]
    pub color_mode: Option<ColorMode>,

    #[structopt(
        long,
        value_name = "PERCENT",
        help = "Hardware backlight percentage such as 50 or 100",
        long_help = "Set hardware backlight through Mutter's native backlight API when the connector reports support. This is separate from software brightness and expects a percentage from 0 to 100."
    )]
    pub backlight: Option<i32>,

    #[structopt(
        long,
        value_name = "PERCENT",
        help = "Native luminance preference such as 80 or 100",
        long_help = "Set Mutter's native per-monitor luminance preference for the current or requested color mode. This is separate from software brightness and gamma and expects a percentage from 0 to 100."
    )]
    pub luminance: Option<f64>,

    #[structopt(
        long = "reset-luminance",
        help = "Reset the native luminance preference to default",
        long_help = "Reset Mutter's native luminance preference for the current or requested color mode back to the default value for this monitor."
    )]
    pub reset_luminance: bool,

    #[structopt(
        long = "same-as",
        value_name = "CONNECTOR",
        help = "Mirror this output onto CONNECTOR",
        long_help = "Mirror this output onto another outputs logical monitor, similar to xrandr same-as. gnome-randr validates obvious clone impossibilities first using Mutter's resource model, then moves this connector into the reference logical monitor with a matching mode when possible. This conflicts with off, mode selection, rotation, explicit position, relative placement, and scale."
    )]
    pub same_as: Option<String>,

    #[structopt(
        long = "left-of",
        value_name = "CONNECTOR",
        help = "Place this output immediately left of CONNECTOR",
        long_help = "Place this output immediately left of another enabled output. The final coordinates are computed from the planner's post-mode, post-scale, and post-rotation geometry. This conflicts with --position, --right-of, --above, --below, and --off."
    )]
    pub left_of: Option<String>,

    #[structopt(
        long = "right-of",
        value_name = "CONNECTOR",
        help = "Place this output immediately right of CONNECTOR",
        long_help = "Place this output immediately right of another enabled output. The final coordinates are computed from the planner's post-mode, post-scale, and post-rotation geometry. This conflicts with --position, --left-of, --above, --below, and --off."
    )]
    pub right_of: Option<String>,

    #[structopt(
        long,
        value_name = "CONNECTOR",
        help = "Place this output immediately above CONNECTOR",
        long_help = "Place this output immediately above another enabled output. The final coordinates are computed from the planner's post-mode, post-scale, and post-rotation geometry. This conflicts with --position, --left-of, --right-of, --below, and --off."
    )]
    pub above: Option<String>,

    #[structopt(
        long,
        value_name = "CONNECTOR",
        help = "Place this output immediately below CONNECTOR",
        long_help = "Place this output immediately below another enabled output. The final coordinates are computed from the planner's post-mode, post-scale, and post-rotation geometry. This conflicts with --position, --left-of, --right-of, --above, and --off."
    )]
    pub below: Option<String>,

    #[structopt(
        long,
        value_name = "SCALE",
        help = "Scale such as 1, 1.25, 1.5, or 2 from query",
        long_help = "Scale factor reported by \"gnome-randr query CONNECTOR\" for this output, typically values like \"1\", \"1.25\", \"1.5\", or \"2\". You can type the displayed value directly even when Mutter's exact supported float has more precision internally; gnome-randr will choose the nearest advertised supported scale for the selected mode. Run \"gnome-randr query CONNECTOR\" to list the supported scales for that output."
    )]
    pub scale: Option<f64>,

    #[structopt(
        long,
        value_name = "BRIGHTNESS",
        help = "Brightness factor such as 0.5, 1, or 2",
        long_help = "Non-negative software brightness factor. \"1\" leaves the current ramp unchanged, \"0.5\" dims it, and \"2\" brightens it. With the default \"linear\" filter this exactly scales the current gamma-adjusted ramp. The brightness filters named \"gamma\" and \"filmic\" only affect highlight mapping for --brightness; they are separate from the per-channel --gamma control. Common presets are 0, 0.25, 0.5, 0.75, 1, 1.25, 1.5, and 2. This does not touch hardware backlight controls."
    )]
    pub brightness: Option<f64>,

    #[structopt(
        long,
        value_name = "R[:G:B]",
        parse(try_from_str = brightness::parse_gamma_adjustment),
        help = "Per-channel software gamma such as 1 or 1.1:1:0.9",
        long_help = "Per-channel software gamma correction applied on top of the current baseline LUT before any brightness scaling. Use a single value such as \"1.1\" to apply the same gamma to red, green, and blue, or use \"R:G:B\" such as \"1.1:1.0:0.9\" for per-channel control. If green and blue are omitted, the red value is used for all three channels. Values must be greater than 0. This is separate from the brightness filter names."
    )]
    pub gamma: Option<GammaAdjustment>,

    #[structopt(
        long,
        value_name = "FILTER",
        default_value = "linear",
        possible_values = brightness::FILTER_VALUES,
        parse(try_from_str = brightness::parse_filter),
        help = "Tone mapping filter: linear, gamma, or filmic",
        long_help = "Tone mapping filter for software brightness. \"linear\" (default) exactly scales the current gamma-adjusted ramp like xrandr-style software brightness. \"gamma\" brightens midtones more gently when brightening above 1. \"filmic\" adds a stronger highlight rolloff to preserve contrast. All filters behave linearly when dimming below 1. This does not change the separate --gamma values."
    )]
    pub filter: BrightnessFilter,
}

#[derive(StructOpt)]
pub struct CommandOptions {
    #[structopt(
        value_name = "CONNECTOR",
        help = "Connector such as eDP-1 or HDMI-1",
        long_help = "Connector name for the output you want to modify, such as \"eDP-1\" or \"HDMI-1\". If exactly one output is connected it is used by default. Run \"gnome-randr query\" to list the valid connectors, modes, and scales first."
    )]
    pub connector: Option<String>,

    #[structopt(flatten)]
    pub actions: ActionOptions,

    #[structopt(
        short,
        long,
        help = "Persist this layout for this hardware set",
        long_help = "Try to persist this configuration so Mutter can reuse it the next time the same hardware layout appears."
    )]
    persistent: bool,

    #[structopt(
        long,
        help = "Preview the requested changes without applying them",
        long_help = "Preview the requested changes without applying them. This is useful to confirm the resolved connector, off/on state, same-as mirroring target, native layout-mode or power-save change, absolute or relative position, any geometry reflow after rotation or mode changes, preferred/auto/refresh-selected mode, scale, color mode, hardware backlight or luminance change, primary or noprimary state, and software brightness/gamma changes first."
    )]
    dry_run: bool,
}

#[derive(Debug)]
pub enum Error {
    NotFound,
    OutputDisabled {
        connector: String,
    },
    ReferenceOutputDisabled {
        connector: String,
    },
    CloneWithSelf {
        connector: String,
    },
    CloneNotSupported {
        connector: String,
        reference: String,
    },
    CloneCrtcNotShared {
        connector: String,
        reference: String,
    },
    CloneModeUnavailable {
        connector: String,
        reference: String,
        mode: String,
    },
    ColorModeUnsupported {
        connector: String,
    },
    ColorModeUnavailable {
        connector: String,
        requested: ColorMode,
        supported: Vec<ColorMode>,
    },
    LayoutModeUnsupported {
        requested: LayoutMode,
        current: LayoutMode,
    },
    PowerSaveUnsupported,
    BacklightUnsupported {
        connector: String,
    },
    InvalidBacklight(i32),
    LuminanceUnsupported {
        connector: String,
    },
    InvalidLuminance(f64),
    MutterRejectedClone {
        connector: String,
        reference: String,
        details: String,
    },
    ModeNotFound {
        connector: String,
        mode: String,
    },
    PreferredModeNotFound {
        connector: String,
    },
    AutoModeNotFound {
        connector: String,
    },
    CurrentModeNotFound {
        connector: String,
        option: &'static str,
    },
    InvalidScale {
        connector: String,
        mode: String,
        requested: f64,
        supported_scales: Vec<f64>,
    },
    RefreshWithExactMode {
        mode: String,
    },
    ConflictingOptions {
        option: &'static str,
        conflicting: &'static str,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotFound => write!(f, "fatal: unable to find output."),
            Error::OutputDisabled { connector } => write!(
                f,
                "fatal: {} is currently disabled. Re-enabling outputs is not part of this command yet; run \"gnome-randr query\" to inspect the current layout.",
                connector
            ),
            Error::ReferenceOutputDisabled { connector } => write!(
                f,
                "fatal: {} is currently disabled, so it cannot be used as a mirroring reference. Run \"gnome-randr query\" to inspect the active layout.",
                connector
            ),
            Error::CloneWithSelf { connector } => write!(
                f,
                "fatal: {} cannot mirror itself. Choose a different output for --same-as.",
                connector
            ),
            Error::CloneNotSupported {
                connector,
                reference,
            } => write!(
                f,
                "fatal: local clone preflight says {} cannot mirror {} with the current Mutter resource model.",
                connector, reference
            ),
            Error::CloneCrtcNotShared {
                connector,
                reference,
            } => write!(
                f,
                "fatal: local clone preflight says {} and {} do not share a possible CRTC, so the mirror request is obviously impossible.",
                connector, reference
            ),
            Error::CloneModeUnavailable {
                connector,
                reference,
                mode,
            } => write!(
                f,
                "fatal: {} cannot mirror {} because it has no compatible mode for {}. Run \"gnome-randr query {}\" and \"gnome-randr query {}\" to compare the available modes.",
                connector, reference, mode, connector, reference
            ),
            Error::ColorModeUnsupported { connector } => write!(
                f,
                "fatal: {} does not expose writable color modes through Mutter. Run \"gnome-randr query {} --properties\" to inspect the available monitor properties.",
                connector, connector
            ),
            Error::ColorModeUnavailable {
                connector,
                requested,
                supported,
            } => write!(
                f,
                "fatal: {} cannot use color mode {}. Supported color modes are {}.",
                connector,
                requested,
                supported
                    .iter()
                    .map(|mode| mode.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Error::LayoutModeUnsupported { requested, current } => write!(
                f,
                "fatal: requested layout mode {} but the current backend does not allow layout-mode changes from {}.",
                requested, current
            ),
            Error::PowerSaveUnsupported => write!(
                f,
                "fatal: current GNOME/Mutter backend reports power-save mode as unsupported."
            ),
            Error::BacklightUnsupported { connector } => write!(
                f,
                "fatal: {} does not expose native backlight control through Mutter.",
                connector
            ),
            Error::InvalidBacklight(value) => write!(
                f,
                "fatal: backlight value {} is invalid. Use an integer percentage from 0 to 100.",
                value
            ),
            Error::LuminanceUnsupported { connector } => write!(
                f,
                "fatal: {} does not expose native luminance preferences through Mutter.",
                connector
            ),
            Error::InvalidLuminance(value) => write!(
                f,
                "fatal: luminance value {} is invalid. Use a percentage from 0 to 100.",
                value
            ),
            Error::MutterRejectedClone {
                connector,
                reference,
                details,
            } => write!(
                f,
                "fatal: Mutter rejected mirroring {} same-as {}: {}. Partial mirroring is still limited by GNOME's DisplayConfig validation rules.",
                connector, reference, details
            ),
            Error::ModeNotFound { connector, mode } => write!(
                f,
                "fatal: mode or resolution \"{}\" is not available on {}. Run \"gnome-randr query {}\" to list valid mode ids.",
                mode, connector, connector
            ),
            Error::PreferredModeNotFound { connector } => write!(
                f,
                "fatal: {} does not advertise a preferred mode. Run \"gnome-randr query {}\" to inspect the available modes.",
                connector, connector
            ),
            Error::AutoModeNotFound { connector } => write!(
                f,
                "fatal: unable to choose an automatic mode for {}. Run \"gnome-randr query {}\" to inspect the available modes.",
                connector, connector
            ),
            Error::CurrentModeNotFound { connector, option } => write!(
                f,
                "fatal: unable to determine the current mode for {} while resolving {}. Run \"gnome-randr query {}\" and specify --mode explicitly.",
                connector, option, connector
            ),
            Error::InvalidScale {
                connector,
                mode,
                requested,
                supported_scales,
            } => write!(
                f,
                "fatal: scale {} is not valid for {} mode {}. Run \"gnome-randr query {}\" to see supported scales such as {}.",
                format_scale(*requested),
                connector,
                mode,
                connector,
                supported_scales
                    .iter()
                    .map(|scale| format_scale(*scale))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Error::RefreshWithExactMode { mode } => write!(
                f,
                "fatal: --refresh cannot be combined with the exact mode id \"{}\". Use just --mode {}, or use --mode WIDTHxHEIGHT with --refresh HZ.",
                mode, mode
            ),
            Error::ConflictingOptions {
                option,
                conflicting,
            } => write!(f, "fatal: {} cannot be used with {}.", option, conflicting),
        }
    }
}

impl std::error::Error for Error {}

fn validate_actions(actions: &ActionOptions) -> Result<(), Error> {
    let conflicts = [
        (
            actions.mode.is_some(),
            "--mode",
            actions.preferred,
            "--preferred",
        ),
        (
            actions.mode.is_some(),
            "--mode",
            actions.auto_mode,
            "--auto",
        ),
        (
            actions.preferred,
            "--preferred",
            actions.auto_mode,
            "--auto",
        ),
        (
            actions.preferred,
            "--preferred",
            actions.refresh.is_some(),
            "--refresh",
        ),
        (
            actions.auto_mode,
            "--auto",
            actions.refresh.is_some(),
            "--refresh",
        ),
        (
            actions.primary,
            "--primary",
            actions.noprimary,
            "--noprimary",
        ),
        (actions.off, "--off", actions.mode.is_some(), "--mode"),
        (actions.off, "--off", actions.preferred, "--preferred"),
        (actions.off, "--off", actions.auto_mode, "--auto"),
        (actions.off, "--off", actions.refresh.is_some(), "--refresh"),
        (actions.off, "--off", actions.rotation.is_some(), "--rotate"),
        (actions.off, "--off", actions.reflect.is_some(), "--reflect"),
        (
            actions.off,
            "--off",
            actions.position.is_some(),
            "--position",
        ),
        (actions.off, "--off", actions.scale.is_some(), "--scale"),
        (
            actions.off,
            "--off",
            actions.color_mode.is_some(),
            "--color-mode",
        ),
        (actions.off, "--off", actions.primary, "--primary"),
        (actions.off, "--off", actions.noprimary, "--noprimary"),
        (
            actions.off,
            "--off",
            actions.brightness.is_some(),
            "--brightness",
        ),
        (actions.off, "--off", actions.gamma.is_some(), "--gamma"),
        (
            actions.off,
            "--off",
            actions.layout_mode.is_some(),
            "--layout-mode",
        ),
        (
            actions.off,
            "--off",
            actions.power_save.is_some(),
            "--power-save",
        ),
        (
            actions.off,
            "--off",
            actions.backlight.is_some(),
            "--backlight",
        ),
        (
            actions.off,
            "--off",
            actions.luminance.is_some(),
            "--luminance",
        ),
        (
            actions.off,
            "--off",
            actions.reset_luminance,
            "--reset-luminance",
        ),
        (
            actions.position.is_some(),
            "--position",
            actions.left_of.is_some(),
            "--left-of",
        ),
        (
            actions.position.is_some(),
            "--position",
            actions.right_of.is_some(),
            "--right-of",
        ),
        (
            actions.position.is_some(),
            "--position",
            actions.above.is_some(),
            "--above",
        ),
        (
            actions.position.is_some(),
            "--position",
            actions.below.is_some(),
            "--below",
        ),
        (
            actions.left_of.is_some(),
            "--left-of",
            actions.right_of.is_some(),
            "--right-of",
        ),
        (
            actions.left_of.is_some(),
            "--left-of",
            actions.above.is_some(),
            "--above",
        ),
        (
            actions.left_of.is_some(),
            "--left-of",
            actions.below.is_some(),
            "--below",
        ),
        (
            actions.right_of.is_some(),
            "--right-of",
            actions.above.is_some(),
            "--above",
        ),
        (
            actions.right_of.is_some(),
            "--right-of",
            actions.below.is_some(),
            "--below",
        ),
        (
            actions.above.is_some(),
            "--above",
            actions.below.is_some(),
            "--below",
        ),
        (actions.off, "--off", actions.left_of.is_some(), "--left-of"),
        (
            actions.off,
            "--off",
            actions.right_of.is_some(),
            "--right-of",
        ),
        (actions.off, "--off", actions.above.is_some(), "--above"),
        (actions.off, "--off", actions.below.is_some(), "--below"),
        (actions.off, "--off", actions.same_as.is_some(), "--same-as"),
        (
            actions.same_as.is_some(),
            "--same-as",
            actions.mode.is_some(),
            "--mode",
        ),
        (
            actions.same_as.is_some(),
            "--same-as",
            actions.preferred,
            "--preferred",
        ),
        (
            actions.same_as.is_some(),
            "--same-as",
            actions.auto_mode,
            "--auto",
        ),
        (
            actions.same_as.is_some(),
            "--same-as",
            actions.refresh.is_some(),
            "--refresh",
        ),
        (
            actions.same_as.is_some(),
            "--same-as",
            actions.rotation.is_some(),
            "--rotate",
        ),
        (
            actions.same_as.is_some(),
            "--same-as",
            actions.reflect.is_some(),
            "--reflect",
        ),
        (
            actions.same_as.is_some(),
            "--same-as",
            actions.position.is_some(),
            "--position",
        ),
        (
            actions.same_as.is_some(),
            "--same-as",
            actions.left_of.is_some(),
            "--left-of",
        ),
        (
            actions.same_as.is_some(),
            "--same-as",
            actions.right_of.is_some(),
            "--right-of",
        ),
        (
            actions.same_as.is_some(),
            "--same-as",
            actions.above.is_some(),
            "--above",
        ),
        (
            actions.same_as.is_some(),
            "--same-as",
            actions.below.is_some(),
            "--below",
        ),
        (
            actions.same_as.is_some(),
            "--same-as",
            actions.scale.is_some(),
            "--scale",
        ),
        (
            actions.same_as.is_some(),
            "--same-as",
            actions.color_mode.is_some(),
            "--color-mode",
        ),
        (
            actions.same_as.is_some(),
            "--same-as",
            actions.backlight.is_some(),
            "--backlight",
        ),
        (
            actions.same_as.is_some(),
            "--same-as",
            actions.luminance.is_some(),
            "--luminance",
        ),
        (
            actions.same_as.is_some(),
            "--same-as",
            actions.reset_luminance,
            "--reset-luminance",
        ),
        (
            actions.luminance.is_some(),
            "--luminance",
            actions.reset_luminance,
            "--reset-luminance",
        ),
    ];

    for (option_used, option, conflicting_used, conflicting) in conflicts {
        if option_used && conflicting_used {
            return Err(Error::ConflictingOptions {
                option,
                conflicting,
            });
        }
    }

    Ok(())
}

fn parse_layout_mode(value: &str) -> Result<LayoutMode, String> {
    value.parse()
}

fn parse_power_save_mode(value: &str) -> Result<PowerSaveMode, String> {
    match value.parse::<PowerSaveMode>()? {
        PowerSaveMode::Unknown => {
            Err("power-save mode must be on, standby, suspend, or off".to_string())
        }
        mode => Ok(mode),
    }
}

fn parse_color_mode(value: &str) -> Result<ColorMode, String> {
    value.parse()
}

fn per_output_actions_requested(actions: &ActionOptions) -> bool {
    actions.rotation.is_some()
        || actions.reflect.is_some()
        || actions.mode.is_some()
        || actions.preferred
        || actions.auto_mode
        || actions.refresh.is_some()
        || actions.primary
        || actions.noprimary
        || actions.off
        || actions.position.is_some()
        || actions.color_mode.is_some()
        || actions.backlight.is_some()
        || actions.luminance.is_some()
        || actions.reset_luminance
        || actions.same_as.is_some()
        || actions.left_of.is_some()
        || actions.right_of.is_some()
        || actions.above.is_some()
        || actions.below.is_some()
        || actions.scale.is_some()
        || actions.brightness.is_some()
        || actions.gamma.is_some()
}

fn rotation_transform(rotation: Rotation) -> u32 {
    match rotation {
        Rotation::Normal => Transform::NORMAL.bits(),
        Rotation::Left => Transform::R270.bits(),
        Rotation::Right => Transform::R90.bits(),
        Rotation::Inverted => Transform::R180.bits(),
    }
}

fn rotation_bits(transform: u32) -> u32 {
    transform & Transform::R270.bits()
}

fn rotate_180(transform: u32) -> u32 {
    (rotation_bits(transform) + 2) & Transform::R270.bits()
}

fn reflection_from_transform(transform: u32) -> Reflection {
    if transform & Transform::FLIPPED.bits() == 0 {
        Reflection::Normal
    } else if matches!(rotation_bits(transform), bits if bits == Transform::R180.bits() || bits == Transform::R270.bits())
    {
        Reflection::X
    } else {
        Reflection::Y
    }
}

fn transform_from_rotation_and_reflection(rotation: Rotation, reflection: Reflection) -> u32 {
    let rotation = rotation_transform(rotation);

    match reflection {
        Reflection::Normal => rotation,
        Reflection::Y => rotation | Transform::FLIPPED.bits(),
        Reflection::X => rotate_180(rotation) | Transform::FLIPPED.bits(),
        Reflection::XY => rotate_180(rotation),
    }
}

fn effective_transform(
    current_transform: u32,
    rotation: Option<Rotation>,
    reflection: Option<Reflection>,
) -> Option<u32> {
    if rotation.is_none() && reflection.is_none() {
        return None;
    }

    let current_rotation = match rotation_bits(current_transform) {
        bits if bits == Transform::R90.bits() => Rotation::Right,
        bits if bits == Transform::R180.bits() => Rotation::Inverted,
        bits if bits == Transform::R270.bits() => Rotation::Left,
        _ => Rotation::Normal,
    };

    Some(transform_from_rotation_and_reflection(
        rotation.unwrap_or(current_rotation),
        reflection.unwrap_or_else(|| reflection_from_transform(current_transform)),
    ))
}

fn transform_swaps_axes(transform: u32) -> bool {
    matches!(rotation_bits(transform), bits if bits == Transform::R90.bits() || bits == Transform::R270.bits())
}

fn supported_color_modes(physical_monitor: &PhysicalMonitor) -> Vec<ColorMode> {
    physical_monitor
        .properties
        .get("supported-color-modes")
        .and_then(|value| value.0.as_iter())
        .map(|iter| {
            iter.filter_map(|value| value.as_u64())
                .filter_map(|value| ColorMode::from_raw(value as u32))
                .collect()
        })
        .unwrap_or_default()
}

fn relative_placement_request(actions: &ActionOptions) -> Option<RelativePlacementRequest<'_>> {
    if let Some(reference) = actions.left_of.as_deref() {
        Some(RelativePlacementRequest {
            placement: RelativePlacement::LeftOf,
            reference,
        })
    } else if let Some(reference) = actions.right_of.as_deref() {
        Some(RelativePlacementRequest {
            placement: RelativePlacement::RightOf,
            reference,
        })
    } else if let Some(reference) = actions.above.as_deref() {
        Some(RelativePlacementRequest {
            placement: RelativePlacement::Above,
            reference,
        })
    } else {
        actions
            .below
            .as_deref()
            .map(|reference| RelativePlacementRequest {
                placement: RelativePlacement::Below,
                reference,
            })
    }
}

fn output_for_connector<'a>(resources: &'a Resources, connector: &str) -> Option<&'a Output> {
    resources
        .outputs
        .iter()
        .find(|output| output.name == connector)
}

fn resolve_clone_request<'a>(
    connector: &str,
    reference: &'a str,
    target_monitor: &'a PhysicalMonitor,
    config: &'a DisplayConfig,
    resources: &'a Resources,
) -> Result<CloneRequest<'a>, Error> {
    if connector == reference {
        return Err(Error::CloneWithSelf {
            connector: connector.to_string(),
        });
    }

    if config.logical_monitor_for_connector(reference).is_none() {
        return Err(Error::ReferenceOutputDisabled {
            connector: reference.to_string(),
        });
    }

    let reference_monitor = config.physical_monitor(reference).ok_or(Error::NotFound)?;
    let reference_mode = current_mode(reference_monitor, reference, "--same-as")?;

    let target_output = output_for_connector(resources, connector).ok_or(Error::NotFound)?;
    let reference_output = output_for_connector(resources, reference).ok_or(Error::NotFound)?;

    if !target_output.clones.contains(&reference_output.id)
        && !reference_output.clones.contains(&target_output.id)
    {
        return Err(Error::CloneNotSupported {
            connector: connector.to_string(),
            reference: reference.to_string(),
        });
    }

    if !target_output
        .possible_crtcs
        .iter()
        .any(|crtc| reference_output.possible_crtcs.contains(crtc))
    {
        return Err(Error::CloneCrtcNotShared {
            connector: connector.to_string(),
            reference: reference.to_string(),
        });
    }

    let mode = choose_nearest_refresh_mode(
        modes_for_resolution(target_monitor, reference_mode.width, reference_mode.height),
        reference_mode.refresh_rate,
    )
    .ok_or_else(|| Error::CloneModeUnavailable {
        connector: connector.to_string(),
        reference: reference.to_string(),
        mode: reference_mode.id.clone(),
    })?;

    Ok(CloneRequest { reference, mode })
}

fn resolve_color_mode_request(
    connector: &str,
    physical_monitor: &PhysicalMonitor,
    requested: Option<ColorMode>,
) -> Result<Option<ColorMode>, Error> {
    let requested = match requested {
        Some(requested) => requested,
        None => return Ok(None),
    };

    let supported = supported_color_modes(physical_monitor);
    if supported.is_empty() {
        return Err(Error::ColorModeUnsupported {
            connector: connector.to_string(),
        });
    }

    if !supported.contains(&requested) {
        return Err(Error::ColorModeUnavailable {
            connector: connector.to_string(),
            requested,
            supported,
        });
    }

    Ok(Some(requested))
}

fn current_color_mode(physical_monitor: &PhysicalMonitor) -> Option<ColorMode> {
    physical_monitor
        .properties
        .get("color-mode")
        .and_then(|value| value.0.as_u64())
        .and_then(|value| ColorMode::from_raw(value as u32))
}

fn resolve_luminance_color_mode(
    connector: &str,
    physical_monitor: &PhysicalMonitor,
    requested: Option<ColorMode>,
) -> Result<ColorMode, Error> {
    let requested = requested.or_else(|| current_color_mode(physical_monitor));

    match requested {
        Some(color_mode) => {
            resolve_color_mode_request(connector, physical_monitor, Some(color_mode))
                .map(|value| value.unwrap())
        }
        None => Err(Error::LuminanceUnsupported {
            connector: connector.to_string(),
        }),
    }
}

fn layout_mode_properties(layout_mode: LayoutMode) -> PropMap {
    let mut properties = PropMap::new();
    properties.insert(
        "layout-mode".to_string(),
        Variant(Box::new(layout_mode.raw_value())),
    );
    properties
}

fn resolved_layout_mode(
    config: &DisplayConfig,
    requested: Option<LayoutMode>,
) -> Result<Option<LayoutMode>, Error> {
    let requested = match requested {
        Some(requested) => requested,
        None => return Ok(None),
    };

    if requested == config.known_properties.layout_mode {
        return Ok(None);
    }

    if !config.known_properties.supports_changing_layout_mode {
        return Err(Error::LayoutModeUnsupported {
            requested,
            current: config.known_properties.layout_mode,
        });
    }

    Ok(Some(requested))
}

fn backlight_connector<'a>(
    native_state: &'a NativeDisplayState,
    connector: &str,
) -> Option<(u32, &'a BacklightState)> {
    native_state.backlight.as_ref().and_then(|backlight| {
        backlight
            .connectors
            .iter()
            .find(|entry| entry.connector == connector)
            .map(|_| (backlight.serial, backlight))
    })
}

fn validate_backlight_request(value: i32) -> Result<(), Error> {
    if !(0..=100).contains(&value) {
        return Err(Error::InvalidBacklight(value));
    }

    Ok(())
}

fn validate_luminance_request(value: f64) -> Result<(), Error> {
    if !value.is_finite() || !(0.0..=100.0).contains(&value) {
        return Err(Error::InvalidLuminance(value));
    }

    Ok(())
}

fn compare_mode_priority(left: &Mode, right: &Mode) -> Ordering {
    left.known_properties
        .is_preferred
        .cmp(&right.known_properties.is_preferred)
        .then_with(|| {
            left.known_properties
                .is_current
                .cmp(&right.known_properties.is_current)
        })
        .then_with(|| {
            (left.width as i64 * left.height as i64)
                .cmp(&(right.width as i64 * right.height as i64))
        })
        .then_with(|| {
            left.refresh_rate
                .partial_cmp(&right.refresh_rate)
                .unwrap_or(Ordering::Equal)
        })
}

fn preferred_mode<'a>(
    physical_monitor: &'a PhysicalMonitor,
    connector: &str,
) -> Result<&'a Mode, Error> {
    physical_monitor
        .modes
        .iter()
        .find(|mode| mode.known_properties.is_preferred)
        .ok_or_else(|| Error::PreferredModeNotFound {
            connector: connector.to_string(),
        })
}

fn current_mode<'a>(
    physical_monitor: &'a PhysicalMonitor,
    connector: &str,
    option: &'static str,
) -> Result<&'a Mode, Error> {
    physical_monitor
        .modes
        .iter()
        .find(|mode| mode.known_properties.is_current)
        .ok_or_else(|| Error::CurrentModeNotFound {
            connector: connector.to_string(),
            option,
        })
}

fn auto_mode<'a>(
    physical_monitor: &'a PhysicalMonitor,
    connector: &str,
) -> Result<&'a Mode, Error> {
    preferred_mode(physical_monitor, connector)
        .or_else(|_| current_mode(physical_monitor, connector, "--auto"))
        .or_else(|_| {
            physical_monitor
                .modes
                .iter()
                .max_by(|left, right| compare_mode_priority(left, right))
                .ok_or_else(|| Error::AutoModeNotFound {
                    connector: connector.to_string(),
                })
        })
}

fn modes_for_resolution<'a>(
    physical_monitor: &'a PhysicalMonitor,
    width: i32,
    height: i32,
) -> Vec<&'a Mode> {
    physical_monitor
        .modes
        .iter()
        .filter(|mode| mode.width == width && mode.height == height)
        .collect()
}

fn choose_mode_for_resolution<'a>(modes: Vec<&'a Mode>) -> Option<&'a Mode> {
    modes
        .into_iter()
        .max_by(|left, right| compare_mode_priority(left, right))
}

fn choose_nearest_refresh_mode<'a>(
    modes: Vec<&'a Mode>,
    requested_refresh: f64,
) -> Option<&'a Mode> {
    modes.into_iter().min_by(|left, right| {
        let left_distance = (left.refresh_rate - requested_refresh).abs();
        let right_distance = (right.refresh_rate - requested_refresh).abs();

        left_distance
            .partial_cmp(&right_distance)
            .unwrap_or(Ordering::Equal)
            .then_with(|| compare_mode_priority(right, left))
    })
}

fn resolve_mode<'a>(
    physical_monitor: &'a PhysicalMonitor,
    connector: &str,
    actions: &ActionOptions,
) -> Result<Option<&'a Mode>, Error> {
    if actions.preferred {
        return preferred_mode(physical_monitor, connector).map(Some);
    }

    if actions.auto_mode {
        return auto_mode(physical_monitor, connector).map(Some);
    }

    if let Some(requested_mode) = actions.mode.as_deref() {
        if let Some(mode) = physical_monitor
            .modes
            .iter()
            .find(|candidate| candidate.id == requested_mode)
        {
            if actions.refresh.is_some() {
                return Err(Error::RefreshWithExactMode {
                    mode: requested_mode.to_string(),
                });
            }

            return Ok(Some(mode));
        }

        if let Some((width, height)) = parse_resolution(requested_mode) {
            let modes = modes_for_resolution(physical_monitor, width, height);
            let mode = match actions.refresh {
                Some(refresh) => choose_nearest_refresh_mode(modes, refresh),
                None => choose_mode_for_resolution(modes),
            };

            return mode
                .ok_or_else(|| Error::ModeNotFound {
                    connector: connector.to_string(),
                    mode: requested_mode.to_string(),
                })
                .map(Some);
        }

        return Err(Error::ModeNotFound {
            connector: connector.to_string(),
            mode: requested_mode.to_string(),
        });
    }

    if let Some(refresh) = actions.refresh {
        let current = current_mode(physical_monitor, connector, "--refresh")?;
        let modes = modes_for_resolution(physical_monitor, current.width, current.height);
        return Ok(choose_nearest_refresh_mode(modes, refresh));
    }

    Ok(None)
}

fn selected_mode_for_scale<'a>(
    physical_monitor: &'a PhysicalMonitor,
    connector: &str,
    resolved_mode: Option<&'a Mode>,
) -> Result<&'a Mode, Error> {
    match resolved_mode {
        Some(mode) => Ok(mode),
        None => current_mode(physical_monitor, connector, "--scale"),
    }
}

fn resolve_scale(
    physical_monitor: &PhysicalMonitor,
    connector: &str,
    resolved_mode: Option<&Mode>,
    requested_scale: Option<f64>,
) -> Result<Option<f64>, Error> {
    let requested_scale = match requested_scale {
        Some(scale) => scale,
        None => return Ok(None),
    };

    let mode = selected_mode_for_scale(physical_monitor, connector, resolved_mode)?;

    match match_supported_scale(requested_scale, &mode.supported_scales) {
        Some(scale) => Ok(Some(scale)),
        None => Err(Error::InvalidScale {
            connector: connector.to_string(),
            mode: mode.id.clone(),
            requested: requested_scale,
            supported_scales: mode.supported_scales.clone(),
        }),
    }
}

pub fn handle(
    opts: &CommandOptions,
    config: &DisplayConfig,
    proxy: &dbus::blocking::Proxy<&dbus::blocking::Connection>,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_actions(&opts.actions)?;

    let needs_connector = per_output_actions_requested(&opts.actions);
    let connector = if needs_connector {
        Some(resolve_connector(
            opts.connector.as_deref(),
            config
                .monitors
                .iter()
                .map(|monitor| monitor.connector.as_str()),
        )?)
    } else {
        None
    };
    let physical_monitor = connector
        .as_deref()
        .and_then(|connector| config.physical_monitor(connector));
    let logical_monitor = connector
        .as_deref()
        .and_then(|connector| config.logical_monitor_for_connector(connector));
    let output_enabled = logical_monitor.is_some();
    let current_transform = logical_monitor
        .map(|logical_monitor| logical_monitor.transform.bits())
        .unwrap_or(Transform::NORMAL.bits());
    let resolved_layout_mode = resolved_layout_mode(config, opts.actions.layout_mode)?;
    let power_save_mode = opts.actions.power_save;
    let brightness = opts.actions.brightness;
    let gamma_adjustment = opts.actions.gamma;
    let same_as = opts.actions.same_as.as_deref();
    let native_controls_requested = resolved_layout_mode.is_some()
        || power_save_mode.is_some()
        || opts.actions.backlight.is_some()
        || opts.actions.luminance.is_some()
        || opts.actions.reset_luminance;
    let native_state = if native_controls_requested {
        Some(DisplayConfig::native_display_state(proxy)?)
    } else {
        None
    };
    let resources = if brightness.is_some() || gamma_adjustment.is_some() || same_as.is_some() {
        Some(Resources::get_resources(proxy)?)
    } else {
        None
    };
    let clone_request = match same_as {
        Some(reference) => Some(resolve_clone_request(
            connector.as_deref().unwrap(),
            reference,
            physical_monitor.ok_or(Error::NotFound)?,
            config,
            resources.as_ref().unwrap(),
        )?),
        None => None,
    };
    let resolved_mode = if opts.actions.off || clone_request.is_some() {
        None
    } else {
        if needs_connector && !output_enabled {
            return Err(Error::OutputDisabled {
                connector: connector.clone().unwrap(),
            }
            .into());
        }
        match (physical_monitor, connector.as_deref()) {
            (Some(physical_monitor), Some(connector)) => {
                resolve_mode(physical_monitor, connector, &opts.actions)?
            }
            _ => None,
        }
    };
    let resolved_scale = if opts.actions.off || clone_request.is_some() {
        None
    } else {
        match (physical_monitor, connector.as_deref()) {
            (Some(physical_monitor), Some(connector)) => resolve_scale(
                physical_monitor,
                connector,
                resolved_mode,
                opts.actions.scale,
            )?,
            _ => None,
        }
    };
    let resolved_color_mode = if opts.actions.off || clone_request.is_some() {
        None
    } else {
        match (physical_monitor, connector.as_deref()) {
            (Some(physical_monitor), Some(connector)) => {
                resolve_color_mode_request(connector, physical_monitor, opts.actions.color_mode)?
            }
            _ => None,
        }
    };
    let resolved_backlight = match (
        opts.actions.backlight,
        connector.as_deref(),
        native_state.as_ref(),
    ) {
        (Some(value), Some(connector), Some(native_state)) => {
            validate_backlight_request(value)?;
            if backlight_connector(native_state, connector).is_none() {
                return Err(Error::BacklightUnsupported {
                    connector: connector.to_string(),
                }
                .into());
            }
            Some(value)
        }
        _ => None,
    };
    let resolved_luminance_color_mode = match (
        opts.actions.luminance,
        opts.actions.reset_luminance,
        connector.as_deref(),
        physical_monitor,
        native_state.as_ref(),
    ) {
        (Some(luminance), _, Some(connector), Some(physical_monitor), Some(native_state)) => {
            validate_luminance_request(luminance)?;
            if !native_state
                .luminance
                .iter()
                .any(|entry| entry.connector == connector)
            {
                return Err(Error::LuminanceUnsupported {
                    connector: connector.to_string(),
                }
                .into());
            }
            Some(resolve_luminance_color_mode(
                connector,
                physical_monitor,
                resolved_color_mode,
            )?)
        }
        (None, true, Some(connector), Some(physical_monitor), Some(native_state)) => {
            if !native_state
                .luminance
                .iter()
                .any(|entry| entry.connector == connector)
            {
                return Err(Error::LuminanceUnsupported {
                    connector: connector.to_string(),
                }
                .into());
            }
            Some(resolve_luminance_color_mode(
                connector,
                physical_monitor,
                resolved_color_mode,
            )?)
        }
        _ => None,
    };
    let relative_placement = relative_placement_request(&opts.actions);
    let target_transform = effective_transform(
        current_transform,
        opts.actions.rotation,
        opts.actions.reflect,
    );
    let geometry_changes = target_transform
        .map(|transform| transform_swaps_axes(transform) != transform_swaps_axes(current_transform))
        .unwrap_or(false)
        || resolved_mode.is_some()
        || resolved_scale.is_some();
    let explicit_placement = opts.actions.position.is_some() || relative_placement.is_some();

    let has_layout_changes = opts.actions.off
        || clone_request.is_some()
        || target_transform.is_some()
        || resolved_layout_mode.is_some()
        || opts.actions.position.is_some()
        || relative_placement.is_some()
        || resolved_mode.is_some()
        || opts.actions.primary
        || opts.actions.noprimary
        || resolved_scale.is_some()
        || resolved_color_mode.is_some();

    if has_layout_changes
        && needs_connector
        && !opts.actions.off
        && clone_request.is_none()
        && !output_enabled
    {
        return Err(Error::OutputDisabled {
            connector: connector.clone().unwrap(),
        }
        .into());
    }

    if opts.actions.off
        && needs_connector
        && !output_enabled
        && brightness.is_none()
        && gamma_adjustment.is_none()
        && resolved_backlight.is_none()
        && opts.actions.luminance.is_none()
        && !opts.actions.reset_luminance
    {
        println!("no changes made.");
        return Ok(());
    }

    if !has_layout_changes
        && brightness.is_none()
        && gamma_adjustment.is_none()
        && power_save_mode.is_none()
        && resolved_backlight.is_none()
        && opts.actions.luminance.is_none()
        && !opts.actions.reset_luminance
    {
        println!("no changes made.");
        return Ok(());
    }

    let mut planner = if has_layout_changes {
        Some(MonitorPlanner::new(config)?)
    } else {
        None
    };

    if opts.dry_run {
        if has_layout_changes {
            if opts.persistent {
                println!("attempting to persist config to disk")
            }

            if let Some(layout_mode) = resolved_layout_mode {
                println!("setting layout mode to {}", layout_mode);
            }

            let old_geometry = if connector.is_some()
                && !opts.actions.off
                && geometry_changes
                && !explicit_placement
            {
                Some(
                    planner
                        .as_ref()
                        .unwrap()
                        .geometry(connector.as_deref().unwrap())?,
                )
            } else {
                None
            };

            if opts.actions.off {
                println!("disabling output {}", connector.as_deref().unwrap());
                planner
                    .as_mut()
                    .unwrap()
                    .remove_output(connector.as_deref().unwrap())?;
            }

            if let Some(clone) = &clone_request {
                println!(
                    "mirroring output {} same as {} using mode {}",
                    connector.as_deref().unwrap(),
                    clone.reference,
                    clone.mode.id
                );
                planner.as_mut().unwrap().clone_with(
                    connector.as_deref().unwrap(),
                    clone.reference,
                    &clone.mode.id,
                )?;
            }

            if let Some(rotation) = &opts.actions.rotation {
                println!("setting rotation to {}", rotation);
            }

            if let Some(reflection) = &opts.actions.reflect {
                println!("setting reflection to {}", reflection);
            }

            if let Some(transform) = target_transform {
                planner
                    .as_mut()
                    .unwrap()
                    .set_transform(connector.as_deref().unwrap(), transform)?;
            }

            if let Some(mode) = resolved_mode {
                println!("setting mode to {}", mode.id);
                planner
                    .as_mut()
                    .unwrap()
                    .set_mode(connector.as_deref().unwrap(), &mode.id)?;
            }

            if let Some(scale) = resolved_scale {
                println!("setting scale to {}", format_scale(scale));
                planner
                    .as_mut()
                    .unwrap()
                    .set_scale(connector.as_deref().unwrap(), scale)?;
            }

            if let Some(color_mode) = resolved_color_mode {
                println!("setting color mode to {}", color_mode);
                planner
                    .as_mut()
                    .unwrap()
                    .set_color_mode(connector.as_deref().unwrap(), color_mode)?;
            }

            if let Some(position) = opts.actions.position {
                println!("setting position to {}", position);
                planner.as_mut().unwrap().set_position(
                    connector.as_deref().unwrap(),
                    position.x,
                    position.y,
                )?;
            } else if let Some(relative) = relative_placement {
                let (x, y) = planner.as_mut().unwrap().place_relative(
                    connector.as_deref().unwrap(),
                    relative.reference,
                    relative.placement,
                )?;
                println!(
                    "placing output {} {} at {},{}",
                    connector.as_deref().unwrap(),
                    relative.placement.describe(),
                    x,
                    y
                );
            } else if let Some(old_geometry) = old_geometry {
                planner
                    .as_mut()
                    .unwrap()
                    .reflow_after_geometry_change(connector.as_deref().unwrap(), old_geometry)?;
                let (x, y) = planner
                    .as_ref()
                    .unwrap()
                    .position(connector.as_deref().unwrap())?;
                println!(
                    "resolved final position to {},{} after geometry reflow",
                    x, y
                );
            }

            if opts.actions.primary {
                println!("setting monitor as primary");
                planner
                    .as_mut()
                    .unwrap()
                    .set_primary(connector.as_deref().unwrap())?;
            }

            if opts.actions.noprimary {
                println!("clearing primary status from this monitor");
                planner
                    .as_mut()
                    .unwrap()
                    .clear_primary(connector.as_deref().unwrap())?;
            }
        }

        if let Some(mode) = power_save_mode {
            println!("setting power save mode to {}", mode);
        }

        if let Some(backlight) = resolved_backlight {
            println!(
                "setting hardware backlight on {} to {}",
                connector.as_deref().unwrap(),
                backlight
            );
        }

        if let Some(color_mode) = resolved_luminance_color_mode {
            if let Some(luminance) = opts.actions.luminance {
                println!(
                    "setting luminance on {} for {} to {}",
                    connector.as_deref().unwrap(),
                    color_mode,
                    luminance
                );
            }
            if opts.actions.reset_luminance {
                println!(
                    "resetting luminance on {} for {}",
                    connector.as_deref().unwrap(),
                    color_mode
                );
            }
        }

        if brightness.is_some() || gamma_adjustment.is_some() {
            brightness::apply_color(
                connector.as_deref().unwrap(),
                brightness,
                opts.actions.filter,
                gamma_adjustment,
                true,
                resources.as_ref().unwrap(),
                proxy,
            )?;
        }

        println!("dry run: no changes made.");
        return Ok(());
    }

    if has_layout_changes {
        if opts.persistent {
            println!("attempting to persist config to disk")
        }

        let old_geometry = if connector.is_some()
            && !opts.actions.off
            && geometry_changes
            && !explicit_placement
        {
            Some(
                planner
                    .as_ref()
                    .unwrap()
                    .geometry(connector.as_deref().unwrap())?,
            )
        } else {
            None
        };

        if let Some(layout_mode) = resolved_layout_mode {
            println!("setting layout mode to {}", layout_mode);
        }

        if opts.actions.off {
            println!("disabling output {}", connector.as_deref().unwrap());
            planner
                .as_mut()
                .unwrap()
                .remove_output(connector.as_deref().unwrap())?;
        }

        if let Some(clone) = &clone_request {
            println!(
                "mirroring output {} same as {} using mode {}",
                connector.as_deref().unwrap(),
                clone.reference,
                clone.mode.id
            );
            planner.as_mut().unwrap().clone_with(
                connector.as_deref().unwrap(),
                clone.reference,
                &clone.mode.id,
            )?;
        }

        if let Some(rotation) = &opts.actions.rotation {
            println!("setting rotation to {}", rotation);
        }

        if let Some(reflection) = &opts.actions.reflect {
            println!("setting reflection to {}", reflection);
        }

        if let Some(transform) = target_transform {
            planner
                .as_mut()
                .unwrap()
                .set_transform(connector.as_deref().unwrap(), transform)?;
        }

        if let Some(mode) = resolved_mode {
            println!("setting mode to {}", mode.id);
            planner
                .as_mut()
                .unwrap()
                .set_mode(connector.as_deref().unwrap(), &mode.id)?;
        }

        if let Some(scale) = resolved_scale {
            println!("setting scale to {}", format_scale(scale));
            planner
                .as_mut()
                .unwrap()
                .set_scale(connector.as_deref().unwrap(), scale)?;
        }

        if let Some(color_mode) = resolved_color_mode {
            println!("setting color mode to {}", color_mode);
            planner
                .as_mut()
                .unwrap()
                .set_color_mode(connector.as_deref().unwrap(), color_mode)?;
        }

        if let Some(position) = opts.actions.position {
            println!("setting position to {}", position);
            planner.as_mut().unwrap().set_position(
                connector.as_deref().unwrap(),
                position.x,
                position.y,
            )?;
        } else if let Some(relative) = relative_placement {
            let (x, y) = planner.as_mut().unwrap().place_relative(
                connector.as_deref().unwrap(),
                relative.reference,
                relative.placement,
            )?;
            println!(
                "placing output {} {} at {},{}",
                connector.as_deref().unwrap(),
                relative.placement.describe(),
                x,
                y
            );
        } else if let Some(old_geometry) = old_geometry {
            planner
                .as_mut()
                .unwrap()
                .reflow_after_geometry_change(connector.as_deref().unwrap(), old_geometry)?;
            let (x, y) = planner
                .as_ref()
                .unwrap()
                .position(connector.as_deref().unwrap())?;
            println!(
                "resolved final position to {},{} after geometry reflow",
                x, y
            );
        }

        if opts.actions.primary {
            println!("setting monitor as primary");
            planner
                .as_mut()
                .unwrap()
                .set_primary(connector.as_deref().unwrap())?;
        }

        if opts.actions.noprimary {
            println!("clearing primary status from this monitor");
            planner
                .as_mut()
                .unwrap()
                .clear_primary(connector.as_deref().unwrap())?;
        }

        let layout_properties = resolved_layout_mode
            .map(layout_mode_properties)
            .unwrap_or_else(PropMap::new);

        if let Err(error) = config.apply_monitors_config_with_properties(
            proxy,
            planner.unwrap().into_configs(),
            opts.persistent,
            layout_properties,
        ) {
            if let Some(clone) = &clone_request {
                return Err(Error::MutterRejectedClone {
                    connector: connector.clone().unwrap(),
                    reference: clone.reference.to_string(),
                    details: error.to_string(),
                }
                .into());
            }

            return Err(error.into());
        }
    }

    if let Some(mode) = power_save_mode {
        if native_state
            .as_ref()
            .map(|state| state.power_save_mode == PowerSaveMode::Unknown)
            .unwrap_or(true)
        {
            return Err(Error::PowerSaveUnsupported.into());
        }
        println!("setting power save mode to {}", mode);
        DisplayConfig::set_power_save_mode_native(proxy, mode)?;
    }

    if let Some(backlight) = resolved_backlight {
        let connector = connector.as_deref().unwrap();
        let (serial, _) = backlight_connector(native_state.as_ref().unwrap(), connector)
            .ok_or_else(|| Error::BacklightUnsupported {
                connector: connector.to_string(),
            })?;
        println!(
            "setting hardware backlight on {} to {}",
            connector, backlight
        );
        DisplayConfig::set_backlight(proxy, serial, connector, backlight)?;
    }

    if let Some(color_mode) = resolved_luminance_color_mode {
        let connector = connector.as_deref().unwrap();
        if let Some(luminance) = opts.actions.luminance {
            println!(
                "setting luminance on {} for {} to {}",
                connector, color_mode, luminance
            );
            DisplayConfig::set_luminance(proxy, connector, color_mode, luminance)?;
        }
        if opts.actions.reset_luminance {
            println!("resetting luminance on {} for {}", connector, color_mode);
            DisplayConfig::reset_luminance(proxy, connector, color_mode)?;
        }
    }

    if brightness.is_some() || gamma_adjustment.is_some() {
        brightness::apply_color(
            connector.as_deref().unwrap(),
            brightness,
            opts.actions.filter,
            gamma_adjustment,
            false,
            resources.as_ref().unwrap(),
            proxy,
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        resolve_clone_request, resolve_mode, resolve_scale, validate_actions, ActionOptions, Error,
        Position,
    };
    use gnome_randr::display_config::physical_monitor::{
        KnownModeProperties, Mode, PhysicalMonitor,
    };
    use gnome_randr::display_config::proxied_methods::BrightnessFilter;
    use gnome_randr::display_config::resources::{Mode as ResourceMode, Output, Resources};

    fn actions() -> ActionOptions {
        ActionOptions {
            rotation: None,
            reflect: None,
            mode: None,
            preferred: false,
            auto_mode: false,
            refresh: None,
            primary: false,
            noprimary: false,
            off: false,
            layout_mode: None,
            power_save: None,
            position: None,
            backlight: None,
            luminance: None,
            reset_luminance: false,
            same_as: None,
            left_of: None,
            right_of: None,
            above: None,
            below: None,
            scale: None,
            color_mode: None,
            brightness: None,
            gamma: None,
            filter: BrightnessFilter::Linear,
        }
    }

    fn monitor_with_modes(modes: Vec<Mode>) -> PhysicalMonitor {
        PhysicalMonitor {
            connector: "eDP-1".to_string(),
            vendor: "BOE".to_string(),
            product: "0x07c9".to_string(),
            serial: "0x00000000".to_string(),
            modes,
            properties: Default::default(),
        }
    }

    fn resources() -> Resources {
        Resources {
            serial: 1,
            crtcs: vec![],
            outputs: vec![
                Output {
                    id: 1,
                    winsys_id: 1,
                    current_crtc: 1,
                    possible_crtcs: vec![10],
                    name: "eDP-1".to_string(),
                    modes: vec![100],
                    clones: vec![2],
                    properties: Default::default(),
                },
                Output {
                    id: 2,
                    winsys_id: 2,
                    current_crtc: 2,
                    possible_crtcs: vec![10, 20],
                    name: "HDMI-1".to_string(),
                    modes: vec![200],
                    clones: vec![1],
                    properties: Default::default(),
                },
            ],
            modes: vec![ResourceMode {
                id: 100,
                winsys_id: 100,
                width: 1920,
                height: 1080,
                frequency: 60.0,
                flags: 0,
            }],
            max_screen_width: 8192,
            max_screen_height: 8192,
        }
    }

    fn display_config() -> gnome_randr::DisplayConfig {
        use gnome_randr::{
            display_config::{
                logical_monitor::{LogicalMonitor, Monitor, Transform},
                KnownProperties, LayoutMode,
            },
            DisplayConfig,
        };

        DisplayConfig {
            serial: 1,
            monitors: vec![
                PhysicalMonitor {
                    connector: "eDP-1".to_string(),
                    vendor: "BOE".to_string(),
                    product: "0x07c9".to_string(),
                    serial: "0x00000000".to_string(),
                    modes: vec![mode_with_details(
                        "1920x1080@60",
                        1920,
                        1080,
                        60.0,
                        vec![1.0],
                        false,
                        true,
                    )],
                    properties: Default::default(),
                },
                PhysicalMonitor {
                    connector: "HDMI-1".to_string(),
                    vendor: "Dell".to_string(),
                    product: "U2720Q".to_string(),
                    serial: "0x11111111".to_string(),
                    modes: vec![mode_with_details(
                        "1920x1080@59.94",
                        1920,
                        1080,
                        59.94,
                        vec![1.0],
                        true,
                        true,
                    )],
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
                    connector: "HDMI-1".to_string(),
                    vendor: "Dell".to_string(),
                    product: "U2720Q".to_string(),
                    serial: "0x11111111".to_string(),
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

    fn mode(id: &str, supported_scales: Vec<f64>, is_current: bool) -> Mode {
        Mode {
            id: id.to_string(),
            width: 1920,
            height: 1080,
            refresh_rate: 60.0,
            preferred_scale: supported_scales[0],
            supported_scales,
            known_properties: KnownModeProperties {
                is_current,
                is_preferred: is_current,
            },
            properties: Default::default(),
        }
    }

    fn mode_with_details(
        id: &str,
        width: i32,
        height: i32,
        refresh_rate: f64,
        supported_scales: Vec<f64>,
        is_current: bool,
        is_preferred: bool,
    ) -> Mode {
        Mode {
            id: id.to_string(),
            width,
            height,
            refresh_rate,
            preferred_scale: supported_scales[0],
            supported_scales,
            known_properties: KnownModeProperties {
                is_current,
                is_preferred,
            },
            properties: Default::default(),
        }
    }

    #[test]
    fn resolve_mode_accepts_resolution_with_refresh() {
        let monitor = monitor_with_modes(vec![
            mode_with_details(
                "1920x1080@59.934",
                1920,
                1080,
                59.934,
                vec![1.0],
                true,
                false,
            ),
            mode_with_details(
                "1920x1080@47.997",
                1920,
                1080,
                47.997,
                vec![1.0],
                false,
                false,
            ),
            mode_with_details(
                "1920x1080@60.100",
                1920,
                1080,
                60.100,
                vec![1.0],
                false,
                true,
            ),
        ]);
        let mut actions = actions();
        actions.mode = Some("1920x1080".to_string());
        actions.refresh = Some(60.0);

        let resolved = resolve_mode(&monitor, "eDP-1", &actions).unwrap().unwrap();
        assert_eq!(resolved.id, "1920x1080@59.934");
    }

    #[test]
    fn resolve_mode_uses_current_resolution_for_refresh_only() {
        let monitor = monitor_with_modes(vec![
            mode_with_details(
                "1920x1080@59.934",
                1920,
                1080,
                59.934,
                vec![1.0],
                true,
                false,
            ),
            mode_with_details(
                "1920x1080@47.997",
                1920,
                1080,
                47.997,
                vec![1.0],
                false,
                false,
            ),
            mode_with_details("1280x720@60", 1280, 720, 60.0, vec![1.0], false, true),
        ]);
        let mut actions = actions();
        actions.refresh = Some(48.0);

        let resolved = resolve_mode(&monitor, "eDP-1", &actions).unwrap().unwrap();
        assert_eq!(resolved.id, "1920x1080@47.997");
    }

    #[test]
    fn resolve_mode_rejects_refresh_with_exact_mode_id() {
        let monitor = monitor_with_modes(vec![mode("1920x1080@59.999", vec![1.0], true)]);
        let mut actions = actions();
        actions.mode = Some("1920x1080@59.999".to_string());
        actions.refresh = Some(60.0);

        match resolve_mode(&monitor, "eDP-1", &actions).unwrap_err() {
            Error::RefreshWithExactMode { mode } => assert_eq!(mode, "1920x1080@59.999"),
            error => panic!("unexpected error variant: {:?}", error),
        }
    }

    #[test]
    fn resolve_mode_selects_preferred_mode() {
        let monitor = monitor_with_modes(vec![
            mode_with_details(
                "1920x1080@59.934",
                1920,
                1080,
                59.934,
                vec![1.0],
                true,
                false,
            ),
            mode_with_details(
                "1920x1080@60.100",
                1920,
                1080,
                60.100,
                vec![1.0],
                false,
                true,
            ),
        ]);
        let mut actions = actions();
        actions.preferred = true;

        let resolved = resolve_mode(&monitor, "eDP-1", &actions).unwrap().unwrap();
        assert_eq!(resolved.id, "1920x1080@60.100");
    }

    #[test]
    fn resolve_mode_auto_falls_back_to_current_mode() {
        let monitor = monitor_with_modes(vec![
            mode_with_details(
                "1920x1080@59.934",
                1920,
                1080,
                59.934,
                vec![1.0],
                true,
                false,
            ),
            mode_with_details("1280x720@60", 1280, 720, 60.0, vec![1.0], false, false),
        ]);
        let mut actions = actions();
        actions.auto_mode = true;

        let resolved = resolve_mode(&monitor, "eDP-1", &actions).unwrap().unwrap();
        assert_eq!(resolved.id, "1920x1080@59.934");
    }

    #[test]
    fn resolve_scale_uses_selected_mode_supported_scales() {
        let monitor = monitor_with_modes(vec![
            mode("mode-a", vec![1.0, 1.25], true),
            mode("mode-b", vec![1.0, 1.7518248], false),
        ]);
        let resolved_mode = monitor.modes.iter().find(|mode| mode.id == "mode-b");

        let resolved = resolve_scale(&monitor, "eDP-1", resolved_mode, Some(1.75)).unwrap();
        assert_eq!(resolved, Some(1.7518248));
    }

    #[test]
    fn resolve_scale_reports_helpful_error_for_invalid_scale() {
        let monitor = monitor_with_modes(vec![mode("mode-a", vec![1.0, 1.7518248], true)]);

        let error = resolve_scale(&monitor, "eDP-1", None, Some(1.73)).unwrap_err();
        match error {
            Error::InvalidScale {
                connector,
                mode,
                requested,
                supported_scales,
            } => {
                assert_eq!(connector, "eDP-1");
                assert_eq!(mode, "mode-a");
                assert_eq!(requested, 1.73);
                assert_eq!(supported_scales, vec![1.0, 1.7518248]);
            }
            _ => panic!("unexpected error variant"),
        }
    }

    #[test]
    fn validate_actions_rejects_off_with_position() {
        let mut actions = actions();
        actions.off = true;
        actions.position = Some(Position { x: 1920, y: 0 });

        match validate_actions(&actions).unwrap_err() {
            Error::ConflictingOptions {
                option,
                conflicting,
            } => {
                assert_eq!(option, "--off");
                assert_eq!(conflicting, "--position");
            }
            error => panic!("unexpected error variant: {:?}", error),
        }
    }

    #[test]
    fn validate_actions_rejects_absolute_and_relative_position_together() {
        let mut actions = actions();
        actions.position = Some(Position { x: 0, y: 0 });
        actions.left_of = Some("HDMI-1".to_string());

        match validate_actions(&actions).unwrap_err() {
            Error::ConflictingOptions {
                option,
                conflicting,
            } => {
                assert_eq!(option, "--position");
                assert_eq!(conflicting, "--left-of");
            }
            error => panic!("unexpected error variant: {:?}", error),
        }
    }

    #[test]
    fn validate_actions_rejects_same_as_with_position() {
        let mut actions = actions();
        actions.same_as = Some("HDMI-1".to_string());
        actions.position = Some(Position { x: 0, y: 0 });

        match validate_actions(&actions).unwrap_err() {
            Error::ConflictingOptions {
                option,
                conflicting,
            } => {
                assert_eq!(option, "--same-as");
                assert_eq!(conflicting, "--position");
            }
            error => panic!("unexpected error variant: {:?}", error),
        }
    }

    #[test]
    fn resolve_clone_request_uses_matching_mode_on_target() {
        let config = display_config();
        let resources = resources();
        let clone = resolve_clone_request(
            "eDP-1",
            "HDMI-1",
            config.physical_monitor("eDP-1").unwrap(),
            &config,
            &resources,
        )
        .unwrap();

        assert_eq!(clone.reference, "HDMI-1");
        assert_eq!(clone.mode.id, "1920x1080@60");
    }

    #[test]
    fn resolve_clone_request_rejects_missing_clone_capability() {
        let config = display_config();
        let mut resources = resources();
        resources.outputs[0].clones.clear();
        resources.outputs[1].clones.clear();

        match resolve_clone_request(
            "eDP-1",
            "HDMI-1",
            config.physical_monitor("eDP-1").unwrap(),
            &config,
            &resources,
        )
        .unwrap_err()
        {
            Error::CloneNotSupported {
                connector,
                reference,
            } => {
                assert_eq!(connector, "eDP-1");
                assert_eq!(reference, "HDMI-1");
            }
            error => panic!("unexpected error variant: {:?}", error),
        }
    }
}
