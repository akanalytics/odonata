use crate::config::{Config, Configurable};
use crate::log_debug;
use std::fmt;


#[derive(Copy, Clone, Debug)]
pub struct Quiescence {
    enabled: bool,
    see: bool,
    max_ply: u16,
}





impl Configurable for Quiescence {
    fn settings(&self, c: &mut Config) {
        c.set("quiescence.enabled", "type check default true");
        c.set("quiescence.see", "type check default true");
        c.set("quiescence.max_ply", "type spin default 15 min 0 max 100");
    }
    
    fn configure(&mut self, c: &Config) {
        log_debug!("quiescence.configure with {}", c);
        self.enabled = c.bool("quiescence.enabled").unwrap_or(self.enabled);
        self.see = c.bool("quiescence.see").unwrap_or(self.see);
        self.max_ply = c.int("quiescence.max_ply").unwrap_or(self.max_ply as i64) as u16;
    }
}

impl Default for Quiescence {
    fn default() -> Self {
        Quiescence {
            enabled: true,
            see: true,
            max_ply: 100,
        }
    }
}

impl fmt::Display for Quiescence {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "see enabled      : {}", self.see)?;
        writeln!(f, "max_ply          : {}", self.max_ply)?;
        Ok(())
    }
}



