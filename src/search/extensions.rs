use crate::eval::score::Score;
use crate::infra::parsed_config::Component;
use crate::mv::Move;
use crate::search::node::Node;
use crate::types::Ply;
use crate::{board::Board, Algo};
use crate::{Bitboard, Piece};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Extensions {
    pub max_extend: Ply,
    pub check_enabled: bool,
    pub check_max_depth: Ply,
    pub check_max_phase: i32,
    check_see: bool,
    check_see_threshold: Score,
    check_only_captures: bool,
    pub promo_enabled: bool,
    pub promo_max_depth: Ply,
    pub near_promo_enabled: bool,
    pub near_promo_max_depth: Ply,
    pub pv_enabled: bool,
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
            max_extend: 1,
            check_enabled: false,
            check_max_depth: 2,
            check_max_phase: 100,
            check_see: false,
            check_see_threshold: Score::zero(),
            check_only_captures: false,

            promo_enabled: false,
            promo_max_depth: 1,

            near_promo_enabled: false,
            near_promo_max_depth: 1,

            pv_enabled: false,
        }
    }
}

impl Extensions {
    #[inline]
    pub fn extend_at_leaf(&self, _before: &Board) -> i32 {
        0
    }

    #[inline]
    pub fn extend(&self, before: &Board, after: &Board, mv: &Move, n: &Node, algo: &Algo) -> Ply {
        let mut ext = 0;
        if n.is_qs() {
            return 0;
        }
        if self.pv_enabled && n.ply == 2 && n.depth > 3 && n.depth < 100 && n.is_pv() {
            ext += 1;
        }
        if self.check_enabled {
            if after.is_in_check(after.color_us())
                && n.depth <= self.check_max_depth
                && after.phase(&algo.eval.phaser) <= self.check_max_phase
                && (!self.check_only_captures || mv.is_capture())
                && (!self.check_see || algo.eval.see.eval_move_see(before, mv) >= self.check_see_threshold.as_i16() as i32)
            {
                // algo.search_stats().inc_ext_check(n.ply);
                ext += 1;
            }
        }

        if self.promo_enabled && mv.is_promo() && n.depth <= self.promo_max_depth {
            ext += 1;
        }

        if self.near_promo_enabled
            && n.depth <= self.near_promo_max_depth
            && mv.mover_piece() == Piece::Pawn
            && mv.to().is_in(Bitboard::RANK_7 | Bitboard::RANK_2)
        {
            ext += 1;
        }

        // (before.them() & before.pawns() & before.color_them().chooser_wb(Bitboard::RANK_6, Bitboard::RANK_3)).any()
        // && n.ply % 2 == 0
        // && mv.is_promo()

        // mv.mover_piece() == Piece::Pawn
        //     && mv.to().rank_index_as_white(before.color_us()) >= self.promo_rank as usize
        //     && node.depth <= self.promo_max_depth
        // {
        //     // search_stats.inc_ext_check(node.ply);
        //     extend += self.promo_extend;
        // }
        std::cmp::min(ext, self.max_extend)
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
            engine.algo.set_timing_method(TimeControl::Depth(suggested_depth - 1));
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
