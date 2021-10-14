use crate::board::Board;
use crate::config::{Component, ParsedConfig};
use crate::eval::score::Score;
use crate::search::node::Node;
use crate::types::{MoveType, MoveTypes, Ply};
use std::fmt;

// CLOP
// 10+.08
// 1+0.01
//

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

#[derive(Clone, Debug)]
pub struct Razor {
    pub enabled: bool,
    pub max_depth: Ply,
    pub margin1: i32,
    pub margin2: i32,
    pub margin3: i32,
    pub move_types_forbidden: MoveTypes,
}

impl Component for Razor {
    fn settings(&self, c: &mut ParsedConfig) {
        c.set("razor.enabled", &format!("type check default {}", self.enabled));
        c.set(
            "razor.max.depth",
            &format!("type spin min 0 max 100 default {}", self.max_depth),
        );
        c.set(
            "razor.margin1",
            &format!("type spin min -9999 max 9999 default {}", self.margin1),
        );
        c.set(
            "razor.margin2",
            &format!("type spin min -9999 max 9999 default {}", self.margin2),
        );
        c.set(
            "razor.margin3",
            &format!("type spin min -9999 max 9999 default {}", self.margin3),
        );
        c.set(
            "futility.movetypes.forbidden",
            &format!(
                "type string default {}",
                MoveType::to_string(self.move_types_forbidden)
            ),
        );
    }
    fn configure(&mut self, c: &ParsedConfig) {
        debug!("razor.configure");
        self.enabled = c.bool("razor.enabled").unwrap_or(self.enabled);
        self.max_depth = c.int("razor.max.depth").unwrap_or(self.max_depth as i64) as Ply;
        self.margin1 = c.int("razor.margin1").unwrap_or(self.margin1 as i64) as i32;
        self.margin2 = c.int("razor.margin2").unwrap_or(self.margin2 as i64) as i32;
        self.margin3 = c.int("razor.margin3").unwrap_or(self.margin3 as i64) as i32;
        if let Ok(mts) = MoveType::from_str(
            &c.string("razor.movetypes.forbidden")
                .unwrap_or(MoveType::to_string(self.move_types_forbidden)),
        ) {
            self.move_types_forbidden = mts;
        }
    }

    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {}
}

impl Default for Razor {
    fn default() -> Self {
        Self {
            enabled: true,
            max_depth: 3, // 1 means we still prune at frontier (depth=1)
            margin1: 100,
            margin2: 300,
            margin3: 500,
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

// look for beta cuts by using a null move and null window search around beta
// works for moves that are just "too good to be true"
impl Razor {
    #[inline]
    pub fn margin(&self, b: &Board, _eval: Score, n: &Node) -> Option<Score> {
        if !self.enabled {
            return None;
        }
        if n.ply == 0 {
            return None; // no null move at root, might mean no moves (with move restrictions too!)
        }
        if n.depth > self.max_depth {
            return None;
        }
        if !n.beta.is_numeric() {
            return None;
        }
        if !n.alpha.is_numeric() {
            return None;
        }
        if n.alpha + Score::from_cp(1) != n.beta {
            return None;
        }
        if b.is_in_check(b.color_us()) {
            return None;
        }
        let margin = match n.depth {
            0 => unreachable!(),
            1 => self.margin1,
            2 => self.margin2,
            3 => self.margin3,
            _ => 200 * n.depth - 100,

        };
        Some(Score::from_cp(margin))
    }

}







impl fmt::Display for Razor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:#?}", self)?;
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
