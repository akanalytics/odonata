use std::fmt;

use odonata_base::boards::Position;
use odonata_base::domain::node::{Node, SearchType};
use odonata_base::domain::score::ToScore;
use odonata_base::domain::BoundType;
use odonata_base::infra::component::Component;
use odonata_base::infra::metric::{Event, Metrics};
use odonata_base::piece::MoveType;
use odonata_base::prelude::*;

use super::algo::Search;
use super::trail::Trail;
use crate::cache::tt2::{TtNode, TtScore};

// CLOP - gamma
// 10+.08
// 1+0.01
// 2+0.02 76/235/380 on 170k iters
// 2+0.02 76/275/380 on 180k iters
//
// CLOP - linear
// 2+0.02 94/431/865 - 90/381/873 at 362k iters

// http://talkchess.com/forum3/viewtopic.php?f=7&t=41597
//
// Similar code appears in Jury Osipov's open source engine Strelka 2.0 [12] , failing a bit harder.
// The interesting thing is the missing new_value < beta condition in the depth = 1 case.
// If the static evaluation indicates a fail-low node, but q-search fails high, the score of the reduced fail-high search is returned,
// since there was obviously a winning capture raising the score, and one assumes a quiet move near the horizon will not do better [13] .

// value = eval + 125;
// if (value < beta) {
//   if (depth == 1) {
//     new_value = qsearch(...);
//     return max(new_value, value);
//   }
//   value += 175;
//   if (value < beta && depth <= 3) {
//     new_value = qsearch(...);
//     if (new_value < beta)
//        return max(new_value, value);
//   }
// }

#[derive(Clone, Debug)]
pub struct Razor {
    pub enabled:          bool,
    beta_enabled:         bool,
    store_tt:             bool,
    pv_nodes:             bool,
    min_opponents:        i32,
    min_pieces:           i32,
    min_pieces_depth:     Ply,
    max_depth:            Ply,
    pub margin1:          i32,
    pub margin2:          i32,
    pub margin3:          i32,
    move_types_forbidden: MoveType,
}

impl Component for Razor {
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {}
}

impl Default for Razor {
    fn default() -> Self {
        Self {
            enabled:              true,
            beta_enabled:         true,
            store_tt:             true,
            pv_nodes:             false,
            min_opponents:        4,
            min_pieces:           0,
            min_pieces_depth:     1,
            max_depth:            3, // 1 means we still prune at frontier (depth=1)
            margin1:              94,
            margin2:              381,
            margin3:              873,
            move_types_forbidden: MoveType::Hash
                | MoveType::Killer
                | MoveType::Promo
                | MoveType::Capture
                | MoveType::GoodCapture
                | MoveType::GoodCaptureUpfrontSorted
                | MoveType::CounterMove
                | MoveType::QueenPromo,
        }
    }
}

impl Configurable for Razor {
    fn set(&mut self, p: Param) -> Result<bool> {
        self.enabled.set(p.get("enabled"))?;
        self.beta_enabled.set(p.get("beta_enabled"))?;
        self.store_tt.set(p.get("store_tt"))?;
        self.pv_nodes.set(p.get("pv_nodes"))?;
        self.min_opponents.set(p.get("min_opponents"))?;
        self.min_pieces.set(p.get("min_pieces"))?;
        self.min_pieces_depth.set(p.get("min_pieces_depth"))?;
        self.max_depth.set(p.get("max_depth"))?;
        self.margin1.set(p.get("margin1"))?;
        self.margin2.set(p.get("margin2"))?;
        self.margin3.set(p.get("margin3"))?;
        Configurable::set(&mut self.move_types_forbidden, p.get("move_types_forbidden"))?; // method name clash
        Ok(p.is_modified())
    }
}

impl fmt::Display for Razor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{self:#?}")
    }
}

impl Razor {
    #[inline]
    fn can_razor(&self, bd: &Board, n: &Node) -> bool {
        if !self.enabled {
            return false;
        }
        if n.ply == 0 {
            return false; // no razor at root, might mean no moves (with move restrictions too!)
        }
        Metrics::incr_node(n, Event::RazorConsider);

        // no razoring in qs or too far from horizon
        if n.is_qs() || n.depth > self.max_depth {
            Metrics::incr_node(n, Event::RazorDeclineDepth);
            return false;
        }
        if !n.alpha.is_numeric() {
            Metrics::incr_node(n, Event::RazorDeclineAlphaNumeric);
            return false;
        }
        if !n.beta.is_numeric() {
            Metrics::incr_node(n, Event::RazorDeclineBetaNumeric);
            return false;
        }
        if !self.pv_nodes && n.is_fw() {
            Metrics::incr_node(n, Event::RazorDeclinePvNode);
            return false;
        }
        if bd.is_in_check(bd.color_us()) {
            Metrics::incr_node(n, Event::RazorDeclineInCheck);
            return false;
        }
        // "Scalable Search in Computer Chess" limited razoring p43
        if self.min_opponents > 0 && n.depth > 2 && bd.them().popcount() < self.min_opponents {
            Metrics::incr_node(n, Event::RazorDeclineMinOpponents);
            return false;
        }
        if self.min_pieces > 0 && n.depth >= self.min_pieces_depth && bd.occupied().popcount() < self.min_pieces {
            Metrics::incr_node(n, Event::RazorDeclineMinPieces);
            return false;
        }
        true
    }
}

impl Search {
    #[inline]
    pub fn razor_node(
        &mut self,
        trail: &mut Trail,
        last_move: Move,
        pos: &mut Position,
        eval: Score,
        n: &Node,
    ) -> Result<Option<Score>, Event> {
        if !self.razor.can_razor(pos.board(), n) {
            return Ok(None);
        }

        let margin = Score::from_cp(match n.depth {
            1 => self.razor.margin1,
            2 => self.razor.margin2,
            3 => self.razor.margin3,
            _ => unreachable!(),
        });

        let event = match n.depth {
            1 => Event::RazorD1Success,
            2 => Event::RazorD2Success,
            3 => Event::RazorD3Success,
            _ => unreachable!(),
        };
        if self.razor.beta_enabled && eval > n.beta + margin {
            Metrics::incr_node(n, Event::RazorSuccess);
            Metrics::incr_node(n, event);
            return Ok(Some(n.beta));
        }

        // theres no make move here, so no negamax a/b and sign reversals
        if eval <= n.alpha - margin {
            if n.depth <= 2 {
                // drop straight into qsearch
                let (score, _event) = self.alphabeta(
                    "rz",
                    trail,
                    pos,
                    Node {
                        zw:    n.zw,
                        ply:   n.ply,
                        depth: 0,
                        alpha: n.alpha,
                        beta:  n.beta,
                    },
                    last_move,
                )?;
                Metrics::incr_node(n, Event::RazorSuccess);
                Metrics::incr_node(n, event);
                return Ok(Some(score));
            } else {
                // pvs search around {alpha - margin}
                let (score, _event) = self.alphabeta(
                    "rzzw",
                    trail,
                    pos,
                    Node {
                        zw:    (n.zw - SearchType::FullWindow) | SearchType::ZeroWindow | SearchType::Razor,
                        ply:   n.ply,
                        depth: 0,
                        alpha: n.alpha - margin,
                        beta:  n.alpha - margin + 1.cp(),
                    },
                    last_move,
                )?;
                if self.razor.store_tt {
                    let entry = TtNode {
                        score: TtScore::new(score.clamp_score(), n.ply),
                        depth: 1,
                        bt: BoundType::UpperAll,
                        bm: Move::new_null().to_inner(),
                        eval,
                    };
                    self.tt.store(pos.board().hash(), entry);
                }

                // fail low (-inf) or alpha-margin
                if score <= n.alpha - margin {
                    Metrics::incr_node(n, Event::RazorSuccess);
                    Metrics::incr_node(n, event);
                    // score could be a mate score here so clamp
                    return Ok(Some(score.clamp_score() + margin));
                }
                Metrics::incr_node(n, Event::RazorFail);
            }
        } else {
            Metrics::incr_node(n, Event::RazorDeclineMargin);
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::catalog::*;
    // use crate::comms::uci::*;
    // use crate::eval::eval::*;

    #[test]
    fn test_razor() {
        let nmp = Razor::default();
        println!("{:#?}", nmp);
        println!("{}", nmp);
    }
}
