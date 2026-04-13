use std::{
    convert::TryFrom,
    env, fs,
    path::{Path, PathBuf},
};

use gnome_randr::display_config::{
    proxied_methods::{BrightnessFilter, Gamma, GammaAdjustment},
    resources::{Crtc, Output, Resources},
};

use super::common::format_scale;

pub(super) const FILTER_VALUES: &[&str] = &["linear", "gamma", "filmic"];
pub(super) const GAMMA_VALUES: &[&str] = &[
    "1",
    "1:1:1",
    "0.8:0.8:0.8",
    "1.2:1.2:1.2",
    "1:0.9:0.9",
    "0.9:1:0.9",
    "0.9:0.9:1",
];

pub(super) fn parse_filter(value: &str) -> Result<BrightnessFilter, String> {
    value.parse()
}

pub(super) fn parse_gamma_adjustment(value: &str) -> Result<GammaAdjustment, String> {
    value.parse()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentColorState {
    Managed,
    Identity,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CurrentColor {
    pub state: CurrentColorState,
    pub brightness: Option<f64>,
    pub filter: Option<BrightnessFilter>,
    pub gamma_adjustment: Option<GammaAdjustment>,
}

impl CurrentColor {
    pub fn managed(
        brightness: f64,
        filter: BrightnessFilter,
        gamma_adjustment: GammaAdjustment,
    ) -> CurrentColor {
        CurrentColor {
            state: CurrentColorState::Managed,
            brightness: Some(brightness),
            filter: Some(filter),
            gamma_adjustment: Some(gamma_adjustment),
        }
    }

    pub fn identity() -> CurrentColor {
        CurrentColor {
            state: CurrentColorState::Identity,
            brightness: Some(1.0),
            filter: Some(BrightnessFilter::Linear),
            gamma_adjustment: Some(GammaAdjustment::identity()),
        }
    }

    pub fn unknown() -> CurrentColor {
        CurrentColor {
            state: CurrentColorState::Unknown,
            brightness: None,
            filter: None,
            gamma_adjustment: None,
        }
    }

    pub fn brightness_display(&self) -> String {
        match (self.brightness, self.filter) {
            (Some(brightness), Some(filter)) => {
                format!("{} ({})", format_scale(brightness), filter)
            }
            _ => "unknown".to_string(),
        }
    }

    pub fn gamma_display(&self) -> String {
        match self.gamma_adjustment {
            Some(gamma_adjustment) => gamma_adjustment.to_string(),
            None => "unknown".to_string(),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    NotFound,
    OutputDisabled,
    InvalidBrightness,
    InvalidGamma,
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
                Error::InvalidGamma => {
                    "fatal: gamma must be finite values greater than 0, using R or R:G:B."
                }
                Error::CrtcNotFound => "fatal: unable to find CRTC for output.",
            }
        )
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone)]
struct SavedColorState {
    brightness: f64,
    filter: BrightnessFilter,
    gamma_adjustment: GammaAdjustment,
    base_gamma: Gamma,
}

impl SavedColorState {
    fn parse(contents: &str) -> Option<SavedColorState> {
        let mut brightness = None;
        let mut filter = None;
        let mut gamma_red = None;
        let mut gamma_green = None;
        let mut gamma_blue = None;
        let mut base_red = None;
        let mut base_green = None;
        let mut base_blue = None;
        let mut legacy_red = None;
        let mut legacy_green = None;
        let mut legacy_blue = None;

        for line in contents.lines() {
            let (key, value) = line.split_once('=')?;
            match key {
                "brightness" => brightness = value.parse::<f64>().ok(),
                "filter" => filter = value.parse::<BrightnessFilter>().ok(),
                "gamma-red" => gamma_red = value.parse::<f64>().ok(),
                "gamma-green" => gamma_green = value.parse::<f64>().ok(),
                "gamma-blue" => gamma_blue = value.parse::<f64>().ok(),
                "base-red" => base_red = parse_channel(value),
                "base-green" => base_green = parse_channel(value),
                "base-blue" => base_blue = parse_channel(value),
                "red" => legacy_red = parse_channel(value),
                "green" => legacy_green = parse_channel(value),
                "blue" => legacy_blue = parse_channel(value),
                _ => {}
            }
        }

        Some(SavedColorState {
            brightness: brightness?,
            filter: filter.unwrap_or(BrightnessFilter::Linear),
            gamma_adjustment: GammaAdjustment {
                red: gamma_red.unwrap_or(1.0),
                green: gamma_green.unwrap_or(1.0),
                blue: gamma_blue.unwrap_or(1.0),
            },
            base_gamma: Gamma {
                red: base_red.or(legacy_red)?,
                green: base_green.or(legacy_green)?,
                blue: base_blue.or(legacy_blue)?,
            },
        })
    }

    fn serialize(&self) -> String {
        format!(
            "brightness={}\nfilter={}\ngamma-red={}\ngamma-green={}\ngamma-blue={}\nbase-red={}\nbase-green={}\nbase-blue={}\n",
            self.brightness,
            self.filter,
            self.gamma_adjustment.red,
            self.gamma_adjustment.green,
            self.gamma_adjustment.blue,
            serialize_channel(&self.base_gamma.red),
            serialize_channel(&self.base_gamma.green),
            serialize_channel(&self.base_gamma.blue),
        )
    }

    fn applied_gamma(&self) -> Gamma {
        self.base_gamma
            .apply_software_color(self.brightness, self.filter, self.gamma_adjustment)
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

fn load_state(path: &Path) -> Option<SavedColorState> {
    SavedColorState::parse(&fs::read_to_string(path).ok()?)
}

fn save_state(path: &Path, state: &SavedColorState) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, state.serialize())
}

fn resolve_base_gamma(current_gamma: &Gamma, state: Option<&SavedColorState>) -> Gamma {
    match state {
        Some(state) if current_gamma.approx_eq(&state.applied_gamma()) => state.base_gamma.clone(),
        _ => current_gamma.clone(),
    }
}

fn matching_state(current_gamma: &Gamma, state: Option<&SavedColorState>) -> Option<CurrentColor> {
    match state {
        Some(state) if current_gamma.approx_eq(&state.applied_gamma()) => Some(
            CurrentColor::managed(state.brightness, state.filter, state.gamma_adjustment),
        ),
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

pub(super) fn load_current_color(
    connector: &str,
    resources: &Resources,
    proxy: &dbus::blocking::Proxy<&dbus::blocking::Connection>,
) -> Result<CurrentColor, Box<dyn std::error::Error>> {
    let output = find_output(resources, connector)?;
    let crtc = find_crtc(resources, output)?;
    let current_gamma = resources.get_crtc_gamma(proxy, crtc)?;

    let state_path = state_file(connector);
    let saved_state = state_path.as_deref().and_then(load_state);

    if let Some(current) = matching_state(&current_gamma, saved_state.as_ref()) {
        return Ok(current);
    }

    if current_gamma.is_identity() {
        return Ok(CurrentColor::identity());
    }

    Ok(CurrentColor::unknown())
}

pub(super) fn apply_color(
    connector: &str,
    brightness: Option<f64>,
    filter: BrightnessFilter,
    gamma_adjustment: Option<GammaAdjustment>,
    dry_run: bool,
    resources: &Resources,
    proxy: &dbus::blocking::Proxy<&dbus::blocking::Connection>,
) -> Result<(), Box<dyn std::error::Error>> {
    let brightness = brightness.unwrap_or(1.0);
    let gamma_adjustment = gamma_adjustment.unwrap_or_else(GammaAdjustment::identity);

    if !brightness.is_finite() || brightness < 0.0 {
        return Err(Error::InvalidBrightness.into());
    }
    if !gamma_adjustment.red.is_finite()
        || !gamma_adjustment.green.is_finite()
        || !gamma_adjustment.blue.is_finite()
        || gamma_adjustment.red <= 0.0
        || gamma_adjustment.green <= 0.0
        || gamma_adjustment.blue <= 0.0
    {
        return Err(Error::InvalidGamma.into());
    }

    let output = find_output(resources, connector)?;
    let crtc = find_crtc(resources, output)?;
    let current_gamma = resources.get_crtc_gamma(proxy, crtc)?;

    let state_path = state_file(connector);
    let saved_state = state_path.as_deref().and_then(load_state);
    let base_gamma = resolve_base_gamma(&current_gamma, saved_state.as_ref());
    let final_gamma = base_gamma.apply_software_color(brightness, filter, gamma_adjustment);

    if brightness != 1.0 || filter != BrightnessFilter::Linear {
        println!(
            "setting software brightness on {} to {} using {} filter",
            connector,
            format_scale(brightness),
            filter
        );
    }
    if !gamma_adjustment.is_identity() {
        println!(
            "setting software gamma on {} to {}",
            connector, gamma_adjustment
        );
    }

    if dry_run {
        return Ok(());
    }

    resources.set_crtc_gamma(proxy, crtc, final_gamma)?;

    if let Some(path) = state_path.as_deref() {
        if let Err(error) = save_state(
            path,
            &SavedColorState {
                brightness,
                filter,
                gamma_adjustment,
                base_gamma,
            },
        ) {
            eprintln!("warning: failed to save software color state: {}", error);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        matching_state, parse_channel, resolve_base_gamma, CurrentColor, CurrentColorState,
        SavedColorState,
    };
    use gnome_randr::display_config::proxied_methods::{BrightnessFilter, Gamma, GammaAdjustment};

    #[test]
    fn color_state_roundtrip() {
        let state = SavedColorState {
            brightness: 0.75,
            filter: BrightnessFilter::Filmic,
            gamma_adjustment: GammaAdjustment {
                red: 1.1,
                green: 1.0,
                blue: 0.9,
            },
            base_gamma: Gamma {
                red: vec![0, 1000, 2000],
                green: vec![0, 900, 1800],
                blue: vec![0, 800, 1600],
            },
        };

        let parsed = SavedColorState::parse(&state.serialize()).unwrap();

        assert!((parsed.brightness - 0.75).abs() < f64::EPSILON);
        assert_eq!(parsed.filter, BrightnessFilter::Filmic);
        assert_eq!(parsed.gamma_adjustment, state.gamma_adjustment);
        assert_eq!(parsed.base_gamma.red, state.base_gamma.red);
        assert_eq!(parsed.base_gamma.green, state.base_gamma.green);
        assert_eq!(parsed.base_gamma.blue, state.base_gamma.blue);
    }

    #[test]
    fn old_brightness_state_defaults_gamma_to_identity() {
        let parsed = SavedColorState::parse(
            "brightness=0.5\nfilter=linear\nred=0,1000\ngreen=0,1000\nblue=0,1000\n",
        )
        .unwrap();

        assert_eq!(parsed.gamma_adjustment, GammaAdjustment::identity());
    }

    #[test]
    fn parse_channel_supports_empty_channels() {
        assert_eq!(parse_channel(""), Some(Vec::new()));
    }

    #[test]
    fn saved_baseline_is_reused_when_current_gamma_matches_saved_color_state() {
        let base = Gamma {
            red: vec![0, 1000, 2000],
            green: vec![0, 1200, 2400],
            blue: vec![0, 1400, 2800],
        };
        let state = SavedColorState {
            brightness: 2.0,
            filter: BrightnessFilter::Filmic,
            gamma_adjustment: GammaAdjustment {
                red: 1.1,
                green: 1.0,
                blue: 0.9,
            },
            base_gamma: base.clone(),
        };
        let current = state.applied_gamma();
        let resolved = resolve_base_gamma(&current, Some(&state));

        assert!(resolved.approx_eq(&base));
    }

    #[test]
    fn matching_state_reports_saved_gamma_and_brightness() {
        let state = SavedColorState {
            brightness: 1.5,
            filter: BrightnessFilter::Gamma,
            gamma_adjustment: GammaAdjustment {
                red: 1.2,
                green: 1.1,
                blue: 1.0,
            },
            base_gamma: Gamma {
                red: vec![0, 1000, 2000],
                green: vec![0, 1000, 2000],
                blue: vec![0, 1000, 2000],
            },
        };

        let current = state.applied_gamma();

        assert_eq!(
            matching_state(&current, Some(&state)),
            Some(CurrentColor {
                state: CurrentColorState::Managed,
                brightness: Some(1.5),
                filter: Some(BrightnessFilter::Gamma),
                gamma_adjustment: Some(GammaAdjustment {
                    red: 1.2,
                    green: 1.1,
                    blue: 1.0,
                }),
            })
        );
    }
}
