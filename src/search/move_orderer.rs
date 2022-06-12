use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::infra::component::Component;
use crate::infra::metric::Metric;
use crate::movelist::MoveList;
use crate::mv::Move;
use crate::phaser::Phase;
use crate::search::algo::Algo;
use crate::search::stack::Stack;
use crate::trace::stat::{ArrayPlyStat, PlyStat};
use crate::types::{Color, MoveType, Piece, Ply};
use crate::variation::Variation;
use crate::{Bitboard, PreCalc};
use serde::{Deserialize, Serialize};
use std::fmt;

use super::node::{Node, Event, Timing};
use crate::eval::score::{Score, ToScore};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MoveOrderer {
    pub enabled: bool,
    pub prior_pv: bool,
    pub prior_bm: bool,
    pub tt_bm: bool,
    pub mvv_lva: bool,
    pub discovered_checks: bool,
    pub discovered_check_sort_bonus: f32,
    pub rank_index_sort_bonus: f32,
    pub is_counter_move_sort_bonus: f32,
    pub has_counter_move_sort_bonus: f32,
    pub knight_pseudo_mobility: f32,
    pub bishop_pseudo_mobility: f32,
    pub queen_pseudo_mobility: f32,
    pub promo_sort_bonus: f32,
    pub promo_queen_sort_bonus: f32,
    pub promo_knight_sort_bonus: f32,
    pub castle_sort_bonus: f32,
    pub pst_sort_factor: f32,
    pub hh_sort_factor: f32,
    pub see_cutoff: Score,
    pub qsearch_see_cutoff: Score,
    pub order: Vec<MoveType>,
    pub qorder: Vec<MoveType>,
    pub qorder_evasions: Vec<MoveType>,

    #[serde(skip)]
    pub thread: u32,

    #[serde(skip)]
    count_pv: PlyStat,

    #[serde(skip)]
    count_bm: PlyStat,

    #[serde(skip)]
    count_tt_bm: PlyStat,

    #[serde(skip)]
    picker: Stack<OrderedMoveList>,
}

impl Component for MoveOrderer {
    fn new_game(&mut self) {
        self.picker.clear();
    }

    fn new_position(&mut self) {
        self.picker.clear();
        self.thread = 0;
        self.count_pv = PlyStat::new("order pv");
        self.count_bm = PlyStat::new("order bm");
        self.count_tt_bm = PlyStat::new("order tt bm");
    }
}

impl MoveOrderer {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for MoveOrderer {
    fn default() -> Self {
        MoveOrderer {
            enabled: true,
            prior_pv: true,
            prior_bm: false,
            discovered_checks: true,
            tt_bm: true,
            mvv_lva: true,
            see_cutoff: 0.cp(),
            discovered_check_sort_bonus: 0.0,
            rank_index_sort_bonus: 0.0,
            knight_pseudo_mobility: 0.0,
            bishop_pseudo_mobility: 0.0,
            queen_pseudo_mobility: 0.0,
            is_counter_move_sort_bonus: 1000.0,
            has_counter_move_sort_bonus: 900.0,
            promo_sort_bonus: 1200.0,
            promo_queen_sort_bonus: 500.0,
            promo_knight_sort_bonus: 500.0,
            castle_sort_bonus: 500.0,
            pst_sort_factor: 1.0,
            hh_sort_factor: 5.0,
            qsearch_see_cutoff: 1.cp(),
            thread: 0,
            count_pv: PlyStat::new("order pv"),
            count_bm: PlyStat::new("order bm"),
            count_tt_bm: PlyStat::new("order tt bm"),
            picker: Stack::<OrderedMoveList>::default(),
            order: MoveType::vec_from_string("SHIGKPqBE").unwrap(), // , SHICKPQE, SHIGKPQBE
            qorder: MoveType::vec_from_string("SIgE").unwrap(),     //
            qorder_evasions: MoveType::vec_from_string("SIVE").unwrap(), //
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
        writeln!(f, "see_cutoff       : {}", self.see_cutoff)?;
        writeln!(f, "qs_see_cutoff    : {}", self.qsearch_see_cutoff)?;
        writeln!(
            f,
            "order            : {}",
            MoveType::slice_to_string(&self.order)
        )?;
        writeln!(
            f,
            "qorder           : {}",
            MoveType::slice_to_string(&self.qorder)
        )?;
        writeln!(f, "thread           : {}", self.thread)?;
        writeln!(
            f,
            "{}",
            ArrayPlyStat(&[&self.count_pv, &self.count_bm, &self.count_tt_bm])
        )?;
        Ok(())
    }
}

impl MoveOrderer {
    // #[inline]
    // pub fn quiet_score_old(
    //     &self,
    //     n: &Node,
    //     mv: Move,
    //     algo: &Algo,
    //     phase: Phase,
    //     c: Color,
    //     _parent: Move,
    // ) -> i32 {
    //     let mut score = 0;

    //     if mv.is_promo() {
    //         if mv.promo_piece() == Piece::Knight {
    //             score += 1500;
    //         } else {
    //             score += 2000;
    //         }
    //     }
    //     if mv.is_castle() {
    //         score += 1000;
    //     }
    //     // if mv.mover_piece() == Piece::Pawn {
    //     //     score += 0;
    //     // }
    //     // score += mv.mover_piece().centipawns();
    //     if c == Color::White {
    //         score += mv.to().rank_index() as i32;
    //     } else {
    //         score -= mv.to().rank_index() as i32;
    //     }

    //     score += algo.history.history_heuristic_bonus(c, &mv, n);

    //     score += algo
    //         .eval
    //         .pst
    //         .w_eval_square(c, mv.mover_piece(), mv.to())
    //         .interpolate(phase) as i32;
    //     // score -= algo.eval.w_eval_square(c, mv.mover_piece(), mv.from()).interpolate(phase);
    //     -score
    // }

    // # PLAYER                  :  RATING  ERROR  POINTS  PLAYED   (%)  CFS(%)    W    D    L  D(%)
    // 1 0.5.65:mohsf=10000      :      67     26   633.5    1044  60.7     100  549  169  326  16.2
    // 2 0.5.65:moicmsb=10000    :       0   ----   522.5    1042  50.1      55  430  185  427  17.8
    // 3 0.5.65:morisb=10000     :      -2     26   519.0    1041  49.9      77  433  172  436  16.5
    // 4 0.5.65:mokpm=10000      :     -11     25   504.5    1042  48.4      63  416  177  449  17.0
    // 5 0.5.65:modcsb=10000     :     -15     25   497.0    1041  47.7      70  415  164  462  15.8
    // 6 0.5.65:moqpm=10000      :     -22     26   485.5    1042  46.6      51  394  183  465  17.6
    // 7 0.5.65:                 :     -23     26   485.0    1042  46.5     ---  398  174  470  16.7

    #[inline]
    pub fn quiet_score(
        &self,
        n: &Node,
        mv: Move,
        algo: &Algo,
        phase: Phase,
        c: Color,
        b: &Board,
        parent: Move,
    ) -> i32 {
        let mut score = 0.0;
        if mv.is_promo() {
            score += self.promo_sort_bonus;
            if mv.promo_piece() == Piece::Queen {
                score += self.promo_queen_sort_bonus;
            }
            if mv.promo_piece() == Piece::Knight {
                score += self.promo_knight_sort_bonus;
            }
        }
        if mv.is_castle() {
            score += self.castle_sort_bonus;
        }

        // if mv.from().is_in(b.discoverer(b.color_them())) {
        //     score += self.discovered_check_sort_bonus;
        // }

        if b.make_move(&mv).is_in_check(c.opposite()) {
            score += self.discovered_check_sort_bonus;
        }

        // pseudo mobility sort-scoring for knights (easiest).
        // the sort-score is just used to order moves.
        //
        // Formula: (count of attacked squares in move-to) less (count of attacked squares in move-from)
        // multiplied by a knight_pseudo_mobility factor (say 1000)
        //
        // so moving a knight from a1 to b3 scores 1000 * (6 - 2) = +4000
        //
        // for comparison other sort-scoring factors/bonuses are
        // a. promo +2100,
        // b. castling +1200,
        // c. move is a counter-move to opponent's move +1200
        //
        if mv.mover_piece() == Piece::Knight {
            score += self.knight_pseudo_mobility
                * (PreCalc::default().knight_attacks(mv.to()).popcount()
                    - PreCalc::default().knight_attacks(mv.from()).popcount())
                    as f32
        }
        if mv.mover_piece() == Piece::Bishop {
            score += self.bishop_pseudo_mobility
                * (PreCalc::default()
                    .bishop_attacks(Bitboard::EMPTY, mv.to())
                    .popcount()
                    - PreCalc::default()
                        .bishop_attacks(Bitboard::EMPTY, mv.from())
                        .popcount()) as f32
        }
        if mv.mover_piece() == Piece::Queen {
            score += self.queen_pseudo_mobility
                * (PreCalc::default()
                    .bishop_attacks(Bitboard::EMPTY, mv.to())
                    .popcount()
                    - PreCalc::default()
                        .bishop_attacks(Bitboard::EMPTY, mv.from())
                        .popcount()) as f32
        }
        // if mv.mover_piece() == Piece::Pawn {
        //     score += 0;
        // }
        // score += mv.mover_piece().centipawns();
        score += self.rank_index_sort_bonus * mv.to().rank_number_as_white(c) as f32;

        score += self.hh_sort_factor * algo.history.history_heuristic_bonus(c, &mv, n) as f32;

        let cm = algo.counter_move.counter_move_unchecked(c, parent, mv, n);
        if cm > 0 {
            Metric::incr_node(n, Event::MoveSortCounterMove);
            score += cm as f32 * self.is_counter_move_sort_bonus;
        }
        // if self.has_counter_move_sort_bonus > -10000.0
        //     && algo
        //         .counter_move
        //         .counter_move_unchecked(c.opposite(), mv)
        // {
        //     score += self.has_counter_move_sort_bonus;
        // }

        score += self.pst_sort_factor
            * algo
                .eval
                .pst
                .w_eval_square(c, mv.mover_piece(), mv.to())
                .interpolate(phase);
        // score -= algo.eval.w_eval_square(c, mv.mover_piece(), mv.from()).interpolate(phase);
        -score as i32
    }
}

impl Algo {
    pub fn order_moves(&mut self, ply: Ply, movelist: &mut MoveList, tt_mv: &Option<Move>) {
        if !self.move_orderer.enabled {
            return;
        }

        if self.move_orderer.mvv_lva {
            // movelist.sort_unstable_by_key(|m| -m.mvv_lva_score() );
            movelist.sort_unstable_by_key(Move::mvv_lva_score);
            movelist.reverse();
            // if self.move_orderer.thread == 1 && movelist.len() >= 2 {
            //     movelist.swap(0, 1);
            // }
        }

        if self.move_orderer.prior_pv
            && Self::order_from_prior_pv(movelist, &self.current_variation, self.pv())
        {
            self.move_orderer.count_pv.add(ply, 1);
        }
        // if self.move_orderer.prior_bm {
        //     if ply == 0 {
        //         let i = movelist.iter().position(|&mv| mv == self.bm());
        //         if let Some(i) = i {
        //             movelist.swap(0, i);
        //             self.move_orderer.count_bm.add(ply, 1);
        //             return;
        //         }
        //     }
        // }
        if self.move_orderer.tt_bm {
            if let Some(tt_bm) = tt_mv {
                let i = movelist.iter().position(|&mv| mv == *tt_bm);
                if let Some(i) = i {
                    movelist.swap(0, i);
                    self.move_orderer.count_tt_bm.add(ply, 1);
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
                if j != 0 {
                    moves.swap(0, j);
                    return true;
                }
            }
            //println!("{:>30} / [{:>30}] ==> {}", var.to_string(), pv.to_string(), moves);
            return false;
        }
        false
    }

    // pub fn order_by_mvv_lva(moves: &mut MoveList) {
}

// uses Move Orderer and MoveGen to present a sequence of moves
#[derive(Clone, Debug, Default)]
pub struct OrderedMoveList {
    qsearch: bool,
    is_in_check: bool,
    stage: usize,
    moves: MoveList,
    all_moves: MoveList,
    bad_captures: MoveList,
    index: usize,
    tt: Move,
    last: Move,
    n: Node,
}

impl MoveOrderer {
    pub fn create_sorted_moves(&self, n: Node, b: &Board, tt: Move, last: Move) -> OrderedMoveList {
        OrderedMoveList {
            qsearch: n.is_qs(),
            is_in_check: b.is_in_check(b.color_us()),
            stage: 0,
            moves: MoveList::new(),
            all_moves: MoveList::new(),
            // good_captures: MoveList::new(),
            bad_captures: MoveList::new(),
            index: 0,
            n,
            tt,
            last,
        }
    }
}

impl OrderedMoveList {
    fn ordering<'a>(&self, algo: &'a Algo) -> &'a Vec<MoveType> {
        if self.qsearch {
            if self.is_in_check {
                &algo.move_orderer.qorder_evasions
            } else {
                &algo.move_orderer.qorder
            }
        } else {
            &algo.move_orderer.order
        }
    }

    pub fn next_move(&mut self, b: &Board, algo: &mut Algo) -> Option<(MoveType, Move)> {
        let t = Metric::timing_start();
        let m = self.calc_next_move_(b, algo);
        Metric::profile(t, Timing::TimingSortMoves);

        m
    }

    fn calc_next_move_(&mut self, b: &Board, algo: &mut Algo) -> Option<(MoveType, Move)> {
        let move_type = self.ordering(algo)[self.stage];
        if self.index < self.moves.len() {
            if move_type == MoveType::GoodCapture || move_type == MoveType::Capture
            // we dont sort killers
            // || move_type == 'b' as b is sorted by reverse anyway due to push and they are bad captures
            {
                Self::sort_one_capture_move(self.index, &mut self.moves, self.last);
            }
            if move_type == MoveType::GoodCaptureUpfrontSorted || move_type == MoveType::GoodCapture
            {
                let mv = self.moves[self.index];
                Metric::incr_node(&self.n, Event::EvalSee);
                let see = algo.eval.see.eval_move_see(b, mv);
                let see_cutoff = if self.qsearch {
                    algo.move_orderer.qsearch_see_cutoff
                } else {
                    algo.move_orderer.see_cutoff
                };
                let see_cutoff = see_cutoff.as_i16() as i32;

                if see < see_cutoff || see == see_cutoff && self.qsearch && self.n.depth < -1 {
                    if !(algo.move_orderer.discovered_checks
                        && b.maybe_gives_discovered_check(mv))
                    {
                        self.bad_captures.push(mv);
                        self.index += 1;
                        return self.calc_next_move_(b, algo);
                    }
                }
            }
            let some = Some((move_type, self.moves[self.index]));
            self.index += 1;
            return some;
        }
        if self.stage as usize + 1 >= self.ordering(algo).len() {
            None
        } else {
            self.index = 0;
            self.stage += 1;
            self.gen(b, algo);
            self.calc_next_move_(b, algo)
        }
    }

    #[inline]
    fn sort_one_capture_move(i: usize, moves: &mut MoveList, last: Move) {
        if let Some(j) = moves
            .iter()
            .enumerate()
            .skip(i)
            .max_by_key(|(_n, &mv)| mv.mvv_lva_score() - if mv.to() == last.to() { 0 } else { 0 })
            .map(|(n, _mv)| n)
        {
            moves.swap(i, j);
        }
    }

    #[inline]
    fn _sort_one_move_handcrafted(i: usize, moves: &mut MoveList) {
        let mut j = 0;
        let mut max = 0;
        moves.iter().enumerate().skip(i).for_each(|(n, &mv)| {
            let s = mv.mvv_lva_score();
            if s > max {
                j = n;
                max = s;
            }
        });
        moves.swap(i, j);
    }

    fn gen(&mut self, b: &Board, algo: &mut Algo) {
        self.moves.clear();
        // pick.moves.clear();
        // println!("{}", self.move_orderer.order.chars().nth(pick.stage as usize).unwrap());
        let mt = self.ordering(algo)[self.stage];
        let all_moves = &mut self.all_moves;
        let last = self.last;
        let moves = &mut self.moves;
        match mt {
            MoveType::Start => {}

            MoveType::Hash => {
                if !self.tt.is_null() {
                    moves.push(self.tt);
                }
            }
            MoveType::Initialize => {
                b.legal_moves_into(all_moves);
                let mv = &self.tt;
                all_moves.retain(|m| mv != m);
            }
            MoveType::Evasion => {
                all_moves.iter().for_each(|&m| moves.push(m));
                algo.order_moves(self.n.ply, moves, &None);
            }
            MoveType::GoodCaptureUpfrontSorted => {
                all_moves
                    .iter()
                    .filter(|&m| Move::is_capture(m))
                    .for_each(|&m| moves.push(m));
                moves.sort_by_cached_key(|m| {
                    Move::mvv_lva_score(m) + if m.to() == last.to() { 0 } else { 0 }
                });
                moves.reverse();
                if algo.move_orderer.thread == 1 && moves.len() >= 2 {
                    moves.swap(0, 1);
                }
            }
            // Good Captures (sorted later)
            MoveType::GoodCapture => {
                all_moves
                    .iter()
                    .filter(|&m| Move::is_capture(m))
                    .for_each(|&m| moves.push(m));
            }

            // Killers
            MoveType::Killer => {
                algo.killers.legal_moves_for(self.n.ply, b, moves);
                all_moves.retain(|m| !moves.contains(m));
                // moves.sort_unstable_by_key(Move::mvv_lva_score);
                // moves.reverse();
                if algo.move_orderer.thread == 1 && moves.len() >= 2 {
                    moves.swap(0, 1);
                }
            }

            // CounterMove
            MoveType::CounterMove => {
                // if let Some(mv) = algo.counter_move.counter_move_for(b, last) {
                //     moves.push(mv);
                // }
                // all_moves.retain(|m| !moves.contains(m));
            }

            // Promos
            MoveType::Promo => {
                all_moves
                    .iter()
                    .filter(|&m| Move::is_promo(m) && !Move::is_capture(m))
                    .for_each(|&m| moves.push(m));
                // algo.order_moves(self.ply, moves, &None);
            }

            MoveType::QueenPromo => {
                all_moves
                    .iter()
                    .filter(|&m| {
                        Move::is_promo(m) && m.promo_piece() == Piece::Queen && !Move::is_capture(m)
                    })
                    .for_each(|&m| moves.push(m));
                // algo.order_moves(self.ply, moves, &None);
            }
            // Quiets
            MoveType::QuietUnsorted => {
                all_moves
                    .iter()
                    .filter(|m| !Move::is_capture(m) && !Move::is_promo(m))
                    .for_each(|&m| moves.push(m));
                // algo.order_moves(self.ply, moves, &None);
                // moves.sort_unstable_by_key(Move::mvv_lva_score);
                moves.reverse();
                if algo.move_orderer.thread == 1 && moves.len() >= 2 {
                    moves.swap(0, 1);
                }
            }

            MoveType::QuietOrPromo => {
                all_moves
                    .iter()
                    .filter(|&m| !Move::is_capture(m))
                    .for_each(|&m| moves.push(m));
                // algo.order_moves(self.ply, moves, &None);
                let ph = b.phase(&algo.eval.phaser);
                moves.sort_by_cached_key(|&mv| {
                    algo.move_orderer
                        .quiet_score(&self.n, mv, algo, ph, b.color_us(), b, last)
                });
                if algo.move_orderer.thread == 1 && moves.len() >= 2 {
                    moves.swap(0, 1);
                }
            }

            // sorted quiets
            MoveType::Quiet => {
                all_moves
                    .iter()
                    .filter(|m| !Move::is_capture(m) && !Move::is_promo(m))
                    .for_each(|&m| moves.push(m));
                // algo.order_moves(self.ply, moves, &None);
                moves.sort_by_cached_key(|&mv| {
                    algo.move_orderer.quiet_score(
                        &self.n,
                        mv,
                        algo,
                        b.phase(&algo.eval.phaser),
                        b.color_us(),
                        b,
                        last,
                    )
                });
                if algo.move_orderer.thread == 1 && moves.len() >= 2 {
                    moves.swap(0, 1);
                }
            }

            // Bad Captures
            MoveType::BadCapture => {
                moves.extend(self.bad_captures.iter().cloned());
                // if algo.move_orderer.thread == 1 && moves.len() >= 2 {
                //     moves.swap(0, 1);
                // }
            }
            // End
            MoveType::End => {}

            // unorderer
            MoveType::Unsorted => {
                b.legal_moves_into(moves);
                // std::mem::swap(&mut self.moves, &mut self.all_moves);
            }
            // Captures
            MoveType::Capture => {
                all_moves
                    .iter()
                    .filter(|m| Move::is_capture(m))
                    .for_each(|&m| moves.push(m));
                moves.sort_unstable_by_key(Move::mvv_lva_score);
                moves.reverse();
                if algo.move_orderer.thread == 1 && moves.len() >= 2 {
                    moves.swap(0, 1);
                }
            }
            // // Remaining
            // 'R' => {
            //     all_moves.iter().for_each(|&m| moves.push(m));
            //     // algo.order_moves(self.ply, moves, &None);
            //     moves.sort_unstable_by_key(Move::mvv_lva_score);
            //     moves.reverse();
            //     // algo.order_moves(self.ply, moves, &None);
            // }
            _ => {
                unreachable!("unknown move order stage")
            }
        };

        ///////////////////////////////////////
    }
}

// impl Algo {
//     pub fn generate_moves(&mut self, ply: Ply, _b: &Board) {
//         self.move_orderer.picker[ply] = MovePicker {
//             captures: false,
//             stage: 0,
//             moves: MoveList::new(),
//             all_moves: MoveList::new(),
//             index: 0,
//             tt: Move::NULL_MOVE,
//             ply: 0,
//         }
//     }

//     pub fn get_next_move(&mut self, ply: Ply, b: &Board) -> Option<Move> {
//         let pick = &mut self.move_orderer.picker[ply];
//         if pick.index < pick.moves.len() {
//             pick.index += 1;
//             return Some(pick.moves[pick.index - 1]);
//         }
//         if pick.stage as usize + 1 >= self.move_orderer.order.len() {
//             return None;
//         } else {
//             pick.index = 0;
//             pick.stage += 1;
//             let mut moves = MoveList::new();
//             self.gen(ply, b, &mut moves);
//             self.order_moves(ply, &mut moves, &None);
//             self.move_orderer.picker[ply].moves = moves;
//             return self.get_next_move(ply, b);
//         }
//     }
// }

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

// impl Algo {
//     fn gen(&mut self, ply: Ply, b: &Board, moves: &mut MoveList) {
//         let pick = &mut self.move_orderer.picker[ply];
//         // pick.moves.clear();
//         // println!("{}", self.move_orderer.order.chars().nth(pick.stage as usize).unwrap());
//         match self.move_orderer.order.chars().nth(pick.stage as usize).unwrap() {
//             'S' => {}
//             // Init
//             'I' => {
//                 let all_moves = &mut self.move_orderer.picker[ply].all_moves;
//                 b.legal_moves_into(all_moves);
//             }
//             // Captures
//             'C' => self.move_orderer.picker[ply]
//                 .all_moves
//                 .iter()
//                 .filter(|m| Move::is_capture(m))
//                 .for_each(|&m| moves.push(m)),
//             // Killers
//             'K' => self.killers.legal_moves_for(ply, b, moves),

//             // Promos
//             'P' => self.move_orderer.picker[ply]
//                 .all_moves
//                 .iter()
//                 .filter(|m| !Move::is_promo(m))
//                 .for_each(|&m| moves.push(m)),
//             // Quiets
//             'Q' => self.move_orderer.picker[ply]
//                 .all_moves
//                 .iter()
//                 .filter(|m| !Move::is_capture(m))
//                 .for_each(|&m| moves.push(m)),
//             // Remaining
//             'R' => self.move_orderer.picker[ply]
//                 .all_moves
//                 .iter()
//                 .for_each(|&m| moves.push(m)),
//             // End
//             'E' => {}

//             _ => {
//                 unreachable!("unknown move order stage")
//             }
//         };
//         self.move_orderer.picker[ply]
//             .all_moves
//             .retain(|m| !moves.contains(m));
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitboard::castling::*;
    use crate::bitboard::square::*;
    use crate::catalog::Catalog;
    use crate::globals::constants::*;
    use crate::movelist::*;
    use crate::search::algo::*;
    use crate::search::engine::*;
    use crate::search::timecontrol::*;
    use crate::tags::*;
    use crate::types::*;
    use crate::utils::*;
    // use crate::search::timecontrol::*;

    #[test]
    fn test_prior_pv() {
        let a1a2 = Move::new(
            a1.square(),
            a2.square(),
            Square::null(),
            Piece::Pawn,
            Piece::None,
            Piece::None,
            CastlingRights::NONE,
        );
        let a1a3 = Move::new(
            a1.square(),
            a3.square(),
            Square::null(),
            Piece::Pawn,
            Piece::None,
            Piece::None,
            CastlingRights::NONE,
        );
        let a1a4 = Move::new(
            a1.square(),
            a4.square(),
            Square::null(),
            Piece::Pawn,
            Piece::None,
            Piece::None,
            CastlingRights::NONE,
        );
        let b1a2 = Move::new(
            b1.square(),
            a2.square(),
            Square::null(),
            Piece::Pawn,
            Piece::None,
            Piece::None,
            CastlingRights::NONE,
        );
        let b1a3 = Move::new(
            b1.square(),
            a3.square(),
            Square::null(),
            Piece::Pawn,
            Piece::None,
            Piece::None,
            CastlingRights::NONE,
        );
        let b1a4 = Move::new(
            b1.square(),
            a4.square(),
            Square::null(),
            Piece::Pawn,
            Piece::None,
            Piece::None,
            CastlingRights::NONE,
        );
        let c1c2 = Move::new(
            c1.square(),
            c2.square(),
            Square::null(),
            Piece::Pawn,
            Piece::None,
            Piece::None,
            CastlingRights::NONE,
        );

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

    // #[test]
    // fn test_ordering() {
    //     let position = &Catalog::mate_in_2()[0];
    //     let mut algo = Algo::new().set_timing_method(TimeControl::Depth(3)).build();
    //     algo.qsearch.enabled = false;
    //     algo.tt.mb = 1;
    //     algo.move_orderer.enabled = false;
    //     algo.move_orderer.prior_bm = false;
    //     algo.move_orderer.prior_pv = false;
    //     algo.search(position.board());
    //     assert_eq!(algo.move_orderer.count_bm.get(0), 0, "{}", algo);
    //     assert_eq!(algo.move_orderer.count_pv.get(1), 0, "{}", algo);

    //     algo.move_orderer.enabled = true;
    //     algo.move_orderer.prior_bm = true;
    //     algo.move_orderer.prior_pv = false;
    //     algo.new_game();
    //     algo.search(position.board());
    //     // 2 is depth 1, plus depth 2,
    //     assert_eq!(algo.move_orderer.count_bm.get(0), 2, "{}", algo);
    //     assert_eq!(algo.move_orderer.count_bm.get(1), 0, "{}", algo); // bm only variation of lenght 1

    //     algo.move_orderer.mvv_lva = false;
    //     algo.move_orderer.prior_bm = false;
    //     algo.move_orderer.prior_pv = true;
    //     // algo.tt.enabled = false;
    //     algo.new_game();
    //     algo.search(position.board());
    //     println!("{}", algo);
    //     // 2 = pv[0] plus pv[0..1] used once
    //     assert_eq!(algo.move_orderer.count_pv.get(0), 2, "{}", algo);
    //     assert_eq!(algo.move_orderer.count_pv.get(1), 1, "{}", algo);
    // }

    // #[test]
    // fn test_gen_moves() {
    //     let board = Catalog::starting_position();
    //     let mut algo = Algo::new();
    //     algo.generate_moves(0, &board);

    //     let iter = std::iter::from_fn(move || algo.get_next_move(0, &board));
    //     let moves: Vec<Move> = iter.collect();
    //     println!("Moves {:?}", moves);
    // }

    #[test]
    fn test_ordered_movelist() {
        let orderer = MoveOrderer::new();
        let mut algo = Algo::new();
        const TT_MOVE: Move = Move::NULL_MOVE;
        let n = Node::root(3);

        let positions = &Catalog::win_at_chess();
        for pos in positions {
            let mut sorted_moves =
                orderer.create_sorted_moves(n, pos.board(), TT_MOVE, Move::NULL_MOVE);
            let mut moves = MoveList::new();
            while let Some((_stage, mv)) = sorted_moves.next_move(pos.board(), &mut algo) {
                moves.push(mv);
            }
            let lm = pos.board().legal_moves();
            assert_eq!(moves.len(), lm.len(), "{}\n{}\n", moves, lm);
        }
    }

    // SHIGKPQBE    => 28,391,980  // 17 secs
    // SHIgKPQBE    => 28,527,779  // 19 secs
    // SHIgKPqBE    => 22,957,082  // 16 secs
    // w/out PST    => 28,922,226
    // + mover.cp   => 28,000,000
    // w/out cstl+P => 23,466,765
    // w/out pawn   => 22,526,357
    // +to.rank     => 22,326,412
    // w/ from.pst  => 23,761,374

    #[test]
    #[ignore]
    fn test_ordering_node_count() {
        let mut engine = Engine::new();
        engine.new_game();
        engine.algo.analyse_mode = false;
        run_one_game(&mut engine);

        engine.new_game();
        engine.algo.analyse_mode = true;
        run_one_game(&mut engine);
    }

    fn run_one_game(engine: &mut Engine) {
        let positions = &Catalog::example_game();
        let mut nodes_cumul = 0;
        for pos in positions {
            //}.iter().step_by(1).collect_vec() {
            engine.algo.set_position(pos.clone());
            let suggested_depth = pos.acd().unwrap();
            // engine.algo.set_timing_method(TimeControl::NodeCount(200000));

            engine
                .algo
                .set_timing_method(TimeControl::Depth(suggested_depth - 1));
            engine.search();
            let mut results = engine.algo.results_as_position().clone();
            let nodes = results.acn().unwrap();
            nodes_cumul += nodes;

            // just leave acd
            results.tags_mut().remove(Tag::PV);
            results.tags_mut().remove(Tag::SM);
            results.tags_mut().remove(Tag::BM);
            results.tags_mut().remove(Tag::CE);
            results.tags_mut().remove(Tag::ACN);
            println!(
                "{:>12} {:>12} {}",
                Formatting::u128(nodes),
                Formatting::u128(nodes_cumul),
                results
            );
        }
    }
}
