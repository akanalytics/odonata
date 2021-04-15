use crate::board::Board;
use crate::config::{Config, Configurable};
use crate::log_debug;
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
    Centipawns(i32),
    WhiteWin { minus_ply: i32 }, // // WhiteWin(-5) < WhiteWin(-1)
    PlusInfinity,
}

impl Default for Score {
    fn default() -> Self { Self::MinusInfinity }
}



impl Score {
    #[inline]
    pub fn cp(centipawn: i32) -> Score {
        Score::Centipawns(centipawn)
    }

    /// Outcome must be game ending else panic
    #[inline]
    pub fn from_outcome(o: Outcome, ply: i32) -> Score {
        if o.is_draw() {
            return Score::Centipawns(0);
        }
        if let Some(c) = o.winning_color() {
            return c.chooser_wb(Score::WhiteWin { minus_ply: -ply }, Score::WhiteLoss { ply });
        }
        panic!("Tried to final score a non-final board outcome:{}", o);
    }

    pub fn is_mate(&self) -> bool {
        matches!(self, Self::WhiteLoss { ply: _ } | Self::WhiteWin { minus_ply: _ })
    }

    #[inline]
    pub fn negate(self) -> Score {
        match self {
            Self::MinusInfinity => Self::PlusInfinity,
            Self::WhiteLoss { ply } => Self::WhiteWin { minus_ply: -ply },
            Self::Centipawns(s) => Self::Centipawns(-s),
            Self::WhiteWin { minus_ply } => Self::WhiteLoss { ply: -minus_ply },
            Self::PlusInfinity => Self::MinusInfinity,
        }
    }
}

impl std::ops::Add for Score {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        if let Score::Centipawns(s2) = other {
            if let Score::Centipawns(s1) = self {
                return Score::Centipawns(s1 + s2);
            } else {
                return self; // if self is an infinite or mate then adding cp/mp makes no difference
            }
        }
        panic!("Can only add centipawns not {} + {}", self, other);
    }
}

impl std::ops::Sub for Score {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self {
        if let Score::Centipawns(s2) = other {
            if let Score::Centipawns(s1) = self {
                return Score::Centipawns(s1 - s2);
            } else {
                return self; // if self is an infinite or mate then subtracting cp/mp makes no difference
            }
        }
        panic!("Can only subtract centipawns not {} - {}", self, other);
    }
}

impl std::ops::Neg for Score {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        self.negate()
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
    pub material: bool,
    pub position: bool,
    pub mobility: bool,
    pub material_scores: [i32; Piece::ALL.len()],
}

impl Default for SimpleScorer {
    fn default() -> Self {
        SimpleScorer::new()
    }
}

impl Configurable for SimpleScorer {
    fn settings(&self, c: &mut Config) {
        c.set("eval.mobility", "type check default true");
        c.set("eval.position", "type check default true");
        c.set("eval.material", "type check default true");
        c.set(
            "eval.material.p",
            &("type spin min -10000 max 10000 default ".to_string() + &Piece::Pawn.centipawns().to_string()),
        );
        c.set(
            "eval.material.n",
            &("type spin min -10000 max 10000 default ".to_string()
                + &Piece::Knight.centipawns().to_string()),
        );
        c.set(
            "eval.material.b",
            &("type spin min -10000 max 10000 default ".to_string()
                + &Piece::Bishop.centipawns().to_string()),
        );
        c.set(
            "eval.material.r",
            &("type spin min -10000 max 10000 default ".to_string() + &Piece::Rook.centipawns().to_string()),
        );
        c.set(
            "eval.material.q",
            &("type spin min -10000 max 10000 default ".to_string() + &Piece::Queen.centipawns().to_string()),
        );
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
                self.material_scores[*p] = i as i32;
            }
        }
    }
}

impl fmt::Display for SimpleScorer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "material         : {}", self.material)?;
        writeln!(f, "position         : {}", self.position)?;
        writeln!(f, "mobility         : {}", self.mobility)?;
        writeln!(f, "material scores  : {:?}", self.material_scores)?;
        Ok(())
    }
}

// builder methods
impl SimpleScorer {
    pub fn new() -> Self {
        const MATERIAL_SCORES: [i32; Piece::ALL.len()] = [
            Piece::Pawn.centipawns(),
            Piece::Knight.centipawns(),
            Piece::Bishop.centipawns(),
            Piece::Rook.centipawns(),
            Piece::Queen.centipawns(),
            0, // king
        ];
        SimpleScorer { mobility: true, position: true, material: true, material_scores: MATERIAL_SCORES }
    }

    pub fn set_position(&mut self, enabled: bool) -> Self {
        self.position = enabled;
        self.clone()
    }
}

impl SimpleScorer {
    pub fn evaluate(&self, board: &Board) -> Score {
        let outcome = board.outcome();
        if outcome.is_game_over() {
            return Score::from_outcome(outcome, board.ply());
        }

        let s = if self.material {
            let mat = Material::from_board(board);
            self.evaluate_material(&mat)
        } else {
            0
        };
        let p = if self.position { self.evaluate_position(board) } else { 0 };
        Score::Centipawns(s + p)
    }

    //     // too expensive to check for checkmate, so we just quickly check some draw conditions
    //     if let Some(outcome) = board.cursory_outcome() {
    //         return Score::from(outcome);
    //     }

    //     let mat = Material::from_board(board);
    //     let s = Self::evaluate_material(&mat);
    //     // let s = Material::is_insufficient2(board);
    //     Score::Centipawns(s)
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
        sum
    }

    // updated on capture & promo
    pub fn evaluate_material(&self, mat: &Material) -> i32 {
        let mut total = 0_i32;
        for &p in &Piece::ALL {
            total +=
                self.material_scores[p.index()] * (mat.counts(Color::White, p) - mat.counts(Color::Black, p));
        }
        total
    }

    // static_exchangce_evaluation()
    // least_valuable_piece()
}

impl Scorable<SimpleScorer> for Board {
    #[inline]
    fn eval(&self, eval: &SimpleScorer) -> Score {
        eval.evaluate(self)
    }
    #[inline]
    fn eval_material(&self, eval: &SimpleScorer) -> Score {
        let m = Material::from_board(self);
        let s = eval.evaluate_material(&m);
        Score::Centipawns(s)
    }
    #[inline]
    fn eval_position(&self, eval: &SimpleScorer) -> Score {
        let s = eval.evaluate_position(self);
        Score::Centipawns(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::BoardBuf;
    use crate::catalog::Catalog;

    #[test]
    fn score_material() {
        assert_eq!(Score::Centipawns(1).negate(), Score::Centipawns(-1));
        assert_eq!(Score::WhiteWin { minus_ply: -1 }.negate(), Score::WhiteLoss { ply: 1 });
        assert_eq!(Score::WhiteLoss { ply: 1 }.negate(), Score::WhiteWin { minus_ply: -1 });
        assert_eq!(Score::MinusInfinity.negate(), Score::PlusInfinity);
        assert!(Score::MinusInfinity < Score::PlusInfinity);
        assert_eq!(Score::MinusInfinity.is_mate(), false);
        assert_eq!(Score::WhiteWin { minus_ply: 1 }.is_mate(), true );
        assert!(Score::Centipawns(-5) < Score::Centipawns(5));
        assert!(Score::Centipawns(5) < Score::WhiteWin { minus_ply: 0 });
        assert!(Score::Centipawns(100) > Score::Centipawns(0));
        assert!(Score::WhiteWin { minus_ply: 1 } < Score::PlusInfinity);
        assert!(Score::WhiteWin { minus_ply: 0 } == Score::WhiteWin { minus_ply: 0 });

        let board = Catalog::starting_position();
        let eval = &SimpleScorer::new();
        assert_eq!(board.eval(eval), Score::Centipawns(0));

        let starting_pos_score = 8 * 100 + 2 * 325 + 2 * 350 + 2 * 500 + 900;
        let board = Catalog::white_starting_position();
        assert_eq!(board.eval_material(eval), Score::Centipawns(starting_pos_score));

        let board = Catalog::black_starting_position();
        assert_eq!(board.eval_material(eval), Score::Centipawns(starting_pos_score).negate());
    }

    #[test]
    fn eval_configure() {
        let mut eval = SimpleScorer::new();
        eval.configure(&Config::new().set("eval.material.b", "700"));
        assert_eq!(eval.material_scores[Piece::Bishop], 700);

        let mut eval = SimpleScorer::new();
        eval.configure(&Config::new().set("eval.position", "false"));
        assert_eq!(eval.position, false);
    }

    #[test]
    fn score_position() {
        let eval = &SimpleScorer::new();

        let bd = Board::parse_fen("8/P7/8/8/8/8/8/8 w - - 0 1").unwrap().as_board();
        assert_eq!(bd.eval_position(eval), Score::Centipawns(50));

        let bd = Board::parse_fen("8/4p3/8/8/8/8/8/8 w - - 0 1").unwrap().as_board();
        assert_eq!(bd.eval_position(eval), Score::Centipawns(--35));

        let w = Catalog::white_starting_position();
        assert_eq!(w.eval_position(eval), Score::Centipawns(-125));

        let b = Catalog::black_starting_position();
        assert_eq!(w.eval_position(eval), b.eval_position(eval).negate());

        let bd = Board::parse_fen("8/8/8/8/8/8/p7/8 b - - 0 1").unwrap().as_board();
        assert_eq!(bd.eval_position(eval), Score::Centipawns(-50));
    }
}
