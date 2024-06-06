use std::fmt;

use odonata_base::domain::node::{Node, Timing};
use odonata_base::domain::staticeval::StaticEval;
use odonata_base::infra::component::Component;
use odonata_base::infra::metric::{Event, Metrics};
use odonata_base::piece::MoveType;
use odonata_base::prelude::*;
use odonata_base::PreCalc;
use serde::{Deserialize, Serialize};

use super::algo::Search;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoveOrderer {
    pub enabled:                     bool,
    pub prior_pv:                    bool,
    pub prior_bm:                    bool,
    pub tt_bm:                       bool,
    pub mvv_lva:                     bool,
    pub discovered_checks:           bool,
    pub discovered_check_sort_bonus: f32,
    pub rank_index_sort_bonus:       f32,
    pub is_counter_move_sort_bonus:  f32,
    pub has_counter_move_sort_bonus: f32,
    pub knight_pseudo_mobility:      f32,
    pub bishop_pseudo_mobility:      f32,
    pub queen_pseudo_mobility:       f32,
    pub promo_sort_bonus:            f32,
    pub promo_queen_sort_bonus:      f32,
    pub promo_knight_sort_bonus:     f32,
    pub castle_sort_bonus:           f32,
    pub pst_sort_factor:             f32,
    pub hh_sort_factor:              f32,
    pub see_cutoff:                  Score,
    pub qsearch_see_cutoff:          Score,
    pub order:                       Vec<MoveType>,
    pub qorder:                      Vec<MoveType>,
    pub qorder_evasions:             Vec<MoveType>,

    #[serde(skip)]
    pub thread: u32,
}

impl Default for MoveOrderer {
    fn default() -> Self {
        MoveOrderer {
            enabled:                     true,
            prior_pv:                    true,
            prior_bm:                    false,
            tt_bm:                       true,
            mvv_lva:                     true,
            discovered_checks:           true,
            see_cutoff:                  0.cp(),
            qsearch_see_cutoff:          1.cp(),
            discovered_check_sort_bonus: 0.0,
            rank_index_sort_bonus:       0.0,
            knight_pseudo_mobility:      0.0,
            bishop_pseudo_mobility:      0.0,
            queen_pseudo_mobility:       0.0,
            is_counter_move_sort_bonus:  0.0,
            has_counter_move_sort_bonus: 0.0,
            promo_sort_bonus:            0.0,
            promo_queen_sort_bonus:      0.0,
            promo_knight_sort_bonus:     0.0,
            castle_sort_bonus:           0.0,
            pst_sort_factor:             0.0,
            hh_sort_factor:              1000.0,
            // picker:                      Stack::<OrderedMoveList>::default(),
            order:                       MoveType::vec_from_string("SHIGKPqBE").unwrap(), // , SHICKPQE, SHIGKPQBE
            qorder:                      MoveType::vec_from_string("SIgE").unwrap(),      //
            qorder_evasions:             MoveType::vec_from_string("SIVE").unwrap(),      //

            thread: 0,
        }
    }
}

#[rustfmt::skip]
impl Configurable for MoveOrderer {
    fn set(&mut self, p: Param) -> Result<bool> {
        self.enabled.set(p.get("enabled"))?;
        self.prior_pv.set(p.get("prior_pv"))?;
        self.prior_bm.set(p.get("prior_bm"))?;
        self.tt_bm.set(p.get("tt_bm"))?;
        self.mvv_lva.set(p.get("mvv_lva"))?;
        self.discovered_checks.set(p.get("discovered_checks"))?;
        self.see_cutoff.set(p.get("see_cutoff"))?;
        self.qsearch_see_cutoff.set(p.get("qsearch_see_cutoff"))?;
        self.discovered_check_sort_bonus.set(p.get("discovered_check_sort_bonus"))?;
        self.rank_index_sort_bonus.set(p.get("rank_index_sort_bonus"))?;
        self.knight_pseudo_mobility.set(p.get("knight_pseudo_mobility"))?;
        self.bishop_pseudo_mobility.set(p.get("bishop_pseudo_mobility"))?;
        self.queen_pseudo_mobility.set(p.get("queen_pseudo_mobility"))?;
        self.is_counter_move_sort_bonus.set(p.get("is_counter_move_sort_bonus"))?;
        self.has_counter_move_sort_bonus.set(p.get("has_counter_move_sort_bonus"))?;
        self.promo_sort_bonus.set(p.get("promo_sort_bonus"))?;
        self.promo_queen_sort_bonus.set(p.get("promo_queen_sort_bonus"))?;
        self.promo_knight_sort_bonus.set(p.get("promo_knight_sort_bonus"))?;
        self.castle_sort_bonus.set(p.get("castle_sort_bonus"))?;
        self.pst_sort_factor.set(p.get("pst_sort_factor"))?;
        self.hh_sort_factor.set(p.get("hh_sort_factor"))?;
        // self.order.set(p.get("order"))?;  // BUG
        // self.qorder.set(p.get("qorder"))?;
        // self.qorder_evasions.set(p.get("qorder_evasions"))?;
        Ok(p.is_modified())
    }
}

impl Component for MoveOrderer {
    fn new_game(&mut self) {}

    fn new_position(&mut self) {
        self.thread = 0;
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
        writeln!(f, "order            : {}", MoveType::slice_to_string(&self.order))?;
        writeln!(f, "qorder           : {}", MoveType::slice_to_string(&self.qorder))?;
        writeln!(f, "thread           : {}", self.thread)?;
        Ok(())
    }
}

impl MoveOrderer {
    pub fn new() -> Self {
        Self::default()
    }
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
    pub fn quiet_points(&self, n: &Node, mv: Move, algo: &Search, c: Color, b: &Board, parent: Move) -> i32 {
        let mut score = 0.0;
        if let Some(promo) = mv.promo_piece() {
            score += self.promo_sort_bonus;
            if promo == Piece::Queen {
                score += self.promo_queen_sort_bonus;
            }
            if promo == Piece::Knight {
                score += self.promo_knight_sort_bonus;
            }
        }
        if mv.is_castle(b) {
            score += self.castle_sort_bonus;
        }

        // if mv.from().is_in(b.discoverer(b.color_them())) {
        //     score += self.discovered_check_sort_bonus;
        // }

        if self.discovered_check_sort_bonus != 0.0 && b.make_move(mv).is_in_check(c.flip_side()) {
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
        if self.knight_pseudo_mobility != 0.0 && mv.mover_piece(b) == Piece::Knight {
            score += self.knight_pseudo_mobility
                * (PreCalc::instance().knight_attacks(mv.to()).popcount()
                    - PreCalc::instance().knight_attacks(mv.from()).popcount()) as f32
        }
        if self.bishop_pseudo_mobility != 0.0 && mv.mover_piece(b) == Piece::Bishop {
            score += self.bishop_pseudo_mobility
                * (PreCalc::instance().bishop_attacks(Bitboard::EMPTY, mv.to()).popcount()
                    - PreCalc::instance()
                        .bishop_attacks(Bitboard::EMPTY, mv.from())
                        .popcount()) as f32
        }
        if self.queen_pseudo_mobility != 0.0 && mv.mover_piece(b) == Piece::Queen {
            score += self.queen_pseudo_mobility
                * (PreCalc::instance().bishop_attacks(Bitboard::EMPTY, mv.to()).popcount()
                    - PreCalc::instance()
                        .bishop_attacks(Bitboard::EMPTY, mv.from())
                        .popcount()) as f32
        }
        // if mv.mover_piece(b) == Piece::Pawn {
        //     score += 0;
        // }
        // score += mv.mover_piece(b).centipawns();
        score += self.rank_index_sort_bonus * mv.to().rank_number_as_white(c) as f32;

        score += self.hh_sort_factor * algo.history.history_heuristic_bonus(c, &mv, n, b) as f32;

        let cm = algo.counter_move.counter_move_unchecked(c, parent, mv, n);
        if cm > 0 {
            Metrics::incr_node(n, Event::MoveSortCounterMove);
            score += cm as f32 * self.is_counter_move_sort_bonus;
        }
        // if self.has_counter_move_sort_bonus > -10000.0
        //     && algo
        //         .counter_move
        //         .counter_move_unchecked(c.opposite(), mv)
        // {
        //     score += self.has_counter_move_sort_bonus;
        // }

        // if self.pst_sort_factor != 0.0 {
        //     score += self.pst_sort_factor
        //     * algo
        //         .eval
        //         .pst
        //         .w_eval_square(c, mv.mover_piece(b), mv.to())
        //         .interpolate(phase);
        // }
        // score -= algo.eval.w_eval_square(c, mv.mover_piece(), mv.from()).interpolate(phase);
        -score as i32
    }
}

impl Search {
    pub fn order_moves(&mut self, _ply: Ply, movelist: &mut MoveList, tt_mv: &Option<Move>, bd: &Board) {
        if !self.move_orderer.enabled {
            return;
        }

        if self.move_orderer.mvv_lva {
            // movelist.sort_unstable_by_key(|m| -m.mvv_lva_score() );
            movelist.sort_unstable_by_key(|mv| mv.mvv_lva_score(bd));
            movelist.reverse();
            // if self.move_orderer.thread == 1 && movelist.len() >= 2 {
            //     movelist.swap(0, 1);
            // }
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
                }
            }
        }
    }

    pub fn order_from_prior_pv(moves: &mut MoveList, var: &Variation, pv: &Variation) -> bool {
        // println!( "{:>30} / [{:>30}] - {}",var.to_string(), pv.to_string(), moves  );
        if pv.is_empty() {
            return false;
        }
        if var.len() >= pv.len() {
            return false; // we're already exploring beyond what we have pv for
        }

        if pv.starts_with(var) {
            let best = pv.moves().nth(var.len());
            let j = moves.iter().position(|&mv| Some(mv) == best);
            if let Some(j) = j {
                if j != 0 {
                    moves.swap(0, j);
                    return true;
                }
            }
            // println!("{:>30} / [{:>30}] ==> {}", var.to_string(), pv.to_string(), moves);
            return false;
        }
        false
    }

    // pub fn order_by_mvv_lva(moves: &mut MoveList) {
}

// uses Move Orderer and MoveGen to present a sequence of moves
#[derive(Clone, Debug, Default)]
pub struct OrderedMoveList {
    qsearch:      bool,
    is_in_check:  bool,
    stage:        usize,
    moves:        MoveList,
    all_moves:    MoveList,
    bad_captures: MoveList,
    index:        usize,
    tt:           Move,
    last:         Move,
    n:            Node,
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
    fn ordering<'a>(&self, algo: &'a Search) -> &'a Vec<MoveType> {
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

    pub fn next_move(&mut self, b: &Board, algo: &mut Search) -> Option<(MoveType, Move)> {
        let t = Metrics::timing_start();
        let m = self.calc_next_move_(b, algo);
        Metrics::profile(t, Timing::TimingSortMoves);
        m
    }

    fn calc_next_move_(&mut self, b: &Board, algo: &mut Search) -> Option<(MoveType, Move)> {
        let move_type = self.ordering(algo)[self.stage];
        if self.index < self.moves.len() {
            if move_type == MoveType::GoodCapture || move_type == MoveType::Capture
            // we dont sort killers
            // || move_type == 'b' as b is sorted by reverse anyway due to push and they are bad captures
            {
                Self::sort_one_capture_move(self.index, &mut self.moves, self.last, b);
            }
            if move_type == MoveType::GoodCaptureUpfrontSorted || move_type == MoveType::GoodCapture {
                let mv = self.moves[self.index];
                Metrics::incr_node(&self.n, Event::EvalSee);
                let see = algo.eval.eval_move_see(mv, b);
                let see_cutoff = if self.qsearch {
                    algo.move_orderer.qsearch_see_cutoff
                } else {
                    algo.move_orderer.see_cutoff
                };

                if see < see_cutoff || see == see_cutoff && self.qsearch && self.n.depth < -1 {
                    if !(algo.move_orderer.discovered_checks && b.maybe_gives_discovered_check(mv)) {
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
        if self.stage + 1 >= self.ordering(algo).len() {
            None
        } else {
            self.index = 0;
            self.stage += 1;
            self.gen(b, algo);
            self.calc_next_move_(b, algo)
        }
    }

    #[inline]
    fn sort_one_capture_move(i: usize, moves: &mut MoveList, _last: Move, bd: &Board) {
        if let Some(j) = moves
            .iter()
            .enumerate()
            .skip(i)
            // .max_by_key(|(_n, &mv)| mv.mvv_lva_score(bd) - if mv.to() == last.to() { 0 } else { 0 })
            .max_by_key(|(_n, &mv)| mv.mvv_lva_score(bd))
            .map(|(n, _mv)| n)
        {
            moves.swap(i, j);
        }
    }

    #[inline]
    fn _sort_one_move_handcrafted(i: usize, moves: &mut MoveList, bd: &Board) {
        let mut j = 0;
        let mut max = 0;
        moves.iter().enumerate().skip(i).for_each(|(n, &mv)| {
            let s = mv.mvv_lva_score(bd);
            if s > max {
                j = n;
                max = s;
            }
        });
        moves.swap(i, j);
    }

    fn gen(&mut self, b: &Board, algo: &mut Search) {
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
                b.legal_moves_with(|mv| all_moves.push(mv));
                let mv = &self.tt;
                all_moves.retain(|m| mv != m);
            }
            MoveType::Evasion => {
                all_moves.iter().for_each(|&m| moves.push(m));
                algo.order_moves(self.n.ply, moves, &None, b);
            }
            MoveType::GoodCaptureUpfrontSorted => {
                all_moves
                    .iter()
                    .filter(|&m| Move::is_capture(m))
                    .for_each(|&m| moves.push(m));
                moves.sort_by_cached_key(|m| Move::mvv_lva_score(m, b) + if m.to() == last.to() { 0 } else { 0 });
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
                algo.killers.legal_moves_for(&self.n, b, moves);
                if !self.tt.is_null() {
                    moves.retain(|&mut mv| mv != self.tt); // only keep killers that aren't hash moves
                }
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
                    .filter(|&m| m.promo_piece() == Some(Piece::Queen) && !Move::is_capture(m))
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
                moves
                    .sort_by_cached_key(|&mv| algo.move_orderer.quiet_points(&self.n, mv, algo, b.color_us(), b, last));
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
                moves
                    .sort_by_cached_key(|&mv| algo.move_orderer.quiet_points(&self.n, mv, algo, b.color_us(), b, last));
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
                b.legal_moves_with(|mv| moves.push(mv));
                // std::mem::swap(&mut self.moves, &mut self.all_moves);
            }
            // Captures
            MoveType::Capture => {
                all_moves
                    .iter()
                    .filter(|m| Move::is_capture(m))
                    .for_each(|&m| moves.push(m));
                moves.sort_unstable_by_key(|mv| mv.mvv_lva_score(b));
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
    use odonata_base::catalog::Catalog;

    use super::*;
    use crate::search::engine::ThreadedSearch;

    #[test]
    fn test_prior_pv() {
        let bd = &Board::starting_pos();
        let a1a2 = Move::new_quiet(Piece::Pawn, Square::A1, Square::A2, bd);
        let a1a3 = Move::new_quiet(Piece::Pawn, Square::A1, Square::A3, bd);
        let a1a4 = Move::new_quiet(Piece::Pawn, Square::A1, Square::A4, bd);
        let b1a2 = Move::new_quiet(Piece::Pawn, Square::B1, Square::A2, bd);
        let b1a3 = Move::new_quiet(Piece::Pawn, Square::B1, Square::A3, bd);
        let b1a4 = Move::new_quiet(Piece::Pawn, Square::B1, Square::A4, bd);
        let c1c2 = Move::new_quiet(Piece::Pawn, Square::C1, Square::C2, bd);

        let mut moves_orig = MoveList::new();
        moves_orig.extend(vec![b1a2, b1a3, b1a4, a1a3, a1a4, a1a2]);
        assert_eq!(moves_orig.to_string(), "b1a2, b1a3, b1a4, a1a3, a1a4, a1a2");

        let mut pv = Variation::new();
        pv.extend_from_slice(&[a1a2, a1a3, a1a4]);

        // if variation = empty, and pv = empty, no ordering
        let variation = Variation::new();
        let mut moves = moves_orig.clone();
        Search::order_from_prior_pv(&mut moves, &variation, &variation);
        assert_eq!(moves.to_string(), "b1a2, b1a3, b1a4, a1a3, a1a4, a1a2");

        // if variation = empty, use pv[0] if it exists
        let mut variation = Variation::new();
        let mut moves = moves_orig.clone();
        Search::order_from_prior_pv(&mut moves, &variation, &pv);
        assert_eq!(moves.to_string(), "a1a2, b1a3, b1a4, a1a3, a1a4, b1a2");

        // variation strays from PV - no reordering
        // pv = a1a2 a1a3 a1a4   var = a1a2 c1c2
        let mut moves = moves_orig.clone();
        variation.extend_from_slice(&[a1a2, c1c2]);
        Search::order_from_prior_pv(&mut moves, &variation, &pv);
        assert_eq!(moves_orig.to_string(), "b1a2, b1a3, b1a4, a1a3, a1a4, a1a2");

        // variation strays from PV - no reordering
        // pv = a1a2 a1a3 a1a4   var = a1a2 a1a3 c1c2
        let mut variation = Variation::new();
        variation.extend_from_slice(&[a1a2, a1a3, c1c2]);
        let mut moves = moves_orig.clone();
        Search::order_from_prior_pv(&mut moves, &variation, &pv);
        assert_eq!(moves.to_string(), "b1a2, b1a3, b1a4, a1a3, a1a4, a1a2");

        // if variation = pv, no movelist ordering
        let mut moves = moves_orig.clone();
        Search::order_from_prior_pv(&mut moves, &pv, &pv);

        // variation is PV[0] - use PV[1]
        // pv = a1a2 a1a3 a1a4   var = a1a2
        let mut variation = Variation::new();
        variation.extend_from_slice(&[a1a2]);
        let mut moves = moves_orig.clone();
        Search::order_from_prior_pv(&mut moves, &variation, &pv);
        assert_eq!(moves.to_string(), "a1a3, b1a3, b1a4, b1a2, a1a4, a1a2");

        // variation is PV[0] and [1] - use PV[2]
        // pv = a1a2 a1a3 a1a4   var = a1a2 a1a3
        let mut moves = moves_orig.clone();
        let mut variation = Variation::new();
        variation.extend_from_slice(&[a1a2, a1a3]);
        Search::order_from_prior_pv(&mut moves, &variation, &pv);
        assert_eq!(moves.to_string(), "a1a4, b1a3, b1a4, a1a3, b1a2, a1a2");
    }

    #[test]
    fn test_ordering_mvv_lva() {
        let board = Catalog::perft_kiwipete().0;
        let mut moves = board.legal_moves();
        moves.sort(); // alphabetical first
        ThreadedSearch::new().search.order_moves(0, &mut moves, &None, &board);
        println!("{moves:#}");
        assert_eq!(moves[0].to_uci(), "e2a6"); // b x b
        assert_eq!(moves[1].to_uci(), "f3f6"); // q x n
        assert_eq!(moves[2].to_uci(), "d5e6"); // p x p
        assert_eq!(moves[3].to_uci(), "g2h3"); // p x p
        assert_eq!(moves[7].to_uci(), "f3h3"); // q x p

        let positions = Catalog::move_ordering();
        for (i, pos) in positions.iter().enumerate() {
            let mut moves = pos.board().legal_moves();
            ThreadedSearch::new()
                .search
                .order_moves(0, &mut moves, &None, &pos.board());
            println!("{pos}\n{moves:#}");
            if i == 0 {
                assert_eq!(moves[0].to_uci(), "b7a8q"); // p x r = Q)
                assert_eq!(moves[1].to_uci(), "b7a8r"); // p x r = R)
                assert_eq!(moves[2].to_uci(), "b7b8q"); // p  = Q)
                assert!(["b7a8b", "b7a8n"].contains(&moves[3].to_uci().as_str()));
                // p x r  = B/N)
            }
        }
    }

    #[test]
    fn test_ordered_movelist() {
        let orderer = MoveOrderer::new();
        let mut eng = ThreadedSearch::new();
        const TT_MOVE: Move = Move::new_null();
        let n = Node::root(3);

        let positions = &Catalog::win_at_chess();
        for pos in positions {
            let mut sorted_moves = orderer.create_sorted_moves(n, &pos.board(), TT_MOVE, Move::new_null());
            let mut moves = MoveList::new();
            while let Some((_stage, mv)) = sorted_moves.next_move(&pos.board(), &mut eng.search) {
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

    // #[test]
    // #[ignore]
    // fn test_ordering_node_count() {
    //     let mut engine = ThreadedSearch::new();
    //     engine.new_game();
    //     engine.search.controller.analyse_mode = false;
    //     run_one_game(&mut engine);

    //     engine.new_game();
    //     engine.search.controller.analyse_mode = true;
    //     run_one_game(&mut engine);
    // }

    // fn run_one_game(engine: &mut ThreadedSearch) {
    //     let positions = &Catalog::example_game();
    //     let mut nodes_cumul = 0;
    //     for pos in positions {
    //         //}.iter().step_by(1).collect_vec() {
    //         engine.search.set_position(pos.clone());
    //         let suggested_depth = pos.int("acd").unwrap() as i32;
    //         // engine.algo.set_timing_method(TimeControl::NodeCount(200000));

    //         engine
    //             .search
    //             .set_timing_method(TimeControl::Depth(suggested_depth - 1));
    //         engine.search_sync();
    //         let mut results = engine.search.results_as_position().clone();
    //         let nodes = results.int("acn").unwrap();
    //         nodes_cumul += nodes;

    //         // just leave acd
    //         results.tags_mut().pv = None;
    //         results.tags_mut().supplied_move = None;
    //         results.tags_mut().best_moves = None;
    //         results.tags_mut().centipawn_evaluation = None;
    //         results.tags_mut().acn = None;
    //         println!("{nodes:>12} {nodes_cumul:>12} {results}");
    //     }
    // }
}
