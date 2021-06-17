use crate::board::Board;
use crate::config::{Config, Component};
use crate::{debug, logger::LogInit};
use crate::movelist::MoveList;
use crate::variation::Variation;
use crate::mv::Move;
use crate::search::algo::Algo;
use crate::search::stack::Stack;
use crate::stat::{ArrayPlyStat, PlyStat};
use crate::types::Ply;
use std::fmt;

#[derive(Clone, Debug)]
pub struct MoveOrderer {
    pub enabled: bool,
    pub prior_pv: bool,
    pub prior_bm: bool,
    pub tt_bm: bool,
    pub mvv_lva: bool,
    pub order: String,
    pub thread: u32,
    count_pv: PlyStat,
    count_bm: PlyStat,
    count_tt_bm: PlyStat,
    picker: Stack<MovePicker>,
}

impl Component for MoveOrderer {
    fn settings(&self, c: &mut Config) {
        c.set("move_orderer.enabled", "type check default true");
        c.set("move_orderer.prior_pv", "type check default true");
        c.set("move_orderer.prior_bm", "type check default false");
        c.set("move_orderer.tt_bm", "type check default true");
        c.set("move_orderer.mvv_lva", "type check default true");
    }
    fn configure(&mut self, c: &Config) {
        debug!("move_orderer.configure with {}", c);
        self.enabled = c.bool("move_orderer.enabled").unwrap_or(self.enabled);
        self.prior_bm = c.bool("move_orderer.prior_bm").unwrap_or(self.prior_bm);
        self.prior_pv = c.bool("move_orderer.prior_pv").unwrap_or(self.prior_pv);
        self.tt_bm = c.bool("move_orderer.tt_bm").unwrap_or(self.tt_bm);
        self.mvv_lva = c.bool("move_orderer.mvv_lva").unwrap_or(self.mvv_lva);
        self.order = c.string("move_orderer.order").unwrap_or(self.order.clone());
    }

    fn new_game(&mut self) {
        self.picker.clear();
    }

    fn new_search(&mut self) {
        self.picker.clear();
    }

}

impl MoveOrderer {
    // pub const MOVE_SORTS: &'static [&'static str] = &["Natural", "PV from Prior Iteration", "MVV/LVA"];
}

impl Default for MoveOrderer {
    fn default() -> Self {
        MoveOrderer {
            enabled: true,
            prior_pv: true,
            prior_bm: false,
            tt_bm: true,
            mvv_lva: true,
            order: "SICKQE".to_string(),
            thread: 0,
            count_pv: PlyStat::new("order pv"),
            count_bm: PlyStat::new("order bm"),
            count_tt_bm: PlyStat::new("order tt bm"),
            picker: Stack::<MovePicker>::default(),
        }
    }
}

impl fmt::Display for MoveOrderer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "prior pv         : {}", self.prior_pv)?;
        writeln!(f, "prior bm         : {}", self.prior_bm)?;
        writeln!(f, "tt bm            : {}", self.tt_bm)?;
        writeln!(f, "mvv_lva          : {}", self.mvv_lva)?;
        writeln!(f, "order            : {}", self.order)?;
        writeln!(f, "thread           : {}", self.thread)?;
        writeln!(
            f,
            "{}",
            ArrayPlyStat(&[&self.count_pv, &self.count_bm, &self.count_tt_bm])
        )?;
        Ok(())
    }
}

impl Algo {
    pub fn order_moves(&mut self, ply: Ply, movelist: &mut MoveList, tt_mv: &Option<Move>) {
        if !self.move_orderer.enabled {
            return;
        }

        if self.move_orderer.mvv_lva {
            // movelist.sort_unstable_by_key(|m| -m.mvv_lva_score() );
            movelist.sort_unstable_by_key(Move::mvv_lva_score );
            movelist.reverse();
            if self.move_orderer.thread == 1 && movelist.len() >= 2 {
                movelist.swap(0, 1);
            }
        }

        if self.move_orderer.prior_pv {
            if Self::order_from_prior_pv(movelist, &self.current_variation, self.pv()) {
                self.move_orderer.count_pv.add(ply, 1);
            }
        }
        if self.move_orderer.prior_bm {
            if ply == 0 {
                let i = movelist.iter().position(|&mv| mv == self.bm());
                if let Some(i) = i {
                    movelist.swap(0, i);
                    self.move_orderer.count_bm.add(ply, 1);
                    return;
                }
            }
        }
        if self.move_orderer.tt_bm {
            if let Some(tt_bm) = tt_mv {
                let i = movelist.iter().position(|&mv| mv == *tt_bm);
                if let Some(i) = i {
                    movelist.swap(0, i);
                    self.move_orderer.count_tt_bm.add(ply, 1);
                    return;
                }
            }
        }
    }

    pub fn order_from_prior_pv(moves: &mut MoveList, var: &Variation, pv: &Variation) -> bool {
        // println!( "{:>30} / [{:>30}] - {}",var.to_string(), pv.to_string(), moves  );
        if pv.len() == 0 {
            return false;
        }
        if var.len() >= pv.len() {
            return false; // we're already exploring beyond what we have pv for
        }

        if pv.starts_with(var) {
            let best = pv[var.len()];
            let j = moves.iter().position(|mv| mv == &best);
            if let Some(j) = j {
                moves.swap(0, j);
                return true;
            }
            //println!("{:>30} / [{:>30}] ==> {}", var.to_string(), pv.to_string(), moves);
            return false;
        }
        return false;
    }

    // pub fn order_by_mvv_lva(moves: &mut MoveList) {
}

impl Algo {
    pub fn generate_moves(&mut self, ply: Ply, _b: &Board) {
        self.move_orderer.picker[ply] = MovePicker {
            captures: false,
            stage: 0,
            moves: MoveList::new(),
            all_moves: MoveList::new(),
            index: 0,
        }
    }

    pub fn get_next_move(&mut self, ply: Ply, b: &Board) -> Option<Move> {
        let pick = &mut self.move_orderer.picker[ply];
        if pick.index < pick.moves.len() {
            pick.index += 1;
            return Some(pick.moves[pick.index - 1]);
        }
        if pick.stage as usize + 1 >= self.move_orderer.order.len() {
            return None;
        } else {
            pick.index = 0;
            pick.stage += 1;
            let mut moves = MoveList::new();
            self.gen(ply, b, &mut moves);
            self.order_moves(ply, &mut moves, &None);
            self.move_orderer.picker[ply].moves = moves;
            return self.get_next_move(ply, b);
        }
    }
}

// #[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Debug)]
// pub enum MoveGenStage {
//     Start,
//     Init,
//     Killers,
//     Legals,
//     Complete,
// }

// impl Default for MoveGenStage {
//     fn default() -> Self {
//         Self::Start
//     }
// }

// impl MoveGenStage {
//     pub fn next_stage(&self) -> MoveGenStage {
//         match self {
//             Self::Start => Self::Init,
//             Self::Init => Self::Killers,
//             Self::Hash => Self::Killers,
//             Self::Killers => Self::Legals,
//             Self::Legals => Self::Complete,
//             _ => unreachable!(),
//         }
//     }
// }

// uses Move Orderer and MoveGen to present a sequence of moves
#[derive(Clone, Debug, Default)]
pub struct MovePicker {
    captures: bool,
    stage: u8,
    moves: MoveList,
    all_moves: MoveList,
    index: usize,
}

impl Algo {
    fn gen(&mut self, ply: Ply, b: &Board, moves: &mut MoveList) {
        let pick = &mut self.move_orderer.picker[ply];
        // pick.moves.clear();
        // println!("{}", self.move_orderer.order.chars().nth(pick.stage as usize).unwrap());
        match self.move_orderer.order.chars().nth(pick.stage as usize).unwrap() {
            'S' => {}
            // Init
            'I' => {
                let all_moves = &mut self.move_orderer.picker[ply].all_moves;
                b.legal_moves_into(all_moves);
            }
            // Captures
            'C' => self.move_orderer.picker[ply]
                .all_moves
                .iter()
                .filter(|m| Move::is_capture(m))
                .for_each(|&m| moves.push(m)),
            // Killers
            'K' => self.killers.legal_moves_for(ply, b, moves),

            // Promos
            'P' => self.move_orderer.picker[ply]
                .all_moves
                .iter()
                .filter(|m| !Move::is_promo(m))
                .for_each(|&m| moves.push(m)),
            // Quiets
            'Q' => self.move_orderer.picker[ply]
                .all_moves
                .iter()
                .filter(|m| !Move::is_capture(m))
                .for_each(|&m| moves.push(m)),
            // Remaining
            'R' => {self.move_orderer.picker[ply]
                    .all_moves
                    .iter()
                    .for_each(|&m| moves.push(m))}
            // End
            'E' => {}

            _ => { unreachable!("unknown move order stage")}
        };
        self.move_orderer.picker[ply].all_moves.retain(|m| !moves.contains(m));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::globals::constants::*;
    use crate::movelist::*;
    use crate::search::timecontrol::*;

    #[test]
    fn test_prior_pv() {
        let a1a2 = Move {
            from: a1.square(),
            to: a2.square(),
            ..Default::default()
        };
        let a1a3 = Move {
            from: a1.square(),
            to: a3.square(),
            ..Default::default()
        };
        let a1a4 = Move {
            from: a1.square(),
            to: a4.square(),
            ..Default::default()
        };
        let b1a2 = Move {
            from: b1.square(),
            to: a2.square(),
            ..Default::default()
        };
        let b1a3 = Move {
            from: b1.square(),
            to: a3.square(),
            ..Default::default()
        };
        let b1a4 = Move {
            from: b1.square(),
            to: a4.square(),
            ..Default::default()
        };
        let c1c2 = Move {
            from: c1.square(),
            to: c2.square(),
            ..Default::default()
        };

        let mut moves_orig = MoveList::new();
        moves_orig.extend(vec![b1a2, b1a3, b1a4, a1a3, a1a4, a1a2]);
        assert_eq!(moves_orig.to_string(), "b1a2, b1a3, b1a4, a1a3, a1a4, a1a2");

        let mut pv = Variation::new();
        pv.extend(vec![a1a2, a1a3, a1a4]);

        // if variation = empty, and pv = empty, no ordering
        let variation = Variation::new();
        let mut moves = moves_orig.clone();
        Algo::order_from_prior_pv(&mut moves, &variation, &variation);
        assert_eq!(moves.to_string(), "b1a2, b1a3, b1a4, a1a3, a1a4, a1a2");

        // if variation = empty, use pv[0] if it exists
        let mut variation = Variation::new();
        let mut moves = moves_orig.clone();
        Algo::order_from_prior_pv(&mut moves, &variation, &pv);
        assert_eq!(moves.to_string(), "a1a2, b1a3, b1a4, a1a3, a1a4, b1a2");

        // variation strays from PV - no reordering
        // pv = a1a2 a1a3 a1a4   var = a1a2 c1c2
        let mut moves = moves_orig.clone();
        variation.extend(vec![a1a2, c1c2]);
        Algo::order_from_prior_pv(&mut moves, &variation, &pv);
        assert_eq!(moves_orig.to_string(), "b1a2, b1a3, b1a4, a1a3, a1a4, a1a2");

        // variation strays from PV - no reordering
        // pv = a1a2 a1a3 a1a4   var = a1a2 a1a3 c1c2
        let mut variation = Variation::new();
        variation.extend(vec![a1a2, a1a3, c1c2]);
        let mut moves = moves_orig.clone();
        Algo::order_from_prior_pv(&mut moves, &variation, &pv);
        assert_eq!(moves.to_string(), "b1a2, b1a3, b1a4, a1a3, a1a4, a1a2");

        // if variation = pv, no movelist ordering
        let mut moves = moves_orig.clone();
        Algo::order_from_prior_pv(&mut moves, &pv, &pv);

        // variation is PV[0] - use PV[1]
        // pv = a1a2 a1a3 a1a4   var = a1a2
        let mut variation = Variation::new();
        variation.extend(vec![a1a2]);
        let mut moves = moves_orig.clone();
        Algo::order_from_prior_pv(&mut moves, &variation, &pv);
        assert_eq!(moves.to_string(), "a1a3, b1a3, b1a4, b1a2, a1a4, a1a2");

        // variation is PV[0] and [1] - use PV[2]
        // pv = a1a2 a1a3 a1a4   var = a1a2 a1a3
        let mut moves = moves_orig.clone();
        let mut variation = Variation::new();
        variation.extend(vec![a1a2, a1a3]);
        Algo::order_from_prior_pv(&mut moves, &variation, &pv);
        assert_eq!(moves.to_string(), "a1a4, b1a3, b1a4, a1a3, b1a2, a1a2");
    }

    #[test]
    fn test_ordering_mvv_lva() {
        let board = Catalog::perft_kiwipete().0;
        let mut moves = board.legal_moves();
        moves.sort(); // alphabetical first
        Algo::new().order_moves(0, &mut moves, &None);
        println!("{:#}", moves);
        assert_eq!(moves[0].uci(), "e2a6"); // b x b
        assert_eq!(moves[1].uci(), "f3f6"); // q x n
        assert_eq!(moves[2].uci(), "d5e6"); // p x p
        assert_eq!(moves[3].uci(), "g2h3"); // p x p
        assert_eq!(moves[7].uci(), "f3h3"); // q x p

        let positions = Catalog::move_ordering();
        for (i, pos) in positions.iter().enumerate() {
            let mut moves = pos.board().legal_moves();
            Algo::new().order_moves(0, &mut moves, &None);
            println!("{}\n{:#}", pos, moves);
            if i == 0 {
                assert_eq!(moves[0].uci(), "b7a8q"); // p x r = Q)
                assert_eq!(moves[1].uci(), "b7a8r"); // p x r = R)
                assert_eq!(moves[2].uci(), "b7b8q"); // p  = Q)
                assert_eq!(moves[3].uci(), "b7a8b"); // p x r  = B)
            }
        }
    }

    #[test]
    fn test_ordering() {
        let position = &Catalog::mate_in_2()[0];
        let mut algo = Algo::new()
            .set_timing_method(TimeControl::Depth(3))
            .set_qsearch(false)
            .build();
        algo.tt.mb = 1;
        algo.move_orderer.enabled = false;
        algo.move_orderer.prior_bm = false;
        algo.move_orderer.prior_pv = false;
        algo.search(position.board());
        assert_eq!(algo.move_orderer.count_bm.get(0), 0, "{}", algo);
        assert_eq!(algo.move_orderer.count_pv.get(1), 0, "{}", algo);

        algo.move_orderer.enabled = true;
        algo.move_orderer.prior_bm = true;
        algo.move_orderer.prior_pv = false;
        algo.new_game();
        algo.search(position.board());
        // 2 is depth 1, plus depth 2,
        assert_eq!(algo.move_orderer.count_bm.get(0), 2, "{}", algo);
        assert_eq!(algo.move_orderer.count_bm.get(1), 0, "{}", algo); // bm only variation of lenght 1

        algo.move_orderer.mvv_lva = false;
        algo.move_orderer.prior_bm = false;
        algo.move_orderer.prior_pv = true;
        // algo.tt.enabled = false;
        algo.new_game();
        algo.search(position.board());
        println!("{}", algo);
        // 2 = pv[0] plus pv[0..1] used once
        assert_eq!(algo.move_orderer.count_pv.get(0), 2, "{}", algo);
        assert_eq!(algo.move_orderer.count_pv.get(1), 1, "{}", algo);
    }

    #[test]
    fn test_gen_moves() {
        let board = Catalog::starting_position();
        let mut algo = Algo::new();
        algo.generate_moves(0, &board);

        let iter = std::iter::from_fn(move || algo.get_next_move(0, &board));
        let moves: Vec<Move> = iter.collect();
        println!("Moves {:?}", moves);
    }
}
