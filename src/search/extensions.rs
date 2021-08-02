use crate::board::Board;
use crate::search::node::Node;
use crate::mv::Move;
use crate::search::searchstats::SearchStats;
use crate::config::{Config, Component};
use crate::{debug, logger::LogInit};
use crate::types::Ply;
use std::fmt;






#[derive(Clone, Debug)]
pub struct Extensions {
    pub check_enabled: bool,
}

impl Component for Extensions {
    fn settings(&self, c: &mut Config) {
        c.set("ext.check.enabled", &format!("type check default {}", self.check_enabled));
    }
    fn configure(&mut self, c: &Config) {
        debug!("ext.configure");
        self.check_enabled = c.bool("ext.check.enabled").unwrap_or(self.check_enabled);

    }
    fn new_game(&mut self) {
        self.new_search();
    }

    fn new_search(&mut self) {
    }
}

impl Default for Extensions {
    fn default() -> Self {
        Extensions {
            check_enabled: true,
        }
    }
}

impl Extensions {
    pub fn extend(&self, _before: &Board, _mv: &Move, after: &Board, node: &Node, search_stats: &mut SearchStats) -> Ply {
        let mut extend = 0;
        if self.check_enabled {
            if after.is_in_check(after.color_us()) {
                search_stats.inc_ext_check(node.ply);
                extend += 1;
            }
        }
        extend
    }
}



impl fmt::Display for Extensions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:#?}", self)?;
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::search::algo::*;
    use crate::utils::*;
    use crate::search::timecontrol::*;
    use crate::tags::*;
    // use crate::search::timecontrol::*;


    #[test]
    #[ignore]
    fn test_ext() {
        let mut engine = Engine::new();

        let positions = &Catalog::example_game(); // [101..102];
        let mut node_count = 0;
        for pos in positions {
            engine.new_game();
            let suggested_depth = pos.acd().unwrap();
            engine.algo.set_timing_method(TimeControl::Depth(suggested_depth-1));
            engine.algo.board = pos.board().clone();

            engine.search();
            let mut results = engine.algo.results().clone();
            let nodes = results.acn().unwrap();
            node_count += nodes;

            // just leave acd
            results.tags_mut().remove(Tag::PV);
            results.tags_mut().remove(Tag::SM);
            results.tags_mut().remove(Tag::BM);
            results.tags_mut().remove(Tag::CE);
            results.tags_mut().remove(Tag::ACN);
            println!("{:>12} {:>12} {}", Formatter::format_u128(nodes), Formatter::format_u128(node_count), results);
        }
    }
}
