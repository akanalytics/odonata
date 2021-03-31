use std::fmt;
use crate::search::clock::Clock;

#[derive(Copy, Clone, Debug, Default)]
pub struct Stats {
    // nodes
    pub interior_nodes: u64,
    pub leaf_nodes: u64, // FIXME and terminal
    pub clock: Clock,

    pub depth: u32,
    pub seldepth: u32,
    pub improvements: u64,
    pub cuts: u64,
}

impl Stats {
    pub fn new() -> Stats {
        Self::default()
    }

    pub fn interior_nodes(&self) -> u64 {
        self.interior_nodes
    }

    pub fn total_nodes(&self) -> u64 {
        self.interior_nodes() + self.leaf_nodes() // root
    }

    pub fn leaf_nodes(&self) -> u64 {
        self.leaf_nodes
    }

    pub fn branching_factor(&self) -> f64 {
        self.leaf_nodes() as f64 / self.interior_nodes() as f64
    }

    pub fn knps(&self) -> u128 {
        self.total_nodes() as u128 / (1 + self.clock.elapsed().as_millis())
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "clock            : {}", self.clock)?;
        writeln!(f, "nodes/sec (k)    : {}", self.knps())?;
        writeln!(f, "cuts             : {}", self.cuts)?;
        writeln!(f, "deldepth         : {}", self.seldepth)?;
        writeln!(f, "depth            : {}", self.depth)?;
        writeln!(f, "improvements     : {}", self.improvements)?;
        writeln!(f, "node count       : {}", self.total_nodes())?;
        writeln!(f, "interior nodes   : {}", self.interior_nodes)?;
        writeln!(f, "leaf nodes       : {}", self.leaf_nodes())?;
        writeln!(f, "branching factor : {:.02}", self.branching_factor())?;
        Ok(())
    }
}
