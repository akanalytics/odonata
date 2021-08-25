use crate::bitboard::castling::CastlingRights;
use crate::bitboard::precalc::BitboardDefault;
use crate::bitboard::square::Square;
use crate::board::Board;
use crate::material::Material;
use crate::types::Color;
use crate::types::Piece;
use crate::eval::weight::Weight;
use crate::eval::score::Score;
use arrayvec::ArrayVec;

#[derive(Clone, Default, Debug)]
pub struct Model {
    // material
    pub turn: Color, 
    pub mat: Material,
    pub phase: i32,
    pub draw: bool,
    pub board: Board,

    pub white: ModelSide,
    pub black: ModelSide,
}

#[derive(Clone, Default, Debug)]
pub struct ModelSide {

    // material
    pub has_bishop_pair: bool,

    // position
    // pub psq: ArrayVec<(Piece, Square), 32>,

    // pawn structure
    pub doubled_pawns: i32,
    pub isolated_pawns: i32,
    pub passed_pawns: i32,

    // king safety
    pub nearby_pawns: i32,
    pub castling_sides: i32, // 0,1 or 2

    // mobility
    pub move_squares: i32,
    pub non_pawn_defended_moves: i32,
    pub fully_trapped_pieces: i32,
    pub partially_trapped_pieces: i32,
    pub rooks_on_open_files: i32,

    // other
    pub has_tempo: bool,
}



pub trait Scorer {
    fn set_multiplier(&mut self, m: i32);
    fn annotate(&mut self, annotation: &str);
    fn material(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn position(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn pawn(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn mobility(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn safety(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn tempo(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn contempt(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn interpolate(&mut self, attr: &str, phase: i32);
    fn total(&mut self) -> Weight;
} 


#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ModelScore {
    pub mult: i32,
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
    pub fn new() -> Self {
        Self::default()
    }
    pub fn as_score(&self) -> Score {
        Score::from_cp(self.interpolated)
    }
}


impl Scorer for ModelScore {

    #[inline]
    fn annotate(&mut self, _annotation: &str) {
    }

    #[inline]
    fn set_multiplier(&mut self, mult: i32) {
        self.mult = mult;
    }

    #[inline]
    fn material(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.material += self.mult * (w_value - b_value) * score;
    }
    #[inline]
    fn position(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.position += self.mult * (w_value - b_value) * score;
    }
    #[inline]
    fn pawn(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.pawn += self.mult * (w_value - b_value) * score;
    }
    #[inline]
    fn mobility(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.mobility += self.mult * (w_value - b_value) * score;
    }
    
    #[inline]
    fn safety(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.safety += self.mult * (w_value - b_value) * score;
    }

    #[inline]
    fn tempo(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.tempo += self.mult * (w_value - b_value) * score;
    }

    #[inline]
    fn contempt(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.contempt += self.mult * (w_value - b_value) * score;
    }
    
    #[inline]
    fn interpolate(&mut self, _attr: &str, phase: i32) {
        self.interpolated += self.total().interpolate(phase);
    }

    #[inline]
    fn total(&mut self) -> Weight {
        self.material
        + self.position
        + self.pawn
        + self.mobility
        + self.safety
        + self.tempo
        + self.contempt
    }
}


// impl std::ops::Add for ModelScore {
//     type Output = Self;

//     #[inline]
//     fn add(self, o: Self) -> Self {
//         Self {
//             material: self.material + o.material,
//             position: self.position + o.position,
//             pawn: self.pawn + o.pawn,
//             mobility: self.mobility + o.mobility,
//             safety: self.safety + o.safety,
//             tempo: self.tempo + o.tempo,
//             contempt: self.contempt + o.contempt,
//             interpolated: self.interpolated + o.interpolated,
//         }
//     }
// }

// impl std::ops::Sub for ModelScore {
//     type Output = Self;

//     #[inline]
//     fn sub(self, o: Self) -> Self {
//         Self {
//             material: self.material - o.material,
//             position: self.position - o.position,
//             pawn: self.pawn - o.pawn,
//             mobility: self.mobility - o.mobility,
//             safety: self.safety - o.safety,
//             tempo: self.tempo - o.tempo,
//             contempt: self.contempt - o.contempt,
//             interpolated: self.interpolated - o.interpolated,
//         }
//     }
// }


impl Model {

    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_board(b: &Board) -> Self{
        let material = b.material();
        Self {
            turn: b.color_us(),
            board: b.clone(),
            mat: material,
            phase: b.phase(),
            white: ModelSide::from_board(b, Color::White, &material),
            black: ModelSide::from_board(b, Color::Black, &material),
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
    pub fn from_board(b: &Board, c: Color, mat: &Material) -> Self{
        let mut m = Self::default();
        m.init_material(b, c, mat);
        m.init_position(b, c);
        m.init_pawns(b, c);
        m.init_king_safety(b, c);
        m.init_mobility(b, c);
        m.init_other(b, c);
        m
    }

    #[inline]
    fn init_material(&mut self, b: &Board, c: Color, m: &Material) {
        self.has_bishop_pair = m.counts(c, Piece::Bishop) >= 2;
    }

    #[inline]
    fn init_position(&mut self, _b: &Board, _c: Color) {
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

    #[inline]
    fn init_pawns(&mut self, b: &Board, c: Color) {
        let bbd = BitboardDefault::default();
        // self.doubled_pawns = bbd.doubled_pawns(b.color(c) & b.pawns()).popcount();
        self.isolated_pawns = bbd.isolated_pawns(b.color(c) & b.pawns()).popcount();

        for p in (b.pawns() & b.color(c)).squares() {
            let is_doubled = p.is_in(bbd.doubled_pawns(b.color(c) & b.pawns()));
            // self.doubled_pawns += is_doubled as i32;
            let is_passed =
                (bbd.pawn_front_span(c, p) & b.pawns() & b.color(c.opposite())).is_empty() && !is_doubled;
            self.passed_pawns += is_passed as i32;
        }
        // FIXME!
        self.doubled_pawns = bbd.doubled_pawns(b.color(c) & b.pawns()).popcount();
    }

    #[inline]
    fn init_king_safety(&mut self, b: &Board, c: Color) {
        let p = b.pawns() & b.color(c);
        let k = b.kings() & b.color(c);
        let bb = BitboardDefault::default();
        if k.any() {
            self.nearby_pawns = (p & bb.king_attacks(k.square())).popcount();
        }

        self.castling_sides = b.castling().contains(CastlingRights::king_side_right(c)) as i32
            + b.castling().contains(CastlingRights::queen_side_right(c)) as i32;
    }

    #[inline]
    fn init_mobility(&mut self, b: &Board, c: Color) {
        let bb = BitboardDefault::default();
        let us = b.color(c);

        self.rooks_on_open_files = (bb.open_files(b.pawns()) & us & b.rooks()).popcount();

        let their = c.opposite();
        let them = b.color(their);
        let occ = them | us;
        let their_p = b.pawns() & them;
        let (pe, pw) = bb.pawn_attacks(their_p, their);
        let pa = pe | pw;
        let bi = b.bishops() & them;
        let ni = b.knights() & them;
        let r = b.rooks() & them;
        let _q = b.queens() & them;

        for sq in ((b.knights() | b.bishops() | b.rooks() | b.queens()) & us).squares() {
            let p = b.piece_at(sq.as_bb());

            // non-pawn-defended empty or oppoent sq
            let our_attacks = bb.non_pawn_attacks(c, p, us, them, sq) - pa;
            let piece_move_squares = (our_attacks - occ).popcount();

            // those attacks on enemy that arent pawn defended and cant attack back
            let piece_non_pawn_defended_moves = match p {
                Piece::Queen => (our_attacks & them).popcount(),
                Piece::Rook => (our_attacks & them - r).popcount(),
                Piece::Knight => (our_attacks & them - ni).popcount(),
                Piece::Bishop => (our_attacks & them - bi).popcount(),
                _ => 0,
            };
            // trapped piece
            if piece_move_squares + piece_non_pawn_defended_moves == 1 {
                self.partially_trapped_pieces += 1;
            }
            if piece_move_squares + piece_non_pawn_defended_moves == 0 {
                self.fully_trapped_pieces += 1;
            }
            self.move_squares += piece_move_squares;
            self.non_pawn_defended_moves += piece_non_pawn_defended_moves;
        }
      }

}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::BoardBuf;
    use crate::catalog::Catalog;

    #[test]
    fn test_model() {
        let positions = Catalog::pawn_structure();
        for p in positions {
            let model = Model::from_board(p.board());
            println!("{} {:#?}", p, model);
        }

    }
}





