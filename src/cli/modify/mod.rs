mod planner;

use std::cmp::Ordering;

use gnome_randr::display_config::proxied_methods::{BrightnessFilter, GammaAdjustment};
use gnome_randr::{
    display_config::{
        physical_monitor::Mode,
        physical_monitor::PhysicalMonitor,
        resources::{Output, Resources},
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
        long_help = "Disable this output by removing it from the applied logical-monitor layout. This is a real planner-level output disable, not a fake mode id. This conflicts with mode, preferred, auto, refresh, same-as mirroring, rotation, position, scale, primary, noprimary, and brightness options."
    )]
    pub off: bool,

    #[structopt(
        long,
        alias = "pos",
        value_name = "X,Y",
        help = "Absolute position such as 0,0 or 1920x0",
        long_help = "Absolute top-left position for this outputs logical monitor. Use a simple coordinate pair such as 0,0 or 1920x0. --pos is accepted as an alias. This conflicts with the relative placement flags."
    )]
    pub position: Option<Position>,

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
        long_help = "Preview the requested changes without applying them. This is useful to confirm the resolved connector, off/on state, same-as mirroring target, absolute or relative position, any geometry reflow after rotation or mode changes, preferred/auto/refresh-selected mode, scale, primary or noprimary state, brightness, and filter changes first."
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
        (
            actions.off,
            "--off",
            actions.position.is_some(),
            "--position",
        ),
        (actions.off, "--off", actions.scale.is_some(), "--scale"),
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

fn rotation_transform(rotation: Rotation) -> u32 {
    match rotation {
        Rotation::Normal => Transform::NORMAL.bits(),
        Rotation::Left => Transform::R270.bits(),
        Rotation::Right => Transform::R90.bits(),
        Rotation::Inverted => Transform::R180.bits(),
    }
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

    let connector = resolve_connector(
        opts.connector.as_deref(),
        config
            .monitors
            .iter()
            .map(|monitor| monitor.connector.as_str()),
    )?;
    let physical_monitor = config.physical_monitor(&connector).ok_or(Error::NotFound)?;
    let logical_monitor = config.logical_monitor_for_connector(&connector);
    let output_enabled = logical_monitor.is_some();
    let brightness = opts.actions.brightness;
    let gamma_adjustment = opts.actions.gamma;
    let same_as = opts.actions.same_as.as_deref();
    let resources = if brightness.is_some() || gamma_adjustment.is_some() || same_as.is_some() {
        Some(Resources::get_resources(proxy)?)
    } else {
        None
    };
    let clone_request = match same_as {
        Some(reference) => Some(resolve_clone_request(
            &connector,
            reference,
            physical_monitor,
            config,
            resources.as_ref().unwrap(),
        )?),
        None => None,
    };
    let resolved_mode = if opts.actions.off || clone_request.is_some() {
        None
    } else {
        if !output_enabled {
            return Err(Error::OutputDisabled {
                connector: connector.clone(),
            }
            .into());
        }
        resolve_mode(physical_monitor, &connector, &opts.actions)?
    };
    let resolved_scale = if opts.actions.off || clone_request.is_some() {
        None
    } else {
        resolve_scale(
            physical_monitor,
            &connector,
            resolved_mode,
            opts.actions.scale,
        )?
    };
    let relative_placement = relative_placement_request(&opts.actions);
    let geometry_changes =
        opts.actions.rotation.is_some() || resolved_mode.is_some() || resolved_scale.is_some();
    let explicit_placement = opts.actions.position.is_some() || relative_placement.is_some();

    let has_layout_changes = opts.actions.off
        || clone_request.is_some()
        || opts.actions.rotation.is_some()
        || opts.actions.position.is_some()
        || relative_placement.is_some()
        || resolved_mode.is_some()
        || opts.actions.primary
        || opts.actions.noprimary
        || resolved_scale.is_some();

    if opts.actions.off && !output_enabled && brightness.is_none() && gamma_adjustment.is_none() {
        println!("no changes made.");
        return Ok(());
    }

    if !has_layout_changes && brightness.is_none() && gamma_adjustment.is_none() {
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

            let old_geometry = if !opts.actions.off && geometry_changes && !explicit_placement {
                Some(planner.as_ref().unwrap().geometry(&connector)?)
            } else {
                None
            };

            if opts.actions.off {
                println!("disabling output {}", connector);
                planner.as_mut().unwrap().remove_output(&connector)?;
            }

            if let Some(clone) = &clone_request {
                println!(
                    "mirroring output {} same as {} using mode {}",
                    connector, clone.reference, clone.mode.id
                );
                planner.as_mut().unwrap().clone_with(
                    &connector,
                    clone.reference,
                    &clone.mode.id,
                )?;
            }

            if let Some(rotation) = &opts.actions.rotation {
                println!("setting rotation to {}", rotation);
                planner
                    .as_mut()
                    .unwrap()
                    .set_transform(&connector, rotation_transform(*rotation))?;
            }

            if let Some(mode) = resolved_mode {
                println!("setting mode to {}", mode.id);
                planner.as_mut().unwrap().set_mode(&connector, &mode.id)?;
            }

            if let Some(scale) = resolved_scale {
                println!("setting scale to {}", format_scale(scale));
                planner.as_mut().unwrap().set_scale(&connector, scale)?;
            }

            if let Some(position) = opts.actions.position {
                println!("setting position to {}", position);
                planner
                    .as_mut()
                    .unwrap()
                    .set_position(&connector, position.x, position.y)?;
            } else if let Some(relative) = relative_placement {
                let (x, y) = planner.as_mut().unwrap().place_relative(
                    &connector,
                    relative.reference,
                    relative.placement,
                )?;
                println!(
                    "placing output {} {} at {},{}",
                    connector,
                    relative.placement.describe(),
                    x,
                    y
                );
            } else if let Some(old_geometry) = old_geometry {
                planner
                    .as_mut()
                    .unwrap()
                    .reflow_after_geometry_change(&connector, old_geometry)?;
                let (x, y) = planner.as_ref().unwrap().position(&connector)?;
                println!(
                    "resolved final position to {},{} after geometry reflow",
                    x, y
                );
            }

            if opts.actions.primary {
                println!("setting monitor as primary");
                planner.as_mut().unwrap().set_primary(&connector)?;
            }

            if opts.actions.noprimary {
                println!("clearing primary status from this monitor");
                planner.as_mut().unwrap().clear_primary(&connector)?;
            }
        }

        if brightness.is_some() || gamma_adjustment.is_some() {
            brightness::apply_color(
                &connector,
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

        let old_geometry = if !opts.actions.off && geometry_changes && !explicit_placement {
            Some(planner.as_ref().unwrap().geometry(&connector)?)
        } else {
            None
        };

        if opts.actions.off {
            println!("disabling output {}", connector);
            planner.as_mut().unwrap().remove_output(&connector)?;
        }

        if let Some(clone) = &clone_request {
            println!(
                "mirroring output {} same as {} using mode {}",
                connector, clone.reference, clone.mode.id
            );
            planner
                .as_mut()
                .unwrap()
                .clone_with(&connector, clone.reference, &clone.mode.id)?;
        }

        if let Some(rotation) = &opts.actions.rotation {
            println!("setting rotation to {}", rotation);
            planner
                .as_mut()
                .unwrap()
                .set_transform(&connector, rotation_transform(*rotation))?;
        }

        if let Some(mode) = resolved_mode {
            println!("setting mode to {}", mode.id);
            planner.as_mut().unwrap().set_mode(&connector, &mode.id)?;
        }

        if let Some(scale) = resolved_scale {
            println!("setting scale to {}", format_scale(scale));
            planner.as_mut().unwrap().set_scale(&connector, scale)?;
        }

        if let Some(position) = opts.actions.position {
            println!("setting position to {}", position);
            planner
                .as_mut()
                .unwrap()
                .set_position(&connector, position.x, position.y)?;
        } else if let Some(relative) = relative_placement {
            let (x, y) = planner.as_mut().unwrap().place_relative(
                &connector,
                relative.reference,
                relative.placement,
            )?;
            println!(
                "placing output {} {} at {},{}",
                connector,
                relative.placement.describe(),
                x,
                y
            );
        } else if let Some(old_geometry) = old_geometry {
            planner
                .as_mut()
                .unwrap()
                .reflow_after_geometry_change(&connector, old_geometry)?;
            let (x, y) = planner.as_ref().unwrap().position(&connector)?;
            println!(
                "resolved final position to {},{} after geometry reflow",
                x, y
            );
        }

        if opts.actions.primary {
            println!("setting monitor as primary");
            planner.as_mut().unwrap().set_primary(&connector)?;
        }

        if opts.actions.noprimary {
            println!("clearing primary status from this monitor");
            planner.as_mut().unwrap().clear_primary(&connector)?;
        }

        if let Err(error) =
            config.apply_monitors_config(proxy, planner.unwrap().into_configs(), opts.persistent)
        {
            if let Some(clone) = &clone_request {
                return Err(Error::MutterRejectedClone {
                    connector: connector.clone(),
                    reference: clone.reference.to_string(),
                    details: error.to_string(),
                }
                .into());
            }

            return Err(error.into());
        }
    }

    if brightness.is_some() || gamma_adjustment.is_some() {
        brightness::apply_color(
            &connector,
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
            mode: None,
            preferred: false,
            auto_mode: false,
            refresh: None,
            primary: false,
            noprimary: false,
            off: false,
            position: None,
            same_as: None,
            left_of: None,
            right_of: None,
            above: None,
            below: None,
            scale: None,
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
