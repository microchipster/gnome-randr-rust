use gnome_randr::display_config::{physical_monitor::PhysicalMonitor, ApplyConfig};

use super::Action;

pub struct PrimaryAction {
    pub primary: bool,
}

impl<'a> Action<'a> for PrimaryAction {
    fn apply(&self, config: &mut ApplyConfig<'a>, _: &PhysicalMonitor) {
        config.primary = self.primary;
    }
}

impl std::fmt::Display for PrimaryAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.primary {
            write!(f, "setting monitor as primary")
        } else {
            write!(f, "clearing primary status from this monitor")
        }
    }
}
