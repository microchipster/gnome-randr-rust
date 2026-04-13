use std::collections::HashMap;

use gnome_randr::{
    display_config::{
        logical_monitor::{LogicalMonitor, Transform},
        physical_monitor::{Mode, PhysicalMonitor},
        proxied_methods::{ApplyMonitorProperty, ColorMode},
        ApplyConfig, ApplyMonitor, LayoutMode,
    },
    DisplayConfig,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelativePlacement {
    LeftOf,
    RightOf,
    Above,
    Below,
}

impl RelativePlacement {
    pub fn describe(self) -> &'static str {
        match self {
            RelativePlacement::LeftOf => "left of",
            RelativePlacement::RightOf => "right of",
            RelativePlacement::Above => "above",
            RelativePlacement::Below => "below",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Geometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Geometry {
    fn right(self) -> i32 {
        self.x + self.width
    }

    fn bottom(self) -> i32 {
        self.y + self.height
    }
}

#[derive(Debug)]
pub enum Error {
    ConnectorNotFound(String),
    PhysicalMonitorNotFound(String),
    CurrentModeNotFound(String),
    ModeNotFound {
        connector: String,
        mode_id: String,
    },
    SameLogicalMonitor {
        connector: String,
        reference: String,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ConnectorNotFound(connector) => {
                write!(f, "fatal: planner could not find connector {}.", connector)
            }
            Error::PhysicalMonitorNotFound(connector) => write!(
                f,
                "fatal: planner could not find physical monitor {} in the current config.",
                connector
            ),
            Error::CurrentModeNotFound(connector) => write!(
                f,
                "fatal: planner could not determine the current mode for {}.",
                connector
            ),
            Error::ModeNotFound { connector, mode_id } => write!(
                f,
                "fatal: planner could not find mode {} for connector {}.",
                mode_id, connector
            ),
            Error::SameLogicalMonitor {
                connector,
                reference,
            } => write!(
                f,
                "fatal: cannot place {} relative to {} because they belong to the same logical monitor.",
                connector, reference
            ),
        }
    }
}

impl std::error::Error for Error {}

pub struct MonitorPlanner<'a> {
    display: &'a DisplayConfig,
    configs: Vec<ApplyConfig<'a>>,
    connector_to_config: HashMap<&'a str, usize>,
}

impl<'a> MonitorPlanner<'a> {
    pub fn new(config: &'a DisplayConfig) -> Result<Self, Error> {
        let mut configs = Vec::new();
        let mut connector_to_config = HashMap::new();

        for logical_monitor in &config.logical_monitors {
            let apply_config = Self::apply_config_for_logical_monitor(config, logical_monitor)?;
            let index = configs.len();

            for monitor in &apply_config.monitors {
                connector_to_config.insert(monitor.connector, index);
            }

            configs.push(apply_config);
        }

        Ok(Self {
            display: config,
            configs,
            connector_to_config,
        })
    }

    fn apply_config_for_logical_monitor(
        config: &'a DisplayConfig,
        logical_monitor: &LogicalMonitor,
    ) -> Result<ApplyConfig<'a>, Error> {
        let mut monitors = Vec::new();

        for associated in &logical_monitor.monitors {
            let physical_monitor = config
                .monitors
                .iter()
                .find(|monitor| monitor.connector == associated.connector)
                .ok_or_else(|| Error::PhysicalMonitorNotFound(associated.connector.clone()))?;
            let mode = physical_monitor
                .modes
                .iter()
                .find(|mode| mode.known_properties.is_current)
                .or_else(|| {
                    physical_monitor
                        .modes
                        .iter()
                        .find(|mode| mode.known_properties.is_preferred)
                })
                .ok_or_else(|| Error::CurrentModeNotFound(physical_monitor.connector.clone()))?;

            monitors.push(ApplyMonitor {
                connector: &physical_monitor.connector,
                mode_id: &mode.id,
                properties: vec![],
            });
        }

        Ok(ApplyConfig {
            x_pos: logical_monitor.x,
            y_pos: logical_monitor.y,
            scale: logical_monitor.scale,
            transform: logical_monitor.transform.bits(),
            primary: logical_monitor.primary,
            monitors,
        })
    }

    fn target_index(&self, connector: &str) -> Result<usize, Error> {
        self.connector_to_config
            .get(connector)
            .copied()
            .ok_or_else(|| Error::ConnectorNotFound(connector.to_string()))
    }

    fn target_config_mut(&mut self, connector: &str) -> Result<&mut ApplyConfig<'a>, Error> {
        let index = self.target_index(connector)?;
        Ok(&mut self.configs[index])
    }

    fn physical_monitor(&self, connector: &str) -> Result<&'a PhysicalMonitor, Error> {
        self.display
            .physical_monitor(connector)
            .ok_or_else(|| Error::PhysicalMonitorNotFound(connector.to_string()))
    }

    fn mode_for_monitor(&self, connector: &str, mode_id: &str) -> Result<&'a Mode, Error> {
        self.physical_monitor(connector)?
            .modes
            .iter()
            .find(|mode| mode.id == mode_id)
            .ok_or_else(|| Error::ModeNotFound {
                connector: connector.to_string(),
                mode_id: mode_id.to_string(),
            })
    }

    fn config_geometry(&self, index: usize) -> Result<Geometry, Error> {
        let config = &self.configs[index];
        let mut width = 0;
        let mut height = 0;

        for monitor in &config.monitors {
            let mode = self.mode_for_monitor(monitor.connector, monitor.mode_id)?;
            width = width.max(mode.width);
            height = height.max(mode.height);
        }

        let (mut width, mut height) = match self.display.known_properties.layout_mode {
            LayoutMode::Logical if config.scale > 0.0 => (
                ((width as f64) / config.scale).round() as i32,
                ((height as f64) / config.scale).round() as i32,
            ),
            _ => (width, height),
        };

        let rotation = config.transform & Transform::R270.bits();
        if rotation == Transform::R90.bits() || rotation == Transform::R270.bits() {
            std::mem::swap(&mut width, &mut height);
        }

        Ok(Geometry {
            x: config.x_pos,
            y: config.y_pos,
            width,
            height,
        })
    }

    pub fn geometry(&self, connector: &str) -> Result<Geometry, Error> {
        self.config_geometry(self.target_index(connector)?)
    }

    pub fn position(&self, connector: &str) -> Result<(i32, i32), Error> {
        let geometry = self.geometry(connector)?;
        Ok((geometry.x, geometry.y))
    }

    pub fn set_mode(&mut self, connector: &str, mode_id: &'a str) -> Result<(), Error> {
        let config = self.target_config_mut(connector)?;
        let monitor = config
            .monitors
            .iter_mut()
            .find(|monitor| monitor.connector == connector)
            .ok_or_else(|| Error::ConnectorNotFound(connector.to_string()))?;
        monitor.mode_id = mode_id;
        Ok(())
    }

    pub fn set_scale(&mut self, connector: &str, scale: f64) -> Result<(), Error> {
        self.target_config_mut(connector)?.scale = scale;
        Ok(())
    }

    pub fn set_color_mode(&mut self, connector: &str, color_mode: ColorMode) -> Result<(), Error> {
        let config = self.target_config_mut(connector)?;
        let monitor = config
            .monitors
            .iter_mut()
            .find(|monitor| monitor.connector == connector)
            .ok_or_else(|| Error::ConnectorNotFound(connector.to_string()))?;

        monitor
            .properties
            .retain(|property| !matches!(property, ApplyMonitorProperty::ColorMode(_)));
        monitor
            .properties
            .push(ApplyMonitorProperty::ColorMode(color_mode));
        Ok(())
    }

    pub fn set_transform(&mut self, connector: &str, transform: u32) -> Result<(), Error> {
        self.target_config_mut(connector)?.transform = transform;
        Ok(())
    }

    pub fn set_primary(&mut self, connector: &str) -> Result<(), Error> {
        let index = self.target_index(connector)?;
        for config in &mut self.configs {
            config.primary = false;
        }
        self.configs[index].primary = true;
        Ok(())
    }

    pub fn clear_primary(&mut self, connector: &str) -> Result<(), Error> {
        self.target_config_mut(connector)?.primary = false;
        Ok(())
    }

    pub fn set_position(&mut self, connector: &str, x: i32, y: i32) -> Result<(), Error> {
        let config = self.target_config_mut(connector)?;
        config.x_pos = x;
        config.y_pos = y;
        Ok(())
    }

    pub fn place_relative(
        &mut self,
        connector: &str,
        reference: &str,
        placement: RelativePlacement,
    ) -> Result<(i32, i32), Error> {
        let target_index = self.target_index(connector)?;
        let reference_index = self.target_index(reference)?;
        if target_index == reference_index {
            return Err(Error::SameLogicalMonitor {
                connector: connector.to_string(),
                reference: reference.to_string(),
            });
        }

        let target = self.config_geometry(target_index)?;
        let reference_geometry = self.config_geometry(reference_index)?;
        let (x, y) = match placement {
            RelativePlacement::LeftOf => {
                (reference_geometry.x - target.width, reference_geometry.y)
            }
            RelativePlacement::RightOf => (reference_geometry.right(), reference_geometry.y),
            RelativePlacement::Above => {
                (reference_geometry.x, reference_geometry.y - target.height)
            }
            RelativePlacement::Below => (reference_geometry.x, reference_geometry.bottom()),
        };

        let config = &mut self.configs[target_index];
        config.x_pos = x;
        config.y_pos = y;
        Ok((x, y))
    }

    pub fn reflow_after_geometry_change(
        &mut self,
        connector: &str,
        old_geometry: Geometry,
    ) -> Result<(), Error> {
        let target_index = self.target_index(connector)?;
        let new_geometry = self.config_geometry(target_index)?;
        let delta_width = new_geometry.width - old_geometry.width;
        let delta_height = new_geometry.height - old_geometry.height;

        if delta_width == 0 && delta_height == 0 {
            return Ok(());
        }

        let old_right = old_geometry.right();
        let old_bottom = old_geometry.bottom();

        for (index, config) in self.configs.iter_mut().enumerate() {
            if index == target_index {
                continue;
            }

            if delta_width != 0 && config.x_pos >= old_right {
                config.x_pos += delta_width;
            }

            if delta_height != 0 && config.y_pos >= old_bottom {
                config.y_pos += delta_height;
            }
        }

        Ok(())
    }

    pub fn remove_output(&mut self, connector: &str) -> Result<(), Error> {
        self.detach_connector(connector)?;
        Ok(())
    }

    fn detach_connector(&mut self, connector: &str) -> Result<(), Error> {
        let index = self.target_index(connector)?;
        self.configs[index]
            .monitors
            .retain(|monitor| monitor.connector != connector);

        self.rebuild_index();

        if self
            .configs
            .get(index)
            .map(|config| config.monitors.is_empty())
            .unwrap_or(false)
        {
            self.configs.remove(index);
            self.rebuild_index();
        }

        Ok(())
    }

    pub fn clone_with(
        &mut self,
        connector: &str,
        reference: &str,
        mode_id: &'a str,
    ) -> Result<(), Error> {
        if connector != reference {
            if let Some(target_index) = self.connector_to_config.get(connector).copied() {
                if target_index != self.target_index(reference)? {
                    self.detach_connector(connector)?;
                }
            }
        }

        let reference_index = self.target_index(reference)?;
        let connector_ref = &self.physical_monitor(connector)?.connector;
        let config = &mut self.configs[reference_index];

        if let Some(monitor) = config
            .monitors
            .iter_mut()
            .find(|monitor| monitor.connector == connector_ref)
        {
            monitor.mode_id = mode_id;
        } else {
            config.monitors.push(ApplyMonitor {
                connector: connector_ref,
                mode_id,
                properties: vec![],
            });
        }

        self.rebuild_index();
        Ok(())
    }

    fn rebuild_index(&mut self) {
        self.connector_to_config.clear();

        for (index, config) in self.configs.iter().enumerate() {
            for monitor in &config.monitors {
                self.connector_to_config.insert(monitor.connector, index);
            }
        }
    }

    pub fn into_configs(self) -> Vec<ApplyConfig<'a>> {
        self.configs
    }
}

#[cfg(test)]
mod tests {
    use super::{Geometry, MonitorPlanner, RelativePlacement};
    use gnome_randr::display_config::{
        logical_monitor::{LogicalMonitor, Monitor, Transform},
        physical_monitor::{KnownModeProperties, Mode, PhysicalMonitor},
        DisplayConfig, KnownProperties, LayoutMode,
    };

    fn mode(id: &str, width: i32, height: i32, is_current: bool, is_preferred: bool) -> Mode {
        Mode {
            id: id.to_string(),
            width,
            height,
            refresh_rate: 60.0,
            preferred_scale: 1.0,
            supported_scales: vec![1.0, 2.0],
            known_properties: KnownModeProperties {
                is_current,
                is_preferred,
            },
            properties: Default::default(),
        }
    }

    fn physical_monitor(connector: &str, modes: Vec<Mode>) -> PhysicalMonitor {
        PhysicalMonitor {
            connector: connector.to_string(),
            vendor: "Vendor".to_string(),
            product: "Product".to_string(),
            serial: format!("serial-{}", connector),
            modes,
            properties: Default::default(),
        }
    }

    fn associated_monitor(connector: &str) -> Monitor {
        Monitor {
            connector: connector.to_string(),
            vendor: "Vendor".to_string(),
            product: "Product".to_string(),
            serial: format!("serial-{}", connector),
        }
    }

    fn sample_config() -> DisplayConfig {
        DisplayConfig {
            serial: 1,
            monitors: vec![
                physical_monitor(
                    "eDP-1",
                    vec![
                        mode("1920x1080@60", 1920, 1080, true, true),
                        mode("1080x1920@60", 1080, 1920, false, false),
                    ],
                ),
                physical_monitor("HDMI-1", vec![mode("2560x1440@60", 2560, 1440, true, true)]),
                physical_monitor(
                    "DP-1",
                    vec![
                        mode("2560x1440@75", 2560, 1440, true, true),
                        mode("2560x1440@60", 2560, 1440, false, false),
                    ],
                ),
                physical_monitor(
                    "USB-C-1",
                    vec![mode("2560x1440@60", 2560, 1440, false, true)],
                ),
            ],
            logical_monitors: vec![
                LogicalMonitor {
                    x: 0,
                    y: 0,
                    scale: 1.0,
                    transform: Transform::NORMAL,
                    primary: true,
                    monitors: vec![associated_monitor("eDP-1")],
                    properties: Default::default(),
                },
                LogicalMonitor {
                    x: 1920,
                    y: 0,
                    scale: 1.0,
                    transform: Transform::NORMAL,
                    primary: false,
                    monitors: vec![associated_monitor("HDMI-1"), associated_monitor("DP-1")],
                    properties: Default::default(),
                },
            ],
            known_properties: KnownProperties {
                supports_mirroring: true,
                layout_mode: LayoutMode::Logical,
                supports_changing_layout_mode: false,
                global_scale_required: false,
            },
            properties: Default::default(),
        }
    }

    #[test]
    fn planner_starts_from_full_current_state() {
        let config = sample_config();
        let planner = MonitorPlanner::new(&config).unwrap();
        let configs = planner.into_configs();

        assert_eq!(configs.len(), 2);
        assert_eq!(configs[0].monitors.len(), 1);
        assert_eq!(configs[0].monitors[0].connector, "eDP-1");
        assert_eq!(configs[0].monitors[0].mode_id, "1920x1080@60");
        assert_eq!(configs[1].monitors.len(), 2);
        assert_eq!(configs[1].monitors[0].connector, "HDMI-1");
        assert_eq!(configs[1].monitors[1].connector, "DP-1");
    }

    #[test]
    fn planner_can_compose_primary_position_transform_and_mode_changes() {
        let config = sample_config();
        let mut planner = MonitorPlanner::new(&config).unwrap();
        planner.set_primary("HDMI-1").unwrap();
        planner.set_position("HDMI-1", 3200, 180).unwrap();
        planner
            .set_transform("HDMI-1", Transform::R90.bits())
            .unwrap();
        planner.set_mode("DP-1", "2560x1440@60").unwrap();
        planner.set_scale("HDMI-1", 1.25).unwrap();

        let configs = planner.into_configs();
        assert!(!configs[0].primary);
        assert!(configs[1].primary);
        assert_eq!(configs[1].x_pos, 3200);
        assert_eq!(configs[1].y_pos, 180);
        assert_eq!(configs[1].transform, Transform::R90.bits());
        assert_eq!(configs[1].scale, 1.25);
        assert_eq!(configs[1].monitors[1].mode_id, "2560x1440@60");
    }

    #[test]
    fn planner_can_remove_outputs_and_drop_empty_logical_configs() {
        let config = sample_config();
        let mut planner = MonitorPlanner::new(&config).unwrap();
        planner.remove_output("DP-1").unwrap();
        planner.remove_output("HDMI-1").unwrap();

        let configs = planner.into_configs();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].monitors.len(), 1);
        assert_eq!(configs[0].monitors[0].connector, "eDP-1");
    }

    #[test]
    fn relative_placement_uses_final_rotated_geometry() {
        let config = sample_config();
        let mut planner = MonitorPlanner::new(&config).unwrap();
        planner
            .set_transform("eDP-1", Transform::R90.bits())
            .unwrap();

        let position = planner
            .place_relative("eDP-1", "HDMI-1", RelativePlacement::LeftOf)
            .unwrap();
        assert_eq!(position, (840, 0));
        assert_eq!(planner.position("eDP-1").unwrap(), (840, 0));
    }

    #[test]
    fn reflow_moves_right_neighbors_after_rotation_changes_width() {
        let config = sample_config();
        let mut planner = MonitorPlanner::new(&config).unwrap();
        let old_geometry = planner.geometry("eDP-1").unwrap();

        planner
            .set_transform("eDP-1", Transform::R90.bits())
            .unwrap();
        planner
            .reflow_after_geometry_change("eDP-1", old_geometry)
            .unwrap();

        assert_eq!(planner.position("HDMI-1").unwrap(), (1080, 0));
        assert_eq!(planner.geometry("eDP-1").unwrap().width, 1080);
    }

    #[test]
    fn relative_placement_rejects_same_logical_monitor_reference() {
        let config = sample_config();
        let mut planner = MonitorPlanner::new(&config).unwrap();
        let error = planner
            .place_relative("HDMI-1", "DP-1", RelativePlacement::RightOf)
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("belong to the same logical monitor"));
    }

    #[test]
    fn geometry_reflects_layout_mode_scaling() {
        let mut config = sample_config();
        config.logical_monitors[0].scale = 2.0;
        let planner = MonitorPlanner::new(&config).unwrap();

        assert_eq!(
            planner.geometry("eDP-1").unwrap(),
            Geometry {
                x: 0,
                y: 0,
                width: 960,
                height: 540,
            }
        );
    }

    #[test]
    fn clone_with_moves_connector_into_reference_group() {
        let config = sample_config();
        let mut planner = MonitorPlanner::new(&config).unwrap();

        planner
            .clone_with("eDP-1", "HDMI-1", "1920x1080@60")
            .unwrap();

        let configs = planner.into_configs();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].monitors.len(), 3);
        assert!(configs[0]
            .monitors
            .iter()
            .any(|monitor| monitor.connector == "eDP-1"));
        assert!(configs[0]
            .monitors
            .iter()
            .any(|monitor| monitor.connector == "HDMI-1"));
        assert!(configs[0]
            .monitors
            .iter()
            .any(|monitor| monitor.connector == "DP-1"));
    }

    #[test]
    fn clone_with_can_add_disabled_output_to_reference_group() {
        let config = sample_config();
        let mut planner = MonitorPlanner::new(&config).unwrap();

        planner
            .clone_with("USB-C-1", "HDMI-1", "2560x1440@60")
            .unwrap();

        let configs = planner.into_configs();
        assert_eq!(configs.len(), 2);
        assert_eq!(configs[1].monitors.len(), 3);
        assert!(configs[1]
            .monitors
            .iter()
            .any(|monitor| monitor.connector == "USB-C-1"));
    }
}
