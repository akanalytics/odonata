use std::fmt;

use crate::bitboard::castling::CastlingRights;
use crate::bitboard::precalc::BitboardDefault;
use crate::bitboard::square::Square;
use crate::board::multiboard::Multiboard;
use crate::board::Board;
use crate::eval::score::Score;
use crate::eval::switches::Switches;
use crate::eval::weight::Weight;
use crate::globals::constants::{FILE_D, FILE_E};
use crate::domain::material::Material;
use crate::types::Color;
use crate::types::Piece;
use crate::utils::Formatting;
use crate::{Bitboard, PreCalc};
// use arrayvec::ArrayVec;

use super::endgame::EndGame;

#[derive(Clone, Default, Debug)]
pub struct Model {
    // material
    pub switches: Switches,
    pub turn: Color,
    pub mat: Material,
    pub endgame: EndGame,
    pub multiboard: Multiboard,

    pub white: ModelSide,
    pub black: ModelSide,
}

#[derive(Clone, Default, Debug)]
pub struct ModelSide {
    pub endgame_metric1: i32,
    pub endgame_metric2: i32,

    // bishops
    pub has_bishop_pair: bool,
    pub fianchetti: i32,
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
    pub rooks_behind_passer: i32, // passed pawn with a rook behind 

    pub queens_on_open_files: i32,
    pub queen_early_develop: i32,

    // position
    // pub psq: ArrayVec<(Piece, Square), 32>,

    // pawn structure
    pub doubled_pawns: i32,
    pub isolated_pawns: i32,
    pub passed_pawns: i32,       // includes passed pawns on r7
    pub passed_pawns_on_r6: i32, // r7 by definition are passed
    pub passed_pawns_on_r5: i32,
    pub passers_on_rim: i32,    // files a & h
    pub blockaded: i32,         // enemy in front of pawn
    pub blockaded_passers: i32, // passed pawn with enemy right in front
    pub space: i32, // empty squares behind rammed pawns
    pub rammed_pawns: i32, 

    // king safety
    pub king_tropism_d1: i32,
    pub king_tropism_d2: i32,
    pub king_tropism_d3: i32,
    pub adjacent_shield: i32,
    pub nearby_shield: i32,
    pub open_files_near_king: i32,
    pub attacks_on_opponent_king_area: i32,

    pub castling_sides: i32, // 0,1 or 2

    // mobility
    pub move_squares: i32,
    pub center_attacks: i32,
    pub non_pawn_defended_moves: i32,
    pub fully_trapped_pieces: i32,
    pub partially_trapped_pieces: i32,
    pub defended_non_pawn: i32,
    pub xrayed: i32,
    // pub mv: ArrayVec<(Piece, i32), 32>,

    pub attacks: [[i32; Piece::ALL.len()]; Piece::ALL.len()],
    pub defends: [[i32; Piece::ALL.len()]; Piece::ALL.len()],

    // other
    pub has_tempo: bool,
}

pub trait Scorer {
    // fn set_multiplier(&mut self, m: i32);
    fn annotate(&mut self, annotation: &str);
    fn material(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn position(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn pawn(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn mobility(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn safety(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn tempo(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn contempt(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn interpolate(&mut self, attr: &str);
    fn total(&self) -> Weight;
    fn phase(&self) -> i32;
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExplainScorer {
    mat: Vec<(String, i32, i32, Weight)>,
    pos: Vec<(String, i32, i32, Weight)>,
    paw: Vec<(String, i32, i32, Weight)>,
    mob: Vec<(String, i32, i32, Weight)>,
    saf: Vec<(String, i32, i32, Weight)>,
    tem: Vec<(String, i32, i32, Weight)>,
    con: Vec<(String, i32, i32, Weight)>,
    delegate: ModelScore,
}

pub enum ReportLine {
    Header,
    Body,
}

impl ExplainScorer {
    pub fn new(phase: i32) -> Self {
        Self {
            delegate: ModelScore::new(phase),
            ..Self::default()
        }
    }
    pub fn as_score(&self) -> Score {
        self.delegate.as_score()
    }

    pub fn as_csv(&self, line: ReportLine) -> String {
        let mut output = String::new();
        for (i, _sw) in Switches::all_scoring().iter().enumerate() {
            let vec = vec![
                &self.mat, &self.pos, &self.mob, &self.paw, &self.saf, &self.con, &self.tem,
            ][i];
            for (attr, w, b, wt) in vec {
                let (attr, w, b, _wt) = (attr, *w, *b, *wt);
                let field_s = match line {
                    ReportLine::Header => format!("{}.s", attr.replace(" ", "_")),
                    ReportLine::Body => {
                        format!("{}", (w - b) as f32 * (100 - self.delegate.phase) as f32 / 100.0)
                    }
                };
                output.push_str(&field_s);
                output.push_str(", ");

                let field_e = match line {
                    ReportLine::Header => format!("{}.e", attr.replace(" ", "_")),
                    ReportLine::Body => format!("{}", (w - b) as f32 * self.delegate.phase as f32 / 100.0),
                };
                output.push_str(&field_e);
                output.push_str(", ");
            }
        }
        output
    }
}

impl Scorer for ExplainScorer {
    #[inline]
    fn annotate(&mut self, _annotation: &str) {}

    #[inline]
    // fn set_multiplier(&mut self, mult: i32) {
    //     self.delegate.set_multiplier(mult);
    // }
    #[inline]
    fn material(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.mat.push((_attr.to_string(), w_value, b_value, score));
        self.delegate.material(_attr, w_value, b_value, score);
    }
    #[inline]
    fn position(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.pos.push((_attr.to_string(), w_value, b_value, score));
        self.delegate.position(_attr, w_value, b_value, score);
    }
    #[inline]
    fn pawn(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.paw.push((_attr.to_string(), w_value, b_value, score));
        self.delegate.pawn(_attr, w_value, b_value, score);
    }

    #[inline]
    fn mobility(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.mob.push((_attr.to_string(), w_value, b_value, score));
        self.delegate.mobility(_attr, w_value, b_value, score);
    }

    #[inline]
    fn safety(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.saf.push((_attr.to_string(), w_value, b_value, score));
        self.delegate.safety(_attr, w_value, b_value, score);
    }

    #[inline]
    fn tempo(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.tem.push((_attr.to_string(), w_value, b_value, score));
        self.delegate.tempo(_attr, w_value, b_value, score);
    }

    #[inline]
    fn contempt(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.con.push((_attr.to_string(), w_value, b_value, score));
        self.delegate.contempt(_attr, w_value, b_value, score);
    }

    #[inline]
    fn interpolate(&mut self, _attr: &str) {
        self.delegate.interpolate(_attr);
    }

    #[inline]
    fn total(&self) -> Weight {
        self.delegate.total()
    }

    #[inline]
    fn phase(&self) -> i32 {
        self.delegate.phase()
    }
}

impl fmt::Display for ExplainScorer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn pad<T: Into<f64>>(t: T) -> String {
            Formatting::format_decimal(2, t)
        }

        writeln!(
            f,
            "{:>20} | {:>7} {:>7} {:>7} | {:>7}  {:>7} {:>7} | {:>7} {:>7} {:>7} |   {:<15}",
            "attr", "w", "w mg", "w eg", "int", "mg", "eg", "b", "b mg", "b eg", "wt"
        )?;
        for (i, sw) in Switches::all_scoring().iter().enumerate() {
            let vec = vec![
                &self.mat, &self.pos, &self.mob, &self.paw, &self.saf, &self.con, &self.tem,
            ][i];
            for (attr, w, b, wt) in vec {
                let (attr, w, b, wt) = (attr, *w, *b, *wt);
                writeln!(
                    f,
                    "{:>20} | {:>7} {:>7} {:>7} | {:>7}  {:>7} {:>7} | {:>7} {:>7} {:>7} |   {:<15}",
                    attr,
                    w,
                    pad((w * wt).s()),
                    pad((w * wt).e()),
                    pad(((w * wt) - (b * wt)).interpolate(self.phase())),
                    pad((w * wt).s() - (b * wt).s()),
                    pad((w * wt).e() - (b * wt).e()),
                    b,
                    pad((b * wt).s()),
                    pad((b * wt).e()),
                    wt.to_string()
                )?;
            }
            if !sw.intersects(Switches::TEMPO | Switches::CONTEMPT) {
                let attr = sw.name();
                let wwt: Weight = vec.into_iter().map(|&(_, w, _b, wt)| w * wt).sum();
                let bwt: Weight = vec.into_iter().map(|&(_, _w, b, wt)| b * wt).sum();
                let twt: Weight = vec.into_iter().map(|&(_, w, b, wt)| w * wt - b * wt).sum();
                writeln!(
                    f,
                    "{:>20} | {:>7} {:>7} {:>7} | {:>7}  {:>7} {:>7} | {:>7} {:>7} {:>7} |   {:<15}",
                    "", "-----", "-----", "-----", "-----", "-----", "-----", "-----", "-----", "-----", ""
                )?;
                writeln!(
                    f,
                    "{:>20} | {:>7} {:>7} {:>7} | {:>7}  {:>7} {:>7} | {:>7} {:>7} {:>7} |   {:<15}",
                    attr,
                    "",
                    pad(wwt.s()),
                    pad(wwt.e()),
                    pad(twt.interpolate(self.phase())),
                    pad(twt.s()),
                    pad(twt.e()),
                    "",
                    pad(bwt.s()),
                    pad(bwt.e()),
                    ""
                )?;
                writeln!(
                    f,
                    "{:>20} | {:>7} {:>7} {:>7} | {:>7}  {:>7} {:>7} | {:>7} {:>7} {:>7} |   {:<15}",
                    "", "", "", "", "", "", "", "", "", "", ""
                )?;
            }
        }
        writeln!(
            f,
            "{:>20} | {:>7} {:>7} {:>7} | {:>7}  {:>7} {:>7} | {:>7} {:>7} {:>7} |   {:<15}",
            "", "-----", "-----", "-----", "=====", "-----", "-----", "-----", "-----", "-----", "=========="
        )?;
        writeln!(
            f,
            "{:>20} | {:>7} {:>7} {:>7} | {:>7}  {:>7} {:>7} | {:>7} {:>7} {:>7} |      Phase{:>3} %",
            "EVALUATION",
            "",
            "",
            "",
            pad(self.total().interpolate(self.phase())),
            pad(self.total().s()),
            pad(self.total().e()),
            "",
            "",
            "",
            self.phase()
        )?;
        writeln!(
            f,
            "{:>20} | {:>7} {:>7} {:>7} | {:>7}  {:>7} {:>7} | {:>7} {:>7} {:>7} |   {:<15}",
            "", "", "", "", "=====", "-----", "-----", "", "", "", "=========="
        )?;
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModelScore {
    phase: i32,
    material: Weight,
    position: Weight,
    pawn: Weight,
    mobility: Weight,
    safety: Weight,
    tempo: Weight,
    contempt: Weight,
    interpolated: f32,
}

impl ModelScore {
    pub fn new(phase: i32) -> Self {
        Self {
            phase,
            ..Self::default()
        }
    }

    pub fn as_f32(&self) -> f32 {
        self.interpolated
    }

    pub fn as_score(&self) -> Score {
        Score::from_cp(self.interpolated.round() as i32)
    }
}

impl Scorer for ModelScore {
    #[inline]
    fn annotate(&mut self, _annotation: &str) {}

    // #[inline]
    // fn set_multiplier(&mut self, mult: i32) {
    //     self.mult = mult;
    // }

    #[inline]
    fn material(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.material += (w_value - b_value) * score;
    }
    #[inline]
    fn position(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.position += (w_value - b_value) * score;
    }
    #[inline]
    fn pawn(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.pawn += (w_value - b_value) * score;
    }
    #[inline]
    fn mobility(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.mobility += (w_value - b_value) * score;
    }

    #[inline]
    fn safety(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.safety += (w_value - b_value) * score;
    }

    #[inline]
    fn tempo(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.tempo += (w_value - b_value) * score;
    }

    #[inline]
    fn contempt(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.contempt += (w_value - b_value) * score;
    }

    #[inline]
    fn interpolate(&mut self, _attr: &str) {
        self.interpolated = self.total().interpolate(self.phase) as f32;
    }

    #[inline]
    fn total(&self) -> Weight {
        self.material + self.position + self.pawn + self.mobility + self.safety + self.tempo + self.contempt
    }

    #[inline]
    fn phase(&self) -> i32 {
        self.phase
    }
}

impl Model {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_board(b: &Board, switches: Switches) -> Self {
        let material = b.material();
        let endgame = EndGame::from_board(b);
        Self {
            switches,
            turn: b.color_us(),
            multiboard: b.multiboard().clone(),
            mat: material,
            white: ModelSide::from_board(b, Color::White, &material, endgame, switches),
            black: ModelSide::from_board(b, Color::Black, &material, endgame, switches),
            endgame, 
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
        
        // if eg.try_winner().is_some() {
        //     return m;
        // }
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
        // if eg.try_winner() == Some(c.opposite()) {
        //     // c = losing colour - the winning side doesnt get a score (just the negative of the loser)
        //     use EndGame::*;
        //     match eg {
        //         BishopKnightVsKing(_) => {
        //             self.endgame_metric1 = 4 * Self::king_distance_to_bishops_corner(b, c);
        //             self.endgame_metric2 = Self::king_distance(b);
        //         }

        //         TwoBishopsOppositeColorSquares(_)  => {
        //             self.endgame_metric1 = 2 * Self::king_distance_to_any_corner(b, c);
        //             self.endgame_metric2 = Self::king_distance(b);
        //         }

        //         KingMajorsVsKing(_) | _ =>  {
        //             self.endgame_metric1 = 2 * Self::king_distance_to_side(b, c);
        //             self.endgame_metric2 = Self::king_distance(b);
        //         }

        //     }
        // }
    }

    fn king_distance(b: &Board) -> i32 {
        let wk = b.kings() & b.white();
        let bk = b.kings() & b.black();
        if wk.popcount() == 1 && bk.popcount() == 1 {
            PreCalc::default().chebyshev_distance(wk.square(), bk.square()) as i32
        } else {
            0
        }
    }

    fn king_distance_to_side(b: &Board, c: Color) -> i32 {
        let k = b.kings() & b.color(c);
        if k.popcount() == 1 {
            let r = k.square().rank_index() as i32;
            let f = k.square().file_index() as i32;
            let m1 = std::cmp::min(r, f);
            let m2 = std::cmp::min(7 - r, 7 - f);
            std::cmp::min(m1, m2)
        } else {
            0
        }
    }

    fn king_distance_to_any_corner(b: &Board, c: Color) -> i32 {
        let k = b.kings() & b.color(c);
        if k.popcount() == 1 {
            let ksq = k.square();
            let d1 = PreCalc::default().chebyshev_distance(Square::A1, ksq);
            let d2 = PreCalc::default().chebyshev_distance(Square::A8, ksq);
            let d3 = PreCalc::default().chebyshev_distance(Square::H1, ksq);
            let d4 = PreCalc::default().chebyshev_distance(Square::H8, ksq);
            std::cmp::min(std::cmp::min(d1, d2), std::cmp::min(d3, d4)) as i32
        } else {
            0
        }
    }

    fn king_distance_to_bishops_corner(b: &Board, c: Color) -> i32 {
        let k = b.kings() & b.color(c);
        let bis = b.bishops();
        let corner1;
        let corner2;
        if bis.intersects(Bitboard::WHITE_SQUARES) {
            corner1 = Square::H1;
            corner2 = Square::A8;
        } else {
            corner1 = Square::A1;
            corner2 = Square::H8;
        };

        if k.popcount() == 1 {
            let ksq = k.square();
            let d1 = PreCalc::default().chebyshev_distance(corner1, ksq);
            let d2 = PreCalc::default().chebyshev_distance(corner2, ksq);
            std::cmp::min(d1, d2) as i32
        } else {
            0
        }
    }


    #[inline]
    fn init_position(&mut self, b: &Board, c: Color, m: &Material) {
        let us = b.color(c);

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

        let bishop = c.chooser_wb(W_BISHOP, B_BISHOP);
        let pawns = c.chooser_wb(W_PAWNS, B_PAWNS);
        let no_pawns = c.chooser_wb(W_NO_PAWNS, B_NO_PAWNS);
        let king = c.chooser_wb(W_KING, B_KING);

        if (b.bishops() & us).intersects(bishop)
            && (b.pawns() & us).contains(pawns)
            && (b.pawns() & us).disjoint(no_pawns)
            && (b.kings() & us).contains(king)
        {
            self.fianchetti += 1
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
                & ((b.bishops() & (Bitboard::FILE_C.or(Bitboard::FILE_F)))
                    | (b.knights() & (Bitboard::FILE_B.or(Bitboard::FILE_G)))))
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

    // isolated pawns have no neighbours of same color. Doubled pawns that are isolated count as two
    // doubled pawns are doubled or tripled or more. Two pawns count as one doubled. Three as two doubled.
    // passed. No neighbouring pawns of opposite colour ahead
    #[inline]
    fn init_pawns(&mut self, b: &Board, c: Color) {
        let bbd = BitboardDefault::default();
        // self.doubled_pawns = bbd.doubled_pawns(b.color(c) & b.pawns()).popcount();
        self.isolated_pawns = bbd.isolated_pawns(b.color(c) & b.pawns()).popcount();
        let them = b.color(c.opposite());
        let us = b.color(c);

        for p in (b.pawns() & us).squares() {
            // self.doubled_pawns += is_doubled as i32;
            // we still count doubled pawns as passed pawns (since 0.3.37)
            let is_passed = (bbd.pawn_front_span_union_attack_span(c, p) & b.pawns() & them).is_empty();
            self.passed_pawns += is_passed as i32;

            let rank6 = c.chooser_wb(Bitboard::RANK_6, Bitboard::RANK_3);
            let rank5 = c.chooser_wb(Bitboard::RANK_5, Bitboard::RANK_4);
            // all pawns on r7 are passed as an opponent pawn cannot be on rank 8
            self.passed_pawns_on_r6 += (is_passed && p.is_in(rank6)) as i32;
            self.passed_pawns_on_r5 += (is_passed && p.is_in(rank5)) as i32;
            self.passers_on_rim += (is_passed && p.is_in(Bitboard::RIM)) as i32;
            self.blockaded += bbd.pawn_stop(c, p).intersects(them) as i32;
            self.blockaded_passers += (bbd.pawn_stop(c, p).intersects(them) && is_passed) as i32;
            self.rooks_behind_passer += (is_passed && (bbd.pawn_front_span(c.opposite(), p) & b.rooks() & us).any()) as i32;
            let rammed = bbd.pawn_stop(c, p).intersects(them & b.pawns());
            self.space += (rammed as i32) * p.rank_index_as_white(c) as i32 * (1 + p.is_in(FILE_D | FILE_E) as i32);
            self.rammed_pawns += rammed as i32;
        }
        self.doubled_pawns = bbd.doubled_pawns(us & b.pawns()).popcount();
        self.rammed_pawns *= self.rammed_pawns * (b.knights() & us).any() as i32;
    }

    #[inline]
    fn init_king_safety(&mut self, b: &Board, c: Color) {
        let p = b.pawns() & b.color(c);
        let k = b.kings() & b.color(c);
        let ksq = k.square();
        let bb = BitboardDefault::default();
        if k.any() {
            let p_fr_att_span = bb.pawn_front_span_union_attack_span(c, ksq);
            let d1 = bb.within_chebyshev_distance_inclusive(ksq, 1);
            let d2 = bb.within_chebyshev_distance_inclusive(ksq, 2);
            let d3 = bb.within_chebyshev_distance_inclusive(ksq, 3);

            //self.nearby_pawns = (p & k_att).popcount();
            self.adjacent_shield = (p & p_fr_att_span & d1).popcount();
            self.nearby_shield = (p & p_fr_att_span & d2).popcount() - self.adjacent_shield;
            let them = b.color(c.opposite());
            self.king_tropism_d1 = (d1 & (b.pawns() | b.kings()) & them).popcount()
                + (d1 & (b.knights() | b.bishops()) & them).popcount() * 2
                + (d1 & (b.rooks() | b.queens()) & them).popcount() * 4;

            self.king_tropism_d2 = (d2 & (b.pawns() | b.kings()) & them).popcount()
                + (d2 & (b.knights() | b.bishops()) & them).popcount() * 2
                + (d2 & (b.rooks() | b.queens()) & them).popcount() * 4;

            self.king_tropism_d3 = (d3 & (b.pawns() | b.kings()) & them).popcount()
                + (d3 & (b.knights() | b.bishops()) & them).popcount() * 2
                + (d3 & (b.rooks() | b.queens()) & them).popcount() * 4;

            self.open_files_near_king = (p_fr_att_span & bb.open_files(b.pawns())).popcount();
            // FIXME Urgent!
            // self.open_files_near_king = (d1 & bb.open_files(b.pawns()) & ksq.rank()).popcount();
        }

        self.castling_sides = b.castling().contains(CastlingRights::king_side_right(c)) as i32
            + b.castling().contains(CastlingRights::queen_side_right(c)) as i32;
    }

    #[inline]
    fn init_mobility(&mut self, b: &Board, c: Color) {
        let bb = BitboardDefault::default();
        let us = b.color(c);
        let open_files = bb.open_files(b.pawns());
        self.doubled_rooks = (self.has_rook_pair
            && (b.rooks() & us).first_square().file_index() == (b.rooks() & us).last_square().file_index())
            as i32;
        self.doubled_rooks_open_file =
            (self.doubled_rooks == 1 && (open_files & b.rooks() & us).popcount() >= 2) as i32;
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
            let our_raw_attacks = bb.non_pawn_attacks(c, p, Bitboard::empty(), occ, sq);

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
                self.attacks_on_opponent_king_area +=
                    (our_raw_attacks & bb.within_chebyshev_distance_inclusive(ksq, 1)).popcount();
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
                    && ((sq.rank_index() >= 4 && c == Color::White)
                        || (sq.rank_index() <= 4 && c == Color::Black))
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
    use crate::eval::eval::SimpleScorer;
    use crate::tags::Tag;
    use crate::test_env_log::test;
    use crate::utils::StringUtils;

    #[test]
    fn test_model() {
        let positions = Catalog::pawn_structure();
        for p in positions {
            let model = Model::from_board(p.board(), Switches::ALL_SCORING);
            if let Tag::Comment(_n, s) = p.tag("c0") {
                let map = s.split_vars_int();
                assert_eq!(model.white.isolated_pawns, map["isolated"], "{}", p);
                assert_eq!(model.white.doubled_pawns, map["doubled"], "{}", p);
                assert_eq!(model.white.passed_pawns, map["passed"], "{}", p);
            }
            if let Tag::Comment(_n, s) = p.tag("c1") {
                let map = s.split_vars_int();
                assert_eq!(model.black.isolated_pawns, map["isolated"], "{}", p);
                assert_eq!(model.black.doubled_pawns, map["doubled"], "{}", p);
                assert_eq!(model.black.passed_pawns, map["passed"], "{}", p);
            }
        }
    }

    #[test]
    fn model_csv_test() {
        let eval = &mut SimpleScorer::new();
        eval.tempo = false;

        let positions = Catalog::example_game();
        for (i, p) in positions.iter().enumerate() {
            // let model = Model::from_board(p.board(), Switches::ALL_SCORING);
            if i == 0 {
                info!("\n{}", eval.w_eval_explain(&p.board()).as_csv(ReportLine::Header));
            }
            info!("\n{}", eval.w_eval_explain(&p.board()).as_csv(ReportLine::Body));
        }
    }
}
