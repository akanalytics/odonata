use crate::infra::component::Component;
use crate::search::node::Node;
use crate::types::Ply;
use crate::{board::Board, Algo};
use serde::{Deserialize, Serialize};
use std::fmt;

use super::node::Event;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Extensions {
    pub enabled: bool,
    max_extend: Ply,
    in_check_enabled: bool,
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
            enabled: true,
            max_extend: 1,
            in_check_enabled: true,
        }
    }
}

impl Algo {
    #[inline]
    pub fn extensions(&mut self, b: &Board, n: &Node) -> Ply {
        let mut ext = 0;
        if !self.ext.enabled || n.is_qs() {
            return 0;
        }

        if self.ext.in_check_enabled && b.is_in_check(b.color_us()) {
            ext += 1;
        }

        // if self.ext.near_promo_enabled
        //     && n.depth <= self.ext.near_promo_max_depth
        //     && mv.mover_piece() == Piece::Pawn
        //     && mv.to().is_in(Bitboard::RANK_7 | Bitboard::RANK_2)
        // {
        //     ext += 1;
        // }

        // if self.ext.recapture_enabled
        //     && mv.is_capture()
        //     && last.is_capture()
        //     && (!self.ext.recapture_same_square || mv.to() == last.to())
        //     && (!self.ext.recapture_only_pv_node || n.is_pv())
        //     && n.depth <= self.ext.recapture_max_depth
        //     && (MoveType::GoodCapture | MoveType::GoodCaptureUpfrontSorted).contains(mt)
        //     && mv.capture_piece().centipawns() < last.capture_piece().centipawns()
        // // proxy for last = GoodCapture
        // {
        //     ext += 1;
        // }

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
                Formatting::u128(nodes),
                Formatting::u128(node_count),
                results
            );
        }
    }
}
