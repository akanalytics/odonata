use crate::board::Board;
use crate::bound::NodeType;
use crate::cache::tt2::TtNode;
use crate::eval::score::{Score, ToScore};
use crate::infra::component::Component;
use crate::infra::metric::Metrics;
use crate::mv::Move;
use crate::search::node::{Event, Node};
use crate::piece::{MoveType, MoveTypes, Ply};
use crate::Algo;
use serde::{Deserialize, Serialize};
use std::fmt;

// CLOP - gamma
// 10+.08
// 1+0.01
// 2+0.02 76/235/380 on 170k iters
// 2+0.02 76/275/380 on 180k iters
//
// CLOP - linear
// 2+0.02 94/431/865 - 90/381/873 at 362k iters

//
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Razor {
    pub enabled: bool,
    pub beta_enabled: bool,
    pub store_tt: bool,
    pub pv_nodes: bool,
    pub min_opponents: i32,
    pub max_depth: Ply,
    pub margin1: i32,
    pub margin2: i32,
    pub margin3: i32,
    pub move_types_forbidden: MoveTypes,
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
            enabled: true,
            beta_enabled: true,
            store_tt: true,
            pv_nodes: false,
            min_opponents: 4,
            max_depth: 3, // 1 means we still prune at frontier (depth=1)
            margin1: 94,
            margin2: 381,
            margin3: 873,
            move_types_forbidden: MoveType::Hash
                | MoveType::Killer
                | MoveType::Promo
                | MoveType::Capture
                | MoveType::GoodCapture
                | MoveType::GoodCapture
                | MoveType::GoodCaptureUpfrontSorted,
        }
    }
}

impl Razor {
    #[inline]
    fn can_razor(&self, b: &Board, n: &Node) -> bool {
        if !self.enabled {
            return false;
        }
        if n.ply == 0 {
            return false; // no null move at root, might mean no moves (with move restrictions too!)
        }
        // no razoring in qs or too far from horizon
        if n.is_qs() || n.depth > self.max_depth {
            return false;
        }
        if !n.beta.is_numeric() {
            return false;
        }
        if !n.alpha.is_numeric() {
            return false;
        }
        if !self.pv_nodes && n.is_fw() {
            return false;
        }
        if b.is_in_check(b.color_us()) {
            return false;
        }
        // "Scalable Search in Computer Chess" limited razoring p43
        if n.depth > 2 && self.min_opponents > 0 && b.them().popcount() < self.min_opponents {
            return false;
        }
        true
    }
}

impl Algo {
    #[inline]
    pub fn razor_node(
        &mut self,
        last_move: Move,
        b: &mut Board,
        eval: Score,
        n: &Node,
    ) -> Result<Option<Score>, Event> {
        if !self.razor.can_razor(b, n) {
            return Ok(None);
        }

        let margin = Score::from_cp(match n.depth {
            1 => self.razor.margin1,
            2 => self.razor.margin2,
            3 => self.razor.margin3,
            _ => unreachable!(),
        });

        if self.razor.beta_enabled && eval > n.beta + margin {
            return Ok(Some(n.beta));
        }

        // theres no make move here, so no negamax a/b and sign reversals
        if eval <= n.alpha - margin {
            if n.depth <= 2 {
                // drop straight into qsearch
                let (score, _event) = self.alphabeta(b, n.ply, 0, n.alpha, n.beta, last_move)?;
                Metrics::incr_node(n, Event::RazorD3Success);
                return Ok(Some(score));
            } else {
                // pvs search around {alpha - margin}
                let (score, _event) = self.alphabeta(
                    b,
                    n.ply,
                    0,
                    n.alpha - margin,
                    n.alpha - margin + 1.cp(),
                    last_move,
                )?;
                if self.razor.store_tt {
                    let entry = TtNode {
                        score: score.clamp_score(),
                        depth: 1,
                        nt: NodeType::UpperAll,
                        bm: Move::NULL_MOVE,
                    };
                    self.tt.store(b.hash(), entry);
                }

                // fail low (-inf) or alpha-margin
                if score <= n.alpha - margin {
                    Metrics::incr_node(n, Event::RazorD3Success);
                    return Ok(Some(n.alpha));
                }
                Metrics::incr_node(n, Event::RazorFail);
            }
        }
        Ok(None)
    }
}

impl fmt::Display for Razor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", toml::to_string(self).unwrap())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::catalog::*;
    //use crate::comms::uci::*;
    // use crate::eval::eval::*;

    #[test]
    fn test_nmp() {
        let nmp = Razor::default();
        println!("{:#?}", nmp);
        println!("{}", nmp);
    }
}
