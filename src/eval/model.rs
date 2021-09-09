use std::fmt;

use crate::bitboard::castling::CastlingRights;
use crate::bitboard::precalc::BitboardDefault;
use crate::board::multiboard::Multiboard;
use crate::board::Board;
use crate::eval::score::Score;
use crate::eval::switches::Switches;
use crate::eval::weight::Weight;
use crate::material::Material;
use crate::types::Color;
use crate::types::Piece;
use crate::Bitboard;

#[derive(Clone, Default, Debug)]
pub struct Model {
    // material
    pub switches: Switches,
    pub turn: Color,
    pub mat: Material,
    pub draw: bool,
    pub multiboard: Multiboard,

    pub white: ModelSide,
    pub black: ModelSide,
}

#[derive(Clone, Default, Debug)]
pub struct ModelSide {
    // bishops
    pub has_bishop_pair: bool,
    pub fianchetti: i32,

    // rooks
    pub has_rook_pair: bool,
    pub rooks_on_open_files: i32,

    pub queens_on_open_files: i32,

    // position
    // pub psq: ArrayVec<(Piece, Square), 32>,

    // pawn structure
    pub doubled_pawns: i32,
    pub isolated_pawns: i32,
    pub passed_pawns: i32, // includes passed pawns on r7
    pub passed_pawns_on_r6: i32, // r7 by definition are passed

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
    pub non_pawn_defended_moves: i32,
    pub fully_trapped_pieces: i32,
    pub partially_trapped_pieces: i32,
    pub defended_non_pawn: i32,
    pub xrayed: i32,

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
        writeln!(
            f,
            "{:>20} | {:>7} {:>7} {:>7} | {:>7}  {:>7} {:>7} | {:>7} {:>7} {:>7} | {:>15}",
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
                    "{:>20} | {:>7} {:>7} {:>7} | {:>7}  {:>7} {:>7} | {:>7} {:>7} {:>7} | {:>15}",
                    attr,
                    w,
                    (w * wt).s(),
                    (w * wt).e(),
                    ((w * wt) - (b * wt)).interpolate(self.phase()),
                    (w * wt).s() - (b * wt).s(),
                    (w * wt).e() - (b * wt).e(),
                    b,
                    (b * wt).s(),
                    (b * wt).e(),
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
                    "{:>20} | {:>7} {:>7} {:>7} | {:>7}  {:>7} {:>7} | {:>7} {:>7} {:>7} | {:>15}",
                    "", "-----", "-----", "-----", "-----", "-----", "-----", "-----", "-----", "-----", ""
                )?;
                writeln!(
                    f,
                    "{:>20} | {:>7} {:>7} {:>7} | {:>7}  {:>7} {:>7} | {:>7} {:>7} {:>7} | {:>15}",
                    attr,
                    "",
                    wwt.s(),
                    wwt.e(),
                    twt.interpolate(self.phase()),
                    twt.s(),
                    twt.e(),
                    "",
                    bwt.s(),
                    bwt.e(),
                    ""
                )?;
                writeln!(f)?;
            }
        }
        writeln!(
            f,
            "{:>20} | {:>7} {:>7} {:>7} | {:>7}  {:>7} {:>7} | {:>7} {:>7} {:>7} | {:>15}",
            "", "-----", "-----", "-----", "=====", "-----", "-----", "-----", "-----", "-----", "=========="
        )?;
        writeln!(
            f,
            "{:>20} | {:>7} {:>7} {:>7} | {:>7}  {:>7} {:>7} | {:>7} {:>7} {:>7} |      Phase{:>3} %",
            "EVALUATION",
            "",
            "",
            "",
            self.total().interpolate(self.phase()),
            self.total().s(),
            self.total().e(),
            "",
            "",
            "",
            self.phase()
        )?;
        writeln!(
            f,
            "{:>20} | {:>7} {:>7} {:>7} | {:>7}  {:>7} {:>7} | {:>7} {:>7} {:>7} | {:>15}",
            "", "", "", "", "=====", "-----", "-----", "", "", "", "=========="
        )?;
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModelScore {
    pub phase: i32,
    pub material: Weight,
    pub position: Weight,
    pub pawn: Weight,
    pub mobility: Weight,
    pub safety: Weight,
    pub tempo: Weight,
    pub contempt: Weight,
    pub interpolated: i32,
}

impl ModelScore {
    pub fn new(phase: i32) -> Self {
        Self {
            phase,
            ..Self::default()
        }
    }

    pub fn as_score(&self) -> Score {
        Score::from_cp(self.interpolated)
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
        self.interpolated = self.total().interpolate(self.phase).round() as i32;
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
        Self {
            switches,
            turn: b.color_us(),
            multiboard: b.multiboard().clone(),
            mat: material,
            white: ModelSide::from_board(b, Color::White, &material, switches),
            black: ModelSide::from_board(b, Color::Black, &material, switches),
            draw: false,
        }
    }
}

impl ModelSide {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn from_board(b: &Board, c: Color, mat: &Material, sw: Switches) -> Self {
        let mut m = Self::default();
        if sw.contains(Switches::MATERIAL) {
            m.init_material(b, c, mat);
        }
        if sw.contains(Switches::POSITION) {
            m.init_position(b, c);
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
    fn init_material(&mut self, _b: &Board, c: Color, m: &Material) {
        self.has_bishop_pair = m.counts(c, Piece::Bishop) >= 2;
        self.has_rook_pair = m.counts(c, Piece::Rook) >= 2;
    }

    #[inline]
    fn init_position(&mut self, b: &Board, c: Color) {
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
            && (b.pawns() & us).contains(pawns)
            && (b.pawns() & us).disjoint(no_pawns)
            && (b.kings() & us).contains(king)
        {
            self.fianchetti += 1
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

        for p in (b.pawns() & b.color(c)).squares() {
            // self.doubled_pawns += is_doubled as i32;
            // we still count doubled pawns as passed pawns (since 0.3.37)
            let is_passed =
                (bbd.pawn_front_span_union_attack_span(c, p) & b.pawns() & b.color(c.opposite())).is_empty();
            self.passed_pawns += is_passed as i32;

            let rank = c.chooser_wb(Bitboard::RANK_6, Bitboard::RANK_3);
            self.passed_pawns_on_r6 += (is_passed && rank.intersects(p.as_bb())) as i32;
        }
        self.doubled_pawns = bbd.doubled_pawns(b.color(c) & b.pawns()).popcount();
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
            self.nearby_shield =
                (p & p_fr_att_span & d2).popcount() - self.adjacent_shield;
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
        }

        
        self.castling_sides = b.castling().contains(CastlingRights::king_side_right(c)) as i32
            + b.castling().contains(CastlingRights::queen_side_right(c)) as i32;
    }

    #[inline]
    fn init_mobility(&mut self, b: &Board, c: Color) {
        let bb = BitboardDefault::default();
        let us = b.color(c);

        self.rooks_on_open_files = (bb.open_files(b.pawns()) & us & b.rooks()).popcount();
        self.queens_on_open_files = (bb.open_files(b.pawns()) & us & b.queens()).popcount();

        let their = c.opposite();
        let them = b.color(their);
        let occ = them | us;
        let their_p = b.pawns() & them;
        // let our_p = b.pawns() & us;
        let (pe, pw) = bb.pawn_attacks(their_p, their);
        // let (ope, opw) = bb.pawn_attacks(our_p, c);
        let pa = pe | pw;
        // let opa = ope | opw;
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
            self.move_squares += piece_move_squares;
            self.non_pawn_defended_moves += piece_non_pawn_defended_moves;

            if k.any() {
                self.attacks_on_opponent_king_area += (our_raw_attacks & bb.within_chebyshev_distance_inclusive(ksq, 1)).popcount();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::tags::Tag;
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
}
