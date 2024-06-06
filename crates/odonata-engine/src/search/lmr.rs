use std::fmt;
use std::fmt::Debug;

use odonata_base::domain::node::Node;
use odonata_base::domain::BoundType;
use odonata_base::infra::component::Component;
use odonata_base::infra::metric::{Event, Metrics};
use odonata_base::piece::MoveType;
use odonata_base::prelude::*;
use serde::{Deserialize, Serialize};
use strum_macros::EnumString;

use super::algo::Search;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, from = "LmrConfig", into = "LmrConfig")]
pub struct Lmr {
    pub cfg: LmrConfig,
    table:   Box<[[f32; 64]; 64]>,
}

impl Debug for Lmr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.cfg, f)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, EnumString)]
pub enum LmrDepthReductionStrategy {
    LogFormula,
    BinaryFormula,
}

impl LmrDepthReductionStrategy {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LmrConfig {
    pub enabled:           bool,
    depth_reduction_strat: LmrDepthReductionStrategy,
    pub table_intercept:   f32,
    pub table_gradient:    f32,
    pub table_aspect:      f32,
    first_move:            bool,
    fw_node:               bool,
    only_nt_all:           bool,
    bad_captures:          bool,
    pawns:                 bool,
    max_pawn_rank:         u8,
    killers:               bool,
    in_check:              bool,
    gives_check:           bool,
    discoverer:            bool,
    alpha_numeric:         bool,
    beta_numeric:          bool,
    extensions:            bool,
    reduce_pv:             f32,
    reduce_killer:         f32,
    reduce_bad_capture:    f32,
    reduce_hash:           f32,
    min_remaining_depth:   i32,
    iir:                   f32,
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

impl Configurable for Lmr {
    fn set(&mut self, p: Param) -> Result<bool> {
        self.cfg.enabled.set(p.get("enabled"))?;
        self.cfg.depth_reduction_strat.set(p.get("depth_reduction_strat"))?;
        let modified = self.cfg.table_intercept.set(p.get("table_intercept"))?
            || self.cfg.table_gradient.set(p.get("table_gradient"))?
            || self.cfg.table_aspect.set(p.get("table_aspect"))?;
        modified.then(|| self.regen_table());
        Ok(p.is_modified())
    }
}

impl From<Lmr> for LmrConfig {
    fn from(lmr: Lmr) -> Self {
        lmr.cfg
    }
}

impl From<LmrConfig> for Lmr {
    fn from(config: LmrConfig) -> Self {
        let mut me = Self {
            cfg:   config,
            table: Box::new([[0.0; 64]; 64]),
        };
        me.regen_table();
        me
    }
}

impl Lmr {
    pub fn regen_table(&mut self) {
        let log_formula = |depth: Ply, mv: usize| {
            self.cfg.table_intercept
                + f32::ln(depth as f32) * f32::ln(mv as f32 * self.cfg.table_aspect) * self.cfg.table_gradient
        };
        let binary_formula = |depth: Ply, mv: usize| if (depth - 1) * (mv as i32 - 2) < 80 { 0 } else { 1 } as f32;
        for depth in 1..64 {
            for mv in 2..64 {
                self.table[depth][mv] = match self.cfg.depth_reduction_strat {
                    LmrDepthReductionStrategy::LogFormula => log_formula(depth as Ply, mv),
                    LmrDepthReductionStrategy::BinaryFormula => binary_formula(depth as Ply, mv),
                };
            }
        }
    }
}

impl Default for Lmr {
    fn default() -> Self {
        Lmr::from(LmrConfig::default())
    }
}

impl Default for LmrConfig {
    fn default() -> Self {
        LmrConfig {
            enabled:               true,
            depth_reduction_strat: LmrDepthReductionStrategy::LogFormula,
            // table_intercept:       0.12,
            // table_gradient:        0.50,
            // table_aspect:          1.32,
            table_intercept:       0.000818,
            table_gradient:        0.508495,
            table_aspect:          1.466703,
            first_move:            false,
            fw_node:               true,
            only_nt_all:           false,
            alpha_numeric:         false,
            beta_numeric:          false,
            bad_captures:          true,
            pawns:                 true,
            max_pawn_rank:         6, // dont allow promos
            killers:               true,
            in_check:              false,
            gives_check:           false,
            discoverer:            false,
            extensions:            false,
            reduce_pv:             -0.97,
            reduce_killer:         -0.85,
            reduce_bad_capture:    0.0,
            reduce_hash:           0.0,
            min_remaining_depth:   0,
            iir:                   5.0,
        }
    }
}

impl Component for Lmr {
    fn new_game(&mut self) {}
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
impl Search {
    #[inline]
    pub fn lmr(
        &mut self,
        before: &Board,
        mv: Move,
        mv_num: u32,
        _quiets: i32,
        stage: MoveType,
        after: &Board,
        n: &Node,
        nt: BoundType,
        ext: Ply,
        _tt_mv: Move,
    ) -> Ply {
        if !self.lmr.cfg.enabled {
            return 0;
        }
        if n.is_qs() {
            return 0;
        }

        if !self.lmr.cfg.first_move && mv_num <= 1 {
            return 0;
        }

        if !self.lmr.cfg.fw_node && n.is_fw() {
            return 0;
        }

        if n.depth <= self.lmr.cfg.min_remaining_depth {
            return 0;
        }

        Metrics::incr_node(n, Event::LmrConsider);

        if mv.is_capture() {
            Metrics::incr_node(n, Event::LmrDeclineCapture);
            return 0;
        }

        let mut reduce = self.lmr.table[n.depth.min(63) as usize][mv_num.min(63) as usize];

        reduce += match stage {
            MoveType::BadCapture => self.lmr.cfg.reduce_bad_capture,
            MoveType::Killer => self.lmr.cfg.reduce_killer,
            MoveType::Hash => self.lmr.cfg.reduce_hash,
            _ => 0.0,
        };

        reduce += match n.is_fw() {
            true => self.lmr.cfg.reduce_pv,
            _ => 0.0,
        };

        if mv.mover_piece(before) == Piece::Pawn
            && mv.from().rank_number_as_white(before.color_us()) > self.lmr.cfg.max_pawn_rank as usize
        {
            Metrics::incr_node(n, Event::LmrDeclinePawnRank);
            reduce = 0.0;
        }

        // depth - lmr - 1 >= min_remaining_depth
        // => lmr <= depth - 1 - min_remaining_depth
        let reduce = (reduce as i32).clamp(0, n.depth - 1 - self.lmr.cfg.min_remaining_depth);

        if !self.lmr.cfg.pawns && mv.mover_piece(before) == Piece::Pawn {
            return 0;
        }
        if !self.lmr.cfg.killers && stage == MoveType::Killer
            || !self.lmr.cfg.bad_captures && stage == MoveType::BadCapture
        {
            Metrics::incr_node(n, Event::LmrDeclineKiller);
            return 0;
        }
        if self.lmr.cfg.only_nt_all && nt != BoundType::UpperAll {
            return 0;
        }
        if !self.lmr.cfg.extensions && ext > 0
            || !self.lmr.cfg.in_check && before.is_in_check(before.color_us())
            || !self.lmr.cfg.discoverer && before.maybe_gives_discovered_check(mv)
            ||
            // gives check a more precise and costly version of discoverers
            !self.lmr.cfg.gives_check && after.is_in_check(after.color_us())
        {
            return 0;
        }
        if self.lmr.cfg.alpha_numeric && !n.alpha.is_numeric() {
            return 0;
        }

        if self.lmr.cfg.beta_numeric && !n.beta.is_numeric() {
            return 0;
        }

        match reduce {
            1 => Metrics::incr_node(n, Event::LmrD1),
            2 => Metrics::incr_node(n, Event::LmrD2),
            3 => Metrics::incr_node(n, Event::LmrD3),
            4 => Metrics::incr_node(n, Event::LmrD4),
            5.. => Metrics::incr_node(n, Event::LmrDN),
            _ => {}
        }

        reduce
    }
}

impl fmt::Display for Lmr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", toml::to_string_pretty(self).unwrap())?;
        Ok(())
    }
}

#[cfg(test)]

mod tests {
    use test_log::test;

    use crate::search::engine::ThreadedSearch;

    #[test]
    fn test_lmr() {
        let lmr = ThreadedSearch::new().search.lmr;
        println!("lmr={}", lmr.table[20][21]);
    }
}
