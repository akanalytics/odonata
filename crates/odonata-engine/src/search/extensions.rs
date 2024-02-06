use odonata_base::{
    bits::bitboard::Dir,
    domain::node::Node,
    infra::component::Component,
    piece::{MoveType, Ply},
    prelude::*,
    Piece,
};
use serde::{Deserialize, Serialize};
use std::fmt;

use super::algo::Search;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
    pub enabled:          bool,
    max_extend:           Ply,
    gives_check_enabled:  bool,
    in_check_enabled:     bool,
    check_max_depth:      Ply,
    check_see:            bool,
    check_see_threshold:  Score,
    check_only_captures:  bool,
    promo_enabled:        bool,
    promo_max_depth:      Ply,
    near_promo_enabled:   bool,
    near_promo_max_depth: Ply,

    recapture_enabled:      bool,
    recapture_same_square:  bool,
    recapture_only_pv_node: bool,
    recapture_max_depth:    Ply,
    pawn_double_attacks:    bool,

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
            enabled:             true,
            max_extend:          1,
            gives_check_enabled: true,
            in_check_enabled:    false,
            check_max_depth:     2,
            check_see:           false,
            check_see_threshold: Score::zero(),
            check_only_captures: false,

            promo_enabled:        false,
            promo_max_depth:      1,
            near_promo_enabled:   false,
            near_promo_max_depth: 1,

            recapture_enabled:      false,
            recapture_same_square:  true,
            recapture_only_pv_node: false,
            recapture_max_depth:    3,
            pawn_double_attacks:    false,

            pv_enabled: false,
        }
    }
}

impl Search {
    #[inline]
    pub fn extend(
        &mut self,
        before: &Board,
        after: &Board,
        mv: Move,
        _mt: MoveType,
        mv_num: u32,
        n: &Node,
        _last: Move,
    ) -> Ply {
        let mut ext = 0;
        if !self.ext.enabled || n.is_qs() {
            return 0;
        }
        if self.ext.pv_enabled && n.depth == 1 && mv_num == 1 {
            ext += 1;
        }
        if self.ext.gives_check_enabled && after.is_in_check(after.color_us())
            || self.ext.in_check_enabled && before.is_in_check(before.color_us())
        {
            #[allow(clippy::collapsible_else_if)]
            if n.depth <= self.ext.check_max_depth
                && (!self.ext.check_only_captures || mv.is_capture())
                && (!self.ext.check_see
                    || self.eval.eval_move_see(mv, before) >= self.ext.check_see_threshold)
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
            && mv.mover_piece(before) == Piece::Pawn
            && mv.to().is_in(Bitboard::RANK_7 | Bitboard::RANK_2)
        {
            ext += 1;
        }

        if self.ext.pawn_double_attacks && n.depth <= 2 && mv.mover_piece(before) == Piece::Pawn {
            // let wp = after.pawns() & after.white();
            // let bp = after.pawns() & after.black();
            let to = mv.to().as_bb();
            let forks = match before.turn() {
                Color::White => {
                    let white_attacks = to.shift(Dir::NW) | to.shift(Dir::NE);
                    (white_attacks & (after.black() - after.pawns())).two_or_more()
                },
                Color::Black => {
                    let black_attacks = to.shift(Dir::SW) | to.shift(Dir::SE);
                    (black_attacks & (after.white() - after.pawns())).two_or_more()
                },
                
            };
            if forks {
                ext += 1;
            }
        }

        // TODO! recapture extension
        // if self.ext.recapture_enabled {
        //     if let Some(cap) = mv.capture_piece(before) {
        //         if let Some(last_cap) = last.capture_piece() {
        //             if (!self.ext.recapture_same_square || mv.to() == last.to())
        //                 && (!self.ext.recapture_only_pv_node || n.is_fw())
        //                 && n.depth <= self.ext.recapture_max_depth
        //                 && (MoveType::GoodCapture | MoveType::GoodCaptureUpfrontSorted).contains(mt)
        //                 && cap.centipawns() < last_cap.centipawns()
        //             // proxy for last = GoodCapture
        //             {
        //                 ext += 1;
        //             }
        //         }
        //     }
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
        std::cmp::min(ext, self.ext.max_extend)
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

    // use crate::search::timecontrol::*;

    // #[test]
    // #[ignore]
    // fn test_ext() {
    //     let mut engine = ThreadedSearch::new();

    //     let positions = &Catalog::example_game()[118..119]; // [101..102];
    //     let mut node_count = 0;
    //     for pos in positions {
    //         engine.new_game();
    //         let suggested_depth = pos.int("acd").unwrap() as i32;
    //         engine
    //             .search
    //             .set_timing_method(TimeControl::Depth(suggested_depth - 1));
    //         engine.search.board = pos.board().clone();

    //         engine.search_sync();
    //         let mut results = engine.search.results_as_position().clone();
    //         let nodes = results.int("acn").unwrap();
    //         node_count += nodes;

    //         // just leave acd
    //         results.tags_mut().pv = None;
    //         results.tags_mut().supplied_move = None;
    //         results.tags_mut().best_moves = None;
    //         results.tags_mut().centipawn_evaluation = None;
    //         results.tags_mut().acn = None;
    //         println!("{nodes:>12} {node_count:>12} {results}");
    //     }
    // }
}
