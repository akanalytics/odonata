use crate::board::Board;
use crate::material::Material;
use crate::outcome::{GameEnd, Outcome};
use crate::types::{Color, Piece};
use std::fmt;

// eval1 = bl.scoring.material(p=300, b=400, n=700)
// eval2 = bl.scoring.position(endgame)

// for m in legal_moves:
//     bl.evaluate(m)
//     bl.evaluate(board + m)
//     score1 = eval1(board + m)
//     print(score1)
// '''
//         w     b  total
// pawns   3     5   -200
// bishops 1     5   -400
// total   -     -   1100
// '''
// print(score1.total)
// print(score1 + score2)
// '''
//              w     b  total
// pawns        3     5   -200
// bishops      1     5   -400
// passed pawns 4     0     50
// total        -     -   1100

// EndGame/Midgame and interp
// Tempo
// default scores
// position is by white/black as directional

// https://www.chessprogramming.org/Simplified_Evaluation_Function
const SQUARE_VALUES: [[i32; 64]; 6] = [PAWN_PST, KNIGHT_PST, BISHOP_PST, ROOK_PST, QUEEN_PST, KING_PST];

#[rustfmt::skip]
const PAWN_PST: [i32; 64] = [
0,  0,  0,  0,  0,  0,  0,  0,
50, 50, 50, 50, 50, 50, 50, 50,
10, 10, 20, 30, 30, 20, 10, 10,
 5,  5, 10, 25, 25, 10,  5,  5,
 0,  0,  0, 20, 20,  0,  0,  0,
 5, -5,-10,  0,  0,-10, -5,  5,
 5, 10, 10,-20,-20, 10, 10,  5,
 0,  0,  0,  0,  0,  0,  0,  0];

#[rustfmt::skip]
const KNIGHT_PST: [i32; 64] = [
 -50,-40,-30,-30,-30,-30,-40,-50,
 -40,-20,  0,  0,  0,  0,-20,-40,
 -30,  0, 10, 15, 15, 10,  0,-30,
 -30,  5, 15, 20, 20, 15,  5,-30,
 -30,  0, 15, 20, 20, 15,  0,-30,
 -30,  5, 10, 15, 15, 10,  5,-30,
 -40,-20,  0,  5,  5,  0,-20,-40,
 -50,-40,-30,-30,-30,-30,-40,-50];

#[rustfmt::skip]
const BISHOP_PST: [i32; 64] = [
-20,-10,-10,-10,-10,-10,-10,-20,
-10,  0,  0,  0,  0,  0,  0,-10,
-10,  0,  5, 10, 10,  5,  0,-10,
-10,  5,  5, 10, 10,  5,  5,-10,
-10,  0, 10, 10, 10, 10,  0,-10,
-10, 10, 10, 10, 10, 10, 10,-10,
-10,  5,  0,  0,  0,  0,  5,-10,
-20,-10,-10,-10,-10,-10,-10,-20];

#[rustfmt::skip]
const ROOK_PST: [i32; 64] = [
  0,  0,  0,  0,  0,  0,  0,  0,
  5, 10, 10, 10, 10, 10, 10,  5,
 -5,  0,  0,  0,  0,  0,  0, -5,
 -5,  0,  0,  0,  0,  0,  0, -5,
 -5,  0,  0,  0,  0,  0,  0, -5,
 -5,  0,  0,  0,  0,  0,  0, -5,
 -5,  0,  0,  0,  0,  0,  0, -5,
  0,  0,  0,  5,  5,  0,  0,  0];

#[rustfmt::skip]
const QUEEN_PST: [i32; 64] = [
-20,-10,-10, -5, -5,-10,-10,-20,
-10,  0,  0,  0,  0,  0,  0,-10,
-10,  0,  5,  5,  5,  5,  0,-10,
 -5,  0,  5,  5,  5,  5,  0, -5,
  0,  0,  5,  5,  5,  5,  0, -5,
-10,  5,  5,  5,  5,  5,  0,-10,
-10,  0,  5,  0,  0,  0,  0,-10,
-20,-10,-10, -5, -5,-10,-10,-20];

#[rustfmt::skip]
const KING_PST: [i32; 64] = [
-30,-40,-40,-50,-50,-40,-40,-30,
-30,-40,-40,-50,-50,-40,-40,-30,
-30,-40,-40,-50,-50,-40,-40,-30,
-30,-40,-40,-50,-50,-40,-40,-30,
-20,-30,-30,-40,-40,-30,-30,-20,
-10,-20,-20,-20,-20,-20,-20,-10,
 20, 20,  0,  0,  0,  0, 20, 20,
 20, 30, 10,  0,  0, 10, 30, 20];

#[rustfmt::skip]
const KING_EG_PST: [i32; 64] = [
-50,-40,-30,-20,-20,-30,-40,-50,
-30,-20,-10,  0,  0,-10,-20,-30,
-30,-10, 20, 30, 30, 20,-10,-30,
-30,-10, 30, 40, 40, 30,-10,-30,
-30,-10, 30, 40, 40, 30,-10,-30,
-30,-10, 20, 30, 30, 20,-10,-30,
-30,-30,  0,  0,  0,  0,-30,-30,
-50,-30,-30,-30,-30,-30,-30,-50];

pub trait Scorable<Strategy> {
    fn eval(&self) -> Score;
    fn eval_material(&self) -> Score;
    fn eval_position(&self) -> Score;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Score {
    MinusInfinity,
    WhiteLoss,
    Millipawns(i32),
    WhiteWin,
    PlusInfinity,
}

impl Score {
    /// Outcome must be game ending else panic
    #[inline]
    pub fn from(o: Outcome) -> Score {
        if o.is_draw() {
            return Score::Millipawns(0);
        }
        if let Some(c) = o.winning_color() {
            return c.chooser_wb(Score::WhiteWin, Score::WhiteLoss);
        }
        panic!(format!("Tried to final score a non-final board outcome:{}", o));
    }

    #[inline]
    pub fn negate(self) -> Score {
        match self {
            Self::MinusInfinity => Self::PlusInfinity,
            Self::WhiteLoss => Self::WhiteWin,
            Self::Millipawns(s) => Self::Millipawns(-s),
            Self::WhiteWin => Self::WhiteLoss,
            Self::PlusInfinity => Self::MinusInfinity,
        }
    }
}

impl fmt::Display for Score {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// score config needs to be by colour and by MG/EG
// option to have minimizing nodes use different config
// what can we cache
// pass in alpha beta so eval can short circuit (lazy evaluation)
// some human-like tweaks: aggresive/defensive, open/closed preference, test an opening, lay traps, complicate the position,
// consider odd / even parity and tempo

pub struct SimpleScorer;

impl SimpleScorer {
    pub const MATERIAL_SCORES: [i32; Piece::ALL.len()] = [1000, 3250, 3500, 5000, 9000, 0];

    pub fn evaluate(board: &Board) -> Score {
        let outcome = board.outcome();
        if outcome.is_game_over() {
            return Score::from(outcome);
        }

        let mat = Material::from_board(board);
        let s = Self::evaluate_material(&mat);
        // let p = Self::evaluate_position(board);
        Score::Millipawns(s)
    }

    // always updated
    pub fn evaluate_mobility(_board: &Board) -> Score {
        panic!("Not implmented");
    }

    pub fn pst(p: Piece, sq: usize) -> i32 {
        SQUARE_VALUES[p][sq]
    }

    // piece positions, king safety, centre control
    // only updated for the colour thats moved - opponents(blockes) not relevant
    pub fn evaluate_position(board: &Board) -> Score {
        let mut sum = 0_i32;
        for &p in &Piece::ALL {
            let w = (board.pieces(p) & board.white()).swap_bytes();
            let b = board.pieces(p) & board.black();
            let score_w: i32 = w.iter().map(|bb| Self::pst(p, bb.first_square())).sum();
            let score_b: i32 = b.iter().map(|bb| Self::pst(p, bb.first_square())).sum();
            sum += score_w - score_b;
        }
        Score::Millipawns(sum * 10)
    }

    // updated on capture & promo
    pub fn evaluate_material(mat: &Material) -> i32 {
        let mut total = 0_i32;
        for &p in &Piece::ALL {
            total += Self::MATERIAL_SCORES[p.index()]
                * (mat.counts(Color::White, p) - mat.counts(Color::Black, p));
        }
        total
    }

    // static_exchangce_evaluation()
    // least_valuable_piece()
}

impl Scorable<SimpleScorer> for Board {
    fn eval(&self) -> Score {
        SimpleScorer::evaluate(self)
    }
    fn eval_material(&self) -> Score {
        let m = Material::from_board(self);
        let s = SimpleScorer::evaluate_material(&m);
        Score::Millipawns(s)
    }
    fn eval_position(&self) -> Score {
        SimpleScorer::evaluate_position(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::BoardBuf;
    use crate::catalog::Catalog;

    #[test]
    fn score_material() {
        assert_eq!(Score::Millipawns(1).negate(), Score::Millipawns(-1));
        assert_eq!(Score::WhiteWin.negate(), Score::WhiteLoss);
        assert_eq!(Score::WhiteLoss.negate(), Score::WhiteWin);
        assert_eq!(Score::MinusInfinity.negate(), Score::PlusInfinity);
        assert!(Score::MinusInfinity < Score::PlusInfinity);
        assert!(Score::Millipawns(-5) < Score::Millipawns(5));
        assert!(Score::Millipawns(5) < Score::WhiteWin);
        assert!(Score::Millipawns(1000) > Score::Millipawns(0));
        assert!(Score::WhiteWin < Score::PlusInfinity);
        assert!(Score::WhiteWin == Score::WhiteWin);

        let board = Catalog::starting_position();
        assert_eq!(board.eval(), Score::Millipawns(0));

        let starting_pos_score = 8 * 1000 + 2 * 3250 + 2 * 3500 + 2 * 5000 + 9000;
        let board = Catalog::white_starting_position();
        assert_eq!(board.eval_material(), Score::Millipawns(starting_pos_score));

        let board = Catalog::black_starting_position();
        assert_eq!(board.eval_material(), Score::Millipawns(starting_pos_score).negate());
    }

    #[test]
    fn score_position() {
        let bd = Board::parse_fen("8/P7/8/8/8/8/8/8 w - - 0 1").unwrap().as_board();
        assert_eq!(bd.eval_position(), Score::Millipawns(10 * 50));
        let bd = Board::parse_fen("8/4p3/8/8/8/8/8/8 w - - 0 1").unwrap().as_board();
        assert_eq!(bd.eval_position(), Score::Millipawns(10 * --20));
        let w = Catalog::white_starting_position();
        assert_eq!(w.eval_position(), Score::Millipawns(-950)); // 950 = 2 * (5-0-40-10)-5-0
        let b = Catalog::black_starting_position();
        assert_eq!(w.eval_position(), b.eval_position().negate());
    }
}
