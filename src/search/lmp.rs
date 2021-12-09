use crate::board::Board;
use crate::infra::component::Component;
use crate::mv::Move;
use crate::search::node::Event;
use crate::search::node::Node;
use crate::types::{MoveType, Ply};
use crate::Algo;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Lmp {
    pub enabled: bool,
    first_move: bool,
    alpha_numeric: bool,
    bad_captures: bool,
    promos: bool,
    killers: bool,
    in_check: bool,
    gives_check: bool,
    extensions: bool,
}

impl Default for Lmp {
    fn default() -> Self {
        Lmp {
            enabled: true,
            first_move: false,
            alpha_numeric: false,
            bad_captures: true,
            promos: false,
            killers: false,
            in_check: false,
            gives_check: false,
            extensions: false,
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
    pub fn lmp(
        &mut self,
        before: &Board,
        _mv: &Move,
        mv_num: u32,
        quiets: i32,
        stage: MoveType,
        after: &Board,
        n: &Node,
        ext: Ply,
        reduction: Ply,
        _tt_mv: Move,
    ) -> bool {
        if !self.lmp.enabled {
            return false;
        }
        if n.is_qs() || n.is_root() || reduction == 0 {
            return false;
        }

        if !self.lmp.first_move && mv_num <= 1 {
            return false;
        }

        // has to be one of these
        if !(MoveType::QuietUnsorted | MoveType::Quiet | MoveType::Remaining | MoveType::Killer | MoveType::Promo | MoveType::BadCapture)
            .contains(stage)
        {
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

        if self.lmp.alpha_numeric && !n.alpha.is_numeric() {
            return false;
        }

        let is_pv = n.is_pv();

        if quiets <= (4 + n.depth * n.depth) / 2 {
            return false;
        }
        if is_pv && quiets <= 4 + n.depth * n.depth {
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
