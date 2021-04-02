use crate::board::Board;
use crate::material::Material;
use crate::outcome::{GameEnd, Outcome};
use crate::types::{Color, Piece};
use std::fmt;
use crate::config::{Config, Configurable};

#[macro_use]
use crate::log_debug;


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
 5, 10, 10,-35,-35, 10, 10,  5,
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
const _KING_EG_PST: [i32; 64] = [
-50,-40,-30,-20,-20,-30,-40,-50,
-30,-20,-10,  0,  0,-10,-20,-30,
-30,-10, 20, 30, 30, 20,-10,-30,
-30,-10, 30, 40, 40, 30,-10,-30,
-30,-10, 30, 40, 40, 30,-10,-30,
-30,-10, 20, 30, 30, 20,-10,-30,
-30,-30,  0,  0,  0,  0,-30,-30,
-50,-30,-30,-30,-30,-30,-30,-50];

pub trait Scorable<Strategy> {
    fn eval(&self, eval: &SimpleScorer) -> Score;
    fn eval_material(&self, eval: &SimpleScorer) -> Score;
    fn eval_position(&self, eval: &SimpleScorer) -> Score;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Score {
    MinusInfinity,
    WhiteLoss { ply: i32 }, // WhiteLoss(1) < WhiteLoss(5)
    Millipawns(i32),
    WhiteWin { minus_ply: i32 }, // // WhiteWin(-5) < WhiteWin(-1)
    PlusInfinity,
}

impl Score {
    /// Outcome must be game ending else panic
    #[inline]
    pub fn from(o: Outcome, ply: i32) -> Score {
        if o.is_draw() {
            return Score::Millipawns(0);
        }
        if let Some(c) = o.winning_color() {
            return c.chooser_wb(Score::WhiteWin { minus_ply: -ply }, Score::WhiteLoss { ply });
        }
        panic!("Tried to final score a non-final board outcome:{}", o);
    }

    #[inline]
    pub fn negate(self) -> Score {
        match self {
            Self::MinusInfinity => Self::PlusInfinity,
            Self::WhiteLoss { ply } => Self::WhiteWin { minus_ply: -ply },
            Self::Millipawns(s) => Self::Millipawns(-s),
            Self::WhiteWin { minus_ply } => Self::WhiteLoss { ply: -minus_ply },
            Self::PlusInfinity => Self::MinusInfinity,
        }
    }
}

impl std::ops::Add for Score {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        if let Score::Millipawns(s1) = self {
            if let Score::Millipawns(s2) = other {
                return Score::Millipawns(s1 + s2);
            }
        }
        panic!("Can only add millipawns not {} + {}", self, other);
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

#[derive(Copy, Clone, Debug)]
pub struct SimpleScorer {
    pub mobility: bool,
    pub position: bool,
    pub material: bool,
    pub material_scores: [i32; Piece::ALL.len()],
}

impl Default for SimpleScorer {
    fn default() -> Self {
        SimpleScorer::new()
    }
}

impl Configurable for SimpleScorer {
    fn define() -> Config {
        let mut c = Config::new();
        c.set("eval.mobility", "type check default true");
        c.set("eval.position", "type check default true");
        c.set("eval.material", "type check default true");
        c.set("eval.material.p", "type spin default 100 min -10000 max 10000");
        c.set("eval.material.n", "type spin default 325 min -10000 max 10000");
        c.set("eval.material.b", "type spin default 350 min -10000 max 10000");
        c.set("eval.material.r", "type spin default 500 min -10000 max 10000");
        c.set("eval.material.q", "type spin default 900 min -10000 max 10000");
        c
    }

    fn configure(&mut self, c: &Config) {
        log_debug!("eval.configure with {}", c);
        self.mobility = c.bool("eval.mobility").unwrap_or(self.mobility);
        self.position = c.bool("eval.position").unwrap_or(self.position);
        self.material = c.bool("eval.material").unwrap_or(self.material);
        for p in &Piece::ALL {
            let mut name = "eval.material.".to_string();
            name.push(p.to_char(Some(Color::Black)));
            if let Some(i) = c.int(&name) {
                self.material_scores[*p] = 10 * i as i32;
            }
        }
    }
}





// builder methods
impl SimpleScorer {
    pub fn new() -> Self {
        const MATERIAL_SCORES: [i32; Piece::ALL.len()] = [1000, 3250, 3500, 5000, 9000, 0];
        SimpleScorer { mobility: true, position: true, material: true, material_scores: MATERIAL_SCORES }
    }

    pub fn set_position(&mut self, enabled: bool ) -> Self {
        self.position = enabled;
        self.clone()
    }
}



impl SimpleScorer {

    pub fn evaluate(&self, board: &Board) -> Score {
        let outcome = board.outcome();
        if outcome.is_game_over() {
            return Score::from(outcome, board.ply());
        }

        let s = if self.material {
            let mat = Material::from_board(board);
            self.evaluate_material(&mat)
        } else {
            0
        };
        let p = if self.position { self.evaluate_position(board) } else { 0 };
        Score::Millipawns(s + p)
    }

    //     // too expensive to check for checkmate, so we just quickly check some draw conditions
    //     if let Some(outcome) = board.cursory_outcome() {
    //         return Score::from(outcome);
    //     }

    //     let mat = Material::from_board(board);
    //     let s = Self::evaluate_material(&mat);
    //     // let s = Material::is_insufficient2(board);
    //     Score::Millipawns(s)
    // }

    // always updated
    pub fn evaluate_mobility(&self, _board: &Board) -> Score {
        panic!("Not implmented");
    }

    pub fn pst(p: Piece, sq: usize) -> i32 {
        SQUARE_VALUES[p][sq]
    }

    // piece positions, king safety, centre control
    // only updated for the colour thats moved - opponents(blockes) not relevant
    pub fn evaluate_position(&self, board: &Board) -> i32 {
        let mut sum = 0_i32;
        for &p in &Piece::ALL {
            let w = (board.pieces(p) & board.white()).swap_bytes();
            let b = board.pieces(p) & board.black();
            let score_w: i32 = w.iter().map(|bb| Self::pst(p, bb.first_square())).sum();
            let score_b: i32 = b.iter().map(|bb| Self::pst(p, bb.first_square())).sum();
            sum += score_w - score_b;
        }
        sum * 10
    }

    // updated on capture & promo
    pub fn evaluate_material(&self, mat: &Material) -> i32 {
        let mut total = 0_i32;
        for &p in &Piece::ALL {
            total += self.material_scores[p.index()]
                * (mat.counts(Color::White, p) - mat.counts(Color::Black, p));
        }
        total
    }

    // static_exchangce_evaluation()
    // least_valuable_piece()
}

impl Scorable<SimpleScorer> for Board {
    fn eval(&self, eval: &SimpleScorer) -> Score {
        eval.evaluate(self)
    }
    fn eval_material(&self, eval: &SimpleScorer) -> Score {
        let m = Material::from_board(self);
        let s = eval.evaluate_material(&m);
        Score::Millipawns(s)
    }
    fn eval_position(&self, eval: &SimpleScorer) -> Score {
        let s = eval.evaluate_position(self);
        Score::Millipawns(s)
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
        assert_eq!(Score::WhiteWin { minus_ply: -1 }.negate(), Score::WhiteLoss { ply: 1 });
        assert_eq!(Score::WhiteLoss { ply: 1 }.negate(), Score::WhiteWin { minus_ply: -1 });
        assert_eq!(Score::MinusInfinity.negate(), Score::PlusInfinity);
        assert!(Score::MinusInfinity < Score::PlusInfinity);
        assert!(Score::Millipawns(-5) < Score::Millipawns(5));
        assert!(Score::Millipawns(5) < Score::WhiteWin { minus_ply: 0 });
        assert!(Score::Millipawns(1000) > Score::Millipawns(0));
        assert!(Score::WhiteWin { minus_ply: 1 } < Score::PlusInfinity);
        assert!(Score::WhiteWin { minus_ply: 0 } == Score::WhiteWin { minus_ply: 0 });

        let board = Catalog::starting_position();
        let eval = &SimpleScorer::new();
        assert_eq!(board.eval(eval), Score::Millipawns(0));

        let starting_pos_score = 8 * 1000 + 2 * 3250 + 2 * 3500 + 2 * 5000 + 9000;
        let board = Catalog::white_starting_position();
        assert_eq!(board.eval_material(eval), Score::Millipawns(starting_pos_score));

        let board = Catalog::black_starting_position();
        assert_eq!(board.eval_material(eval), Score::Millipawns(starting_pos_score).negate());
    }


    #[test]
    fn eval_configure() {
        let mut eval = SimpleScorer::new();
        eval.configure(&Config::new().set("eval.material.b", "700"));
        assert_eq!(eval.material_scores[Piece::Bishop], 7000);

        let mut eval = SimpleScorer::new();
        eval.configure(&Config::new().set("eval.position", "false"));
        assert_eq!(eval.position, false);
    }

    #[test]
    fn score_position() {
        let eval = &SimpleScorer::new();

        let bd = Board::parse_fen("8/P7/8/8/8/8/8/8 w - - 0 1").unwrap().as_board();
        assert_eq!(bd.eval_position(eval), Score::Millipawns(10 * 50));

        let bd = Board::parse_fen("8/4p3/8/8/8/8/8/8 w - - 0 1").unwrap().as_board();
        assert_eq!(bd.eval_position(eval), Score::Millipawns(10 * --35));

        let w = Catalog::white_starting_position();
        assert_eq!(w.eval_position(eval), Score::Millipawns(-1250));

        let b = Catalog::black_starting_position();
        assert_eq!(w.eval_position(eval), b.eval_position(eval).negate());

        let bd = Board::parse_fen("8/8/8/8/8/8/p7/8 b - - 0 1").unwrap().as_board();
        assert_eq!(bd.eval_position(eval), Score::Millipawns(10 * -50));
    }
}
