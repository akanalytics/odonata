use std::fmt;

use odonata_base::domain::node::Node;
use odonata_base::infra::component::Component;
use odonata_base::mv::BareMove;
use odonata_base::prelude::*;

#[derive(Clone, Debug)]
pub struct Killers {
    enabled:                 bool,
    clear_every_move:        bool,
    clear_scores_every_move: bool,
    use_ply_below:           bool,
    castles:                 bool,
    captures:                bool,
    record_mover:            bool,
    order:                   bool,
    min_depth:               Ply,
    max_depth:               Ply,
    min_ply:                 Ply,
    max_ply:                 Ply,
    killers_by_ply:          Vec<[(BareMove, u32); 2]>,
}

impl Default for Killers {
    fn default() -> Self {
        Killers {
            enabled:                 true,
            clear_every_move:        false,
            clear_scores_every_move: true,
            use_ply_below:           true,
            castles:                 true,
            captures:                false,
            record_mover:            true,
            order:                   false,
            min_depth:               0,
            max_depth:               256,
            min_ply:                 0,
            max_ply:                 256,
            killers_by_ply:          vec![],
        }
    }
}

impl Configurable for Killers {
    fn set(&mut self, p: Param) -> Result<bool> {
        self.enabled.set(p.get("enabled"))?;
        self.clear_every_move.set(p.get("clear_every_move"))?;
        self.clear_scores_every_move.set(p.get("clear_scores_every_move"))?;
        self.use_ply_below.set(p.get("use_ply_below"))?;
        self.castles.set(p.get("castles"))?;
        self.captures.set(p.get("captures"))?;
        self.record_mover.set(p.get("record_mover"))?;
        self.order.set(p.get("order"))?;
        self.min_depth.set(p.get("min_depth"))?;
        self.max_depth.set(p.get("max_depth"))?;
        self.min_ply.set(p.get("min_ply"))?;
        self.max_ply.set(p.get("max_ply"))?;
        Ok(p.is_modified())
    }
}

impl Component for Killers {
    fn new_game(&mut self) {
        self.killers_by_ply.clear();
    }

    fn new_position(&mut self) {
        if self.clear_every_move {
            self.killers_by_ply.clear();
        } else if self.killers_by_ply.len() >= 2 {
            // reduce the ply count by 2
            self.killers_by_ply.remove(0);
            self.killers_by_ply.push([(BareMove::null(), 0); 2]);
            self.killers_by_ply.remove(0);
            self.killers_by_ply.push([(BareMove::null(), 0); 2]);
        }
        if self.clear_scores_every_move {
            for sm in self.killers_by_ply.iter_mut() {
                sm[0].1 = 0;
                sm[1].1 = 0;
            }
        }
    }
}

impl fmt::Display for Killers {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled                 : {}", self.enabled)?;
        writeln!(f, "clear_every_move        : {}", self.clear_every_move)?;
        writeln!(f, "clear_scores_every_move : {}", self.clear_scores_every_move)?;
        writeln!(f, "use_ply_below           : {}", self.use_ply_below)?;
        writeln!(f, "captures                : {}", self.captures)?;
        writeln!(f, "castles                 : {}", self.castles)?;
        writeln!(f, "record_mover            : {}", self.record_mover)?;
        writeln!(f, "order                   : {}", self.order)?;
        // writeln!(f, "{}", ArrayPlyStat(&[&self.dummy,]))?;
        Ok(())
    }
}

impl Killers {
    pub fn legal_moves_for(&self, n: &Node, b: &Board, moves: &mut MoveList) {
        if !self.enabled {
            return;
        }
        self.legal_moves_for_single_ply(n, n.ply, b, moves);
        if n.ply > 2 && self.use_ply_below {
            self.legal_moves_for_single_ply(n, n.ply - 2, b, moves);
        }
    }

    fn add_if_valid(&self, bm: BareMove, b: &Board, moves: &mut MoveList) {
        if bm.is_null() {
            return;
        }
        if let Ok(m) = bm.validate(b) {
            if !self.captures && m.is_capture() {
                return;
            }
            debug_assert!(b.validate().is_ok(), "board:{} is not valid", b);
            debug_assert!(
                b.legal_moves().iter().any(|&mv| mv == m),
                "board:{b:#} mv: {m} {m:?} is not in board.legal_moves {lm:#?}",
                lm = b.legal_moves()
            );
            moves.push(m);
        }
    }

    fn legal_moves_for_single_ply(&self, n: &Node, y: Ply, b: &Board, moves: &mut MoveList) {
        let y = y as usize;
        if self.order
            && (n.depth <= self.max_depth && n.depth >= self.min_depth
                || n.ply <= self.max_ply && n.ply >= self.min_ply)
            && y < self.killers_by_ply.len()
        {
            if self.killers_by_ply[y][1].1 > self.killers_by_ply[y][0].1 {
                self.add_if_valid(self.killers_by_ply[y][1].0, b, moves);
                self.add_if_valid(self.killers_by_ply[y][0].0, b, moves);
            } else {
                self.add_if_valid(self.killers_by_ply[y][0].0, b, moves);
                self.add_if_valid(self.killers_by_ply[y][1].0, b, moves);
            }
        }
    }
    pub fn store(&mut self, n: &Node, m: &Move, b: &Board) {
        // killers are quiet
        if !self.enabled || (m.is_castle(b) && !self.castles) || m.is_capture() || m.is_null() {
            return;
        }
        let y = n.ply as usize;
        if y >= self.killers_by_ply.len() {
            self.killers_by_ply.resize(y + 1, [(BareMove::null(), 0); 2]);
        }
        let mut bm = m.to_inner();
        if self.record_mover {
            bm.mover = Some(m.mover_piece(b))
        }
        // dont store duplicates
        if bm == self.killers_by_ply[y][0].0 {
            self.killers_by_ply[y][0].1 = self.killers_by_ply[y][0].1.saturating_add(1);
            return;
        }
        if bm == self.killers_by_ply[y][1].0 {
            self.killers_by_ply[y][0].1 = self.killers_by_ply[y][1].1.saturating_add(1);
            return;
        }
        // self.killers[y][3] = self.killers[y][2];
        // self.killers[y][2] = self.killers[y][1];
        if self.order
            && (n.depth <= self.max_depth && n.depth >= self.min_depth
                || n.ply <= self.max_ply && n.ply >= self.min_ply)
        {
            if self.killers_by_ply[y][0].1 > self.killers_by_ply[y][1].1 {
                self.killers_by_ply[y][1] = (bm, 0);
                self.killers_by_ply[y][0].1 = 0;
            } else {
                self.killers_by_ply[y][0] = (bm, 0);
                self.killers_by_ply[y][1].1 = 0;
            }
        } else {
            self.killers_by_ply[y][1] = self.killers_by_ply[y][0];
            self.killers_by_ply[y][0] = (bm, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use odonata_base::catalog::Catalog;
    use odonata_base::domain::timecontrol::TimeControl;
    use odonata_base::infra::utils::DecimalFormatter;
    use test_log::test;

    use crate::search::engine::ThreadedSearch;

    #[test]
    fn nodes_to_depth() {
        let positions = Catalog::example_game();
        let mut eng1 = ThreadedSearch::new();
        let mut eng2 = ThreadedSearch::new();
        // eng2.set_option("tt.enabled", "false").unwrap();
        eng2.search.killers.enabled = true;
        eng2.search.killers.use_ply_below = true; // 11.4%
        eng2.search.killers.clear_every_move = true;

        let tc = TimeControl::Depth(4);
        let mut nodes1 = 0.;
        let mut nodes2 = 0.;

        // example game has every move, black and white
        for (i, pos) in positions.iter().step_by(2).enumerate() {
            let sr1 = eng1.search(pos.clone(), tc.clone()).unwrap();
            let sr2 = eng2.search(pos.clone(), tc.clone()).unwrap();
            // metrics1.include(&sr1.metrics.unwrap());
            // metrics2.include(&sr2.metrics.unwrap());
            nodes1 += sr1.nodes as f64;
            nodes2 += sr2.nodes as f64;
            println!(
                "[{i}] nodes = {cmp} {sr1} {sr2}",
                cmp = sr1.nodes > sr2.nodes,
                sr1 = sr1.nodes,
                sr2 = sr2.nodes
            );
        }

        println!(
            "nodes = {nodes1} vs {nodes2} => eng2/eng1 improvement = {perc}%",
            perc = (100. - nodes2 * 100. / nodes1).dp(2)
        );

        let _metrics1 = eng1.metrics("").unwrap();
        let _metrics2 = eng2.metrics("").unwrap();
        // #[derive(Tabled)]
        // struct SideBySide {
        //     left:  String,
        //     right: String,
        // }

        // let sbs = SideBySide {
        //     left:  metrics1.to_string(),
        //     right: metrics2.to_string(),
        // };

        // println!("{}", Table::new([&sbs]).with(Style::markdown()));
    }
}
