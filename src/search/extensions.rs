use crate::eval::score::Score;
use crate::infra::component::Component;
use crate::mv::Move;
use crate::search::node::Node;
use crate::types::Ply;
use crate::{board::Board, Algo};
use crate::{Bitboard, Piece};
use serde::{Deserialize, Serialize};
use std::fmt;

use super::node::Event;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Extensions {
    max_extend: Ply,
    gives_check_enabled: bool,
    in_check_enabled: bool,
    check_max_depth: Ply,
    check_max_phase: i32,
    check_see: bool,
    check_see_threshold: Score,
    check_only_captures: bool,
    promo_enabled: bool,
    promo_max_depth: Ply,
    near_promo_enabled: bool,
    near_promo_max_depth: Ply,
    pv_enabled: bool,
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
            gives_check_enabled: true,
            in_check_enabled: false,
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

impl Algo {
    #[inline]
    pub fn extend_at_leaf(&self, _before: &Board) -> i32 {
        0
    }

    #[inline]
    pub fn extend(&mut self, before: &Board, after: &Board, mv: &Move, mv_num: u32, n: &Node) -> Ply {
        let mut ext = 0;
        if n.is_qs() {
            return 0;
        }
        if self.ext.pv_enabled && n.depth == 1 && mv_num == 1 {
            ext += 1;
        }
        if self.ext.gives_check_enabled && after.is_in_check(after.color_us())
        ||
        self.ext.in_check_enabled && before.is_in_check(before.color_us()) {
            if n.depth <= self.ext.check_max_depth
                && after.phase(&self.eval.phaser) <= self.ext.check_max_phase
                && (!self.ext.check_only_captures || mv.is_capture())
                && (!self.ext.check_see || self.eval.see.eval_move_see(before, mv) >= self.ext.check_see_threshold.as_i16() as i32)
            {
                // algo.search_stats().inc_ext_check(n.ply);
                ext += 1;
            }
        }

        if self.ext.promo_enabled && mv.is_promo() && n.depth <= self.ext.promo_max_depth {
            ext += 1;
        }

        if self.ext.near_promo_enabled
            && n.depth <= self.ext.near_promo_max_depth
            && mv.mover_piece() == Piece::Pawn
            && mv.to().is_in(Bitboard::RANK_7 | Bitboard::RANK_2)
        {
            ext += 1;
        }

        // (before.them() & before.pawns() & before.color_them().chooser_wb(Bitboard::RANK_6, Bitboard::RANK_3)).any()
        // && n.ply % 2 == 0
        // && mv.is_promo()

        // mv.mover_piece() == Piece::Pawn
        //     && mv.to().rank_index_as_white(before.color_us()) >= self.ext.promo_rank as usize
        //     && node.depth <= self.ext.promo_max_depth
        // {
        //     // search_stats.inc_ext_check(node.ply);
        //     extend += self.ext.promo_extend;
        // }
        let ext = std::cmp::min(ext, self.ext.max_extend);
        if ext > 0 {
            self.counts.inc(n, Event::Extension);
        }
        ext
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
                Formatting::u128(nodes),
                Formatting::u128(node_count),
                results
            );
        }
    }
}
