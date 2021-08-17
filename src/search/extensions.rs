use crate::board::Board;
use crate::config::{Component, Config};
use crate::mv::Move;
use crate::search::node::Node;
use crate::search::searchstats::SearchStats;
use crate::types::Ply;
use crate::{debug, logger::LogInit};
use std::fmt;

#[derive(Clone, Debug)]
pub struct Extensions {
    pub check_enabled: bool,
    pub check_max_depth: Ply,
    pub check_max_phase: i32,
}

impl Component for Extensions {
    fn settings(&self, c: &mut Config) {
        c.set(
            "ext.check.enabled",
            &format!("type check default {}", self.check_enabled),
        );
        c.set(
            "ext.check.max.depth",
            &format!("type spin min 0 max 100 default {}", self.check_max_depth),
        );
        c.set(
            "ext.check.max.phase",
            &format!("type spin min 0 max 100 default {}", self.check_max_phase),
        );
    }
    fn configure(&mut self, c: &Config) {
        debug!("ext.configure");
        self.check_enabled = c.bool("ext.check.enabled").unwrap_or(self.check_enabled);
        self.check_max_depth = c
            .int("ext.check.max.depth")
            .unwrap_or(self.check_max_depth as i64) as Ply;
        self.check_max_phase = c
            .int("ext.check.max.phase")
            .unwrap_or(self.check_max_phase as i64) as i32;
    }
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {}
}

impl Default for Extensions {
    fn default() -> Self {
        Extensions {
            check_enabled: false,
            check_max_depth: 1,
            check_max_phase: 60,
        }
    }
}

impl Extensions {
    #[inline]
    pub fn extend(
        &self,
        before: &Board,
        _mv: &Move,
        _after: &Board,
        node: &Node,
        search_stats: &mut SearchStats,
    ) -> Ply {
        let mut extend = 0;
        if self.check_enabled {
            if before.is_in_check(before.color_us())
                && node.depth <= self.check_max_depth
                && before.phase() < self.check_max_phase
            {
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
    use crate::search::timecontrol::*;
    use crate::tags::*;
    use crate::utils::*;
    // use crate::search::timecontrol::*;

    #[test]
    #[ignore]
    fn test_ext() {
        let mut engine = Engine::new();

        let positions = &Catalog::example_game()[118..119]; // [101..102];
        let mut node_count = 0;
        for pos in positions {
            engine.new_game();
            let suggested_depth = pos.acd().unwrap();
            engine
                .algo
                .set_timing_method(TimeControl::Depth(suggested_depth - 1));
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
            println!(
                "{:>12} {:>12} {}",
                Formatter::format_u128(nodes),
                Formatter::format_u128(node_count),
                results
            );
        }
    }
}
