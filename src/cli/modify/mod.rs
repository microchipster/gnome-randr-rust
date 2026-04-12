mod actions;

use gnome_randr::display_config::proxied_methods::BrightnessFilter;
use gnome_randr::{
    display_config::resources::Resources, display_config::ApplyConfig, DisplayConfig,
};
use structopt::StructOpt;

use self::actions::{Action, ModeAction, PrimaryAction, RotationAction, ScaleAction};
use super::{brightness, common::resolve_connector};

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
        help = "Mode id such as 1920x1080@60 from query",
        long_help = "Mode id reported by \"gnome-randr query CONNECTOR\" for this output, for example \"1920x1080@59.999\". Run \"gnome-randr query\" to list connectors, then \"gnome-randr query CONNECTOR\" to see that output's valid mode ids."
    )]
    pub mode: Option<String>,

    #[structopt(
        long,
        help = "Make this output the primary monitor",
        long_help = "Make this output the primary logical monitor. If another monitor is currently primary, it will be cleared."
    )]
    pub primary: bool,

    #[structopt(
        long,
        value_name = "SCALE",
        help = "Scale such as 1, 1.25, 1.5, or 2 from query",
        long_help = "Scale factor reported by \"gnome-randr query CONNECTOR\" for this output, typically values like \"1\", \"1.25\", \"1.5\", or \"2\". Run \"gnome-randr query CONNECTOR\" to list the supported scales for that output."
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
        long_help = "Preview the requested changes without applying them. This is useful to confirm the resolved connector, mode, scale, rotation, primary-monitor, brightness, and filter changes first."
    )]
    dry_run: bool,
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
) -> Result<(), Box<dyn std::error::Error>> {
    let connector = resolve_connector(
        opts.connector.as_deref(),
        config
            .monitors
            .iter()
            .map(|monitor| monitor.connector.as_str()),
    )?;
    let (logical_monitor, physical_monitor) = config.search(&connector).ok_or(Error::NotFound)?;

    let mut actions = Vec::<Box<dyn Action>>::new();
    let primary_is_changing = opts.actions.primary;
    let brightness = opts.actions.brightness;

    if let Some(rotation) = &opts.actions.rotation {
        actions.push(Box::new(RotationAction {
            rotation: *rotation,
        }));
    }

    if let Some(mode_id) = &opts.actions.mode {
        actions.push(Box::new(ModeAction { mode: mode_id }))
    }

    if opts.actions.primary {
        actions.push(Box::new(PrimaryAction {}));
    }

    if let Some(scale) = &opts.actions.scale {
        actions.push(Box::new(ScaleAction { scale: *scale }))
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
