use crate::board::Board;
use crate::infra::parsed_config::Component;
use crate::mv::Move;
use crate::phaser::Phaser;
use crate::search::node::Node;
use crate::search::searchstats::SearchStats;
use crate::types::{Ply};
use crate::Bitboard;

// use crate::{debug, logger::LogInit};
use serde::{Deserialize, Serialize};
use std::fmt;

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
    pub fn extend_at_leaf(&self, _before: &Board) -> i32 {
        0
    }

    #[inline]
    pub fn extend(
        &self,
        b: &Board,
        _mv: &Move,
        node: &Node,
        phaser: &Phaser,
        stats: &mut SearchStats,
    ) -> (Ply, bool) {
        if node.is_qs() {
            return (0, true)
        }
        if self.check_enabled {
            if b.is_in_check(b.color_us())
                && node.depth <= self.check_max_depth
                && b.phase(phaser) < self.check_max_phase
            {
                stats.inc_ext_check(node.ply);
                (1, false);
            }
        }
        if self.promo_enabled {
            if (b.them() & b.pawns() & b.color_them().chooser_wb(Bitboard::RANK_7, Bitboard::RANK_2)).any()
                && node.depth <= self.promo_max_depth
            {
                return (self.promo_extend, false);
            }
            // mv.mover_piece() == Piece::Pawn
            //     && mv.to().rank_index_as_white(before.color_us()) >= self.promo_rank as usize
            //     && node.depth <= self.promo_max_depth
            // {
            //     // search_stats.inc_ext_check(node.ply);
            //     extend += self.promo_extend;
            // }
        }
        (0, true)
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
    use crate::search::engine::*;
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
            let mut results = engine.algo.results_as_position().clone();
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
