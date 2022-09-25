use crate::bits::{CastlingRights, PreCalc, Square};
use crate::board::analysis::Analysis;
use crate::board::Board;
use crate::eval::endgame::EndGame;
use crate::eval::eval::Feature;
use crate::infra::metric::Metrics;
use crate::piece::Color::{self, *};
use crate::piece::Piece;
use crate::piece::Piece::*;
use crate::search::node::Timing;
use crate::Bitboard;

use super::eval::Attr;
use super::scorer::ScorerBase;

#[derive(Copy, Clone, Debug)]
pub struct ColorPiece {
    c: Color,
    p: Piece,
}

impl ColorPiece {
    pub const fn len() -> usize {
        Color::len() * Piece::len()
    }

    pub const fn index(&self) -> usize {
        self.c.index() + self.p.index() * Color::len()
    }

    pub const fn all() -> [ColorPiece; ColorPiece::len()] {
        [
            Self { c: White, p: Pawn },
            Self {
                c: White,
                p: Knight,
            },
            Self {
                c: White,
                p: Bishop,
            },
            Self { c: White, p: Rook },
            Self { c: White, p: Queen },
            Self { c: White, p: King },
            Self { c: Black, p: Pawn },
            Self {
                c: Black,
                p: Knight,
            },
            Self {
                c: Black,
                p: Bishop,
            },
            Self { c: Black, p: Rook },
            Self { c: Black, p: Queen },
            Self { c: Black, p: King },
        ]
    }
}

trait White {
    const PAWN: ColorPiece = ColorPiece {
        c: Color::White,
        p: Piece::Pawn,
    };
}

// #[derive(Default)]
// pub struct Calc {
//     attacks: [Bitboard; ColorPiece::len()],
// }

// impl Display for Calc {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         let mut builder = Builder::new();
//         for y in &self.attacks.iter().enumerate().chunks(5) {
//             let mut row = vec![];
//             for (i, bb) in y {
//                 row.push(format!("{:?}\n{bb:#}", ColorPiece::all()[i]));
//             }
//             builder = builder.add_record(row);
//         }
//         let tab = builder.build();
//         tab.fmt(f)?;
//         Ok(())
//     }
// }

// impl Calc {
//     #[inline]
//     pub fn new(b: &Board) -> Self {
//         let bb = PreCalc::default();
//         let occ = b.occupied();

//         Calc {
//             attacks: [
//                 Bitboard::EMPTY,
//                 bb.all_attacks(Color::White, Piece::Pawn, b.pawns() & b.white(), occ),
//                 bb.all_attacks(Color::White, Piece::Knight, b.knights() & b.white(), occ),
//                 bb.all_attacks(Color::White, Piece::Bishop, b.bishops() & b.white(), occ),
//                 bb.all_attacks(Color::White, Piece::Rook, b.rooks() & b.white(), occ),
//                 bb.all_attacks(Color::White, Piece::Queen, b.queens() & b.white(), occ),
//                 bb.all_attacks(Color::White, Piece::King, b.kings() & b.white(), occ),
//                 Bitboard::EMPTY,
//                 bb.all_attacks(Color::Black, Piece::Pawn, b.pawns() & b.black(), occ),
//                 bb.all_attacks(Color::Black, Piece::Knight, b.knights() & b.black(), occ),
//                 bb.all_attacks(Color::Black, Piece::Bishop, b.bishops() & b.black(), occ),
//                 bb.all_attacks(Color::Black, Piece::Rook, b.rooks() & b.black(), occ),
//                 bb.all_attacks(Color::Black, Piece::Queen, b.queens() & b.black(), occ),
//                 bb.all_attacks(Color::Black, Piece::King, b.kings() & b.black(), occ),
//             ],
//         }
//     }
// }

#[derive()]
pub struct Calc<'a> {
    _analysis: Analysis<'a>,
    // a: &'a (),
}

impl<'a> Calc<'a> {
    #[inline]
    pub fn new(_b: &'a Board) -> Self {
        Self {
            // a: &(),
            // analysis: Analysis::of(b),
            _analysis: Default::default(),
        }
    }
}

// impl Display for Calc {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         let mut builder = Builder::new();
//         for y in &self.attacks.iter().enumerate().chunks(5) {
//             let mut row = vec![];
//             for (i, bb) in y {
//                 row.push(format!("{:?}\n{bb:#}", ColorPiece::all()[i]));
//             }
//             builder = builder.add_record(row);
//         }
//         let tab = builder.build();
//         tab.fmt(f)?;
//         Ok(())
//     }
// }
impl<'a> Calc<'a> {
    // 0, 1, 3, 8
    #[inline]
    pub fn linear(x: u32, n: i32) -> (i32, i32, i32, i32, i32, i32) {
        match x {
            0 => (16 * n, 0, 0, 0, 0, 0),
            1 => (0, 16 * n, 0, 0, 0, 0),
            2 => (0, 0, 16 * n, 0, 0, 0),
            3 => (0, 0, 0, 16 * n, 0, 0),
            4 => (0, 0, 0, 12 * n, 4 * n, 0),
            5 => (0, 0, 0, 8 * n, 8 * n, 0),
            6 => (0, 0, 0, 4 * n, 12 * n, 0),
            7 => (0, 0, 0, 0, 16 * n, 0),
            8 => (0, 0, 0, 0, 0, 16 * n),
            _ => (0, 0, 0, 0, 0, 16 * n),
        }
    }
}

impl<'a> Calc<'a> {
    #[inline]
    pub fn score(&mut self, scorer: &mut impl ScorerBase, b: &Board) {
        let t = Metrics::timing_start();
        self.material(scorer, b);
        if !self.endgame(scorer, b) {
            self.position(scorer, b);
            self.pst(scorer, b);
            self.other(scorer, b);
            self.pawns(White, scorer, b);
            self.pawns(Black, scorer, b);
            self.king_safety(White, scorer, b);
            self.king_safety(Black, scorer, b);
            self.mobility(White, scorer, b);
            self.mobility(Black, scorer, b);
        }
        // scorer.set_phase(b.phase(ph));
        // scorer.interpolate_and_scale("interpolate");
        Metrics::profile(t, Timing::TimingEval);
    }

    fn other(&mut self, s: &mut impl ScorerBase, b: &Board) {
        s.accumulate(
            Attr::TempoBonus.as_feature(),
            (b.color_us() == White) as i32,
            (b.color_us() == Black) as i32,
        );
    }

    fn material(&mut self, scorer: &mut impl ScorerBase, b: &Board) {
        let m = b.material();

        Piece::ALL_BAR_KING.iter().for_each(|&p| {
            scorer.accumulate(
                Feature::Piece(p),
                m.count(Color::White, p),
                m.count(Color::Black, p),
            )
        });

        scorer.accumulate(
            Attr::BishopPair.as_feature(),
            (m.count(White, Bishop) >= 2) as i32,
            (m.count(Black, Bishop) >= 2) as i32,
        );
        scorer.accumulate(
            Attr::RookPair.as_feature(),
            (m.count(White, Rook) >= 2) as i32,
            (m.count(Black, Rook) >= 2) as i32,
        );
    }

    fn position(&mut self, scorer: &mut impl ScorerBase, b: &Board) {
        // fianchetto (short)
        const W_BISHOP: Bitboard = Bitboard::G2;
        const W_KING: Bitboard = Bitboard::F1.or(Bitboard::G1).or(Bitboard::H1);
        const W_PAWNS: Bitboard = Bitboard::F2
            .or(Bitboard::G3)
            .or(Bitboard::H2)
            .or(Bitboard::H3)
            .or(Bitboard::H4);
        const W_NO_PAWNS: Bitboard = Bitboard::F3;

        const B_BISHOP: Bitboard = W_BISHOP.flip_vertical();
        const B_KING: Bitboard = W_KING.flip_vertical();
        const B_PAWNS: Bitboard = W_PAWNS.flip_vertical();
        const B_NO_PAWNS: Bitboard = W_NO_PAWNS.flip_vertical();

        let fianchetto = |c: Color| {
            let us = b.color(c);
            // let w = b.color(White);
            // let b = b.color(Black);
            let bishop = c.chooser_wb(W_BISHOP, B_BISHOP);
            let pawns = c.chooser_wb(W_PAWNS, B_PAWNS);
            let no_pawns = c.chooser_wb(W_NO_PAWNS, B_NO_PAWNS);
            let king = c.chooser_wb(W_KING, B_KING);

            ((b.bishops() & us).intersects(bishop)
                && (b.pawns() & us & pawns).popcount() >= 3
                && (b.pawns() & us).disjoint(no_pawns)
                && (b.kings() & us).intersects(king)) as i32
        };
        scorer.accumulate(
            Attr::Fianchetto.as_feature(),
            fianchetto(White),
            fianchetto(Black),
        );

        // if queen has moved but other pieces havent (FIXME! not quite exactly right (QxQ))
        let queen_early_develop = |c: Color| {
            let us = b.color(c);
            if (us & b.queens() & Bitboard::FILE_D & Bitboard::RANKS_18).is_empty() {
                (us & Bitboard::RANKS_18
                    & ((b.bishops() & (Bitboard::FILE_C.or(Bitboard::FILE_F)))
                        | (b.knights() & (Bitboard::FILE_B.or(Bitboard::FILE_G)))))
                .popcount()
            } else {
                0
            }
        };
        scorer.accumulate(
            Attr::QueenEarlyDevelop.as_feature(),
            queen_early_develop(White),
            queen_early_develop(Black),
        );
    }

    fn endgame(&mut self, scorer: &mut impl ScorerBase, b: &Board) -> bool {
        let endgame = EndGame::from_board(b);

        // match endgame.likely_outcome(b) {
        //     LikelyOutcome::DrawImmediate | LikelyOutcome::Draw => return true,
        //     _ => {},
        // };

        // self.material(scorer, b);

        if let Some(winner) = endgame.try_winner(b) {
            if let Some((metric1, metric2)) = endgame.metrics(winner, b) {
                scorer.accum(winner, Attr::WinMetric1.as_feature(), -metric1);
                scorer.accum(winner, Attr::WinMetric2.as_feature(), -metric2);

                // win specific scoring, so we award win_bonus as other features will be ignored
                scorer.accum(winner, Attr::WinBonus.as_feature(), 1);
                return true;
            }
            // award a win bonus even if we dont have win-specific scoring
            scorer.accum(winner, Attr::WinBonus.as_feature(), 1);
            return false; // TODO! we have a winner, but no specific win scoring. Do we still use just material
        }
        false
    }

    fn pst(&mut self, s: &mut impl ScorerBase, b: &Board) {
        // if s.csv() {
        for &p in &Piece::ALL {
            let w = (b.pieces(p) & b.white()).flip_vertical();
            let b = b.pieces(p) & b.black();

            for sq in (w | b).squares() {
                s.accumulate(Feature::Pst(p, sq), sq.is_in(w) as i32, sq.is_in(b) as i32);
            }
        }
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
    // - Isolated Pawn (half open) - even weaker if rooks around
    // - Backward Pawn
    // - Overly advanced
    // - Hanging Pawns -  are an open, half-isolated duo. It means that they are standing next to each other on the adjacent half-open files, usually on the fourth rank, mutually protecting their stop squares.

    #[inline]
    fn pawns(&mut self, c: Color, s: &mut impl ScorerBase, b: &Board) {
        let us = b.color(c);
        let them = b.color(c.opposite());
        let bbd = PreCalc::default();
        // self.doubled_pawns = bbd.doubled_pawns(b.color(c) & b.pawns()).popcount();
        let isolated_pawns_bb = bbd.isolated_pawns(us & b.pawns());
        let isolated_pawns = isolated_pawns_bb.popcount();
        let (pawn_atts_e, pawn_atts_w) = bbd.pawn_attacks_ew(b.pawns() & us, c);
        let pawn_atts = pawn_atts_e | pawn_atts_w;
        let pawn_duos = bbd.pawn_duos(b.pawns() & us);
        let distant_neighbours = bbd.pawn_distant_neighbours(b.pawns() & us);
        let doubled_pawns_bb = bbd.doubled_pawns(us & b.pawns());
        let pawn_isolated_doubled_bb = bbd.doubled_pawns(us & b.pawns()) & isolated_pawns_bb;
        s.set_bits(Attr::PawnIsolatedDoubled.into(), pawn_isolated_doubled_bb);
        let pawn_isolated_doubled = pawn_isolated_doubled_bb.popcount();

        // let enemy_rook_on_passer = pawn_isolated_doubled;
        let doubled_pawns = doubled_pawns_bb.popcount() - pawn_isolated_doubled;
        s.set_bits(
            Attr::PawnDoubled.into(),
            doubled_pawns_bb - pawn_isolated_doubled_bb,
        );

        let (enemy_pawn_atts_e, enemy_pawn_atts_w) =
            bbd.pawn_attacks_ew(b.pawns() & them, c.opposite());
        let enemy_pawn_atts = enemy_pawn_atts_e | enemy_pawn_atts_w;

        let mut passed_pawns_on_r7 = 0;
        let mut passed_pawns_on_r6 = 0;
        let mut passed_pawns_on_r5 = 0;
        let mut passed_pawns_on_r4 = 0;
        let mut passed_pawns = 0;
        let mut pawn_connected_r67 = 0;
        let mut pawn_connected_r345 = 0;
        let mut pawn_duo_r67 = 0;
        let mut pawn_duo_r2345 = 0;
        let mut backward = 0;
        let mut backward_half_open = 0;
        let mut _passed_duo_r67 = 0;
        let mut _passed_duo_r2345 = 0;
        // let mut pawn_directly_doubled = 0;
        let candidate_passed_pawn = 0;
        let mut _pawn_directly_doubled = 0;
        let mut _passed_connected_r345 = 0;
        let mut _passed_connected_r67 = 0;
        let semi_isolated = 0;
        let space = 0;
        let mut passers_on_rim = 0;
        let mut blockaded = 0;
        let mut blockaded_passers = 0;
        let mut rooks_behind_passer = 0;
        // let mut rammed_pawns = 0;
        let mut closedness = 0;
        let mut bishop_color_rammed_pawns = 0;
        let mut pawn_distant_neighbours_r7 = 0;
        let mut pawn_distant_neighbours_r6 = 0;
        let mut pawn_distant_neighbours_r5 = 0;

        for p in (b.pawns() & us).squares() {
            // self.doubled_pawns += is_doubled as i32;
            // we still count doubled pawns as passed pawns (since 0.3.37)
            let rank_num = p.rank_number_as_white(c) as i32;
            let pawn_stop = bbd.pawn_stop(c, p);
            // use pawns not pawns&them so we only count front of doubled pawns (+8 elo in sp)
            let is_passed =
                (bbd.pawn_front_span_union_attack_span(c, p) & b.pawns() & them).is_empty();
            // self.passed_pawns += is_passed as i32;
            // let rank7 = c.chooser_wb(Bitboard::RANK_7, Bitboard::RANK_2);
            // let rank6 = c.chooser_wb(Bitboard::RANK_6, Bitboard::RANK_3);
            // let rank5 = c.chooser_wb(Bitboard::RANK_5, Bitboard::RANK_4);
            // all pawns on r7 are passed as an opponent pawn cannot be on rank 8
            let is_passer_on_rim = is_passed && p.is_in(Bitboard::RIM);
            passers_on_rim += is_passer_on_rim as i32;
            s.set_bits(Attr::PassersOnRim.into(), p.as_bb().iff(is_passer_on_rim));

            let is_blockaded = pawn_stop.intersects(them);
            blockaded += is_blockaded as i32;
            blockaded_passers += (is_blockaded && is_passed) as i32;
            rooks_behind_passer +=
                (is_passed && (bbd.pawn_front_span(c.opposite(), p) & b.rooks() & us).any()) as i32;

            let rammed = pawn_stop.intersects(them & b.pawns());
            if rammed & p.is_in((b.bishops() & us).squares_of_matching_color()) {
                bishop_color_rammed_pawns +=
                    1 + (p.as_bb() | pawn_stop).intersects(Bitboard::CENTER_16_SQ) as i32;
            }
            let _nearly_rammed =
                bbd.pawn_double_stop(c, p).intersects(them & b.pawns()) || is_blockaded;
            // rammed_pawns += rammed as i32;
            closedness += rammed as i32;
            // try and ensure closedness is symmetric
            if p.is_in(Bitboard::CENTER_16_SQ) || pawn_stop.intersects(Bitboard::CENTER_16_SQ) {
                closedness += rammed as i32;
            }
            s.set_bits(Attr::RammedPawns.into(), p.as_bb().iff(rammed));

            // // old let is_passed = (bbd.pawn_front_span_union_attack_span(c, p) & b.pawns()).is_empty();
            // let blockaded = pawn_stop.intersects(them);

            // if p.is_in(doubled_pawns) && pawn_stop.intersects(doubled_pawns) {
            //     self.doubled_pawns -=1;
            //     self.pawn_directly_doubled +=1;
            // }

            // let _semi_isolated = if !p.is_in(isolated_pawns_bb)
            //     && (((bbd.neighbouring_files(p) & bbd.within_chebyshev_distance_inclusive(p, 2))
            //         - p.as_bb()
            //         - p.file())
            //         & b.pawns()
            //         & us)
            //         .is_empty()
            // {
            //     // semi_isolated += 1;
            //     scorer.set_bits(Attr::SemiIsolated.into(), p.as_bb());
            //     true
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
            //         if ours >= theirs && p.rank_index_as_white(c) >= 4 {
            //             candidate_passed_pawn += 1;
            //         }
            // }
            // self.enemy_rook_on_passer += (!p.is_in(isolated_pawns_bb)) as i32;

            // passers
            if is_passed {
                passed_pawns += 1;
                s.set_bits(Attr::PawnPassed.into(), p.as_bb());
                match rank_num {
                    7 => passed_pawns_on_r7 += 1,
                    6 => passed_pawns_on_r6 += 1,
                    5 => passed_pawns_on_r5 += 1,
                    4 => passed_pawns_on_r4 += 1,
                    _ => {}
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
            //     // if !p.is_in(isolated_pawns_bb)
            //     //     &&
            //     //     (((bbd.neighbouring_files(p) & bbd.within_chebyshev_distance_inclusive(p, 3)) - p.as_bb()) & b.pawns() & us).is_empty() {
            //     //     self.semi_isolated += 1;
            //     // }
            //     continue;
            // }
            // self.blockaded += blockaded as i32;
            // let rammed = pawn_stop.intersects(them & b.pawns());
            // self.space += (rammed as i32) * rank_index * (1 + p.is_in(FILE_D | FILE_E) as i32);

            // space
            // if (rammed || nearly_rammed) && p.is_in(pawn_duos | pawn_atts) {
            //     space += rank_index + p.is_in(Bitboard::FILE_D | Bitboard::FILE_E) as i32;
            // }

            if p.is_in(pawn_atts) {
                match rank_num {
                    6 | 7 => {
                        pawn_connected_r67 += 1;
                        s.set_bits(Attr::PawnConnectedR67.into(), p.as_bb());
                    }
                    _ => {
                        pawn_connected_r345 += 1;
                        s.set_bits(Attr::PawnConnectedR345.into(), p.as_bb());
                    }
                }
            }
            if p.is_in(distant_neighbours) {
                match rank_num {
                    7 => pawn_distant_neighbours_r7 += 1,
                    6 => pawn_distant_neighbours_r6 += 1,
                    5 => pawn_distant_neighbours_r5 += 1,
                    _ => {}
                }
            }

            if p.is_in(pawn_duos) {
                match rank_num {
                    6 | 7 => pawn_duo_r67 += 1,
                    _ => pawn_duo_r2345 += 1,
                }
            } else {
                // half open backward pawns - cannot be defended by other pawns and cannot move fwd
                if pawn_stop.intersects(enemy_pawn_atts)
                    && bbd
                        .pawn_attack_span(c.opposite(), p)
                        .disjoint(b.pawns() & us)
                {
                    // we already know from duo-else-clause there is no pawn either side too
                    if (bbd.pawn_front_span(c, p) & b.pawns() & them).is_empty() {
                        //  &&
                        //  (b.rooks_or_queens() & them).any() { //
                        backward_half_open += 1;
                        s.set_bits(Attr::BackwardHalfOpen.into(), p.as_bb());
                    } else {
                        backward += 1;
                        s.set_bits(Attr::Backward.into(), p.as_bb());
                    }
                }
            }
        }
        // self.doubled_pawns = bbd.doubled_pawns(us & b.pawns()).popcount();
        // lots of rammed pawns and having a knight an advantage
        // rammed_pawns = rammed_pawns * rammed_pawns * (b.knights() & us).any() as i32;

        let bishop_far_pawns = ((b.pawns() & Bitboard::FILE_A.or(Bitboard::FILE_B)).any()
            && (b.pawns() & Bitboard::FILE_G.or(Bitboard::FILE_H)).any()
            && b.rooks_or_queens().is_empty()) as i32
            * (b.bishops() & us).popcount();

        let knight_closedness = closedness * closedness * (b.knights() & us).popcount();
        let bishop_closedness = closedness * closedness * (b.bishops() & us).popcount();
        let rook_closedness = closedness * closedness * (b.rooks() & us).popcount();
        // space adjustment
        // space = match b.pawns().popcount() {
        //     10.. => space,
        //     8 | 9 => space / 2,
        //     _ => 0,
        // };

        s.accum(c, Attr::BishopFarPawns.as_feature(), bishop_far_pawns);
        s.accum(
            c,
            Attr::BishopColorRammedPawns.as_feature(),
            bishop_color_rammed_pawns,
        );
        s.accum(c, Attr::Closedness.as_feature(), closedness * closedness);
        s.accum(c, Attr::KnightClosedness.as_feature(), knight_closedness);
        s.accum(c, Attr::BishopClosedness.as_feature(), bishop_closedness);
        s.accum(c, Attr::RookClosedness.as_feature(), rook_closedness);
        s.accum(c, Attr::PawnDoubled.as_feature(), doubled_pawns);
        s.accum(
            c,
            Attr::PawnDirectlyDoubled.as_feature(),
            _pawn_directly_doubled,
        );
        s.accum(c, Attr::PawnIsolated.as_feature(), isolated_pawns);
        s.accum(c, Attr::SemiIsolated.as_feature(), semi_isolated);
        s.accum(
            c,
            Attr::PawnDistantNeighboursR7.as_feature(),
            pawn_distant_neighbours_r7,
        );
        s.accum(
            c,
            Attr::PawnDistantNeighboursR6.as_feature(),
            pawn_distant_neighbours_r6,
        );
        s.accum(
            c,
            Attr::PawnDistantNeighboursR5.as_feature(),
            pawn_distant_neighbours_r5,
        );
        s.accum(c, Attr::PawnPassed.as_feature(), passed_pawns);
        s.accum(c, Attr::PawnPassedR7.as_feature(), passed_pawns_on_r7);
        s.accum(c, Attr::PawnPassedR6.as_feature(), passed_pawns_on_r6);
        s.accum(c, Attr::PawnPassedR5.as_feature(), passed_pawns_on_r5);
        s.accum(c, Attr::PawnPassedR4.as_feature(), passed_pawns_on_r4);
        s.accum(c, Attr::PassersOnRim.as_feature(), passers_on_rim);
        s.accum(
            c,
            Attr::CandidatePassedPawn.as_feature(),
            candidate_passed_pawn,
        );
        s.accum(c, Attr::Blockaded.as_feature(), blockaded);
        s.accum(c, Attr::BlockadedPassers.as_feature(), blockaded_passers);
        s.accum(c, Attr::RooksBehindPasser.as_feature(), rooks_behind_passer);
        s.accum(
            c,
            Attr::PawnIsolatedDoubled.as_feature(),
            pawn_isolated_doubled,
        );
        // scorer.accum(c, Attr::RammedPawns.as_feature(), rammed_pawns);
        s.accum(c, Attr::Space.as_feature(), space);
        s.accum(c, Attr::PawnConnectedR67.as_feature(), pawn_connected_r67);
        s.accum(c, Attr::PawnConnectedR345.as_feature(), pawn_connected_r345);
        s.accum(
            c,
            Attr::PassedConnectedR67.as_feature(),
            _passed_connected_r67,
        );
        s.accum(
            c,
            Attr::PassedConnectedR345.as_feature(),
            _passed_connected_r345,
        );
        s.accum(c, Attr::PawnDuoR67.as_feature(), pawn_duo_r67);
        s.accum(c, Attr::PawnDuoR2345.as_feature(), pawn_duo_r2345);
        s.accum(c, Attr::PassedDuoR67.as_feature(), _passed_duo_r67);
        s.accum(c, Attr::PassedDuoR2345.as_feature(), _passed_duo_r2345);
        s.accum(c, Attr::BackwardHalfOpen.as_feature(), backward_half_open);
        s.accum(c, Attr::Backward.as_feature(), backward);
    }

    #[inline]
    fn king_safety(&mut self, c: Color, s: &mut impl ScorerBase, b: &Board) {
        let us = b.color(c);
        let k = b.kings() & us;
        if k.is_empty() {
            return;
        }
        let them = b.color(c.opposite());
        let p = b.pawns() & us;
        let ksq = k.square();
        let bb = PreCalc::default();
        let castling_rights = b.castling().contains(CastlingRights::king_side_right(c)) as i32
            + b.castling().contains(CastlingRights::queen_side_right(c)) as i32;

        let uncastled = (bb
            .within_chebyshev_distance_inclusive(Square::E8, 1)
            .or(bb.within_chebyshev_distance_inclusive(Square::E1, 1))
            .contains(k)) as i32;

        //
        // tropism
        //
        let d1 = bb.within_chebyshev_distance_inclusive(ksq, 1);
        let d2 = bb.within_chebyshev_distance_inclusive(ksq, 2);
        let d3 = bb.within_chebyshev_distance_inclusive(ksq, 3);
        let d4 = bb.within_chebyshev_distance_inclusive(ksq, 4);
        let king_tropism_d1 = (d1 & (b.pawns() | b.kings()) & them).popcount()
            + (d1 & (b.knights() | b.bishops()) & them).popcount() * 2
            + (d1 & (b.rooks()) & them).popcount() * 2
            + (d1 & (b.queens()) & them).popcount() * 4;

        let king_tropism_d2 = (d2 & (b.pawns() | b.kings()) & them).popcount()
            + (d2 & (b.knights() | b.bishops()) & them).popcount() * 2
            + (d2 & (b.rooks()) & them).popcount() * 2
            + (d2 & (b.queens()) & them).popcount() * 4;

        let king_tropism_d3 = (d3 & (b.pawns() | b.kings()) & them).popcount()
            + (d3 & (b.knights() | b.bishops()) & them).popcount() * 2
            + (d3 & (b.rooks()) & them).popcount() * 2
            + (d3 & (b.queens()) & them).popcount() * 4;

        let king_tropism_d4 = (d4 & (b.pawns() | b.kings()) & them).popcount()
            + (d3 & (b.knights() | b.bishops()) & them).popcount() * 2
            + (d3 & (b.rooks()) & them).popcount() * 2
            + (d3 & (b.queens()) & them).popcount() * 4;
        s.accum(c, Attr::TropismD1.as_feature(), king_tropism_d1);
        s.accum(c, Attr::TropismD2.as_feature(), king_tropism_d2);
        s.accum(c, Attr::TropismD3.as_feature(), king_tropism_d3);
        s.accum(c, Attr::TropismD4.as_feature(), king_tropism_d4);

        //
        // pawn shield
        //
        let (adjacent, nearby) = bb.adjacent_and_nearby_pawn_shield(c, ksq);
        let adjacent_shield = (p & adjacent).popcount();
        s.set_bits(Attr::PawnAdjacentShield.into(), adjacent & p);
        let nearby_shield = (p & nearby).popcount();
        s.set_bits(Attr::PawnNearbyShield.into(), nearby & p);
        let isolated_shield = bb.isolated_pawns(p) & (adjacent | nearby);
        s.accum(c, Attr::PawnAdjacentShield.as_feature(), adjacent_shield);
        s.accum(c, Attr::PawnNearbyShield.as_feature(), nearby_shield);
        s.accum(
            c,
            Attr::PawnShieldFaulty.as_feature(),
            isolated_shield.popcount(),
        );
        let king_safety_bonus = if b.queens().any() {
            adjacent_shield + nearby_shield
        } else {
            0
        };
        s.accum(c, Attr::KingSafetyBonus.as_feature(), king_safety_bonus);

        //
        // storming pawms - from POV of defending king color c
        //
        // ranks 4-7
        let storm_ranks = Bitboard::home_half(c.opposite())
            .shift(c.backward())
            .shift(c.backward())
            .shift(c.backward());
        let storm_files = ksq.kq_side();
        let storm = storm_ranks & storm_files;
        let storming_pawns = b.pawns() & them & storm;
        let blocked = storming_pawns & (b.pawns() & us).shift(c.forward());
        let unblocked = storming_pawns - blocked;
        s.set_bits(Attr::StormBlocked.into(), blocked);
        s.set_bits(Attr::StormUnblocked.into(), unblocked);
        let blocked_r3 = blocked & c.rank_as_white_num(3);
        let blocked_r4 = blocked & c.rank_as_white_num(4);
        let unblocked_r23 = unblocked & (c.rank_as_white_num(2) | c.rank_as_white_num(3));
        s.accum(
            c,
            Attr::StormBlocked.as_feature(),
            blocked.popcount() - blocked_r3.popcount() - blocked_r4.popcount(),
        );
        s.accum(c, Attr::StormBlockedR3.as_feature(), blocked_r3.popcount());
        s.accum(c, Attr::StormBlockedR4.as_feature(), blocked_r4.popcount());
        s.accum(
            c,
            Attr::StormUnblocked.as_feature(),
            unblocked.popcount() - unblocked_r23.popcount(),
        );
        s.accum(
            c,
            Attr::StormUnblockedR23.as_feature(),
            unblocked_r23.popcount(),
        );

        let open_files_near_king_bb = d3 & ksq.rank() & bb.open_files(b.pawns());
        let open_files_near_king = (open_files_near_king_bb).popcount();
        let open_files_adjacent_king = (d1 & ksq.rank() & bb.open_files(b.pawns())).popcount();
        let rq_on_open_files_near_king =
            (open_files_near_king_bb.file_flood() & b.rooks_or_queens() & them).popcount();

        let king_trapped_on_back_rank = (b.rooks_or_queens().any()
            && k.intersects(Bitboard::RANKS_18)
            // && Bitboard::RANKS_18 & ksq.rank() & us == k
            && (d1 - Bitboard::RANKS_18 - b.occupied()).is_empty())
            as i32;

        let checkers = b.checkers_of(c).popcount();
        //        self.attacks_on_opponent_king_area += (our_raw_attacks & bb.within_chebyshev_distance_inclusive(ksq, 1)).popcount();

        let pieces_near_king = (d2 & b.occupied() & us & (b.queens() | b.knights())).popcount();
        let pinned_near_king = (b.pinned(c) & d1).popcount();
        let pinned_far = (b.pinned(c)).popcount() - pinned_near_king;
        let discovered_checks = (b.discoverer(c) - b.pawns()).popcount();

        let tempo_safety_d12 = if b.color_them() == c {
            king_tropism_d1 + king_tropism_d2 + open_files_adjacent_king + open_files_near_king
        } else {
            0
        };
        s.accum(
            c,
            Attr::OpenFilesNearKing.as_feature(),
            open_files_near_king,
        );
        s.accum(
            c,
            Attr::OpenFilesAdjacentKing.as_feature(),
            open_files_adjacent_king,
        );
        s.accum(c, Attr::TempoSafety.as_feature(), tempo_safety_d12);
        s.accum(
            c,
            Attr::KingTrappedOnBackRank.as_feature(),
            king_trapped_on_back_rank,
        );
        s.accum(
            c,
            Attr::RqOnOpenFilesNearKing.as_feature(),
            rq_on_open_files_near_king,
        );

        s.accum(c, Attr::CastlingRights.as_feature(), castling_rights);
        s.accum(c, Attr::Uncastled.as_feature(), uncastled);
        s.accum(c, Attr::Checkers.as_feature(), checkers);
        s.accum(c, Attr::PiecesNearKing.as_feature(), pieces_near_king);
        s.accum(c, Attr::PinnedNearKing.as_feature(), pinned_near_king);
        s.accum(c, Attr::PinnedFar.as_feature(), pinned_far);
        s.accum(c, Attr::DiscoveredChecks.as_feature(), discovered_checks);
    }


    fn mobility(&mut self, c: Color, s: &mut impl ScorerBase, b: &Board) {
        let bb = PreCalc::default();
        let us = b.color(c);
        let opponent = c.opposite();
        let them = b.color(opponent);
        let occ = them | us;
        let open_files = bb.open_files(b.pawns());
        let semi_open_files = bb.open_files(b.pawns() & us) - open_files; // free of our pawns
        let their_p = b.pawns() & them;
        let our_p = b.pawns() & us;
        let (pe, pw) = bb.pawn_attacks_ew(their_p, opponent);
        let (ope, opw) = bb.pawn_attacks_ew(our_p, c);
        let pa = pe | pw;
        let our_pa = ope | opw;
        let bi = b.bishops() & them;
        let ni = b.knights() & them;
        let r = b.rooks() & them;


        // general
        let mut partially_trapped_pieces = 0;
        let mut fully_trapped_pieces = 0;
        let mut attacks_near_king = 0;
        // let mut moves_near_king = 0;
        let mut move_squares = 0;
        let mut non_pawn_defended_moves = 0;
        let mut center_attacks = 0;
        let mut all_attacks = Bitboard::empty();
        let mut double_attacks = Bitboard::empty();

        // match free_squares {
        //     0 => rook_moves[0] += 1,
        //     1 => rook_moves[1] += 1,
        //     2 => rook_moves[2] += 1,
        //     3 | 4 | 5 => rook_moves[3] += 1,
        //     6..=10 => rook_moves[4] += 1,
        //     _ => rook_moves[5] += 1,
        // }

        // fewer pawns rook bonus

        // knight
        let mut knight_connected = false;
        let knight_outpost = 0;
        let knight_outpost_rook_safe = 0;
        let mut knight_outpost_pawn_defended = 0;
        let mut knight_forks = 0;
        let mut knight_attacks_center = 0;
        let knight_trapped = 0;

        // bishop
        let mut bishop_outposts = 0;
        let bishop_trapped = 0;

        // rook
        let mut enemy_pawns_on_rook_rank = 0;
        let connected_rooks = false;
        let rook_trapped = 0;

        // queen
        let queen_trapped = 0;
        let queens_on_open_files = (open_files & us & b.queens()).popcount();

        let k = b.kings() & them;
        
        let (adjacent, nearby) =  if k.any() {
            bb.adjacent_and_nearby_pawn_shield(opponent, k.square())
        } else {
            (Bitboard::empty(), Bitboard::empty())
        };
        let pawn_shield = adjacent | nearby;
        // let pawn_shield = bb.within_chebyshev_distance_inclusive(ksq, 1);

        for sq in ((b.knights() | b.rooks() | b.bishops() | b.queens()) & us).squares() {
            let p = b.piece_unchecked(sq);

            // non-pawn-defended empty or oppoent sq
            // include "attacking" our own pieces
            let our_raw_attacks = bb.attacks(c, p, Bitboard::empty(), occ, sq);

            let our_attacks = our_raw_attacks - us - pa;
            center_attacks += (our_attacks & Bitboard::CENTER_16_SQ).popcount();

            let piece_move_squares = (our_attacks - occ).popcount();

            // those attacks on enemy that arent pawn defended and cant attack back
            let asym_attacks;
            match p {
                Piece::Knight => {
                    knight_attacks_center += (our_raw_attacks & Bitboard::CENTER_4_SQ).popcount();
                    knight_connected |= (our_raw_attacks & b.knights() & us).any();
                    for sq in (our_raw_attacks).squares() {
                        let atts = bb.knight_attacks(sq);
                        if (atts & them & (b.queens() | b.bishops() | b.rooks() | b.kings()))
                            .popcount()
                            >= 2
                            && b.color_us() == c
                        {
                            knight_forks += 1;
                        }
                    }
                    if bb.pawn_attack_span(c, sq).disjoint(their_p)
                        && sq.is_in(Bitboard::home_half(opponent))
                        && sq.is_in(
                            Bitboard::FILE_C
                                | Bitboard::FILE_D
                                | Bitboard::FILE_E
                                | Bitboard::FILE_F,
                        )
                    {
                        // knight_outpost += 1;
                        if sq.is_in(our_pa) {
                            knight_outpost_pawn_defended += 1;
                        }
                        // if bb.pawn_stop(c, sq).intersects(their_p) {
                        //     knight_outpost_rook_safe += 1;
                        // }
                    }
                    asym_attacks = ((our_attacks & them) - ni).popcount();
                    // knight_trapped += (piece_move_squares + asym_attacks == 0) as i32;
                }
                Piece::Bishop => {
                    if bb.pawn_attack_span(c, sq).disjoint(their_p)
                        && sq.is_in(Bitboard::home_half(opponent))
                        && sq.is_in(our_pa)
                    {
                        bishop_outposts += 1;
                    }
                    asym_attacks = ((our_attacks & them) - bi).popcount();
                    // bishop_trapped += (piece_move_squares + asym_attacks == 0) as i32;
                }
                Piece::Rook => {
                    // connected_rooks |= (our_raw_attacks & b.rooks() & us).any();
                    enemy_pawns_on_rook_rank +=
                        (sq.rank() & b.pawns() & them & Bitboard::home_half(opponent)).popcount()
                            as i32;
                    asym_attacks = ((our_attacks & them) - r).popcount();
                    // rook_trapped += (piece_move_squares + asym_attacks == 0) as i32;
                }
                Piece::Queen => {
                    asym_attacks = (our_attacks & them).popcount();
                    // queen_trapped += (piece_move_squares + asym_attacks == 0) as i32;
                }
                _ => unreachable!(),
            };
            // trapped piece
            if piece_move_squares + asym_attacks == 1 {
                partially_trapped_pieces += 1;
            }
            if piece_move_squares == 0 {
                fully_trapped_pieces += 1;
            }
            // self.mv.push((p, our_attacks.popcount()));
            move_squares += piece_move_squares;
            non_pawn_defended_moves += asym_attacks;

            double_attacks |= ((our_raw_attacks & them) - us) & all_attacks;
            all_attacks |= (our_raw_attacks & them) - us;

            // moves_near_king += (our_raw_attacks
            //     & bb.within_chebyshev_distance_inclusive(ksq, 1)
            //     & !b.occupied())
            // .popcount();
            attacks_near_king +=
                (our_raw_attacks & pawn_shield & !us).any() as i32 * (p.centipawns() / 64);
            // * match p {
            //     Piece::Bishop => 10,
            //     Piece::Knight => 20,
            //     Piece::Rook => 30,
            //     Piece::Queen => 50,
            //     _ => unreachable!(),
            // };
            s.set_bits(
                Attr::AttacksNearKing.as_feature(),
                our_raw_attacks & pawn_shield,
            );
        }

        s.accum(c, Attr::AttacksNearKing.as_feature(), attacks_near_king);

        // let double_attacks_near_king = double_attacks & bb.within_chebyshev_distance_inclusive(ksq, 1);
        //
        // knight
        //
        s.accum(c, Attr::TempoKnightForks.as_feature(), knight_forks);
        s.accum(c, Attr::KnightOutpost.as_feature(), knight_outpost);
        s.accum(
            c,
            Attr::KnightOutpostPawnDefended.as_feature(),
            knight_outpost_pawn_defended,
        );
        s.accum(
            c,
            Attr::KnightOutpostRookSafe.as_feature(),
            knight_outpost_rook_safe,
        );
        s.accum(
            c,
            Attr::KnightConnected.as_feature(),
            knight_connected as i32,
        );
        s.accum(
            c,
            Attr::KnightAttacksCenter.as_feature(),
            knight_attacks_center,
        );
        s.accum(c, Attr::KnightTrapped.as_feature(), knight_trapped);

        //
        // Bishop
        //
        let bishop_color_pawns = |c: Color| {
            if (b.bishops() & b.color(c)).exactly_one() {
                if Bitboard::WHITE_SQUARES.contains(b.bishops() & b.color(c)) {
                    return (b.pawns() & b.color(c) & Bitboard::WHITE_SQUARES).popcount()
                        - (b.pawns() & b.color(c) & Bitboard::BLACK_SQUARES).popcount();
                } else if Bitboard::BLACK_SQUARES.contains(b.bishops() & b.color(c)) {
                    return (b.pawns() & b.color(c) & Bitboard::BLACK_SQUARES).popcount()
                        - (b.pawns() & b.color(c) & Bitboard::WHITE_SQUARES).popcount();
                }
            }
            0
        };
        s.accumulate(
            Attr::BishopColorPawns.as_feature(),
            bishop_color_pawns(White),
            bishop_color_pawns(Black),
        );
        s.accum(c, Attr::BishopOutposts.as_feature(), bishop_outposts);
        s.accum(c, Attr::BishopTrapped.as_feature(), bishop_trapped);

        //
        // Rook
        //
        let doubled_rooks = ((b.rooks() & us).two_or_more()
            && (b.rooks() & us).first_square().file_index()
                == (b.rooks() & us).last_square().file_index()) as i32;
        let doubled_rooks_open_file =
            (doubled_rooks == 1 && (open_files & b.rooks() & us).popcount() >= 2) as i32;
        let rook_on_open_file = (open_files & us & b.rooks()).popcount();
        s.accum(c, Attr::RookOpenFile.as_feature(), rook_on_open_file);

        let rook_semi_open_file = (semi_open_files & us & b.rooks()).popcount();
        s.accum(c, Attr::RookSemiOpenFile.as_feature(), rook_semi_open_file);

        s.set_bits(Attr::RookOpenFile.into(), open_files & us & b.rooks());
        s.accum(c, Attr::ConnectedRooks.as_feature(), connected_rooks as i32);
        s.accum(c, Attr::DoubledRooks.as_feature(), doubled_rooks);
        s.accum(
            c,
            Attr::DoubledRooksOpenFile.as_feature(),
            doubled_rooks_open_file,
        );
        s.accum(
            c,
            Attr::EnemyPawnsOnRookRank.as_feature(),
            enemy_pawns_on_rook_rank,
        );
        s.accum(c, Attr::RookTrapped.as_feature(), rook_trapped);

        //
        // General
        //
        // s.set_bits(Attr::DoubleAttacks.into(), double_attacks);
        // s.accum(c, Attr::DoubleAttacks.as_feature(), double_attacks.popcount());
        // s.accum(
        //     c,
        //     Attr::DoubleAttacksNearKing.as_feature(),
        //     double_attacks_near_king.popcount(),
        // );
        // s.accum(c, Attr::MovesNearKing.as_feature(), moves_near_king);
        s.accum(c, Attr::CenterAttacks.as_feature(), center_attacks);
        s.accum(c, Attr::UndefendedSq.as_feature(), move_squares);
        s.accum(
            c,
            Attr::UndefendedPiece.as_feature(),
            non_pawn_defended_moves,
        );
        s.accum(c, Attr::TrappedPiece.as_feature(), fully_trapped_pieces);
        s.accum(
            c,
            Attr::PartiallyTrappedPiece.as_feature(),
            partially_trapped_pieces,
        );
        s.accum(c, Attr::QueenOpenFile.as_feature(), queens_on_open_files);
        s.accum(c, Attr::QueenTrapped.as_feature(), queen_trapped);
    }




}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::eval::eval::Eval;
    use crate::eval::scorer::{ExplainScore, TotalScore};
    use crate::eval::weight::Weight;
    use crate::infra::black_box;
    use crate::infra::profiler::*;
    use crate::phaser::Phaser;
    use crate::test_log::test;
    use crate::Position;
    // use crate::utils::StringUtils;

    #[test]
    fn test_scoring() {
        let positions = Catalog::example_game();
        for pos in &positions {
            let b = pos.board();
            // let mut scorer1 = ExplainScorer::new(b.to_fen(), false);
            // scorer1.set_phase(b.phase(&phaser));
            // let mut model = Model::from_board(b, b.phase(&phaser), Switches::ALL_SCORING);
            // model.csv = false;
            // eval.predict(&model, &mut scorer1);
            // black_box(&scorer1);
            // let w1 = scorer1.total();

            // let mut scorer2 = ExplainScorer::new(b.to_fen(), false);
            // Calc::score(&mut scorer2, &b, &eval, &phaser);
            // black_box(&scorer2);
            // let w2 = scorer2.total();

            let mut scorer3 = ExplainScore::default();
            Calc::new(&b).score(&mut scorer3, &b);
            black_box(&scorer3);
            let _w3 = scorer3.total();

            // assert_eq!(w1, w2, "{}\n", pos);
            // assert_eq!(w1, w2, "{}\n{}\n===========\n{}\n", pos, scorer1, scorer2);
            // assert_eq!(w1, w3, "{}\n{}\n===========\n{}\n", pos, scorer1, scorer3);
        }
    }

    #[ignore]
    #[test]
    fn bench_scoring() {
        // fn bench_old(b: &Board, p: &Phaser, e: &Eval, pr: &mut Profiler) -> Weight {
        //     let mut scorer1 = ExplainScorer::new(b.to_fen(), false);
        //     // let mut scorer1 = ModelScore::new();
        //     scorer1.set_phase(b.phase(p));
        //     pr.start();
        //     for _ in 0..1000 {
        //         let mut model = Model::from_board(b, b.phase(p), Switches::ALL_SCORING);
        //         model.csv = false;
        //         e.predict(&model, &mut scorer1);
        //         black_box(&scorer1);
        //         // scorer1 = ExplainScorer::new(b.to_fen());
        //     }
        //     pr.stop();
        //     scorer1.total()
        // }

        fn bench_new(b: &Board, phr: &Phaser, e: &Eval, pr: &mut Profiler) -> Weight {
            let mut scorer2 = TotalScore::new(&e.feature_weights, b.phase(phr));
            // scorer2.csv = false;
            // let mut scorer2 = ModelScore::new();

            pr.start();
            for _ in 0..1000 {
                Calc::new(&b).score(&mut scorer2, &b);
                black_box(&scorer2);
                // scorer2 = ExplainScorer::new(b.to_fen());
            }
            pr.stop();
            scorer2.total()
        }

        let _prof_old = Profiler::new("old".into());
        let mut prof_new = Profiler::new("new".into());
        let positions = Catalog::example_game();
        let eval = Eval::new();
        let phr = Phaser::default();
        for pos in &positions {
            // let _w1 = bench_old(&pos.board(), &phaser, &eval, &mut prof_old);
            let _w2 = bench_new(&pos.board(), &phr, &eval, &mut prof_new);
        }
    }

    #[test]
    fn test_model_pawn() {
        fn score_for(s: &str) -> ExplainScore {
            let e = Eval::new();
            let phr = Phaser::default();
            let pos = Position::parse_epd(s).unwrap();
            let mut sc = ExplainScore::new(
                pos.board().phase(&phr),
                format!("{:#}", pos.board().to_string()),
            );
            Calc::new(&pos.board()).score(&mut sc, pos.board());
            sc
        }
        let sc = score_for(
            r"
            ........
            .....P..
            P....P..
            P.p.....
            .....p..
            ........
            .P.P...p
            ........ w KQkq - 1 1",
        );
        assert_eq!(sc.value(Attr::PawnDoubled.into()), 1);
        assert_eq!(sc.value(Attr::PawnIsolatedDoubled.into()), 1);
        assert_eq!(sc.value(Attr::PawnPassed.into()), 4 - 2);
        assert_eq!(sc.value(Attr::PassersOnRim.into()), 2 - 1);
        assert_eq!(sc.value(Attr::PawnPassedR7.into()), 1 - 1);
        assert_eq!(sc.value(Attr::PawnPassedR6.into()), 2 - 0);
        assert_eq!(sc.value(Attr::SemiIsolated.into()), 0); // semi-isolated not used
        println!("{sc:#}");
    }

    #[test]
    fn king_safety_test() {
        let bb = PreCalc::default();
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
}
