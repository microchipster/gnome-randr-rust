use dbus::{
    arg::PropMap,
    blocking::{Connection, Proxy},
};

use super::{
    logical_monitor::LogicalMonitor,
    physical_monitor::PhysicalMonitor,
    resources::{Crtc, Resources},
    DisplayConfig,
};

type Result<T> = std::prelude::rust_2021::Result<T, dbus::Error>;

#[derive(Debug, Clone, Copy)]
pub struct ApplyMonitor<'a> {
    pub connector: &'a str,
    pub mode_id: &'a str,
}

impl ApplyMonitor<'_> {
    pub fn serialize(&self) -> (&str, &str, PropMap) {
        (self.connector, self.mode_id, PropMap::new())
    }
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
    pub fn apply_monitors_config(
        &self,
        proxy: &Proxy<&Connection>,
        configs: Vec<ApplyConfig>,
        persistent: bool,
    ) -> Result<()> {
        use super::raw::OrgGnomeMutterDisplayConfig;

        let result = proxy.apply_monitors_config(
            self.serial,
            if persistent { 2 } else { 1 },
            configs.iter().map(|config| config.serialize()).collect(),
            PropMap::new(),
        );

        if let Err(err) = &result {
            println!("{:?}", err);
        }
        result
    }

    pub fn get_current_state(proxy: &Proxy<&Connection>) -> Result<DisplayConfig> {
        use super::raw::OrgGnomeMutterDisplayConfig;

        let raw_output = proxy.get_current_state()?;
        Ok(DisplayConfig::from(raw_output))
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
    use super::{BrightnessFilter, Gamma};

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
}
