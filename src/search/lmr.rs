use crate::board::Board;
use crate::bound::NodeType;
use crate::infra::parsed_config::Component;
use crate::mv::Move;
use crate::search::node::Node;
use crate::search::searchstats::SearchStats;
use crate::types::{MoveType, Ply};
// use crate::{debug, logger::LogInit};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Lmr {
    enabled: bool,
    pv_node: bool,
    only_nt_all: bool,
    bad_captures: bool,
    pawns: bool,
    promos: bool,
    killers: bool,
    pub re_search: bool,
    alpha_numeric: bool,
    reduce_extensions: bool,
    quiets1: i32,
    quiets2: i32,
    depth1: Ply,
    depth2: Ply,
    depth3: Ply,
    depth4: Ply,
    iir: bool
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
            reduce_extensions: false,
            quiets1: 20,
            quiets2: 30,
            depth1: 3,
            depth2: 7,
            depth3: 13,
            depth4: 17,
            iir: false, 
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
        _mv: &Move,
        _move_number: u32,
        quiets: i32,
        stage: MoveType,
        after: &Board,
        node: &Node,
        nt: NodeType,
        allow_red: bool,
        _tt_mv: Move,
        search_stats: &mut SearchStats,
    ) -> Ply {
        if !self.enabled {
            return 0;
        }
        if !allow_red {
            return 0;
        }
        if node.is_qs() {
            return 0;
        }
        let mut reduce = match node.depth  {
            d if d >= self.depth4 => 4,
            d if d >= self.depth3 => 3,
            d if d >= self.depth2 => 2,
            d if d >= self.depth1 => 1,
            _ => 0
        };
        reduce += match quiets {
            q if q >= self.quiets2 => 2,
            q if q >= self.quiets1 => 1,
            _ => 0
        };



        if reduce == 0 {
            return reduce;
        }
        // has to be one of these
        if !(MoveType::QuietUnsorted
            | MoveType::Quiet
            | MoveType::Remaining
            | MoveType::Killer
            | MoveType::Promo
            | MoveType::BadCapture)
            .contains(stage)
        {
            return 0;
        }
        if !self.promos && stage == MoveType::Promo
            || !self.killers && stage == MoveType::Killer
            || !self.bad_captures && stage == MoveType::BadCapture
        {
            return 0;
        }
        if self.only_nt_all && nt != NodeType::UpperAll {
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

        reduce
    }
}

impl fmt::Display for Lmr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:#?}", self)?;
        Ok(())
    }
}
