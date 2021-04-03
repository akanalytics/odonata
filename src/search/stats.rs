use std::fmt;
use crate::search::clock::Clock;
use crate::types::MAX_PLY;



#[derive(Clone, Debug)]
pub struct SearchStats {
    pub clock: Clock,
    total: Stats,
    plies: Vec<Stats>,
}


impl fmt::Display for SearchStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "clock            : {}", self.clock)?;
        writeln!(f, "nodes/sec (k)    : {}", self.total_knps())?;
        writeln!(f, "branching factor : {:.02}", self.branching_factor())?;

        writeln!(f, "Totals...")?;
        write!(f, "{}", self.total())?;
        
        writeln!(f, "Plies...")?;
        for p in self.plies().iter() {
            write!(f, "{}", p)?;
        }
        Ok(())
    }
}


impl Default for SearchStats {
    fn default() -> Self { Self::new() }
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
        self.plies.iter().position( |stats| stats.nodes() == 0 ).unwrap_or(MAX_PLY) as u32
    } 

    #[inline]
    pub fn inc_leaf_nodes(&mut self, ply: u32) {
        self.total.leaf_nodes += 1;
        self.plies[ply as usize].leaf_nodes +=1;
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
        self.plies[ply as usize].cuts +=1;
    }

    #[inline]
    pub fn inc_improvements(&mut self, ply: u32) {
        self.total.improvements += 1;
        self.plies[ply as usize].improvements +=1;
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
        self.total().leaf_nodes() as f64 / (self.total().interior_nodes()+1) as f64
    }

}




#[derive(Copy, Clone, Debug, Default)]
pub struct Stats {
    // nodes
    pub interior_nodes: u64,
    pub leaf_nodes: u64, // FIXME and terminal

    pub depth: u32,
    pub seldepth: u32,
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


}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:>6}",  "seldp")?; 
        write!(f, "{:>6}",  "depth")?; 
        write!(f, "{:>6}",  "cuts ")?; 
        write!(f, "{:>6}",  "imps ")?; 
        write!(f, "{:>11}", "node count")?; 
        write!(f, "{:>11}", "int nodes ")?; 
        write!(f, "{:>11}", "leaf nodes")?; 

        writeln!(f)?;



        write!(f, "{:>6}", self.seldepth)?;
        write!(f, "{:>6}", self.depth)?;
        write!(f, "{:>6}", self.cuts)?;
        write!(f, "{:>6}", self.improvements)?;
        write!(f, "{:>11}", self.nodes())?;
        write!(f, "{:>11}", self.interior_nodes)?;
        write!(f, "{:>11}", self.leaf_nodes())?;
        writeln!(f)?;
        Ok(())
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

        let stats = Stats::default();
        println!("{}", stats);
        println!("{:?}", stats);
        println!("{:#?}", stats);


    }
}