use std::fmt;

use odonata_base::domain::node::Node;
use odonata_base::infra::component::Component;
use odonata_base::infra::metric::{Event, Metrics};
use odonata_base::prelude::*;

use super::algo::Search;

#[derive(Clone, Debug)]
pub struct MateDistance {
    pub enabled:     bool,
    pub raise_alpha: bool,
    pub reduce_beta: bool,
}

impl Component for MateDistance {
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {}
}

impl Default for MateDistance {
    fn default() -> Self {
        MateDistance {
            enabled:     true,
            raise_alpha: true,
            reduce_beta: true,
        }
    }
}

impl Configurable for MateDistance {
    fn set(&mut self, p: Param) -> Result<bool> {
        self.enabled.set(p.get("enabled"))?;
        self.raise_alpha.set(p.get("raise_alpha"))?;
        self.reduce_beta.set(p.get("reduce_beta"))?;
        Ok(p.is_modified())
    }
}

impl fmt::Display for MateDistance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{self:#?}")
    }
}

impl Search {
    #[inline]
    pub fn mate_distance(&mut self, n: &mut Node) -> Option<Score> {
        if !self.mate_dist.enabled {
            return None;
        }

        Metrics::incr_node(n, Event::MateDistConsider);
        if self.mate_dist.raise_alpha {
            n.alpha = std::cmp::max(n.alpha, Score::we_lose_in(n.ply));
        }
        if self.mate_dist.reduce_beta {
            n.beta = std::cmp::min(n.beta, Score::we_win_in(n.ply));
        }

        if n.alpha >= n.beta {
            Metrics::incr_node(n, Event::MateDistSuccess);
            return Some(n.alpha);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mate_dist() {
        let md = MateDistance::default();
        println!("{:#?}", md);
        println!("{}", md);
    }
}
