use crate::{boards::Board, infra::component::Component};
// use crate::{debug, logger::LogInit};
use crate::{
    movelist::MoveList,
    mv::Move,
    piece::{Ply, MAX_PLY},
};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Killers {
    enabled:          bool,
    clear_every_move: bool,
    use_ply_below:    bool,
    castles:    bool,

    #[serde(skip)]
    killers: Vec<[Move; 2]>,
}

impl Default for Killers {
    fn default() -> Self {
        Killers {
            enabled:          true,
            clear_every_move: false,
            use_ply_below:    true,
            castles:          false,
            killers:          vec![[Move::new_null(); 2]; MAX_PLY as usize],
        }
    }
}

impl Component for Killers {
    fn new_game(&mut self) {
        self.killers.fill([Move::new_null(); 2]);
    }

    fn new_position(&mut self) {
        if self.clear_every_move {
            self.killers.fill([Move::new_null(); 2]);
        } else {
            // reduce the ply count by 2
            self.killers.remove(0);
            self.killers.push([Move::new_null(); 2]);
            self.killers.remove(0);
            self.killers.push([Move::new_null(); 2]);
        }
    }
}

impl fmt::Display for Killers {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "clear.every.move : {}", self.clear_every_move)?;
        writeln!(f, "use.ply.below    : {}", self.use_ply_below)?;
        // writeln!(f, "{}", ArrayPlyStat(&[&self.dummy,]))?;
        Ok(())
    }
}

impl Killers {
    pub fn legal_moves_for(&self, y: Ply, b: &Board, moves: &mut MoveList) {
        if !self.enabled {
            return;
        }
        self.legal_moves_for_single_ply(y, b, moves);
        if y > 2 && self.use_ply_below {
            self.legal_moves_for_single_ply(y - 2, b, moves);
        }
    }

    fn legal_moves_for_single_ply(&self, y: Ply, b: &Board, moves: &mut MoveList) {
        for m in self.killers[y as usize].iter() {
            if !m.is_null() && b.is_pseudo_legal_and_legal_move(*m) {
                debug_assert!(b.validate().is_ok(), "board:{} is not valid", b);
                debug_assert!(
                    b.legal_moves().iter().any(|mv| mv == m),
                    "board:{b:#} mv: {m} {m:?} is not in board.legal_moves {lm:#?}",
                    lm = b.legal_moves()
                );
                moves.push(*m);
            }
        }
    }

    pub fn store(&mut self, y: Ply, m: &Move, b: &Board) {
        // killers are quiet
        if !self.enabled || (m.is_castle(b) && !self.castles) || m.is_capture() {
            return;
        }
        // dont store duplicates
        if m == &self.killers[y as usize][0] || m == &self.killers[y as usize][1] {
            return;
        }
        // self.killers[y as usize][3] = self.killers[y as usize][2];
        // self.killers[y as usize][2] = self.killers[y as usize][1];
        self.killers[y as usize][1] = self.killers[y as usize][0];
        self.killers[y as usize][0] = *m;
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        catalog::Catalog,
        domain::engine::Engine,
        infra::metric::Metrics,
        search::{engine::ThreadedSearch, timecontrol::TimeControl},
    };

    use crate::infra::utils::DecimalFormatter;

    // use tabled::{Style, Table, Tabled};
    use test_log::test;

    #[test]
    fn nodes_to_depth() {
        let positions = Catalog::example_game();
        let mut eng1 = ThreadedSearch::new();
        let mut eng2 = ThreadedSearch::new();
        // eng2.set_option("tt.enabled", "false").unwrap();
        eng2.set_option("killers.enabled", "true").unwrap();
        eng2.set_option("killers.use_ply_below", "true").unwrap(); // 11.4%
        eng2.set_option("killers.clear_every_move", "true")
            .unwrap();

        let tc = TimeControl::Depth(4);
        let mut nodes1 = 0.;
        let mut nodes2 = 0.;

        let mut metrics1 = Metrics::new();
        let mut metrics2 = Metrics::new();

        // example game has every move, black and white
        for (i, pos) in positions.iter().step_by(2).enumerate() {
            let sr1 = eng1.search(pos.clone(), tc).unwrap();
            let sr2 = eng2.search(pos.clone(), tc).unwrap();
            metrics1.include(&sr1.metrics.unwrap());
            metrics2.include(&sr2.metrics.unwrap());
            nodes1 += sr1.nodes as f64;
            nodes2 += sr2.nodes as f64;
            println!("[{i}] nodes = {cmp} {sr1} {sr2}", cmp = sr1.nodes > sr2.nodes, sr1 = sr1.nodes, sr2 = sr2.nodes);
        }

        println!(
            "nodes = {nodes1} vs {nodes2} => eng2/eng1 improvement = {perc}%",
            perc = (100. - nodes2 * 100. / nodes1).dp(2)
        );

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
