use crate::search::clock::Clock;
use crate::types::MAX_PLY;
use std::fmt;

#[derive(Clone, Debug)]
pub struct SearchStats {
    pub clock: Clock,
    total: Stats,
    plies: Vec<Stats>,
}

impl fmt::Display for SearchStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "clock            : {}", self.clock)?;
        writeln!(f, "depth            : {}", self.depth())?;
        writeln!(f, "selective depth  : {}", self.selective_depth())?;
        writeln!(f, "nodes/sec (k)    : {}", self.total_knps())?;
        writeln!(f, "branching factor : {:.02}", self.branching_factor())?;
        writeln!(f)?;

        write!(f, "{:<7}", "Ply")?;
        Stats::fmt_header(f)?;
        write!(f, "{:<7}", "---")?;
        Stats::fmt_underline(f)?;

        for (i, p) in self.plies().iter().enumerate() {
            write!(f, "{:<7}", i)?;
            p.fmt_data(f)?;
        }
        write!(f, "{:<7}", "---")?;
        Stats::fmt_underline(f)?;
        write!(f, "{:<7}", "tot")?;
        self.total().fmt_data(f)?;
        Ok(())
    }
}

impl Default for SearchStats {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchStats {
    pub fn new() -> Self {
        SearchStats {
            clock: Clock::default(),
            total: Stats::default(),
            plies: std::iter::repeat(Stats::new()).take(MAX_PLY).collect(),
        }
    }

    #[inline]
    pub fn total(&self) -> &Stats {
        &self.total
    }

    pub fn plies(&self) -> &[Stats] {
        &self.plies[0..self.depth() as usize]
    }

    pub fn depth(&self) -> u32 {
        self.selective_depth()
    }

    pub fn selective_depth(&self) -> u32 {
        if let Some(d) = self.plies.iter().rposition(|stats| stats.nodes() != 0) {
            return 1 + d as u32;  // 1 off the end for all "size" types
        }
        0
    }

    #[inline]
    pub fn inc_leaf_nodes(&mut self, ply: u32) {
        self.total.leaf_nodes += 1;
        self.plies[ply as usize].leaf_nodes += 1;
    }

    #[inline]
    pub fn inc_interior_nodes(&mut self, ply: u32) {
        self.total.interior_nodes += 1;
        self.plies[ply as usize].interior_nodes += 1;
    }

    // #[inline]
    // pub fn inc_nodes(&mut self, ply: u32) {
    //     self.total.interior_nodes += 1;
    //     self.plies[ply as usize].interior_nodes +=1;
    // }

    #[inline]
    pub fn inc_cuts(&mut self, ply: u32) {
        self.total.cuts += 1;
        self.plies[ply as usize].cuts += 1;
    }

    #[inline]
    pub fn inc_improvements(&mut self, ply: u32) {
        self.total.improvements += 1;
        self.plies[ply as usize].improvements += 1;
    }

    #[inline]
    pub fn stats_mut(&mut self, ply: u32) -> &mut Stats {
        &mut self.plies[ply as usize]
    }

    #[inline]
    pub fn stats(&self, ply: u32) -> &Stats {
        &self.plies[ply as usize]
    }

    #[inline]
    pub fn total_knps(&self) -> u128 {
        self.total.nodes() as u128 / (1 + self.clock.elapsed().as_millis())
    }

    #[inline]
    pub fn branching_factor(&self) -> f64 {
        self.total().leaf_nodes() as f64 / (self.total().interior_nodes() + 1) as f64
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Stats {
    // nodes
    pub interior_nodes: u64,
    pub leaf_nodes: u64, // FIXME and terminal

    pub improvements: u64,
    pub cuts: u64,
}

impl Stats {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn interior_nodes(&self) -> u64 {
        self.interior_nodes
    }

    #[inline]
    pub fn nodes(&self) -> u64 {
        self.interior_nodes() + self.leaf_nodes() // root
    }

    #[inline]
    pub fn leaf_nodes(&self) -> u64 {
        self.leaf_nodes
    }

    fn fmt_header(f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{node:>11} {interior:>11} {leaf:>11} {cut:>6} {improv:>6}",
            cut = "cuts",
            improv = "improv",
            node = "total nodes",
            interior = "interior",
            leaf = "leaf nodes",
        )?;
        writeln!(f)
    }

    fn fmt_underline(f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{node:>11} {interior:>11} {leaf:>11} {cut:>6} {improv:>6}",
            cut = "----",
            improv = "------",
            node = "-----------",
            interior = "--------",
            leaf = "----------",
        )?;
        writeln!(f)
    }

    fn fmt_data(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{node:>11} {interior:>11} {leaf:>11} {cut:>6} {improv:>6}",
            improv = self.improvements,
            node = self.nodes(),
            interior = self.interior_nodes,
            leaf = self.leaf_nodes(),
            cut = self.cuts
        )?;
        writeln!(f)
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Self::fmt_header(f)?;
        self.fmt_data(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_stats() {
        let ply_stats = Stats::default();
        println!("{}", ply_stats);
        println!("{:?}", ply_stats);
        println!("{:#?}", ply_stats);

        let mut search = SearchStats::default();
        search.inc_leaf_nodes(2);
        search.inc_leaf_nodes(2);
        search.inc_interior_nodes(0);
        println!("{}", search);
    }
}
