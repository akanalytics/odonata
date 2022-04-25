use crate::bitboard::castling::CastlingRights;
use crate::bitboard::precalc::BitboardDefault;
use crate::bitboard::square::Square;
use crate::board::boardcalcs::BoardCalcs;
use crate::board::Board;
use crate::domain::material::Material;
use crate::eval::switches::Switches;
use crate::phaser::Phase;
use crate::types::Color;
use crate::types::Piece;
use crate::{Bitboard, PreCalc};
use std::fmt::Debug;
// use arrayvec::ArrayVec;

use super::endgame::EndGame;

#[derive(Clone, Default, Debug)]
pub struct Model {
    pub switches: Switches,
    pub csv: bool,
    pub turn: Color,
    pub mat: Material,
    pub endgame: EndGame,
    pub board: Board,
    pub phase: Phase, 

    pub white: ModelSide,
    pub black: ModelSide,
}

#[derive(Clone, Default, Debug)]
pub struct ModelSide {
    pub endgame_metric1: i32,
    pub endgame_metric2: i32,

    // bishops
    pub has_bishop_pair: bool,
    pub fianchetto: i32,
    pub bishop_color_pawns: i32,
    pub bishop_outposts: i32,

    // knights
    pub knight_forks: i32,
    pub knight_outposts: i32,

    // rooks
    pub has_rook_pair: bool,
    pub rooks_on_open_files: i32,
    pub doubled_rooks: i32,
    pub doubled_rooks_open_file: i32,
    pub enemy_pawns_on_rook_rank: i32,
    pub rooks_behind_passer: i32,
    pub enemy_rook_on_passer: i32,

    pub queens_on_open_files: i32,
    pub queen_early_develop: i32,

    // position
    // pub psq: ArrayVec<(Piece, Square), 32>,

    // pawn structure
    pub doubled_pawns: i32,
    pub pawn_directly_doubled: i32,
    pub isolated_pawns: i32,
    pub semi_isolated: i32,
    pub passed_pawns: i32,
    pub passed_pawns_on_r7: i32, // r7 by definition are passed
    pub passed_pawns_on_r6: i32, //
    pub passed_pawns_on_r5: i32,
    pub passed_pawns_on_r4: i32,
    pub passers_on_rim: i32, // files a & h
    pub candidate_passed_pawn: i32,
    pub blockaded: i32,         // enemy in front of pawn
    pub blockaded_passers: i32, // passed pawn with enemy right in front
    pub space: i32,             // empty squares behind rammed pawns
    pub rammed_pawns: i32,
    pub pawn_connected_r67: i32,
    pub pawn_connected_r345: i32,
    pub passed_connected_r67: i32,
    pub passed_connected_r345: i32,
    pub pawn_duo_r67: i32,
    pub pawn_duo_r2345: i32,
    pub passed_duo_r67: i32,
    pub passed_duo_r2345: i32,
    pub backward_half_open: i32,
    pub backward: i32,

    // king safety
    pub king_tropism_d1: i32,
    pub king_tropism_d2: i32,
    pub king_tropism_d3: i32,
    pub king_tropism_d4: i32,
    pub adjacent_shield: i32,
    pub nearby_shield: i32,
    pub open_files_near_king: i32,
    pub open_files_adjacent_king: i32,
    pub rq_on_open_files_near_king: i32,
    pub king_trapped_on_back_rank: i32,
    pub attacks_on_opponent_king_area: i32,
    pub king_safety_bonus: i32,
    pub checkers: i32,
    pub pieces_near_king: i32,
    pub pinned_near_king: i32,
    pub pinned_far: i32,
    pub discovered_checks: i32,

    pub castling_rights: i32, // 0,1 or 2
    pub uncastled: i32,

    // mobility
    pub move_squares: i32,
    pub center_attacks: i32,
    pub non_pawn_defended_moves: i32,
    pub fully_trapped_pieces: i32,
    pub partially_trapped_pieces: i32,
    pub defended_non_pawn: i32,
    pub xrayed: i32,
    // pub mv: ArrayVec<(Piece, i32), 32>,
    // pub attacks: [[i32; Piece::ALL.len()]; Piece::ALL.len()],
    // pub defends: [[i32; Piece::ALL.len()]; Piece::ALL.len()],

    // other
    pub has_tempo: bool,
}


impl Model {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn drawish(&self) -> i32 {
        self.board.fifty_halfmove_clock()
    }

    pub fn from_board(b: &Board, phase: Phase, switches: Switches) -> Self {
        let material = b.material();
        let endgame = EndGame::from_board(b);
        Self {
            switches,
            turn: b.color_us(),
            board: b.clone(),
            mat: material,
            white: ModelSide::from_board(b, Color::White, &material, endgame, switches),
            black: ModelSide::from_board(b, Color::Black, &material, endgame, switches),
            endgame,
            phase,
            csv: false,
        }
    }
}

impl ModelSide {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn from_board(b: &Board, c: Color, mat: &Material, eg: EndGame, sw: Switches) -> Self {
        let mut m = Self::default();
        if sw.contains(Switches::MATERIAL) {
            m.init_material(b, c, mat, eg);
        }
        if eg.try_winner().is_some() {
            return m;
        }
        if sw.contains(Switches::POSITION) {
            m.init_position(b, c, mat);
        }
        if sw.contains(Switches::PAWN) {
            m.init_pawns(b, c);
        }
        if sw.contains(Switches::SAFETY) {
            m.init_king_safety(b, c);
        }
        if sw.contains(Switches::MOBILITY) {
            m.init_mobility(b, c);
        }
        m.init_other(b, c);
        m
    }

    #[inline]
    fn init_material(&mut self, b: &Board, c: Color, m: &Material, eg: EndGame) {
        self.has_bishop_pair = m.counts(c, Piece::Bishop) >= 2;
        self.has_rook_pair = m.counts(c, Piece::Rook) >= 2;
        // let has_pawns = m.counts(c, Piece::Pawn) >= 1;
        // let mut net = m.net_piece(Piece::Knight) * Piece::Knight.centipawns()
        //     + m.net_piece(Piece::Bishop) * Piece::Bishop.centipawns()
        //     + m.net_piece(Piece::Rook) * Piece::Rook.centipawns()
        //     + m.net_piece(Piece::Queen) * Piece::Queen.centipawns();
        // let total = m.counts_piece(Piece::Knight) * Piece::Knight.centipawns()
        //     + m.counts_piece(Piece::Bishop) * Piece::Bishop.centipawns()
        //     + m.counts_piece(Piece::Rook) * Piece::Rook.centipawns()
        //     + m.counts_piece(Piece::Queen) * Piece::Queen.centipawns();
        // net *= c.chooser_wb(1, -1);
        // if net > 0 & has_pawns {
        //     lead = net
        // }
        let winning = c.opposite();
        if eg.try_winner() == Some(winning) {
            // c = losing colour - the winning side doesnt get a score (just the negative of the loser)
            use EndGame::*;
            match eg {
                BishopKnightVsKing(_) => {
                    use std::cmp::max;
                    let ksq = (b.kings() & b.color(c)).square();
                    let wksq = (b.kings() & b.color(winning)).square();
                    self.endgame_metric1 = 40 * Self::king_distance_to_bishops_corner(b, ksq, wksq);
                    let king_distance = Self::king_distance(b);
                    let ksq = (b.kings() & b.color(c)).square();
                    let nsq = (b.knights() & b.color(winning)).square();
                    let bsq = (b.bishops() & b.color(winning)).square();
                    let knight_distance = max(0, PreCalc::default().chebyshev_distance(nsq, ksq));
                    let bishop_distance = max(0, PreCalc::default().chebyshev_distance(bsq, ksq));
                    self.endgame_metric2 =
                        20 * king_distance + 2 * bishop_distance + 3 * knight_distance + 2 * Self::king_distance_to_side(b, c);
                }

                TwoBishopsOppositeColorSquares(_) => {
                    self.endgame_metric1 = 20 * Self::king_distance_to_any_corner(b, c);
                    self.endgame_metric2 = 10 * Self::king_distance(b);
                }

                KingMajorsVsKing(_) | _ => {
                    self.endgame_metric1 = 20 * Self::king_distance_to_side(b, c);
                    self.endgame_metric2 = 10 * Self::king_distance(b);
                }
            }
        }
    }

    fn king_distance(b: &Board) -> i32 {
        let wk = b.kings() & b.white();
        let bk = b.kings() & b.black();
        PreCalc::default().chebyshev_distance(wk.square(), bk.square())
    }

    fn king_distance_to_side(b: &Board, c: Color) -> i32 {
        use std::cmp::min;
        let k = b.kings() & b.color(c);
        if k.popcount() == 1 {
            let r = k.square().rank_index() as i32;
            let f = k.square().file_index() as i32;
            let m1 = min(r, f);
            let m2 = min(7 - r, 7 - f);
            min(m1, m2)
        } else {
            0
        }
    }

    fn king_distance_to_any_corner(b: &Board, c: Color) -> i32 {
        use std::cmp::min;
        let k = b.kings() & b.color(c);
        if k.popcount() == 1 {
            let ksq = k.square();
            let d1 = PreCalc::default().chebyshev_distance(Square::A1, ksq);
            let d2 = PreCalc::default().chebyshev_distance(Square::A8, ksq);
            let d3 = PreCalc::default().chebyshev_distance(Square::H1, ksq);
            let d4 = PreCalc::default().chebyshev_distance(Square::H8, ksq);
            min(min(d1, d2), min(d3, d4))
        } else {
            0
        }
    }

    fn king_distance_to_bishops_corner(b: &Board, ksq: Square, wksq: Square) -> i32 {
        let bis = b.bishops();
        let bad_corner1;
        let bad_corner2;
        // let gd_corner1;
        // let gd_corner2;
        // for losing king, these are undesirable corners
        if bis.intersects(Bitboard::WHITE_SQUARES) {
            bad_corner1 = Square::H1;
            bad_corner2 = Square::A8;
            // gd_corner1 = Square::A1;
            // gd_corner2 = Square::H8;
        } else {
            bad_corner1 = Square::A1;
            bad_corner2 = Square::H8;
            // gd_corner1 = Square::H1;
            // gd_corner2 = Square::A8;
        };

        // losing king distance to bad corner
        let bad_d1 = PreCalc::default().manhattan_distance(bad_corner1, ksq);
        let gd_d1 = PreCalc::default().manhattan_distance(bad_corner1, wksq);
        let bad_d2 = PreCalc::default().manhattan_distance(bad_corner2, ksq);
        let gd_d2 = PreCalc::default().manhattan_distance(bad_corner2, wksq);

        let d1 = if bad_d1 < gd_d1 { bad_d1 } else { bad_d1 };
        let d2 = if bad_d2 < gd_d2 { bad_d2 } else { bad_d1 };
        let dist = std::cmp::min(d1, d2);
        dist
        // let gd_d1 = PreCalc::default().chebyshev_distance(gd_corner1, ksq);
        // let gd_d2 = PreCalc::default().chebyshev_distance(gd_corner2, ksq);
        // let gd_dist = if gd_d1 < gd_d2 {
        //     PreCalc::default().chebyshev_distance(gd_corner1, wksq)
        // } else {
        //     PreCalc::default().chebyshev_distance(gd_corner2, wksq)
        // };
        // // give a bonus for winning king being nearer the nearest corner
        // if gd_dist < std::cmp::min(gd_d1, gd_d2) {
        //     dist - 1
        // } else {
        //     dist
        // }
    }

    #[inline]
    fn init_position(&mut self, b: &Board, c: Color, m: &Material) {
        let us = b.color(c);

        // fianchetto (short)
        const W_BISHOP: Bitboard = Bitboard::G2;
        const W_KING: Bitboard = Bitboard::F1.or(Bitboard::G1).or(Bitboard::H1);
        const W_PAWNS: Bitboard = Bitboard::F2.or(Bitboard::G3).or(Bitboard::H2).or(Bitboard::H3).or(Bitboard::H4);
        const W_NO_PAWNS: Bitboard = Bitboard::F3;

        const B_BISHOP: Bitboard = W_BISHOP.flip_vertical();
        const B_KING: Bitboard = W_KING.flip_vertical();
        const B_PAWNS: Bitboard = W_PAWNS.flip_vertical();
        const B_NO_PAWNS: Bitboard = W_NO_PAWNS.flip_vertical();

        let bishop = c.chooser_wb(W_BISHOP, B_BISHOP);
        let pawns = c.chooser_wb(W_PAWNS, B_PAWNS);
        let no_pawns = c.chooser_wb(W_NO_PAWNS, B_NO_PAWNS);
        let king = c.chooser_wb(W_KING, B_KING);

        if (b.bishops() & us).intersects(bishop)
            && (b.pawns() & us & pawns).popcount() >= 3
            && (b.pawns() & us).disjoint(no_pawns)
            && (b.kings() & us).intersects(king)
        {
            self.fianchetto += 1
        }

        if m.counts(c, Piece::Bishop) == 1 {
            if Bitboard::WHITE_SQUARES.contains(b.bishops() & b.color(c)) {
                self.bishop_color_pawns = (b.pawns() & b.color(c) & Bitboard::WHITE_SQUARES).popcount()
                    - (b.pawns() & b.color(c) & Bitboard::BLACK_SQUARES).popcount();
            } else if Bitboard::BLACK_SQUARES.contains(b.bishops() & b.color(c)) {
                self.bishop_color_pawns = (b.pawns() & b.color(c) & Bitboard::BLACK_SQUARES).popcount()
                    - (b.pawns() & b.color(c) & Bitboard::WHITE_SQUARES).popcount();
            }
        }

        // if queen has moved but other pieces havent (FIXME! not quite exactly right (QxQ))
        if (us & b.queens() & Bitboard::FILE_D & Bitboard::PROMO_RANKS).is_empty() {
            self.queen_early_develop = (us
                & Bitboard::PROMO_RANKS
                & ((b.bishops() & (Bitboard::FILE_C.or(Bitboard::FILE_F))) | (b.knights() & (Bitboard::FILE_B.or(Bitboard::FILE_G)))))
            .popcount();
        }
        // for &p in &Piece::ALL_BAR_NONE {
        //     let mut pieces = b.pieces(p) & b.color(c);
        //     if c == Color::White {
        //         pieces = pieces.flip_vertical();
        //     }
        //     pieces.squares().for_each(|sq| self.psq.push((p, sq)));
        // }
    }

    #[inline]
    fn init_other(&mut self, b: &Board, c: Color) {
        self.has_tempo = b.color_us() == c;
    }

    // front span - strictly in front
    // front fill - inclusive in front

    // isolated pawns have no neighbours of same color. Doubled pawns that are isolated count as two
    // doubled pawns are doubled or tripled or more. Two pawns count as one doubled. Three as two doubled.
    // passed. No neighbouring pawns of opposite colour ahead
    // rammed pawns have an opposing pawn directly in front

    // Open Pawns - have no mechanical obstruction - an opponent pawn in front.
    //                     They are at least half-free or even free passers
    //
    // - Passed Pawn       not counting rear of a doubled pawn.
    //                     front span disjoint from all other pawns, as well as the front fill disjoint
    //                     from opponent pawn attacks
    // -- Unstoppable
    // -- Protected  another (not nec passed pawn) is chained
    // -- Connected        duo or chain with another passed pawn
    // -- Outside          Seprated by several files
    //

    // - Candidate Passed Pawn

    // a pawn on a half-open file, which, if the board had only pawns on it, would eventually become
    // a passed pawn by moving forward. Whereas this definition is obvious for a human,
    // in a form presented above it would require no less than a separate recursive search routine.
    // For that reason, computers have to use approximations of that rule.
    // One possibility is to define a pawn as a candidate, if no square on its path is controlled
    // by more enemy pawns than own pawns.

    // Phalanx = Duo or more (same rank)
    // Connected = Phalanx or Chain
    // Doubled Pawn - types
    // Faker - a "faked" candidate with more opponent sentries than own helpers
    // Hidden Passed Pawn
    // Sentry - is a pawn controlling the square lying on the path or front span of an opponent's pawn,
    //          thereby preventing it from becoming a passed pawn

    // Weak Pawns - pawns not defended and not defensible by the pawns of the same color, whose stop square is also not covered by a friendly pawn.
    // - Isolated Pawn - no neighbouring pawns of same colour
    // - Isolated Pawn (half open) - even weker if rooks around
    // - Backward Pawn
    // - Overly advanced
    // - Hanging Pawns -  are an open, half-isolated duo. It means that they are standing next to each other on the adjacent half-open files, usually on the fourth rank, mutually protecting their stop squares.

    #[inline]
    fn init_pawns(&mut self, b: &Board, c: Color) {
        let us = b.color(c);
        let them = b.color(c.opposite());
        let bbd = BitboardDefault::default();
        // self.doubled_pawns = bbd.doubled_pawns(b.color(c) & b.pawns()).popcount();
        let isolated_pawns = bbd.isolated_pawns(us & b.pawns());
        self.isolated_pawns = isolated_pawns.popcount();
        let (pawn_atts_e, pawn_atts_w) = bbd.pawn_attacks(b.pawns() & us, c);
        let pawn_atts = pawn_atts_e | pawn_atts_w;
        let pawn_duos = bbd.pawn_duos(b.pawns() & us);
        let doubled_pawns = bbd.doubled_pawns(us & b.pawns());
        let isolated_doubled_pawns = (bbd.doubled_pawns(us & b.pawns()) & isolated_pawns).popcount();
        self.enemy_rook_on_passer = isolated_doubled_pawns;
        self.doubled_pawns = doubled_pawns.popcount() - isolated_doubled_pawns;

        let (enemy_pawn_atts_e, enemy_pawn_atts_w) = bbd.pawn_attacks(b.pawns() & them, c.opposite());
        let enemy_pawn_atts = enemy_pawn_atts_e | enemy_pawn_atts_w;

        for p in (b.pawns() & us).squares() {
            // self.doubled_pawns += is_doubled as i32;
            // we still count doubled pawns as passed pawns (since 0.3.37)
            let rank_index = p.rank_index_as_white(c) as i32;
            let pawn_stop = bbd.pawn_stop(c, p);
            // use pawns not pawns&them so we only count front of doubled pawns (+8 elo in sp)
            let is_passed = (bbd.pawn_front_span_union_attack_span(c, p) & b.pawns() & them).is_empty();
            // self.passed_pawns += is_passed as i32;
            // let rank7 = c.chooser_wb(Bitboard::RANK_7, Bitboard::RANK_2);
            // let rank6 = c.chooser_wb(Bitboard::RANK_6, Bitboard::RANK_3);
            // let rank5 = c.chooser_wb(Bitboard::RANK_5, Bitboard::RANK_4);
            // all pawns on r7 are passed as an opponent pawn cannot be on rank 8
            self.passers_on_rim += (is_passed && p.is_in(Bitboard::RIM)) as i32;
            self.blockaded += pawn_stop.intersects(them) as i32;
            self.blockaded_passers += (pawn_stop.intersects(them) && is_passed) as i32;
            self.rooks_behind_passer += (is_passed && (bbd.pawn_front_span(c.opposite(), p) & b.rooks() & us).any()) as i32;
            let rammed = bbd.pawn_stop(c, p).intersects(them & b.pawns());
            self.rammed_pawns += rammed as i32;

            // // old let is_passed = (bbd.pawn_front_span_union_attack_span(c, p) & b.pawns()).is_empty();
            // let blockaded = pawn_stop.intersects(them);

            // if p.is_in(doubled_pawns) && pawn_stop.intersects(doubled_pawns) {
            //     self.doubled_pawns -=1;
            //     self.pawn_directly_doubled +=1;
            // }

            // let _semi_isolated =
            // if !p.is_in(isolated_pawns)
            //     &&
            //     (
            //         ((bbd.neighbouring_files(p) & bbd.within_chebyshev_distance_inclusive(p, 3)) - p.as_bb())
            //         & b.pawns() & us
            //     ).is_empty() {
            //         self.semi_isolated += 1;
            //         true
            // } else {
            //     false
            // };

            // candidate passers
            // if !is_passed
            //     &&
            //     (bbd.pawn_front_span(c, p) & b.pawns() & them).is_empty() && p.rank_index_as_white(c) >= 5 {
            //         let mut ours = (bbd.pawn_attack_span(c.opposite(),p) & b.pawns() & us & bbd.within_chebyshev_distance_inclusive(p, 2)).popcount();
            //         ours += ((p.as_bb().shift(Dir::E) | p.as_bb().shift(Dir::W)) & b.pawns() & us).popcount();
            //         let theirs = (bbd.pawn_attack_span(c, p) & b.pawns() & them).popcount();
            //         if ours >= theirs {
            //             self.candidate_passed_pawn += 1;
            //         }
            // }
            // self.enemy_rook_on_passer += (!p.is_in(isolated_pawns)) as i32;

            // passers
            if is_passed {
                match rank_index {
                    6 => self.passed_pawns_on_r7 += 1,
                    5 => self.passed_pawns_on_r6 += 1,
                    4 => self.passed_pawns_on_r5 += 1,
                    3 => self.passed_pawns_on_r4 += 1,
                    _ => self.passed_pawns += 1,
                }
            }
            //     self.passers_on_rim += p.is_in(Bitboard::RIM) as i32;
            //     self.blockaded_passers += blockaded as i32;
            //     let no_pawns_behind = (bbd.pawn_front_span(c.opposite(), p) & b.pawns()).is_empty();
            //     self.rooks_behind_passer += (no_pawns_behind && (bbd.pawn_front_span(c.opposite(), p) & b.rooks_or_queens() & us).any()) as i32;
            //     if p.is_in(pawn_atts) || p.is_in(pawn_duos)  {
            //         match rank_index {
            //             4 | 5 | 6 => self.passed_connected_r67 += 1,
            //             _ => {}, // self.passed_connected_r345 += 1,
            //         }
            //     }
            //     //     match rank_index {
            //     //         // 4 | 5 | 6 => self.passed_connected_r67 += 1,
            //     //         _ => self.passed_connected_r345 += 1,
            //     //     }
            //     // }
            //     // if p.is_in(pawn_duos) {
            //     //     match rank_index {
            //     //         // 5 | 6 => self.passed_connected_r67 += 1,
            //     //         _ => self.passed_connected_r345 += 1,
            //     //     }
            //     // }
            //     // if !p.is_in(isolated_pawns)
            //     //     &&
            //     //     (((bbd.neighbouring_files(p) & bbd.within_chebyshev_distance_inclusive(p, 3)) - p.as_bb()) & b.pawns() & us).is_empty() {
            //     //     self.semi_isolated += 1;
            //     // }
            //     continue;
            // }
            // self.blockaded += blockaded as i32;
            // let rammed = pawn_stop.intersects(them & b.pawns());
            // self.space += (rammed as i32) * rank_index * (1 + p.is_in(FILE_D | FILE_E) as i32);
            if p.is_in(pawn_atts) {
                match rank_index {
                    5 | 6 => self.pawn_connected_r67 += 1,
                    _ => self.pawn_connected_r345 += 1,
                }
            }
            if p.is_in(pawn_duos) {
                match rank_index {
                    5 | 6 => self.pawn_duo_r67 += 1,
                    _ => self.pawn_duo_r2345 += 1,
                }
            } else {
                // half open backward pawns - cannot be defended by other pawns and cannot move fwd
                if pawn_stop.intersects(enemy_pawn_atts) && bbd.pawn_attack_span(c.opposite(), p).disjoint(b.pawns() & us) {
                    // we already know from duo-else-clause there is no pawn either side too
                    if (bbd.pawn_front_span(c, p) & b.pawns() & them).is_empty() {
                        //  &&
                        //  (b.rooks_or_queens() & them).any() { //
                        self.backward_half_open += 1;
                    } else {
                        self.backward += 1;
                    }
                }
            }
        }
        // self.doubled_pawns = bbd.doubled_pawns(us & b.pawns()).popcount();
        // lots of rammed pawns and having a knight an advantage
        self.rammed_pawns = self.rammed_pawns * self.rammed_pawns * (b.knights() & us).any() as i32;
    }

    #[inline]
    fn init_king_safety(&mut self, b: &Board, c: Color) {
        let us = b.color(c);
        let k = b.kings() & us;
        if k.is_empty() {
            return;
        }
        let them = b.color(c.opposite());
        let p = b.pawns() & us;
        let ksq = k.square();
        let bb = BitboardDefault::default();
        self.castling_rights = b.castling().contains(CastlingRights::king_side_right(c)) as i32
            + b.castling().contains(CastlingRights::queen_side_right(c)) as i32;

        if bb
            .within_chebyshev_distance_inclusive(Square::E8, 1)
            .or(bb.within_chebyshev_distance_inclusive(Square::E1, 1))
            .contains(k)
        {
            self.uncastled = 1
        }

        // if (b.rooks_or_queens() & them).any() {
        let p_fr_att_span = bb.pawn_front_span_union_attack_span(c, ksq);
        let d1 = bb.within_chebyshev_distance_inclusive(ksq, 1);
        let d2 = bb.within_chebyshev_distance_inclusive(ksq, 2);
        let d3 = bb.within_chebyshev_distance_inclusive(ksq, 3);
        let d4 = bb.within_chebyshev_distance_inclusive(ksq, 4);

        // if (b.rooks_or_queens() & them).any() {
        //self.nearby_pawns = (p & k_att).popcount();
        self.adjacent_shield = (p & p_fr_att_span & d1).popcount();
        self.nearby_shield = (p & p_fr_att_span & d2).popcount() - self.adjacent_shield;
        // }
        self.king_tropism_d1 = (d1 & (b.pawns() | b.kings()) & them).popcount()
            + (d1 & (b.knights() | b.bishops()) & them).popcount() * 2
            + (d1 & (b.rooks()) & them).popcount() * 4
            + (d1 & (b.queens()) & them).popcount() * 4;

        self.king_tropism_d2 = (d2 & (b.pawns() | b.kings()) & them).popcount()
            + (d2 & (b.knights() | b.bishops()) & them).popcount() * 2
            + (d2 & (b.rooks()) & them).popcount() * 4
            + (d2 & (b.queens()) & them).popcount() * 4;

        self.king_tropism_d3 = (d3 & (b.pawns() | b.kings()) & them).popcount()
            + (d3 & (b.knights() | b.bishops()) & them).popcount() * 2
            + (d3 & (b.rooks()) & them).popcount() * 4
            + (d3 & (b.queens()) & them).popcount() * 4;

        self.king_tropism_d4 = (d4 & (b.pawns() | b.kings()) & them).popcount()
            + (d3 & (b.knights() | b.bishops()) & them).popcount() * 2
            + (d3 & (b.rooks()) & them).popcount() * 4
            + (d3 & (b.queens()) & them).popcount() * 4;

        if b.queens().any() {
            self.king_safety_bonus = self.adjacent_shield + self.nearby_shield;
        }
        // self.king_tropism_d4 = (ksq.
        // self.king_tropism_d2 = (d2 & (b.pawns() | b.kings()) & them).popcount()
        //     + (d2 & (b.knights() | b.bishops()) & them).popcount() * 2
        //     + (d2 & (b.rooks()) & them).popcount() * 3
        //     + (d2 & (b.queens()) & them).popcount() * 5;

        // self.king_tropism_d3 = (d3 & (b.pawns() | b.kings()) & them).popcount()
        //     + (d3 & (b.knights() | b.bishops()) & them).popcount() * 2
        //     + (d3 & (b.rooks()) & them).popcount() * 3
        //     + (d3 & (b.queens()) & them).popcount() * 5;

        let open_files_near_king = d3 & ksq.rank() & bb.open_files(b.pawns());
        self.open_files_near_king = (open_files_near_king).popcount();
        self.open_files_adjacent_king = (d1 & ksq.rank() & bb.open_files(b.pawns())).popcount();
        self.rq_on_open_files_near_king = (open_files_near_king.file_flood() & b.rooks_or_queens() & them).popcount();

        if b.rooks_or_queens().any()
            && k.intersects(Bitboard::RANKS_18)
            // && Bitboard::RANKS_18 & ksq.rank() & us == k
            && (d1 - Bitboard::RANKS_18 - b.occupied()).is_empty()
        {
            self.king_trapped_on_back_rank = 1;
        }

        self.checkers = b.checkers_of(c).popcount();
        //        self.attacks_on_opponent_king_area += (our_raw_attacks & bb.within_chebyshev_distance_inclusive(ksq, 1)).popcount();

        self.pieces_near_king = (d2 & b.occupied() & us & (b.queens() | b.knights())).popcount();
        self.pinned_near_king = (b.pinned(c) & d1).popcount();
        self.pinned_far = (b.pinned(c)).popcount() - self.pinned_near_king;
        self.discovered_checks = (BoardCalcs::pinned_and_unmaskers(b, c).1 - b.pawns()).popcount();

        // }
        // FIXME Urgent!
        // self.open_files_near_king = (d1 & bb.open_files(b.pawns()) & ksq.rank()).popcount();

        // we give a castling bonus if either yet to castle or we have achieved the aim of
        // castling by having king before (or after) both rooks or pawns < 6 per side
        // if ( b.castling().contains(CastlingRights::king_side_right(c)) &&
        //     b.castling().contains(CastlingRights::queen_side_right(c)))
        //     ||
        //     ((b.rooks() & b.us()).any() &&
        //     (ksq < (b.rooks() & b.us()).first_square()  || ksq > (b.rooks() & b.us()).last_square() ))
        //     ||
        //     b.pawns().popcount() < 10 {
        //         self.castling_sides = 1;
        //     }
        // if (b.castling().contains(CastlingRights::king_side_right(c)) &&
        // b.castling().contains(CastlingRights::queen_side_right(c)))
        // ||

        //     ||
        //       ( b.castling().contains(CastlingRights::king_side_right(c)) ||
        //     b.castling().contains(CastlingRights::queen_side_right(c)))

        //     {
        //     self.castling_position = std::cmp::max((b.pawns() & Bitboard::RANKS_27).popcount() - 6, 0);
        // }
        // if self.adjacent_shield + self.nearby_shield >= 2
        //         &&
        //         ((b.rooks() & us).popcount() == 2
        //         &&
        //         (ksq.file_index() < (b.rooks() & us).first_square().file_index()  || ksq.file_index() > (b.rooks() & us).last_square().file_index() ))
        //     ||
        //     self.adjacent_shield + self.nearby_shield >= 2
        //         &&
        //         ((b.rooks() & us).popcount() == 1 && (3 - (b.rooks() & us).first_square().file_index()) >= ksq.file_index() )
        //         {
        //         self.castling_position = 1; //std::cmp::max((b.pawns() & Bitboard::RANKS_27).popcount() - 6, 0);
        // }

        //  if ( b.castling().contains(CastlingRights::king_side_right(c)) &&
        // b.castling().contains(CastlingRights::queen_side_right(c))) {
        //     self.castling_rights = 1
        // }
        // ||
        // (Bitboard::E1.or(Bitboard::E8)).contains(k)
        // ||
    }

    #[inline]
    fn init_mobility(&mut self, b: &Board, c: Color) {
        let bb = BitboardDefault::default();
        let us = b.color(c);
        let open_files = bb.open_files(b.pawns());
        self.doubled_rooks =
            (self.has_rook_pair && (b.rooks() & us).first_square().file_index() == (b.rooks() & us).last_square().file_index()) as i32;
        self.doubled_rooks_open_file = (self.doubled_rooks == 1 && (open_files & b.rooks() & us).popcount() >= 2) as i32;
        self.rooks_on_open_files = (open_files & us & b.rooks()).popcount();
        self.queens_on_open_files = (open_files & us & b.queens()).popcount();
        let their = c.opposite();
        let them = b.color(their);
        let occ = them | us;
        let their_p = b.pawns() & them;
        let our_p = b.pawns() & us;
        let (pe, pw) = bb.pawn_attacks(their_p, their);
        let (ope, opw) = bb.pawn_attacks(our_p, c);
        let pa = pe | pw;
        let our_pa = ope | opw;
        let bi = b.bishops() & them;
        let ni = b.knights() & them;
        let r = b.rooks() & them;
        // let q = b.queens() & them;

        let k = b.kings() & them;
        let ksq = k.square();
        for sq in ((b.knights() | b.bishops() | b.rooks() | b.queens()) & us).squares() {
            let p = b.piece_at(sq.as_bb());

            // non-pawn-defended empty or oppoent sq
            // include "attacking" our own pieces
            let our_raw_attacks = bb.attacks(c, p, Bitboard::empty(), occ, sq);

            let our_attacks = our_raw_attacks - pa;
            self.center_attacks += (our_attacks & Bitboard::CENTER_16_SQ).popcount();
            let piece_move_squares = (our_attacks - occ).popcount();
            // let our_xray_attacks = bb.non_pawn_attacks(c, p, us - our_p, them, sq);

            // FIXME v0.3.33 version
            // Piece::Queen => (our_attacks & occ - q - r - bi).popcount(),
            // Piece::Rook => (our_attacks & occ - r).popcount(),
            // Piece::Knight => (our_attacks & occ - ni).popcount(),
            // Piece::Bishop => (our_attacks & occ - bi - q).popcount(),

            // those attacks on enemy that arent pawn defended and cant attack back
            let piece_non_pawn_defended_moves = match p {
                Piece::Queen => (our_attacks & them).popcount(),
                Piece::Rook => (our_attacks & them - r).popcount(),
                Piece::Knight => (our_attacks & them - ni).popcount(),
                Piece::Bishop => (our_attacks & them - bi).popcount(),
                _ => 0,
            };

            // self.attacks[p][Piece::Pawn] += (our_raw_attacks & b.pawns() & them).popcount();
            // self.attacks[p][Piece::Knight] += (our_raw_attacks & b.knights() & them).popcount();
            // self.attacks[p][Piece::Bishop] += (our_raw_attacks & b.bishops() & them).popcount();
            // self.attacks[p][Piece::Rook] += (our_raw_attacks & b.rooks() & them).popcount();
            // self.attacks[p][Piece::Queen] += (our_raw_attacks & b.queens() & them).popcount();
            // self.attacks[p][Piece::King] += (our_raw_attacks & b.kings() & them).popcount();

            // self.defends[p][Piece::Pawn] += (our_raw_attacks & b.pawns() & us).popcount();
            // self.defends[p][Piece::Knight] += (our_raw_attacks & b.knights() & us).popcount();
            // self.defends[p][Piece::Bishop] += (our_raw_attacks & b.bishops() & us).popcount();
            // self.defends[p][Piece::Rook] += (our_raw_attacks & b.rooks() & us).popcount();
            // self.defends[p][Piece::Queen] += (our_raw_attacks & b.queens() & us).popcount();
            // self.defends[p][Piece::King] += (our_raw_attacks & b.kings() & us).popcount();
            // self.xrayed = match p {
            //     Piece::Queen => (our_xray_attacks & them - q).popcount(),
            //     Piece::Rook => (our_xray_attacks & them - r).popcount(),
            //     Piece::Knight => (our_xray_attacks & them - ni).popcount(),
            //     Piece::Bishop => (our_xray_attacks & them - bi).popcount(),
            //     _ => 0,
            // };
            // self.defended_non_pawn = ((our_raw_attacks|opa) & us).popcount();

            // trapped piece
            if piece_move_squares + piece_non_pawn_defended_moves == 1 {
                self.partially_trapped_pieces += 1;
            }
            if piece_move_squares + piece_non_pawn_defended_moves == 0 {
                self.fully_trapped_pieces += 1;
            }
            // self.mv.push((p, our_attacks.popcount()));
            self.move_squares += piece_move_squares;
            self.non_pawn_defended_moves += piece_non_pawn_defended_moves;

            if k.any() {
                self.attacks_on_opponent_king_area += (our_raw_attacks & bb.within_chebyshev_distance_inclusive(ksq, 1)).popcount();
            }
            if p == Piece::Knight {
                // knight forks
                for sq in (our_raw_attacks).squares() {
                    let atts = bb.knight_attacks(sq);
                    if (atts & them & (b.queens() | b.rooks() | b.bishops() | b.kings())).popcount() >= 2 {
                        self.knight_forks += 1;
                    }
                }
            }
            if p == Piece::Rook {
                self.enemy_pawns_on_rook_rank += (Bitboard::RANKS[sq.rank_index()] & b.pawns() & them).popcount();
            }

            if p == Piece::Knight || p == Piece::Bishop {
                // outposts
                // treat the piece as a pawn and make sure its attack span is clear of enemy pawns
                // and is on enemy half of board
                if bb.pawn_attack_span(c, sq).disjoint(their_p)
                    && ((sq.rank_index() >= 4 && c == Color::White) || (sq.rank_index() <= 4 && c == Color::Black))
                    && sq.is_in(our_pa)
                {
                    self.knight_outposts += (p == Piece::Knight) as i32;
                    self.bishop_outposts += (p == Piece::Bishop) as i32;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::eval::eval::Eval;
    use crate::tags::Tag;
    use crate::test_log::test;
    use crate::BoardBuf;
    use crate::eval::scorer::ReportLine;
    // use crate::utils::StringUtils;

    #[test]
    fn test_model_pawn() {
        let phaser = &Eval::new().phaser;
        let positions = Catalog::pawn_structure();
        let mut comparisons = 0;
        for p in &positions {
            let model = Model::from_board(p.board(), p.board().phase(phaser), Switches::ALL_SCORING);
            if let Tag::Comment(_n, var_name) = p.tag("c0") {
                info!("position {} c0 {}", p, var_name);
                if let Tag::Comment(_n, white) = p.tag("c1") {
                    if let Tag::Comment(_n, black) = p.tag("c2") {
                        let (w, b) = match var_name.as_str() {
                            "isolated" => (model.white.isolated_pawns, model.black.isolated_pawns),
                            "passed" => (model.white.passed_pawns, model.black.passed_pawns),
                            "passed_r7" => (model.white.passed_pawns_on_r7, model.black.passed_pawns_on_r7),
                            "passed_r6" => (model.white.passed_pawns_on_r6, model.black.passed_pawns_on_r6),
                            "passed_r5" => (model.white.passed_pawns_on_r5, model.black.passed_pawns_on_r5),
                            "passed_r4" => (model.white.passed_pawns_on_r4, model.black.passed_pawns_on_r4),
                            "doubled" => (model.white.doubled_pawns, model.black.doubled_pawns),
                            "connected_r67" => (model.white.pawn_connected_r67, model.black.pawn_connected_r67),
                            "connected_r345" => (model.white.pawn_connected_r345, model.black.pawn_connected_r345),
                            "backward_half_open" => (model.white.backward_half_open, model.black.backward_half_open),
                            _ => unreachable!(),
                        };
                        assert_eq!(w, white.parse::<i32>().unwrap(), "{}\n{:?}", p, model);
                        assert_eq!(b, black.parse::<i32>().unwrap(), "{}\n{:?}", p, model);
                        comparisons += 1;
                    }
                }
            }
        }
        assert_eq!(comparisons, positions.len());
    }

    //         // let map = s.split_vars_int();
    //     assert_eq!(model.white.isolated_pawns, map["isolated"], "{}\n{:?}", p, model);
    //     assert_eq!(model.white.doubled_pawns, map["doubled"], "{}\n{:?}", p, model);
    //     assert_eq!(model.white.passed_pawns, map["passed"], "{}", p);
    //     assert_eq!(model.white.pawn_connected_r67, map["con67"], "{}", p);
    //     assert_eq!(model.white.pawn_connected_r345, map["con345"], "{}\n{:?}", p, model);
    //     assert_eq!(model.white.backward_half_open, map["backward"], "{}\n{:?}", p, model);
    // }
    // if let Tag::Comment(_n, s) = p.tag("c1") {
    //     let map = s.split_vars_int();
    //     assert_eq!(model.black.isolated_pawns, map["isolated"], "{}", p);
    //     assert_eq!(model.black.doubled_pawns, map["doubled"], "{}", p);
    //     assert_eq!(model.black.passed_pawns, map["passed"], "{}", p);
    //     if map.contains_key("con67") {
    //         assert_eq!(model.black.pawn_connected_r67, map["con67"], "{}", p);
    //         assert_eq!(model.black.pawn_connected_r345, map["con345"], "{}", p);
    //         assert_eq!(model.black.backward_half_open, map["backward"], "{}", p);
    //     }

    #[test]
    fn king_safety_test() {
        let bb = BitboardDefault::default();
        let b = Board::parse_fen("6k1/r4ppp/1q1bb3/8/3p4/1P3B2/P2BQPPP/R5K1 w - - 0 1").unwrap();
        let wk = b.kings() & b.white();
        let wksq = wk.square();
        let wd1 = bb.within_chebyshev_distance_inclusive(wksq, 1);
        let bk = b.kings() & b.black();
        let bksq = bk.square();
        let bd1 = bb.within_chebyshev_distance_inclusive(bksq, 1);
        assert_eq!(
            b.rooks_or_queens().any()
                && (Bitboard::RANKS_18 & wksq.rank() & b.white() == wk)
                && (wd1 - Bitboard::RANKS_18 - b.occupied()).is_empty(),
            false,
            "white"
        );

        assert_eq!(b.rooks_or_queens().any(), true);
        assert_eq!((Bitboard::RANKS_18 & bksq.rank() & b.black() == bk), true);
        assert_eq!((bd1 - Bitboard::RANKS_18 - b.occupied()).is_empty(), true);
    }

    #[test]
    fn model_csv_test() {
        let eval = &mut Eval::new();
        eval.tempo = false;

        let positions = Catalog::example_game();
        for (i, p) in positions.iter().enumerate() {
            // let model = Model::from_board(p.board(), Switches::ALL_SCORING);
            if i == 0 {
                info!("\n{}", eval.w_eval_explain(&p.board(), false).as_csv(ReportLine::Header, true));
            }
            info!("\n{}", eval.w_eval_explain(&p.board(), false).as_csv(ReportLine::Body, true));
        }
        for (i, p) in positions.iter().enumerate() {
            let model = eval.w_eval_explain(&p.board(), true);

            if i == 0 {
                info!("\n{}", model.as_csv(ReportLine::Header, true));
            }
            info!("\n{}", model.as_csv(ReportLine::Body, true));
        }
    }
}
