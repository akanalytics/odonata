use crate::{
    eval::score::Score,
    infra::{component::Component, metric::Metrics},
    search::node::{Event, Node},
    Algo,
};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

impl Algo {
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
            Metrics::incr_node(&n, Event::MateDistSuccess);
            return Some(n.alpha);
        }
        None
    }
}

impl fmt::Display for MateDistance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", toml::to_string_pretty(self).unwrap())?;
        Ok(())
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
