use crate::board::Board;
use crate::bound::NodeType;
use crate::infra::parsed_config::{Component};
use crate::mv::Move;
use crate::search::node::Node;
use crate::search::searchstats::SearchStats;
use crate::types::{MoveType, Piece, Ply};
// use crate::{debug, logger::LogInit};
use std::fmt;
use serde::{Deserialize, Serialize};


#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Lmr {
    pub enabled: bool,
    pub pv_node: bool, 
    pub only_nt_all: bool, 
    pub bad_captures: bool,
    pub pawns: bool,
    pub promos: bool,
    pub killers: bool,
    pub min_depth: Ply,
    pub re_search: bool,
    pub alpha_numeric: bool,
    pub red_strat: i32,
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

impl Default for Lmr {
    fn default() -> Self {
        Lmr {
            enabled: true,
            pv_node: true,
            only_nt_all: false,
            alpha_numeric: false,
            re_search: false,
            bad_captures: true,
            pawns: true,
            promos: false,
            killers: false,
            min_depth: 2,
            red_strat: 6,
        }
    }
}


impl Component for Lmr {
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {}
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
impl Lmr {
    #[inline]
    pub fn lmr(
        &self,
        before: &Board,
        mv: &Move,
        move_number: u32,
        stage: MoveType,
        after: &Board,
        node: &Node,
        nt: NodeType,
        search_stats: &mut SearchStats,
    ) -> Ply {
        let mut reduce = 0;
        if self.enabled && node.depth >= self.min_depth {
            if !self.pawns && mv.mover_piece() == Piece::Pawn
                || (stage != MoveType::QuietUnsorted
                    && stage != MoveType::Quiet
                    && stage != MoveType::Remaining
                    && stage != MoveType::Killer
                    && stage != MoveType::Promo
                    && stage != MoveType::BadCapture)
                || !self.promos && stage == MoveType::Promo
                || !self.killers && stage == MoveType::Killer
                || !self.bad_captures && stage == MoveType::BadCapture
            {
                return 0;
            }
            if self.only_nt_all && nt != NodeType::All {
                return 0;
            }
            if before.is_in_check(before.color_us()) || after.is_in_check(after.color_us()) {
                return 0;
            }
            if self.alpha_numeric && !node.alpha.is_numeric() {
                return 0;
            }
            if !self.pv_node && node.is_pv() {
                return 0;
            }
            search_stats.inc_red_lmr(node.ply);
            reduce = match self.red_strat {
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

impl fmt::Display for Lmr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:#?}", self)?;
        Ok(())
    }
}
