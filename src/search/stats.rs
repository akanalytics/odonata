use std::fmt;
use std::time;


#[derive(Copy, Clone, Debug, Default)]
pub struct Stats {
    // nodes
    pub interior_nodes: u64,
    pub leaf_nodes: u64, // FIXME and terminal
    pub elapsed: time::Duration,

    // cuts
    pub alpha_cuts: u64,
    pub beta_cuts: u64,

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
        self.total_nodes() as u128 / (1 + self.elapsed.as_millis())
    }
}



impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "elapsed (ms)     :{}", self.elapsed.as_millis())?;
        writeln!(f, "nodes/sec (k)    :{}", self.knps())?;
        writeln!(f, "node count       :{}", self.total_nodes())?;
        writeln!(f, "interior nodes   :{}", self.interior_nodes)?;
        writeln!(f, "leaf nodes       :{}", self.leaf_nodes())?;
        writeln!(f, "branching factor :{:.02}", self.branching_factor())?;
        Ok(())
    }
}
