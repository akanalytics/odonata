use crate::Algo;
use crate::board::Board;
use crate::bound::NodeType;
use crate::infra::component::Component;
use crate::mv::Move;
use crate::search::node::Node;
use crate::types::{MoveType, Ply};
use crate::search::node::{Event};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Lmr {
    enabled: bool,
    first_move: bool,
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
    reduce_1_at_depth: Ply,
    reduce_2_at_depth: Ply,
    reduce_3_at_depth: Ply,
    reduce_4_at_depth: Ply,
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
            first_move: false,
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
            reduce_1_at_depth: 3,
            reduce_2_at_depth: 7,
            reduce_3_at_depth: 13,
            reduce_4_at_depth: 17,
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
impl Algo {
    #[inline]
    pub fn lmr(
        &mut self,
        before: &Board,
        _mv: &Move,
        mv_num: u32,
        quiets: i32,
        stage: MoveType,
        after: &Board,
        n: &Node,
        nt: NodeType,
        ext: Ply,
        _tt_mv: Move,
    ) -> Ply {
        if !self.lmr.enabled {
            return 0;
        }
        if ext !=0 {
            return 0;
        }
        if n.is_qs() {
            return 0;
        }


        let mut reduce = match n.depth  {
            d if d >= self.lmr.reduce_4_at_depth => 4,
            d if d >= self.lmr.reduce_3_at_depth => 3,
            d if d >= self.lmr.reduce_2_at_depth => 2,
            d if d >= self.lmr.reduce_1_at_depth => 1,
            _ => 0,
        };


        reduce += match quiets {
            q if q >= self.lmr.quiets2 => 2,
            q if q >= self.lmr.quiets1 => 1,
            _ => 0
        };

        if !self.lmr.first_move && mv_num <= 1 {
            return 0;
        }
        
        if reduce == 0 {
            return 0;
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
        if !self.lmr.promos && stage == MoveType::Promo
            || !self.lmr.killers && stage == MoveType::Killer
            || !self.lmr.bad_captures && stage == MoveType::BadCapture
        {
            return 0;
        }
        if self.lmr.only_nt_all && nt != NodeType::UpperAll {
            return 0;
        }
        if before.is_in_check(before.color_us()) || after.is_in_check(after.color_us()) {
            return 0;
        }
        if self.lmr.alpha_numeric && !n.alpha.is_numeric() {
            return 0;
        }

        self.stats.inc_red_lmr(n.ply);
        self.counts.inc(n, Event::Lmr);
        reduce
    }
}

impl fmt::Display for Lmr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:#?}", self)?;
        Ok(())
    }
}
