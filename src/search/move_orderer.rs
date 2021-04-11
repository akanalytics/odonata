use crate::search::algo::{Node, Algo};
use crate::movelist::MoveList;
use crate::config::{Config, Configurable};
use crate::log_debug;
use std::fmt;



#[derive(Copy, Clone, Debug)]
pub struct MoveOrderer {
    pub enabled: bool,
}


impl Configurable for MoveOrderer {
    fn settings(&self, c: &mut Config) {
        c.set("move_orderer.enabled", "type check default true");
    }
    
    fn configure(&mut self, c: &Config) {
        log_debug!("move_orderer.configure with {}", c);
        self.enabled = c.bool("move_orderer.enabled").unwrap_or(self.enabled);
    }
}

impl Default for MoveOrderer {
    fn default() -> Self {
        MoveOrderer {
            enabled: true,
        }
    }
}

impl fmt::Display for MoveOrderer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        Ok(())
    }
}



impl Algo {
    pub fn order_moves(&self, node: &Node, movelist: &mut MoveList) {
        if self.move_orderer.enabled && node.is_root() {
            if let Some(current_best) = self.current_best {
                if let Some(i) = movelist.iter().position(|mv| mv == &current_best) {
                    // println!("Swapped move {} with position {} on depth {}!", current_best, i, self.max_depth);
                    movelist.swap(0, i);
                }
            }
        }
    }
}   
