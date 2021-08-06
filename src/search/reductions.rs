use crate::board::Board;
use crate::cache::tt2::NodeType;
use crate::config::{Component, Config};
use crate::mv::Move;
use crate::search::node::Node;
use crate::search::searchstats::SearchStats;
use crate::types::{MoveType, Piece, Ply};
use crate::{debug, logger::LogInit};
use std::fmt;

#[derive(Clone, Debug)]
pub struct Reductions {
    pub lmr_enabled: bool,
    pub lmr_bad_captures: bool,
    pub lmr_pawns: bool,
    pub lmr_promos: bool,
    pub lmr_killers: bool,
    pub lmr_min_depth: Ply,
    pub lmr_re_search: bool,
    pub lmr_alpha_numeric: bool,
    pub lmr_red_strat: i32,
}

impl Component for Reductions {
    fn settings(&self, c: &mut Config) {
        c.set(
            "red.lmr.enabled",
            &format!("type check default {}", self.lmr_enabled),
        );
        c.set(
            "red.lmr.bad.captures",
            &format!("type check  default {}", self.lmr_bad_captures),
        );
        c.set(
            "red.lmr.re.search",
            &format!("type check  default {}", self.lmr_re_search),
        );
        c.set(
            "red.lmr.alpha.numeric",
            &format!("type check  default {}", self.lmr_alpha_numeric),
        );
        c.set(
            "red.lmr.pawns",
            &format!("type check  default {}", self.lmr_pawns),
        );
        c.set(
            "red.lmr.promos",
            &format!("type check  default {}", self.lmr_promos),
        );
        c.set(
            "red.lmr.killers",
            &format!("type check default {}", self.lmr_killers),
        );
        c.set(
            "red.lmr.min.depth",
            &format!("type spin min 0 max 100 default {}", self.lmr_min_depth),
        );
        c.set(
            "red.lmr.red.strat",
            &format!("type spin min 0 max 10000 default {}", self.lmr_red_strat),
        );
    }
    fn configure(&mut self, c: &Config) {
        debug!("red.configure");
        self.lmr_enabled = c.bool("red.lmr.enabled").unwrap_or(self.lmr_enabled);
        self.lmr_re_search = c.bool("red.lmr.re.search").unwrap_or(self.lmr_re_search);
        self.lmr_alpha_numeric = c.bool("red.lmr.alpha.numeric").unwrap_or(self.lmr_alpha_numeric);
        self.lmr_bad_captures = c.bool("red.lmr.bad.captures").unwrap_or(self.lmr_bad_captures);
        self.lmr_pawns = c.bool("red.lmr.pawns").unwrap_or(self.lmr_pawns);
        self.lmr_promos = c.bool("red.lmr.promos").unwrap_or(self.lmr_promos);
        self.lmr_killers = c.bool("red.lmr.killers").unwrap_or(self.lmr_killers);
        self.lmr_min_depth = c.int("red.lmr.min.depth").unwrap_or(self.lmr_min_depth as i64) as Ply;
        self.lmr_red_strat = c.int("red.lmr.red.strat").unwrap_or(self.lmr_red_strat as i64) as i32;
    }
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {}
}

// WAC @ 1m nodes
// min_depth=3, re-search=false, pawns=true, min_depth=3, reduction=1 => 256
// min_depth=4 => 255
// min_depth=2 => 257   (with re-search => 248)
// min_depth=1 => 257   (with reseach => 248)
// re-search=false, pawns=true, promos=true => 256
// re-search=false, pawns=true, promos=true, killers=true => 256
// re-search=true, pawns=true, promos=true, killers=true => 254
// re-search=true, pawns=true, promos=true => 255
// re-search=true, pawns=true => 255
// alpha_numeric=true => 252

impl Default for Reductions {
    fn default() -> Self {
        Reductions {
            lmr_enabled: true,
            lmr_alpha_numeric: false,
            lmr_re_search: false,
            lmr_bad_captures: true,
            lmr_pawns: true,
            lmr_promos: false,
            lmr_killers: false,
            lmr_min_depth: 2,
            lmr_red_strat: 1,
        }
    }
}

// from CPW
//
// Tactical Moves (captures and promotions)
// Moves while in check
// Moves which give check
// Moves that cause a search extension
// Anytime in a PV-Node in a PVS search
// Depth < 3 (sometimes depth < 2)

// Less Common Conditions
// Less common conditions on moves not to reduce:
// Passed Pawn Moves
// Killer Moves
// Moves threatening the King area
// Tactically threatening moves
// Moves with good past relative history [4]
// Any Pawn Moves
// Allowing reductions of "bad" captures (SEE < 0)
// Moves of a threatened piece to safety (often detected via a Null Move search)
//
// http://www.open-chess.org/viewtopic.php?f=5&t=3084
//
impl Reductions {
    #[inline]
    pub fn lmr(
        &self,
        before: &Board,
        mv: &Move,
        move_number: u32,
        stage: MoveType,
        after: &Board,
        node: &Node,
        _nt: NodeType,
        search_stats: &mut SearchStats,
    ) -> Ply {
        let mut reduce = 0;
        if self.lmr_enabled && node.depth >= self.lmr_min_depth {
            if !self.lmr_pawns && mv.mover_piece() == Piece::Pawn
                || (stage != MoveType::QuietUnsorted
                    && stage != MoveType::Quiet
                    && stage != MoveType::Remaining
                    && stage != MoveType::Killer
                    && stage != MoveType::Promo
                    && stage != MoveType::BadCapture)
                || !self.lmr_promos && stage == MoveType::Promo
                || !self.lmr_killers && stage == MoveType::Killer
                || !self.lmr_bad_captures && stage == MoveType::BadCapture
            {
                return 0;
            }
            if before.is_in_check(before.color_us()) || after.is_in_check(after.color_us()) {
                return 0;
            }
            if self.lmr_alpha_numeric && !node.alpha.is_numeric() {
                return 0;
            }
            search_stats.inc_red_lmr(node.ply);
            reduce = match self.lmr_red_strat {
                1 => 1,
                2 => 2,
                6 => match node.depth {
                    0..=6 => 1,
                    7..=12 => 2,
                    _ => 3,
                },
                9 => match node.depth {
                    0..=9 => 1,
                    10..=15 => 2,
                    _ => 3,
                },
                20 => match move_number {
                    0..=20 => 1,
                    _ => 2,
                },
                920 => match node.depth {
                    0..=9 => match move_number {
                        0..=20 => 1,
                        _ => 2,
                    },
                    10..=15 => match move_number {
                        0..=20 => 2,
                        _ => 3,
                    },
                    _ => 3,
                },
                _ => 1,
            }
        }
        reduce
    }
}

impl fmt::Display for Reductions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:#?}", self)?;
        Ok(())
    }
}
