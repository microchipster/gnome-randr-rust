use std::{
    convert::TryFrom,
    env, fs,
    path::{Path, PathBuf},
};

use gnome_randr::display_config::{
    proxied_methods::{BrightnessFilter, Gamma},
    resources::{Crtc, Output, Resources},
};

use super::common::format_scale;

pub(super) const FILTER_VALUES: &[&str] = &["linear", "gamma", "filmic"];

pub(super) fn parse_filter(value: &str) -> Result<BrightnessFilter, String> {
    value.parse()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentBrightnessState {
    Managed,
    Identity,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CurrentBrightness {
    pub state: CurrentBrightnessState,
    pub brightness: Option<f64>,
    pub filter: Option<BrightnessFilter>,
}

impl CurrentBrightness {
    pub fn managed(brightness: f64, filter: BrightnessFilter) -> CurrentBrightness {
        CurrentBrightness {
            state: CurrentBrightnessState::Managed,
            brightness: Some(brightness),
            filter: Some(filter),
        }
    }

    pub fn identity() -> CurrentBrightness {
        CurrentBrightness {
            state: CurrentBrightnessState::Identity,
            brightness: Some(1.0),
            filter: Some(BrightnessFilter::Linear),
        }
    }

    pub fn unknown() -> CurrentBrightness {
        CurrentBrightness {
            state: CurrentBrightnessState::Unknown,
            brightness: None,
            filter: None,
        }
    }
}

impl std::fmt::Display for CurrentBrightness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.brightness, self.filter) {
            (Some(brightness), Some(filter)) => {
                write!(f, "{} ({})", format_scale(brightness), filter)
            }
            _ => write!(f, "unknown"),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    NotFound,
    OutputDisabled,
    InvalidBrightness,
    CrtcNotFound,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Error::NotFound => "fatal: unable to find output.",
                Error::OutputDisabled => "fatal: output is disabled.",
                Error::InvalidBrightness => {
                    "fatal: brightness must be a finite number greater than or equal to 0."
                }
                Error::CrtcNotFound => "fatal: unable to find CRTC for output.",
            }
        )
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone)]
struct BrightnessState {
    brightness: f64,
    filter: BrightnessFilter,
    gamma: Gamma,
}

impl BrightnessState {
    fn parse(contents: &str) -> Option<BrightnessState> {
        let mut brightness = None;
        let mut filter = None;
        let mut red = None;
        let mut green = None;
        let mut blue = None;

        for line in contents.lines() {
            let (key, value) = line.split_once('=')?;
            match key {
                "brightness" => brightness = value.parse::<f64>().ok(),
                "filter" => filter = value.parse::<BrightnessFilter>().ok(),
                "red" => red = parse_channel(value),
                "green" => green = parse_channel(value),
                "blue" => blue = parse_channel(value),
                _ => {}
            }
        }

        Some(BrightnessState {
            brightness: brightness?,
            filter: filter.unwrap_or(BrightnessFilter::Linear),
            gamma: Gamma {
                red: red?,
                green: green?,
                blue: blue?,
            },
        })
    }

    fn serialize(&self) -> String {
        format!(
            "brightness={}\nfilter={}\nred={}\ngreen={}\nblue={}\n",
            self.brightness,
            self.filter,
            serialize_channel(&self.gamma.red),
            serialize_channel(&self.gamma.green),
            serialize_channel(&self.gamma.blue),
        )
    }
}

fn parse_channel(value: &str) -> Option<Vec<u16>> {
    if value.is_empty() {
        return Some(Vec::new());
    }

    value
        .split(',')
        .map(|entry| entry.parse::<u16>().ok())
        .collect()
}

fn serialize_channel(channel: &[u16]) -> String {
    channel
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<String>>()
        .join(",")
}

fn state_dir() -> Option<PathBuf> {
    if let Some(path) = env::var_os("XDG_RUNTIME_DIR") {
        return Some(PathBuf::from(path).join("gnome-randr"));
    }

    env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .map(|path| path.join("gnome-randr"))
}

fn state_file(connector: &str) -> Option<PathBuf> {
    let file_name = connector
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => ch,
            _ => '_',
        })
        .collect::<String>();

    state_dir().map(|path| path.join(format!("brightness-{}.state", file_name)))
}

fn load_state(path: &Path) -> Option<BrightnessState> {
    BrightnessState::parse(&fs::read_to_string(path).ok()?)
}

fn save_state(path: &Path, state: &BrightnessState) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, state.serialize())
}

fn resolve_base_gamma(current_gamma: &Gamma, state: Option<&BrightnessState>) -> Gamma {
    match state {
        Some(state)
            if current_gamma
                .approx_eq(&state.gamma.apply_brightness(state.brightness, state.filter)) =>
        {
            state.gamma.clone()
        }
        _ => current_gamma.clone(),
    }
}

fn matching_state(
    current_gamma: &Gamma,
    state: Option<&BrightnessState>,
) -> Option<CurrentBrightness> {
    match state {
        Some(state)
            if current_gamma
                .approx_eq(&state.gamma.apply_brightness(state.brightness, state.filter)) =>
        {
            Some(CurrentBrightness::managed(state.brightness, state.filter))
        }
        _ => None,
    }
}

fn find_output<'a>(resources: &'a Resources, connector: &str) -> Result<&'a Output, Error> {
    resources
        .outputs
        .iter()
        .find(|output| output.name == connector)
        .ok_or(Error::NotFound)
}

fn find_crtc<'a>(resources: &'a Resources, output: &Output) -> Result<&'a Crtc, Error> {
    let crtc_id = u32::try_from(output.current_crtc).map_err(|_| Error::OutputDisabled)?;

    resources
        .crtcs
        .iter()
        .find(|crtc| crtc.id == crtc_id)
        .ok_or(Error::CrtcNotFound)
}

pub(super) fn load_current_brightness(
    connector: &str,
    resources: &Resources,
    proxy: &dbus::blocking::Proxy<&dbus::blocking::Connection>,
) -> Result<CurrentBrightness, Box<dyn std::error::Error>> {
    let output = find_output(resources, connector)?;
    let crtc = find_crtc(resources, output)?;
    let current_gamma = resources.get_crtc_gamma(proxy, crtc)?;

    let state_path = state_file(connector);
    let saved_state = state_path.as_deref().and_then(load_state);

    if let Some(current) = matching_state(&current_gamma, saved_state.as_ref()) {
        return Ok(current);
    }

    if current_gamma.is_identity() {
        return Ok(CurrentBrightness::identity());
    }

    Ok(CurrentBrightness::unknown())
}

pub(super) fn apply_brightness(
    connector: &str,
    brightness: f64,
    filter: BrightnessFilter,
    dry_run: bool,
    resources: &Resources,
    proxy: &dbus::blocking::Proxy<&dbus::blocking::Connection>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !brightness.is_finite() || brightness < 0.0 {
        return Err(Error::InvalidBrightness.into());
    }

    let output = find_output(resources, connector)?;
    let crtc = find_crtc(resources, output)?;
    let current_gamma = resources.get_crtc_gamma(proxy, crtc)?;

    let state_path = state_file(connector);
    let saved_state = state_path.as_deref().and_then(load_state);
    let base_gamma = resolve_base_gamma(&current_gamma, saved_state.as_ref());
    let final_gamma = base_gamma.apply_brightness(brightness, filter);

    println!(
        "setting software brightness on {} to {} using {} filter",
        connector,
        format_scale(brightness),
        filter
    );

    if dry_run {
        return Ok(());
    }

    resources.set_crtc_gamma(proxy, crtc, final_gamma)?;

    if let Some(path) = state_path.as_deref() {
        if let Err(error) = save_state(
            path,
            &BrightnessState {
                brightness,
                filter,
                gamma: base_gamma,
            },
        ) {
            eprintln!("warning: failed to save brightness state: {}", error);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        matching_state, parse_channel, resolve_base_gamma, BrightnessState, CurrentBrightness,
        CurrentBrightnessState,
    };
    use gnome_randr::display_config::proxied_methods::{BrightnessFilter, Gamma};

    #[test]
    fn brightness_state_roundtrip() {
        let state = BrightnessState {
            brightness: 0.75,
            filter: BrightnessFilter::Filmic,
            gamma: Gamma {
                red: vec![0, 1000, 2000],
                green: vec![0, 900, 1800],
                blue: vec![0, 800, 1600],
            },
        };

        let parsed = BrightnessState::parse(&state.serialize()).unwrap();

        assert!((parsed.brightness - 0.75).abs() < f64::EPSILON);
        assert_eq!(parsed.filter, BrightnessFilter::Filmic);
        assert_eq!(parsed.gamma.red, state.gamma.red);
        assert_eq!(parsed.gamma.green, state.gamma.green);
        assert_eq!(parsed.gamma.blue, state.gamma.blue);
    }

    #[test]
    fn brightness_state_defaults_missing_filter_to_linear() {
        let parsed =
            BrightnessState::parse("brightness=0.5\nred=0,1000\ngreen=0,1000\nblue=0,1000\n")
                .unwrap();

        assert_eq!(parsed.filter, BrightnessFilter::Linear);
    }

    #[test]
    fn parse_channel_supports_empty_channels() {
        assert_eq!(parse_channel(""), Some(Vec::new()));
    }

    #[test]
    fn saved_baseline_is_reused_when_current_gamma_matches_saved_brightness() {
        let base = Gamma {
            red: vec![0, 1000, 2000],
            green: vec![0, 1200, 2400],
            blue: vec![0, 1400, 2800],
        };
        let current = base.apply_brightness(2.0, BrightnessFilter::Filmic);
        let resolved = resolve_base_gamma(
            &current,
            Some(&BrightnessState {
                brightness: 2.0,
                filter: BrightnessFilter::Filmic,
                gamma: base.clone(),
            }),
        );

        assert!(resolved.approx_eq(&base));
    }

    #[test]
    fn matching_state_reports_saved_filter_and_brightness() {
        let state = BrightnessState {
            brightness: 1.5,
            filter: BrightnessFilter::Gamma,
            gamma: Gamma {
                red: vec![0, 1000, 2000],
                green: vec![0, 1000, 2000],
                blue: vec![0, 1000, 2000],
            },
        };

        let current = state.gamma.apply_brightness(state.brightness, state.filter);

        assert_eq!(
            matching_state(&current, Some(&state)),
            Some(CurrentBrightness {
                state: CurrentBrightnessState::Managed,
                brightness: Some(1.5),
                filter: Some(BrightnessFilter::Gamma),
            })
        );
    }
}
