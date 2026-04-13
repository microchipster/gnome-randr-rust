use std::collections::HashMap;

use gnome_randr::{
    display_config::{logical_monitor::LogicalMonitor, ApplyConfig, ApplyMonitor},
    DisplayConfig,
};

#[derive(Debug)]
pub enum Error {
    ConnectorNotFound(String),
    PhysicalMonitorNotFound(String),
    CurrentModeNotFound(String),
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
        }
    }
}

impl std::error::Error for Error {}

pub struct MonitorPlanner<'a> {
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

    #[allow(dead_code)]
    pub fn set_position(&mut self, connector: &str, x: i32, y: i32) -> Result<(), Error> {
        let config = self.target_config_mut(connector)?;
        config.x_pos = x;
        config.y_pos = y;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn remove_output(&mut self, connector: &str) -> Result<(), Error> {
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
    use super::MonitorPlanner;
    use gnome_randr::display_config::{
        logical_monitor::{LogicalMonitor, Monitor, Transform},
        physical_monitor::{KnownModeProperties, Mode, PhysicalMonitor},
        DisplayConfig, KnownProperties, LayoutMode,
    };

    fn mode(id: &str, is_current: bool, is_preferred: bool) -> Mode {
        Mode {
            id: id.to_string(),
            width: 1920,
            height: 1080,
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

    fn physical_monitor(connector: &str, current_mode: &str) -> PhysicalMonitor {
        PhysicalMonitor {
            connector: connector.to_string(),
            vendor: "Vendor".to_string(),
            product: "Product".to_string(),
            serial: format!("serial-{}", connector),
            modes: vec![mode(current_mode, true, true)],
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
                physical_monitor("eDP-1", "1920x1080@60"),
                physical_monitor("HDMI-1", "2560x1440@60"),
                physical_monitor("DP-1", "2560x1440@75"),
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
}
