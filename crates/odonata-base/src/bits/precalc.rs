use std::collections::HashMap;
use std::fmt;

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tabled::builder::Builder;

use crate::bits::bb_hyperbola::Hyperbola;
use crate::bits::bitboard::{Bitboard, Dir};
use crate::bits::square::Square;
use crate::piece::{Color, FlipSide, Piece};
use crate::prelude::Board;

fn display_bitboards(bitboards: &HashMap<String, Bitboard>) -> String {
    let mut builder = Builder::new();
    for y in &bitboards.iter().chunks(7) {
        let mut row = vec![];
        for (var_name, bb) in y {
            let desc = var_name.replace("white_", "w_").replace("black_", "b_");
            row.push(format!("{desc}\n{bb:#}"));
        }
        builder.push_record(row);
    }
    let tab = builder.build();
    tab.to_string()
}

#[repr(align(64))]
pub struct PreCalc {
    king_moves:            [Bitboard; 64],
    knight_moves:          [Bitboard; 64],
    bishop_moves:          [Bitboard; 64],
    rook_moves:            [Bitboard; 64],
    pawn_front_span:       [[Bitboard; 64]; 2],
    pawn_push:             [[Bitboard; 64]; 2],
    pawn_double_push:      [[Bitboard; 64]; 2],
    pawn_capture_east:     [[Bitboard; 64]; 2],
    pawn_capture_west:     [[Bitboard; 64]; 2],
    pawn_attack_span:      [[Bitboard; 64]; 2],
    strictly_between:      [[Bitboard; 64]; 64],
    line:                  [[Bitboard; 64]; 64],
    surround:              [[Bitboard; 8]; 64],
    sliding_piece_attacks: Hyperbola,
}

#[allow(long_running_const_eval)]
static PRECALC: PreCalc = PreCalc::new();

impl PreCalc {
    // doesnt impl Default as too large to copy by value
    #[inline]
    pub fn instance() -> &'static Self {
        &PRECALC
    }

    fn display(&self, s1: Square, s2: Square, c: Color) -> String {
        let mut bbs = HashMap::new();
        bbs.insert("king".to_string(), self.king_moves[s1]);
        bbs.insert("knight".to_string(), self.knight_moves[s1]);
        bbs.insert("bishop".to_string(), self.bishop_moves[s1]);
        bbs.insert("rook".to_string(), self.rook_moves[s1]);
        bbs.insert("pawn front span".to_string(), self.pawn_front_span[c][s1]);
        bbs.insert("pawn push".to_string(), self.pawn_push[c][s1]);
        bbs.insert("pawn double push".to_string(), self.pawn_double_push[c][s1]);
        bbs.insert("pawn cap east".to_string(), self.pawn_capture_east[c][s1]);
        bbs.insert("pawn cap west".to_string(), self.pawn_capture_west[c][s1]);
        bbs.insert("pawn att span".to_string(), self.pawn_attack_span[c][s1]);
        bbs.insert("strictly between".to_string(), self.strictly_between[s1][s2]);
        bbs.insert("line".to_string(), self.line[s1][s2]);
        bbs.insert("surround[2]".to_string(), self.surround[s1][2]);
        format!("{s1} {s2} {c}\n{}", display_bitboards(&bbs))
    }

    const fn new() -> Self {
        let mut bbs = Self {
            sliding_piece_attacks: Hyperbola::new(),
            king_moves:            [Bitboard::EMPTY; 64],
            knight_moves:          [Bitboard::EMPTY; 64],
            bishop_moves:          [Bitboard::EMPTY; 64],
            rook_moves:            [Bitboard::EMPTY; 64],
            pawn_front_span:       [[Bitboard::EMPTY; 64]; 2],
            pawn_push:             [[Bitboard::EMPTY; 64]; 2],
            pawn_double_push:      [[Bitboard::EMPTY; 64]; 2],
            pawn_capture_east:     [[Bitboard::EMPTY; 64]; 2],
            pawn_capture_west:     [[Bitboard::EMPTY; 64]; 2],
            pawn_attack_span:      [[Bitboard::EMPTY; 64]; 2],
            strictly_between:      [[Bitboard::EMPTY; 64]; 64],
            line:                  [[Bitboard::EMPTY; 64]; 64],
            surround:              [[Bitboard::EMPTY; 8]; 64],
        };

        let mut s = 0;
        while s < 64 {
            let sq = Square::from_usize(s);
            let mut color = 0;
            while color < 2 {
                let c = [Color::White, Color::Black][color];
                bbs.pawn_front_span[color][s] = sq.as_bb().rays(c.forward());
                bbs.pawn_push[color][s] = sq.as_bb().shift(c.forward());
                bbs.pawn_double_push[color][s] = sq.as_bb().shift(c.forward()).shift(c.forward());
                let e = sq.as_bb().shift(c.pawn_capture_east());
                let w = sq.as_bb().shift(c.pawn_capture_west());
                bbs.pawn_capture_east[color][s] = e;
                bbs.pawn_capture_west[color][s] = w;
                bbs.pawn_attack_span[color][s] = (e.or(w)).rays(c.forward()).or(e).or(w);

                color += 1;
            }

            let mut s2 = 0;
            while s2 < 64 {
                let sq2 = Square::from_usize(s2);
                bbs.strictly_between[s][s2] = Square::slow_strictly_between(sq, sq2);
                bbs.line[s][s2] = Square::slow_line_through(sq, sq2);
                s2 += 1;
            }

            let mut d = 0;
            while d < 8 {
                bbs.surround[s][d] = Square::slow_within_chebyshev_distance_inclusive(sq, d as i32);

                let dir = Dir::ALL[d];
                bbs.king_moves[s] = bbs.king_moves[s].or(sq.as_bb().shift(dir));

                // for example a night attack might be step N followed by step NE
                let next_dir = dir.rotate_clockwise();
                bbs.knight_moves[s] = bbs.knight_moves[s].or(sq.as_bb().shift(dir).shift(next_dir));
                d += 1;
            }

            bbs.bishop_moves[s] = bbs.bishop_moves[s].or((sq.diag().or(sq.anti_diag())).sub(sq.as_bb()));
            bbs.rook_moves[s] = bbs.rook_moves[s].or((sq.file_bitboard().or(sq.rank_bitboard())).sub(sq.as_bb()));
            s += 1;
        }
        bbs
    }

    #[inline]
    pub fn bishop_attacks(&self, occupied: Bitboard, from: Square) -> Bitboard {
        self.sliding_piece_attacks.bishop_attacks(occupied, from)
    }

    #[inline]
    pub fn rook_attacks(&self, occupied: Bitboard, from: Square) -> Bitboard {
        self.sliding_piece_attacks.rook_attacks(occupied, from)
    }

    // inclusive of end points
    #[inline]
    pub fn strictly_between(&self, s1: Square, s2: Square) -> Bitboard {
        self.strictly_between[s1][s2]
    }

    #[inline]
    pub fn between(&self, s1: Square, s2: Square) -> Bitboard {
        self.strictly_between(s1, s2).include(s1).include(s2)
    }

    // king moves - see https://www.chessprogramming.org/Distance
    // max difference in rank or file
    #[inline]
    pub fn chebyshev_distance(&self, s1: Square, s2: Square) -> i32 {
        Square::chebyshev_distance(s1, s2)
    }

    // king moves - see https://www.chessprogramming.org/Distance
    #[inline]
    pub fn manhattan_distance(&self, s1: Square, s2: Square) -> i32 {
        Square::manhattan_distance(s1, s2)
    }

    #[inline]
    pub fn within_chebyshev_distance_inclusive(&self, s1: Square, d: u32) -> Bitboard {
        if d >= 8 {
            Bitboard::all()
        } else {
            self.surround[s1][d as usize]
        }
    }

    #[inline]
    // returns empty if not on same line. For s1 == s2, returns just the single square
    pub fn line_through(&self, s1: Square, s2: Square) -> Bitboard {
        self.line[s1][s2]
    }

    #[inline]
    pub fn king_attacks(&self, from: Square) -> Bitboard {
        self.king_moves[from]
    }

    #[inline]
    pub fn knight_attacks(&self, from: Square) -> Bitboard {
        self.knight_moves[from]
    }

    #[inline]
    pub fn bishop_xray_attacks(&self, from: Square) -> Bitboard {
        self.bishop_moves[from]
    }

    #[inline]
    pub fn rook_xray_attacks(&self, from: Square) -> Bitboard {
        self.rook_moves[from]
    }

    pub fn all_pawn_attacks_ext(&self, c: Color, pawns: Bitboard, them: Bitboard) -> Bitboard {
        pawns
            .squares()
            .fold(Bitboard::empty(), |a, sq| a | self.pawn_attacks_ext(c, pawns, them, sq))
    }

    pub fn all_knight_attacks(&self, knights: Bitboard) -> Bitboard {
        knights
            .squares()
            .fold(Bitboard::empty(), |a, sq| a | self.knight_attacks(sq))
    }

    pub fn all_king_attacks(&self, kings: Bitboard) -> Bitboard {
        kings
            .squares()
            .fold(Bitboard::empty(), |a, sq| a | self.king_attacks(sq))
    }

    pub fn all_bishop_attacks(&self, bishops: Bitboard, occ: Bitboard) -> Bitboard {
        bishops
            .squares()
            .fold(Bitboard::EMPTY, |a, sq| a | self.bishop_attacks(occ, sq))
    }

    pub fn all_rook_attacks(&self, rooks: Bitboard, occ: Bitboard) -> Bitboard {
        rooks
            .squares()
            .fold(Bitboard::EMPTY, |a, sq| a | self.rook_attacks(occ, sq))
    }

    pub fn all_queen_attacks(&self, queens: Bitboard, occ: Bitboard) -> Bitboard {
        queens.squares().fold(Bitboard::EMPTY, |a, sq| {
            a | self.rook_attacks(occ, sq) | self.bishop_attacks(occ, sq)
        })
    }

    pub fn all_attacks(&self, c: Color, p: Piece, us: Bitboard, occ: Bitboard) -> Bitboard {
        match p {
            Piece::Bishop => self.all_bishop_attacks(us, occ),
            Piece::Rook => self.all_rook_attacks(us, occ),
            Piece::Queen => self.all_queen_attacks(us, occ),
            Piece::King => self.all_king_attacks(us),
            Piece::Knight => self.all_knight_attacks(us),
            Piece::Pawn => self.all_pawn_attacks_ext(c, us, occ),
        }
    }

    #[inline]
    pub fn attacks(&self, c: Color, p: Piece, us: Bitboard, them: Bitboard, from: Square) -> Bitboard {
        match p {
            Piece::Bishop => self.bishop_attacks(us | them, from),
            Piece::Rook => self.rook_attacks(us | them, from),
            Piece::Queen => self.rook_attacks(us | them, from) | self.bishop_attacks(us | them, from),
            Piece::King => self.king_attacks(from),
            Piece::Knight => self.knight_attacks(from),
            Piece::Pawn => self.pawn_attacks_ext(c, us, them, from),
        }
    }

    #[inline]
    pub fn pawn_attacks_ext(&self, c: Color, us: Bitboard, them: Bitboard, fr: Square) -> Bitboard {
        let empty = !(us | them);
        let single = self.pawn_push[c][fr] & empty;
        let double = self.pawn_double_push[c][fr].iff(single.any()) & empty & c.double_push_dest_rank();
        let capture = them & (self.pawn_capture_east[c][fr] | self.pawn_capture_west[c][fr]);
        single | double | capture
    }

    // #[inline]
    // fn pawn_pushes(&self, occupied: Bitboard, pawns: Bitboard, color: Color) -> Bitboard {
    //     let empty = !occupied;
    //     let single = pawns.shift(color.forward()) & empty;
    //     single | (single.shift(color.forward()) & empty & color.double_push_dest_rank())
    // }

    #[inline]
    pub fn pawn_attacks_ew(&self, pawns: Bitboard, c: Color) -> (Bitboard, Bitboard) {
        (pawns.shift(c.pawn_capture_east()), pawns.shift(c.pawn_capture_west()))
    }

    #[inline]
    pub fn pawn_attacks_from(&self, c: Color, pawns: Bitboard) -> Bitboard {
        pawns.shift(c.pawn_capture_east()) | pawns.shift(c.pawn_capture_west())
    }

    #[inline]
    pub fn pawn_capture_attacks_from_sq(&self, c: Color, pawn: Square) -> Bitboard {
        pawn.as_bb().shift(c.pawn_capture_east()) | pawn.as_bb().shift(c.pawn_capture_west())
    }

    #[inline]
    pub fn pawn_attackers(&self, targets: Bitboard, pawn_color: Color) -> Bitboard {
        // white east attack = black west attacker. hence w/b reversal
        let opp = pawn_color.flip_side();
        targets.shift(opp.pawn_capture_east()) | targets.shift(opp.pawn_capture_west())
    }

    // tripled too (basically every pawn with a pawn north of it)
    // two pawns on same file, only one pawn is doubled
    #[inline]
    pub fn doubled_pawns(&self, pawns: Bitboard) -> Bitboard {
        pawns.fill_north().shift(Dir::N) & pawns
    }

    /// front span = column of squares strictly in front of pawn
    #[inline]
    pub fn pawn_front_span(&self, c: Color, pawn_sq: Square) -> Bitboard {
        self.pawn_front_span[c][pawn_sq]
        // let pawn = pawn_sq.as_bb();
        // let atts = pawn.shift(c.pawn_capture_east()) | pawn.shift(c.pawn_capture_west());
        // if c == Color::White {
        //     (pawn | atts).fill_north()
        // } else {
        //     (pawn | atts).fill_south()
        // }
    }

    /// rear span = column of squares strictly behind a pawn
    #[inline]
    pub fn pawn_rear_span(&self, c: Color, pawn_sq: Square) -> Bitboard {
        self.pawn_front_span[c.flip_side()][pawn_sq]
    }

    /// attack span = 1 or 2 columns of squares attacked and those in front of squares attacked
    #[inline]
    pub fn pawn_attack_span(&self, c: Color, pawn_sq: Square) -> Bitboard {
        self.pawn_attack_span[c][pawn_sq]
    }

    /// front span UNION attack span 2 or 3 columns of squares
    #[inline]
    pub fn pawn_front_span_union_attack_span(&self, c: Color, pawn: Square) -> Bitboard {
        self.pawn_front_span[c][pawn] | self.pawn_attack_span[c][pawn]
    }

    #[inline]
    pub fn neighbouring_files(&self, sq: Square) -> Bitboard {
        sq.file_bitboard() | sq.file_bitboard().shift(Dir::E) | sq.file_bitboard().shift(Dir::W)
    }

    /// square in front of pawn (or empty)
    #[inline]
    pub fn pawn_stop(&self, c: Color, pawn: Square) -> Bitboard {
        self.pawn_push[c][pawn]
    }

    /// square 2 in front of pawn (or empty). excludes single push square
    pub fn pawn_double_stop(&self, c: Color, pawn: Square) -> Bitboard {
        self.pawn_double_push[c][pawn]
    }

    #[inline]
    /// open(all_pawns) = open = pawn free files
    /// open(white_pawns) = open or white_half_open = no white pawns
    /// open(white_pawns) ^ open(all_pawns) = white_half_open = black pawn but no white pawn
    /// (open(white) | open(black)) - open(all) = half open (any) = single pawn colour files
    pub fn open_files(&self, pawns: Bitboard) -> Bitboard {
        !pawns.fill_south().fill_north()
    }

    #[inline]
    /// isolated pawns have no pawns of their colour on neighbouring file(s)
    pub fn pawn_side_isolated(&self, side: Bitboard) -> Bitboard {
        let closed = !self.open_files(side);
        // non-isolated pawns have a closed (wt their color) file next to them on one side
        let non_isolated = (side & closed.shift(Dir::E)) | (side & closed.shift(Dir::W));
        side - non_isolated
    }

    /// duo pawns have neighdours to the immediate east or west
    #[inline]
    pub fn pawn_side_duos(&self, side: Bitboard) -> Bitboard {
        (side.shift(Dir::E) | side.shift(Dir::W)) & side
    }

    // distant neighbours are same rank but seperated by one file
    #[inline]
    pub fn pawn_side_distant_neighbours(&self, side: Bitboard) -> Bitboard {
        let open = self.open_files(side);
        ((side.shift(Dir::E) & open).shift(Dir::E) | (side.shift(Dir::W) & open).shift(Dir::W)) & side
    }

    #[inline]
    pub fn adjacent_and_nearby_pawn_shield(&self, king_color: Color, king_sq: Square) -> (Bitboard, Bitboard) {
        // take the sheild sq as  away from the side if the king is on the side
        let ssq = if king_sq.is_in(Bitboard::FILE_A) {
            king_sq.shift(Dir::E)
        } else if king_sq.is_in(Bitboard::FILE_H) {
            king_sq.shift(Dir::W)
        } else {
            king_sq
        };
        let adjacent = self.within_chebyshev_distance_inclusive(ssq, 1);

        // allow R3 pawns if king on backrank. Excluding non-backrank prevents king lurches
        //  into "safe space" to claim addition R3 pawns as shield pawns.

        let nearby = if king_sq.is_in(Bitboard::RANKS_18) {
            let d2 = self.within_chebyshev_distance_inclusive(ssq, 2);
            (self.pawn_front_span_union_attack_span(king_color, ssq) & d2) - adjacent
        } else {
            Bitboard::EMPTY
        };
        (adjacent, nearby)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pawns {
    pub white:                Bitboard,
    pub black:                Bitboard,
    pub open_files:           Bitboard,
    pub half_open:            Bitboard, // pawns of single color on file
    pub isolated:             Bitboard,
    pub rammed:               Bitboard,
    pub white_double_attacks: Bitboard, // sqs attacked by two white pawns
    pub black_double_attacks: Bitboard,
    pub white_single_attacks: Bitboard, // attacked by one white pawn but not two
    pub black_single_attacks: Bitboard,
    pub white_controlled:     Bitboard,
    pub black_controlled:     Bitboard,
    pub white_outposts:       Bitboard,
    pub black_outposts:       Bitboard,
    pub passed:               Bitboard,
    pub candidate_passed:     Bitboard,
    pub weak:                 Bitboard, // but not isolated
    pub connected:            Bitboard,
    pub duos:                 Bitboard,
    pub distant_neighbours:   Bitboard,
    pub backward:             Bitboard,
    pub doubled:              Bitboard,
}

// files are either closed, open or half-open

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

// Weak Pawns - pawns not defended and not defensible by the pawns of the same color,
//              whose stop square is also not covered by a friendly pawn.
// - Isolated Pawn - no neighbouring pawns of same colour
// - Isolated Pawn (half open) - even weaker if rooks around
// - Backward Pawn
// - Overly advanced
// - Hanging Pawns -  are an open, half-isolated duo. It means that they are standing next
//   to each other on the adjacent half-open files, usually on the fourth rank,
//   mutually protecting their stop squares.

impl Pawns {
    #[inline]
    /// white pawns and black pawns determine the Pawn-structure
    pub fn new(white_pawns: Bitboard, black_pawns: Bitboard) -> Pawns {
        let wp = white_pawns;
        let bp = black_pawns;
        let precalc = PreCalc::instance();
        let pawns = wp | bp;
        let open_files = precalc.open_files(pawns);
        let half_open = (precalc.open_files(wp) | precalc.open_files(bp)) ^ open_files;
        let isolated = precalc.pawn_side_isolated(wp) | precalc.pawn_side_isolated(bp);
        let rammed = wp.shift(Dir::N) & bp | bp.shift(Dir::S) & wp;
        let doubled = precalc.doubled_pawns(wp) | precalc.doubled_pawns(bp);

        let black_attacks = bp.shift(Dir::SW) | bp.shift(Dir::SE);
        let white_outposts = !black_attacks.fill_south();

        // todo counting doubles here! whage to "pawns"
        let rays_north = wp.rays(Dir::N);
        let rays_south = bp.rays(Dir::S);

        let white_attacks = wp.shift(Dir::NW) | wp.shift(Dir::NE);
        let black_outposts = !white_attacks.fill_north(); // cannot be defended by white

        let passed = ((wp & white_outposts) - rays_south) | ((bp & black_outposts) - rays_north);

        // cannot be defended and cannot advance 1 square to be defended
        // => other pawns attack spans so not overlap with stop sq, pawn itself, or rear span
        // we do count weak isolated pawns
        let weak = wp & black_outposts.shift(Dir::S) | bp & white_outposts.shift(Dir::N);

        // connected pawns = those that are currently pawn defended
        let connected = pawns & (wp & white_attacks | bp & black_attacks);

        let duos = (wp.shift(Dir::E) | wp.shift(Dir::W)) & wp | (bp.shift(Dir::E) | bp.shift(Dir::W)) & bp;

        // distant neighbours are same rank but seperated by one file
        let distant_neighbours = precalc.pawn_side_distant_neighbours(wp) | precalc.pawn_side_distant_neighbours(bp);

        // backward pawns - cannot be defended by other pawns and cannot move fwd because of a pawn attack
        // and sq in front not defended (ie not a duo)
        let backward = (wp & black_outposts & black_attacks.shift(Dir::S)
            | bp & white_outposts & white_attacks.shift(Dir::N))
            - duos;

        // One possibility is to define a pawn as a candidate, if no square on its path is controlled
        // by more enemy pawns than own pawns.
        let black_double_attacks = bp.shift(Dir::SW) & bp.shift(Dir::SE);
        let black_single_attacks = black_attacks - black_double_attacks;
        let white_double_attacks = wp.shift(Dir::NW) & wp.shift(Dir::NE);
        let white_single_attacks = white_attacks - white_double_attacks;
        let white_controlled = (white_double_attacks & !black_double_attacks) | (white_single_attacks & !black_attacks);
        let black_controlled = (black_double_attacks & !white_double_attacks) | (black_single_attacks & !white_attacks);
        let candidate_passed_w = !(rays_north & black_controlled).fill_south() & half_open & wp;
        let candidate_passed_b = !(rays_south & white_controlled).fill_north() & half_open & bp;
        let candidate_passed = (candidate_passed_w | candidate_passed_b) - passed;

        //     (bbd.pawn_front_span(c, p) & b.pawns() & them).is_empty() && p.rank_index_as_white(c) >= 5 {
        //         let mut ours = (bbd.pawn_attack_span(c.opposite(),p) & b.pawns() & us & bbd.within_chebyshev_distance_inclusive(p, 2)).popcount();
        //         ours += ((p.as_bb().shift(Dir::E) | p.as_bb().shift(Dir::W)) & b.pawns() & us).popcount();
        //         let theirs = (bbd.pawn_attack_span(c, p) & b.pawns() & them).popcount();
        //         if ours >= theirs && p.rank_index_as_white(c) >= 4 {
        //             candidate_passed_pawn += 1;
        //         }

        Pawns {
            white: wp,
            black: bp,
            open_files,
            half_open,
            isolated,
            rammed,
            white_double_attacks,
            black_double_attacks,
            white_single_attacks,
            black_single_attacks,
            white_controlled,
            black_controlled,
            white_outposts,
            black_outposts,
            passed,
            candidate_passed,
            weak,      // and maybe isolated
            connected, // defended
            duos,
            distant_neighbours,
            backward,
            doubled,
        }
    }

    #[inline]
    /// pawns with a an enemy piece in front
    pub fn blockaded_opponent(&self, bd: &Board) -> Bitboard {
        bd.black().shift(Dir::S) & self.white | bd.white().shift(Dir::N) & self.black
        // bd.occupied().shift(Dir::S) & self.white | bd.occupied().shift(Dir::N) & self.black
    }

    #[inline]
    /// pawns with a piece of our own color in front
    pub fn blockaded_self(&self, bd: &Board) -> Bitboard {
        // bd.black().shift(Dir::S) & self.white | bd.white().shift(Dir::N) & self.black
        bd.white().shift(Dir::S) & self.white | bd.black().shift(Dir::N) & self.black
    }

    #[inline]
    /// rook behind a passer (even if rook attacks obstructed)
    pub fn rooks_behind_passers(&self, bd: &Board) -> Bitboard {
        (self.passed & self.white).fill_south() & (bd.rooks() & bd.white())
            | (self.passed & self.black).fill_north() & (bd.rooks() & bd.black())
    }

    #[inline]
    /// rammed pawns with the same color as a bishop on their team
    pub fn bishop_colored_rammed(&self, bd: &Board) -> Bitboard {
        self.rammed & self.white & (bd.bishops() & bd.white()).color_flood()
            | self.rammed & self.black & (bd.bishops() & bd.black()).color_flood()
    }
}

impl fmt::Display for Pawns {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        let v = serde_json::value::to_value(self).unwrap();
        let bitboards: HashMap<String, Bitboard> = serde_json::value::from_value(v).unwrap();
        display_bitboards(&bitboards).fmt(f)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::hint::black_box;

    use Square::*;

    use super::*;
    use crate::infra::profiler::PerfProfiler;
    use crate::test_log::test;

    #[test]
    fn test_king_attacks() {
        let classical = PreCalc::new();
        let attacks = classical.king_attacks(Square::A6);
        assert_eq!(attacks, A5 | B5 | B6 | B7 | A7);

        let attacks = classical.king_attacks(Square::C6);
        assert_eq!(attacks, B5 | C5 | D5 | B6 | D6 | B7 | C7 | D7)
    }

    #[test]
    fn test_knight_attacks() {
        let classical = PreCalc::new();
        let attacks = classical.knight_attacks(Square::A1);
        assert_eq!(attacks, B3 | C2);

        let attacks = classical.knight_attacks(Square::C6);
        assert_eq!(attacks, A5 | A7 | B4 | B8 | D4 | D8 | E5 | E7)
    }

    #[test]
    fn test_xray_attacks() {
        let bb = PreCalc::instance();
        let atts = bb.rook_xray_attacks(Square::A1);
        assert_eq!(atts, (Bitboard::FILE_A | Bitboard::RANK_1) - Bitboard::A1);

        let atts = bb.bishop_xray_attacks(Square::A1);
        assert_eq!(atts, B2 | C3 | D4 | E5 | F6 | G7 | H8);
    }

    #[test]
    fn test_pawns() {
        let b = Board::parse_diagram(
            r"
            ........
            .....P..
            P....P..
            P.p.....
            ...p.p..
            PP.....p
            .PPPP..P
            ........ w - - 1 1",
        )
        .unwrap();

        let pawns = Pawns::new(b.pawns() & b.white(), b.pawns() & b.black());
        println!("{b}\n{pawns}");
    }

    #[test]
    fn test_precalc_pawns() {
        let bb = PreCalc::instance();
        let pawns_w = A2 | B3 | C2 | D7 | F5 | G4 | H4 | H5;
        let opponent = A4 | B4 | D3 | G5;

        let pawns = Pawns::new(pawns_w, opponent);
        println!("{pawns}");

        // let pawn_single_push = bb.pawn_pushes(occupied, pawns_w, Color::White);
        // let single = A3 | C3 | D8 | F6 | H6;
        // let double = C4;
        // assert_eq!(pawn_single_push, single | double);

        let (pawn_capture_e, pawn_capture_w) = bb.pawn_attacks_ew(pawns_w, Color::White);
        assert_eq!(pawn_capture_e & opponent, Bitboard::D3);

        assert_eq!(pawn_capture_w & opponent, A4 | G5);

        // let ep_square = g6;
        // let (east, west) = bb.pawn_ep_captures(pawns_w, opponent, Color::White, ep_square);
        // assert_eq!(east, g6);
        // assert_eq!(west, g6);

        let pc = PreCalc::instance();
        println!("{}", pc.display(Square::A3, Square::D6, Color::White));

        use crate::bits::bitboard::Bitboard as BB;
        let pawns = B2 | B4 | C5 | C6 | D3 | D7 | H5;
        assert_eq!(pc.doubled_pawns(pawns), B4 | C6 | D7);

        assert_eq!(
            pc.open_files(pawns),
            Bitboard::FILE_A | Bitboard::FILE_E | Bitboard::FILE_F | Bitboard::FILE_G
        );
        assert_eq!(pc.pawn_side_isolated(pawns), Bitboard::H5);
        assert_eq!(pc.pawn_side_isolated(opponent), D3 | G5);

        let calced = pc.pawn_front_span_union_attack_span(Color::White, Square::B2);
        let expect = (BB::FILE_A | BB::FILE_B | BB::FILE_C) - (BB::RANK_1 | BB::RANK_2);
        println!("{}\n{}", calced, expect);
        assert_eq!(calced, expect);

        let calced = pc.pawn_front_span_union_attack_span(Color::White, Square::A2);
        let expect = (BB::FILE_A | BB::FILE_B) - (BB::RANK_1 | BB::RANK_2);
        println!("{}\n{}", calced, expect);
        assert_eq!(calced, expect);

        let calced = pc.pawn_front_span_union_attack_span(Color::White, Square::H8);
        let expect = BB::EMPTY;
        println!("{}\n{}", calced, expect);
        assert_eq!(calced, expect);

        let calced = pc.pawn_front_span_union_attack_span(Color::Black, Square::D7);
        let expect = (BB::FILE_C | BB::FILE_D | BB::FILE_E) - (BB::RANK_8 | BB::RANK_7);
        println!("{}\n{}", calced, expect);
        assert_eq!(calced, expect);
    }

    #[test]
    fn bench_precalc_pawns() {
        let mut prof_new = PerfProfiler::new("precalc pawns");
        let mut prof_stop = PerfProfiler::new("precalc stop");
        let mut prof_stop_calc = PerfProfiler::new("precalc stop calc");
        let mut prof_clone = PerfProfiler::new("precalc pawns clone");

        let pawns_w = A2 | B3 | C2 | D7 | F5 | G4 | H4 | H5;
        let pawns_b = A4 | B4 | D3 | G5;

        for _ in 0..100 {
            prof_new.bench(|| {
                let pawns = Pawns::new(black_box(pawns_w), black_box(pawns_b));
                black_box(pawns);
            });
        }

        let pc = PreCalc::instance();
        for c in Color::ALL {
            for sq in Bitboard::RANKS_234567.squares() {
                prof_stop.bench(|| {
                    black_box(pc.pawn_stop(black_box(c), black_box(sq)));
                });
                prof_stop_calc.bench(|| {
                    black_box(black_box(sq).shift(black_box(c).forward()));
                });
            }
        }

        let pawns = black_box(Pawns::new(pawns_w, pawns_b));
        for _ in 0..100 {
            prof_clone.bench(|| {
                black_box(pawns);
            });
        }
    }

    #[test]
    fn test_between() {
        let bb = PreCalc::instance();
        assert_eq!(bb.between(A1, A3), A1 | A2 | A3);
        assert_eq!(bb.between(A3, A1), A1 | A2 | A3);
        assert_eq!(bb.between(A1, A8), Bitboard::FILE_A);
        assert_eq!(bb.between(A1, A1), Bitboard::A1);
        assert_eq!(bb.between(A1, B2), A1 | B2);

        assert_eq!(bb.strictly_between(A1, A3), Bitboard::A2);
        assert_eq!(bb.strictly_between(A3, A1), Bitboard::A2);
        assert_eq!(bb.strictly_between(A1, A8), Bitboard::FILE_A - (A1 | A8));
        assert_eq!(bb.strictly_between(A1, A1), Bitboard::empty());
        assert_eq!(bb.strictly_between(A1, B2), Bitboard::empty());
    }

    #[test]
    fn test_within_chebyshev_distance_inclusive() {
        let bb = PreCalc::instance();
        assert_eq!(bb.within_chebyshev_distance_inclusive(D4, 4), Bitboard::all());
        assert_eq!(bb.within_chebyshev_distance_inclusive(A4, 2).popcount(), 15);
        info!("{}", bb.within_chebyshev_distance_inclusive(C3, 3));
    }

    #[test]
    fn test_pawn_shield() {
        let bb = PreCalc::instance();
        let a: Bitboard = "8/8/8/8/8/8/5XXX/5XXX".parse().unwrap();
        let n: Bitboard = "8/8/8/8/8/5XXX/8/8".parse().unwrap();
        assert_eq!(bb.adjacent_and_nearby_pawn_shield(Color::White, G1), (a, n));
        assert_eq!(bb.adjacent_and_nearby_pawn_shield(Color::White, H1), (a, n));

        let a: Bitboard = "8/8/8/8/8/8/XXX5/XXX5".parse().unwrap();
        let n: Bitboard = "8/8/8/8/8/XXX5/8/8".parse().unwrap();
        assert_eq!(bb.adjacent_and_nearby_pawn_shield(Color::White, A1), (a, n));
        assert_eq!(bb.adjacent_and_nearby_pawn_shield(Color::White, B1), (a, n));

        let a: Bitboard = "XXX5/XXX5/8/8/8/8/8/8".parse().unwrap();
        let n: Bitboard = "8/8/XXX5/8/8/8/8/8".parse().unwrap();
        assert_eq!(bb.adjacent_and_nearby_pawn_shield(Color::Black, A8), (a, n));
        assert_eq!(bb.adjacent_and_nearby_pawn_shield(Color::Black, B8), (a, n));

        let a: Bitboard = "8/8/.XXX4/.XXX4/.XXX4/8/8/8".parse().unwrap();
        let n: Bitboard = "8/8/8/8/8/8/8/8".parse().unwrap(); // no rank 3 if off backline
        assert_eq!(bb.adjacent_and_nearby_pawn_shield(Color::White, C5), (a, n));
    }
}
