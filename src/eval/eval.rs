use crate::board::Board;
use crate::config::{Config, Configurable};
use crate::eval::score::Score;
use crate::globals::counts;
use crate::attacks::ClassicalBitboard;
use crate::attacks::BitboardAttacks;
use crate::log_debug;
use crate::material::Material;
use crate::outcome::GameEnd;
use crate::types::{Color, Piece};
use std::fmt;

// eval1 = bl.scoring.material(p=300, b=400, n=700)
// eval2 = bl.scoring.position(endgame)

// for m in legal_moves:
//     bl.evaluate(m)
//     bl.evaluate(board + m)
//     score1 = eval1(board + m)
//     print(score1)::write_str
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
const SQUARE_VALUES_MG: [[i32; 64]; 6] =
    [PAWN_PST_MG, KNIGHT_PST, BISHOP_PST, ROOK_PST, QUEEN_PST, KING_PST_MG];
const SQUARE_VALUES_EG: [[i32; 64]; 6] =
    [PAWN_PST_EG, KNIGHT_PST, BISHOP_PST, ROOK_PST, QUEEN_PST, KING_PST_EG];

#[rustfmt::skip]
const PAWN_PST_MG: [i32; 64] = [
0,  0,  0,  0,  0,  0,  0,  0,
50, 50, 50, 50, 50, 50, 50, 50,
10, 10, 20, 30, 30, 20, 10, 10,
 5,  5, 10, 25, 25, 10,  5,  5,
 0,  0,  0, 20, 20,  0,  0,  0,
 5, -5,-10,  0,  0,-10, -5,  5,
 5, 10, 10,-35,-35, 10, 10,  5,
 0,  0,  0,  0,  0,  0,  0,  0];

#[rustfmt::skip]
 const PAWN_PST_EG: [i32; 64] = [
 0,  0,  0,  0,  0,  0,  0,  0,
 50, 50, 50, 50, 50, 50, 50, 50,
 30, 30, 30, 30, 30, 30, 30, 30,
 20, 20, 20, 20, 20, 20, 20, 20,
 10, 10, 10, 10, 10, 10, 10, 10,
  5,  5,  5,  5,  5,  5,  5,  5,
  0,  0,  0,  0,  0,  0,  0,  0,
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
const KING_PST_MG: [i32; 64] = [
-30,-40,-40,-50,-50,-40,-40,-30,
-30,-40,-40,-50,-50,-40,-40,-30,
-30,-40,-40,-50,-50,-40,-40,-30,
-30,-40,-40,-50,-50,-40,-40,-30,
-20,-30,-30,-40,-40,-30,-30,-20,
-10,-20,-20,-20,-20,-20,-20,-10,
 20, 20,  0,  0,  0,  0, 20, 20,
 20, 30, 10,  0,  0, 10, 30, 20];

#[rustfmt::skip]
const KING_PST_EG: [i32; 64] = [
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
    fn eval_quiescence(&self, eval: &SimpleScorer) -> Score;
    fn eval_material(&self, eval: &SimpleScorer) -> Score;
    fn eval_position(&self, eval: &SimpleScorer) -> Score;
}

#[derive(Copy, Clone, Debug)]
pub struct SimpleScorer {
    pub material: bool,
    pub position: bool,
    pub mobility: bool,
    pub pawn_doubled: i32,
    pub phasing: bool,
    pub contempt: i32,
    pub tempo: i32,
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
        c.set("eval.phasing", "type check default true");
        c.set("eval.pawn.doubled", &format!("type spin min -200 max 200 default {}", self.pawn_doubled));
        c.set(
            "eval.draw_score_contempt",
            &format!("type spin min -10000 max 10000 default {}", self.contempt),
        );
        c.set("eval.tempo", &format!("type spin min -1000 max 1000 default {}", self.tempo));
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
        self.phasing = c.bool("eval.phasing").unwrap_or(self.phasing);
        self.pawn_doubled = c.int("eval.pawn.doubled").unwrap_or(self.pawn_doubled as i64) as i32;
        self.contempt = c.int("eval.draw_score_contempt").unwrap_or(self.contempt as i64) as i32;
        self.tempo = c.int("eval.tempo").unwrap_or(self.tempo as i64) as i32;

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
        writeln!(f, "phasing          : {}", self.phasing)?;
        writeln!(f, "pawn.doubled     : {}", self.pawn_doubled)?;
        writeln!(f, "contempt         : {}", self.contempt)?;
        writeln!(f, "tempo            : {}", self.tempo)?;
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
        SimpleScorer {
            mobility: true,
            position: true,
            material: true,
            phasing: true,
            pawn_doubled: -10, 
            contempt: -20, // typically -ve
            tempo: 15,
            material_scores: MATERIAL_SCORES,
        }
    }

    pub fn set_position(&mut self, enabled: bool) -> Self {
        self.position = enabled;
        self.clone()
    }
}

impl SimpleScorer {
    pub fn evaluate(&self, board: &Board) -> Score {
        counts::EVAL_COUNT.increment();
        let outcome = board.outcome();
        if outcome.is_game_over() {
            return Score::score_from_outcome(
                self.contempt,
                outcome,
                board.color_us(),
                board.total_halfmoves(),
            );
        }
        self.evaluate_without_wdl(board)
    }

    pub fn eval_quiescence(&self, board: &Board) -> Score {
        counts::EVAL_COUNT.increment();
        // we check for insufficient material and 50/75 move draws.
        let outcome = board.draw_outcome();
        if let Some(outcome) = outcome {
            if outcome.is_game_over() {
                return Score::score_from_outcome(
                    self.contempt,
                    outcome,
                    board.color_us(),
                    board.total_halfmoves(),
                );
            }
        }
        self.evaluate_without_wdl(board)
    }
    fn evaluate_without_wdl(&self, board: &Board) -> Score {
        let s = if self.material {
            let mat = Material::from_board(board);
            self.evaluate_material(&mat)
        } else {
            0
        };
        let p = if self.position { self.evaluate_position(board) } else { 0 };
        let m = if self.mobility { self.evaluate_mobility(board) } else { 0 };
        let t = Score::side_to_move_score(self.tempo, board.color_us());
        Score::Cp(s + p + m) + t
    }


    // always updated
    pub fn evaluate_mobility(&self, b: &Board) -> i32 {
        let dp = ClassicalBitboard::doubled_pawns(b.white() & b.pawns()).popcount() - ClassicalBitboard::doubled_pawns(b.black() & b.pawns()).popcount();
        return self.pawn_doubled * dp as i32;
    }

    pub fn pst_mg(p: Piece, sq: usize) -> i32 {
        SQUARE_VALUES_MG[p][sq]
    }

    pub fn pst_eg(p: Piece, sq: usize) -> i32 {
        SQUARE_VALUES_EG[p][sq]
    }

    // piece positions, king safety, centre control
    // only updated for the colour thats moved - opponents(blockes) not relevant
    pub fn evaluate_position(&self, board: &Board) -> i32 {
        let mut sum = 0_i32;
        for &p in &Piece::ALL {
            let w = (board.pieces(p) & board.white()).swap_bytes();
            let b = board.pieces(p) & board.black();

            let w_mg: i32 = w.iter().map(|bb| Self::pst_mg(p, bb.first_square())).sum();
            let b_mg: i32 = b.iter().map(|bb| Self::pst_mg(p, bb.first_square())).sum();

            let w_eg: i32 = w.iter().map(|bb| Self::pst_eg(p, bb.first_square())).sum();
            let b_eg: i32 = b.iter().map(|bb| Self::pst_eg(p, bb.first_square())).sum();
            let eg_perc = if self.phasing { board.phase() } else { 0 };

            sum += ((w_mg - b_mg) * (100 - eg_perc) + (w_eg - b_eg) * eg_perc) / 100;
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
    fn eval_quiescence(&self, eval: &SimpleScorer) -> Score {
        eval.eval_quiescence(self)
    }

    #[inline]
    fn eval(&self, eval: &SimpleScorer) -> Score {
        eval.evaluate(self)
    }

    #[inline]
    fn eval_material(&self, eval: &SimpleScorer) -> Score {
        let m = Material::from_board(self);
        let s = eval.evaluate_material(&m);
        Score::Cp(s)
    }
    #[inline]
    fn eval_position(&self, eval: &SimpleScorer) -> Score {
        let s = eval.evaluate_position(self);
        Score::Cp(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::BoardBuf;
    use crate::catalog::Catalog;

    #[test]
    fn score_material() {
        let board = Catalog::starting_position();
        let eval = &SimpleScorer::new();
        assert_eq!(board.eval(eval), Score::Cp(0));

        let starting_pos_score = 8 * 100 + 2 * 325 + 2 * 350 + 2 * 500 + 900;
        let board = Catalog::white_starting_position();
        assert_eq!(board.eval_material(eval), Score::Cp(starting_pos_score));

        let board = Catalog::black_starting_position();
        assert_eq!(board.eval_material(eval), Score::Cp(starting_pos_score).negate());
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
        assert_eq!(bd.eval_position(eval), Score::Cp(50));

        let bd = Board::parse_fen("8/4p3/8/8/8/8/8/8 w - - 0 1").unwrap().as_board();
        assert_eq!(bd.phase(), 100);
        assert_eq!(bd.eval_position(eval), Score::Cp(0));

        let w = Catalog::white_starting_position();
        assert_eq!(w.eval_position(eval), Score::Cp(-130));

        let b = Catalog::black_starting_position();
        assert_eq!(w.eval_position(eval), b.eval_position(eval).negate());

        let bd = Board::parse_fen("8/8/8/8/8/8/p7/8 b - - 0 1").unwrap().as_board();
        assert_eq!(bd.eval_position(eval), Score::Cp(-50));
    }
}
