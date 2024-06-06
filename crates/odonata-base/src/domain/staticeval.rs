use std::fmt::{Debug, Display};

use indexmap::IndexMap;

use crate::boards::Position;
use crate::domain::node::Node;
use crate::prelude::*;

#[derive(Default, Clone, Debug)]
pub struct EvalExplain {
    pub scores:          IndexMap<String, [f64; 6]>,
    pub additional_info: String,
}

// stockfish like evaluation explains
//
//   Contributing terms for the classical eval:
//   +------------+-------------+-------------+-------------+
//   |    Term    |    White    |    Black    |    Total    |
//   |            |   MG    EG  |   MG    EG  |   MG    EG  |
//   +------------+-------------+-------------+-------------+
//   |   Material |  ----  ---- |  ----  ---- |  0.00  0.00 |
//   |  Imbalance |  ----  ---- |  ----  ---- |  0.00  0.00 |
//   |      Pawns |  0.23 -0.04 |  0.23 -0.04 |  0.00  0.00 |
//   |    Knights | -0.04 -0.19 | -0.04 -0.19 |  0.00  0.00 |
//   |    Bishops | -0.00 -0.37 | -0.00 -0.37 |  0.00  0.00 |
//   |      Rooks | -0.26 -0.06 | -0.26 -0.06 |  0.00  0.00 |
//   |     Queens |  0.00  0.00 |  0.00  0.00 |  0.00  0.00 |
//   |   Mobility | -0.88 -1.15 | -0.88 -1.15 |  0.00  0.00 |
//   |King safety |  0.91 -0.11 |  0.91 -0.11 |  0.00  0.00 |
//   |    Threats |  0.00  0.00 |  0.00  0.00 |  0.00  0.00 |
//   |     Passed |  0.00  0.00 |  0.00  0.00 |  0.00  0.00 |
//   |      Space |  0.40  0.00 |  0.40  0.00 |  0.00  0.00 |
//   |   Winnable |  ----  ---- |  ----  ---- |  0.00  0.00 |
//   +------------+-------------+-------------+-------------+
//   |      Total |  ----  ---- |  ----  ---- |  0.00  0.00 |
//   +------------+-------------+-------------+-------------+
//
//   Classical evaluation   +0.00 (white side)
//   Final evaluation       +0.00 (white side)
//
impl Display for EvalExplain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[rustfmt::skip] writeln!(f, "+------------+-------------+-------------+-------------+")?;
        #[rustfmt::skip] writeln!(f, "|    Term    |    White    |    Black    |    Total    |")?;
        #[rustfmt::skip] writeln!(f, "|            |   MG    EG  |   MG    EG  |   MG    EG  |")?;
        #[rustfmt::skip] writeln!(f, "+------------+-------------+-------------+-------------+")?;
        for (category, [s0, s1, s2, s3, s4, s5]) in self.scores.iter() {
            if category == "Total" {
                #[rustfmt::skip] writeln!(f, "+------------+-------------+-------------+-------------+")?;
            }
            writeln!(
                f,
                "|{category:>11} | {s0:>5.2} {s1:>5.2} | {s2:>5.2} {s3:>5.2} | {s4:>5.2} {s5:>5.2} |"
            )?;
            if category == "Total" {
                #[rustfmt::skip] writeln!(f, "+------------+-------------+-------------+-------------+")?;
            }
        }
        writeln!(f, "{}", self.additional_info)?;
        Ok(())
    }
}

/// scores are POV from the side to move
pub trait StaticEval: Debug {
    fn new_game(&mut self);
    fn static_eval(&self, pos: &Position) -> Score;
    fn static_eval_explain(&self, pos: &Position) -> EvalExplain;
    fn move_material_eval(&self, mv: Move, b: &Board) -> Score;
    fn eval_move_see(&self, mv: Move, b: &Board) -> Score;
    fn piece_material_eval(&self, piece: Piece, b: &Board) -> f64;
    fn eval_draw(&self, b: &Board, ply: Ply) -> Score;
}

#[inline]
pub fn eval_with_outcome(eval: &dyn StaticEval, pos: &Position, nd: &Node) -> Score {
    let outcome = pos.outcome();
    if outcome.is_game_over() {
        if outcome.is_draw() {
            return eval.eval_draw(pos.board(), nd.ply);
        }
        if let Some(c) = outcome.winning_color() {
            if pos.board().color_us() == c {
                return Score::we_win_in(nd.ply);
            } else {
                return Score::we_lose_in(nd.ply);
            }
        }
    }
    eval.static_eval(pos)
}
