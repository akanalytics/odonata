use crate::board::Board;
use crate::infra::component::Component;
use crate::infra::metric::Metrics;
use crate::mv::Move;
use crate::piece::{MoveType, Ply};
use crate::search::node::Node;
use crate::Algo;
use crate::Piece;
use serde::{Deserialize, Serialize};
use std::fmt;

use super::node::Event;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Lmp {
    pub enabled: bool,
    first_move: bool,
    fw_node: bool,
    alpha_numeric: bool,
    bad_captures: bool,
    pawns: bool,
    max_pawn_rank: u8,
    killers: bool,
    in_check: bool,
    gives_check: bool,
    discoverer: bool,
    extensions: bool,
    a: f32,
    b: f32,
    c: f32,
    pa: f32,
    pb: f32,
    pc: f32,
}

impl Default for Lmp {
    fn default() -> Self {
        Lmp {
            enabled: true,
            first_move: false,
            fw_node: false,
            alpha_numeric: false,
            bad_captures: false,
            pawns: true,
            max_pawn_rank: 6, // forbid promo pawns on 7th
            killers: false,
            in_check: false,
            discoverer: false,
            gives_check: false,
            extensions: false,
            a: 2.5,
            b: 1.0,
            c: 0.5,
            pa: 4.5,
            pb: 2.0,
            pc: 1.0,
        }
    }
}

impl Component for Lmp {
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {}
}

impl Algo {
    #[inline]
    pub fn is_quiet(
        &mut self,
        before: &Board,
        mv: Move,
        stage: MoveType,
        after: &Board,
        n: &Node,
        ext: Ply,
    ) -> bool {
        if n.is_qs() || n.is_root() {
            return false;
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
            return false;
        }

        if !self.lmp.pawns && mv.mover_piece() == Piece::Pawn {
            return false;
        }

        if !self.lmp.killers && stage == MoveType::Killer
            || !self.lmp.bad_captures && stage == MoveType::BadCapture
        {
            return false;
        }
        if mv.mover_piece() == Piece::Pawn
            && mv.from().rank_number_as_white(before.color_us()) > self.lmp.max_pawn_rank as usize
        {
            return false;
        }

        if !self.lmp.extensions && ext > 0
            || !self.lmp.in_check && before.is_in_check(before.color_us())
            || !self.lmp.discoverer && before.maybe_gives_discovered_check(mv)
            // gives check a more precise and costly version of discoverers
            || !self.lmp.gives_check && after.is_in_check(after.color_us())
        {
            return false;
        }

        true
    }

    pub fn can_lmp_move(
        &mut self,
        bd: &Board,
        mv_num: u32,
        is_quiet: bool,
        quiets: i32,
        n: &Node,
        mv: Move,
    ) -> bool {
        if !self.lmp.enabled || !is_quiet {
            return false;
        }

        Metrics::incr_node(n, Event::LmpConsider);

        if !self.lmp.first_move && mv_num <= 1 {
            return false;
        }

        if !self.lmp.fw_node && n.is_fw() {
            Metrics::incr_node(n, Event::LmpDeclineFwWindow);
            return false;
        }

        if self.lmp.alpha_numeric && !n.alpha.is_numeric() {
            Metrics::incr_node(n, Event::LmpDeclineMateBound);
            return false;
        }

        if n.depth > 4 {
            Metrics::incr_node(n, Event::LmpDeclineMaxDepth);
            return false;
        }

        if bd.is_in_check(bd.color_us()) {
            Metrics::incr_node(n, Event::LmpDeclineInCheck);
            return false;
        }

        if bd.maybe_gives_discovered_check(mv) || bd.gives_check(&mv) {
            Metrics::incr_node(n, Event::LmpDeclineGivesCheck);
            return false;
        }
        if quiets
            <= (self.lmp.a + self.lmp.b * n.depth as f32 + self.lmp.c * (n.depth * n.depth) as f32)
                as i32
        {
            Metrics::incr_node(n, Event::LmpDeclineFormula);
            return false;
        }

        Metrics::incr_node(n, Event::LmpSuccess);
        true
    }
}

impl fmt::Display for Lmp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:#?}", self)?;
        Ok(())
    }
}
