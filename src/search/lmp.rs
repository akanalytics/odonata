use crate::board::Board;
use crate::infra::component::Component;
use crate::mv::Move;
use crate::search::node::Event;
use crate::search::node::Node;
use crate::types::{MoveType, Ply};
use crate::Algo;
use crate::Piece;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Lmp {
    pub enabled: bool,
    first_move: bool,
    alpha_numeric: bool,
    bad_captures: bool,
    pawns: bool,
    promos: bool,
    killers: bool,
    in_check: bool,
    gives_check: bool,
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
            alpha_numeric: false,
            bad_captures: false,
            pawns: true,
            promos: false,
            killers: false,
            in_check: false,
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
        mv_num: u32,
        stage: MoveType,
        after: &Board,
        n: &Node,
        ext: Ply,
    ) -> bool {
        if n.is_qs() || n.is_root() {
            return false;
        }

        if !self.lmp.first_move && mv_num <= 1 {
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

        if !self.lmp.promos && stage == MoveType::Promo
            || !self.lmp.killers && stage == MoveType::Killer
            || !self.lmp.bad_captures && stage == MoveType::BadCapture
        {
            return false;
        }
        if !self.lmp.extensions && ext > 0
            || !self.lmp.in_check && before.is_in_check(before.color_us())
            || !self.lmp.gives_check && after.is_in_check(after.color_us())
        {
            return false;
        }

        true
    }

    pub fn can_lmp(&mut self, is_quiet: bool, quiets: i32, n: &Node) -> bool {
        if !self.lmp.enabled || self.minmax || !is_quiet {
            return false;
        }
        if self.lmp.alpha_numeric && !n.alpha.is_numeric() {
            return false;
        }
        if quiets
            <= (self.lmp.a + self.lmp.b * n.depth as f32 + self.lmp.c * (n.depth * n.depth) as f32)
                as i32
        {
            return false;
        }

        let is_pv = n.is_pv();
        if is_pv
            && quiets
                <= (self.lmp.pa
                    + self.lmp.pb * n.depth as f32
                    + self.lmp.pc * (n.depth * n.depth) as f32) as i32
        {
            return false;
        }

        self.counts.inc(n, Event::Lmp);
        true
    }
}

impl fmt::Display for Lmp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:#?}", self)?;
        Ok(())
    }
}
