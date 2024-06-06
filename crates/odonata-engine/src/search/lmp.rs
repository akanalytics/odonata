use std::fmt;

use odonata_base::domain::node::Node;
use odonata_base::infra::component::Component;
use odonata_base::infra::metric::{Event, Metrics};
use odonata_base::piece::MoveType;
use odonata_base::prelude::*;
use serde::{Deserialize, Serialize};

use super::algo::Search;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lmp {
    pub enabled:   bool,
    pawns:         bool,
    first_move:    bool,
    fw_node:       bool,
    alpha_numeric: bool,
    bad_captures:  bool,
    max_pawn_rank: u8,
    killers:       bool,
    in_check:      bool,
    gives_check:   bool,
    discoverer:    bool,
    extensions:    bool,
    pub a:         f32,
    pub b:         f32,
    pub c:         f32,
}

impl Default for Lmp {
    fn default() -> Self {
        Lmp {
            enabled:       true,
            pawns:         true,
            first_move:    false,
            fw_node:       false,
            alpha_numeric: false,
            bad_captures:  false,
            max_pawn_rank: 6, // forbid promo pawns on 7th
            killers:       false,
            in_check:      false,
            discoverer:    false,
            gives_check:   false,
            extensions:    false,
            a:             4.07,
            b:             0.33,
            c:             1.56,
        }
    }
}

impl Configurable for Lmp {
    fn set(&mut self, p: Param) -> Result<bool> {
        self.enabled.set(p.get("enabled"))?;
        self.pawns.set(p.get("pawns"))?;
        self.first_move.set(p.get("first_move"))?;
        self.fw_node.set(p.get("fw_node"))?;
        self.alpha_numeric.set(p.get("alpha_numeric"))?;
        self.bad_captures.set(p.get("bad_captures"))?;
        self.max_pawn_rank.set(p.get("max_pawn_rank"))?;
        self.killers.set(p.get("killers"))?;
        self.in_check.set(p.get("in_check"))?;
        self.discoverer.set(p.get("discoverer"))?;
        self.gives_check.set(p.get("gives_check"))?;
        self.extensions.set(p.get("extensions"))?;
        self.a.set(p.get("a"))?;
        self.b.set(p.get("b"))?;
        self.c.set(p.get("c"))?;
        Ok(p.is_modified())
    }
}

impl Component for Lmp {
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {}
}

impl Search {
    #[inline]
    pub fn is_quiet(&mut self, before: &Board, mv: Move, stage: MoveType, after: &Board, n: &Node, ext: Ply) -> bool {
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

        if !self.lmp.pawns && mv.mover_piece(before) == Piece::Pawn {
            return false;
        }

        if !self.lmp.killers && stage == MoveType::Killer || !self.lmp.bad_captures && stage == MoveType::BadCapture {
            return false;
        }
        if mv.mover_piece(before) == Piece::Pawn
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

    pub fn can_lmp_move(&mut self, bd: &Board, mv_num: u32, is_quiet: bool, quiets: i32, n: &Node, mv: Move) -> bool {
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

        if bd.maybe_gives_discovered_check(mv) || bd.gives_check(mv) {
            Metrics::incr_node(n, Event::LmpDeclineGivesCheck);
            return false;
        }
        if quiets <= (self.lmp.a + self.lmp.b * n.depth as f32 + self.lmp.c * (n.depth * n.depth) as f32) as i32 {
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
