use crate::board::Board;
use crate::infra::parsed_config::{Component};
use crate::mv::Move;
use crate::phaser::Phaser;
use crate::search::node::Node;
use crate::search::searchstats::SearchStats;
use crate::types::{Color, Piece, Ply};

// use crate::{debug, logger::LogInit};
use std::fmt;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Extensions {
    pub check_enabled: bool,
    pub check_max_depth: Ply,
    pub check_max_phase: i32,
    pub promo_enabled: bool,
    pub promo_max_depth: Ply,
    pub promo_rank: Ply,
    pub promo_extend: Ply,
}

impl Component for Extensions {

    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {}
}

impl Default for Extensions {
    fn default() -> Self {
        Extensions {
            check_enabled: false,
            check_max_depth: 3,
            check_max_phase: 60,

            promo_enabled: false,
            promo_max_depth: 1,
            promo_rank: 7,
            promo_extend: 1,
        }
    }
}

impl Extensions {
    #[inline]
    pub fn extend(
        &self,
        before: &Board,
        mv: &Move,
        _after: &Board,
        node: &Node,
        phaser: &Phaser,
        search_stats: &mut SearchStats,
    ) -> Ply {
        let mut extend = 0;
        if self.check_enabled {
            if before.is_in_check(before.color_us())
                && node.depth <= self.check_max_depth
                && before.phase(phaser) < self.check_max_phase
            {
                search_stats.inc_ext_check(node.ply);
                extend += 1;
            }
        }
        if self.promo_enabled {
            if mv.mover_piece() == Piece::Pawn
                && (before.color_us() == Color::White && mv.to().rank_index() >= self.promo_rank as usize
                    || before.color_us() == Color::Black && 7 - mv.to().rank_index() >= self.promo_rank as usize)
                && node.depth <= self.promo_max_depth
            {
                // search_stats.inc_ext_check(node.ply);
                extend += self.promo_extend;
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
    use crate::search::engine::*;

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
                Formatting::format_u128(nodes),
                Formatting::format_u128(node_count),
                results
            );
        }
    }
}
