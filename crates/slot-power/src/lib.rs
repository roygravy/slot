macro_rules! eprintln {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        let _ = writeln!(std::io::stderr(), $($arg)*);
    }};
}

mod device;
mod motor;
mod power;
mod sim;

pub use device::{
    card_kernel_lines, has_bit, motor_change, read_only, read_only_in, record_first_frame,
    rumble_node, trace_first_frame, uptime_seconds, DevicePlatform,
};
pub use motor::Motor;
pub use power::Power;
pub use sim::SimPlatform;

use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Charge {
    Unknown,
    Discharging,
    Charging,
    Full,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Battery {
    pub percent: u8,
    pub charge: Charge,
}

/// What ended a deep sleep.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Woke {
    Button,
    LowBattery,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum LedState {
    Off,
    Running,
    Low,
    Charging,
    Charged,
}

pub trait Platform: Send {
    fn set_backlight(&mut self, step: u8);
    fn battery(&self) -> Option<Battery>;
    fn charge(&self) -> Charge;
    fn set_led(&mut self, state: LedState);
    fn poweroff(&mut self) -> !;
    fn restart(&mut self) -> !;
    fn root(&self) -> &Path;
    fn now(&self) -> i64;
    fn set_clock(&mut self, secs: i64);
    fn set_rumble(&mut self, strength: u16);

    fn relink_adb(&mut self) -> bool {
        false
    }

    fn headphones(&self) -> bool {
        false
    }

    fn card_read_only(&self) -> bool {
        false
    }

    fn card_log(&self) -> Option<PathBuf> {
        None
    }

    fn card_kernel_lines(&self) -> Vec<String> {
        Vec::new()
    }

    /// Suspend to RAM until POWER wakes the console, or until the battery is down to
    /// `low` percent; None when this console cannot sleep that deep.
    fn deep_sleep(&mut self, _low: u8) -> Option<Woke> {
        None
    }
}

pub trait LidPolicy {
    fn on_close(&mut self);
    fn on_open(&mut self);
    fn timeout(&self) -> Duration;
}
