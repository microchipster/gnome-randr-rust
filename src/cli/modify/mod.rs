mod actions;

use std::cmp::Ordering;

use gnome_randr::display_config::proxied_methods::BrightnessFilter;
use gnome_randr::{
    display_config::ApplyConfig,
    display_config::{
        physical_monitor::Mode, physical_monitor::PhysicalMonitor, resources::Resources,
    },
    DisplayConfig,
};
use structopt::StructOpt;

use self::actions::{Action, ModeAction, PrimaryAction, RotationAction, ScaleAction};
use super::{
    brightness,
    common::{format_scale, match_supported_scale, parse_resolution, resolve_connector},
};

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
        value_name = "SCALE",
        help = "Scale such as 1, 1.25, 1.5, or 2 from query",
        long_help = "Scale factor reported by \"gnome-randr query CONNECTOR\" for this output, typically values like \"1\", \"1.25\", \"1.5\", or \"2\". You can type the displayed value directly even when Mutter's exact supported float has more precision internally; gnome-randr will choose the nearest advertised supported scale for the selected mode. Run \"gnome-randr query CONNECTOR\" to list the supported scales for that output."
    )]
    pub scale: Option<f64>,

    #[structopt(
        long,
        value_name = "BRIGHTNESS",
        help = "Brightness factor such as 0.5, 1, or 2",
        long_help = "Non-negative software brightness factor. \"1\" leaves the current ramp unchanged, \"0.5\" dims it, and \"2\" brightens it. With the default \"linear\" filter this exactly scales the current gamma ramp. The \"gamma\" and \"filmic\" filters keep more highlight detail when brightening above 1. Common presets are 0, 0.25, 0.5, 0.75, 1, 1.25, 1.5, and 2. This does not touch hardware backlight controls."
    )]
    pub brightness: Option<f64>,

    #[structopt(
        long,
        value_name = "FILTER",
        default_value = "linear",
        possible_values = brightness::FILTER_VALUES,
        parse(try_from_str = brightness::parse_filter),
        help = "Tone mapping filter: linear, gamma, or filmic",
        long_help = "Tone mapping filter for software brightness. \"linear\" (default) exactly scales the current gamma ramp like xrandr-style software brightness. \"gamma\" brightens midtones more gently when brightening above 1. \"filmic\" adds a stronger highlight rolloff to preserve contrast. All filters behave linearly when dimming below 1."
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
        long_help = "Preview the requested changes without applying them. This is useful to confirm the resolved connector, preferred/auto/refresh-selected mode, scale, rotation, primary or noprimary state, brightness, and filter changes first."
    )]
    dry_run: bool,
}

#[derive(Debug)]
pub enum Error {
    NotFound,
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
    let (logical_monitor, physical_monitor) = config.search(&connector).ok_or(Error::NotFound)?;
    let resolved_mode = resolve_mode(physical_monitor, &connector, &opts.actions)?;
    let resolved_scale = resolve_scale(
        physical_monitor,
        &connector,
        resolved_mode,
        opts.actions.scale,
    )?;

    let mut actions = Vec::<Box<dyn Action>>::new();
    let primary_is_changing = opts.actions.primary;
    let brightness = opts.actions.brightness;

    if let Some(rotation) = &opts.actions.rotation {
        actions.push(Box::new(RotationAction {
            rotation: *rotation,
        }));
    }

    if let Some(mode) = resolved_mode {
        actions.push(Box::new(ModeAction { mode: &mode.id }))
    }

    if opts.actions.primary {
        actions.push(Box::new(PrimaryAction { primary: true }));
    }

    if opts.actions.noprimary {
        actions.push(Box::new(PrimaryAction { primary: false }));
    }

    if let Some(scale) = resolved_scale {
        actions.push(Box::new(ScaleAction { scale }))
    }

    if actions.is_empty() && brightness.is_none() {
        println!("no changes made.");
        return Ok(());
    }

    if opts.dry_run {
        if !actions.is_empty() {
            let mut apply_config = ApplyConfig::from(logical_monitor, physical_monitor);

            if opts.persistent {
                println!("attempting to persist config to disk")
            }

            for action in actions.iter() {
                println!("{}", &action);
                action.apply(&mut apply_config, physical_monitor);
            }
        }

        if let Some(brightness) = brightness {
            let resources = Resources::get_resources(proxy)?;
            brightness::apply_brightness(
                &connector,
                brightness,
                opts.actions.filter,
                true,
                &resources,
                proxy,
            )?;
        }

        println!("dry run: no changes made.");
        return Ok(());
    }

    if !actions.is_empty() {
        let mut apply_config = ApplyConfig::from(logical_monitor, physical_monitor);

        if opts.persistent {
            println!("attempting to persist config to disk")
        }

        for action in actions.iter() {
            println!("{}", &action);
            action.apply(&mut apply_config, physical_monitor);
        }

        let all_configs = config
            .monitors
            .iter()
            .filter_map(|monitor| {
                if monitor.connector == connector {
                    return Some(apply_config.clone());
                }

                let (logical_monitor, _) = match config.search(&monitor.connector) {
                    Some(monitors) => monitors,
                    None => return None,
                };

                let mut apply_config = ApplyConfig::from(logical_monitor, monitor);

                if primary_is_changing {
                    apply_config.primary = false;
                }

                Some(apply_config)
            })
            .collect();

        config.apply_monitors_config(proxy, all_configs, opts.persistent)?;
    }

    if let Some(brightness) = brightness {
        let resources = Resources::get_resources(proxy)?;
        brightness::apply_brightness(
            &connector,
            brightness,
            opts.actions.filter,
            false,
            &resources,
            proxy,
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{resolve_mode, resolve_scale, ActionOptions, Error};
    use gnome_randr::display_config::physical_monitor::{
        KnownModeProperties, Mode, PhysicalMonitor,
    };
    use gnome_randr::display_config::proxied_methods::BrightnessFilter;

    fn actions() -> ActionOptions {
        ActionOptions {
            rotation: None,
            mode: None,
            preferred: false,
            auto_mode: false,
            refresh: None,
            primary: false,
            noprimary: false,
            scale: None,
            brightness: None,
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
}
