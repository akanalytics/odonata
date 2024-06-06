use std::collections::HashMap;
use std::fmt::Display;
use std::path::PathBuf;

use odonata_base::boards::Position;
use odonata_base::domain::staticeval::{EvalExplain, StaticEval};
use odonata_base::eg::EndGame;
use odonata_base::infra::utils::Formatting;
use odonata_base::prelude::*;
use strum_macros::{Display, EnumString};

use self::hce::Hce;
use self::nnue::Nnue;
use self::weight::WeightOf;

pub mod evaluation;
pub mod feature;
pub mod hce;
pub mod network;
pub mod nnue;
pub mod recognizer;
pub mod scoring;
pub mod see;
pub mod vector;
pub mod weight;

// type NetworkType = Network768xH2<i16>;
// type NetworkType = Network768xH2<f64>;

#[derive(Debug, Clone)]
pub struct Eval {
    pub eval_kind:   EvalKind,
    pub hce:         Box<Hce>,
    pub nnue_file:   PathBuf,
    pub nnue:        Box<Nnue>,
    pub incremental: bool,
}

// impl Clone for Eval {
//     fn clone(&self) -> Self {
//         Self {
//             eval_kind:   self.eval_kind.clone(),
//             hce:         self.hce.clone(),
//             nnue_file:   self.nnue_file.clone(),
//             nnue:        self.nnue.box_clone(),
//             incremental: self.incremental.clone(),
//         }
//     }
// }

#[derive(Debug, Clone, PartialEq, EnumString, Display)]
pub enum EvalKind {
    Nnue,
    Material,
    Blend,
    Hce,
}

impl Default for Eval {
    fn default() -> Self {
        Self {
            eval_kind:   EvalKind::Blend,
            hce:         Default::default(),
            nnue:        Box::new(Nnue::from_file("").expect("unable to load")),
            incremental: true,
            nnue_file:   PathBuf::new(),
        }
    }
}

impl Display for Eval {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let delegate = match self.eval_kind {
            EvalKind::Hce => self.hce.to_string(),
            EvalKind::Material => String::new(),
            EvalKind::Nnue => format!(
                "{} file:{}",
                self.nnue,
                self.nnue_file.file_name().unwrap_or_default().to_string_lossy(),
            ),
            EvalKind::Blend => format!(
                "{} file:{}",
                self.nnue,
                self.nnue_file.file_name().unwrap_or_default().to_string_lossy()
            ),
        };
        write!(f, "{}: {}", self.eval_kind, delegate)?;
        Ok(())
    }
}

impl Eval {
    pub fn reload(&mut self) -> Result<()> {
        self.nnue = Box::new(Nnue::from_file(&self.nnue_file)?);
        // self.hce.reload_weights()?;
        self.new_game();
        Ok(())
    }
}

impl Configurable for Eval {
    fn set(&mut self, p: Param) -> Result<bool> {
        self.eval_kind.set(p.get("eval_kind"))?;
        self.incremental.set(p.get("incremental"))?;
        self.hce.set(p.get("hce"))?;
        if self.nnue_file.set(p.get("nnue_file"))? {
            self.reload()?;
        };
        Ok(p.is_modified())
    }
}

impl StaticEval for Eval {
    fn new_game(&mut self) {
        self.nnue.new_game();
    }

    fn static_eval(&self, eval_pos: &Position) -> Score {
        if self.eval_kind == EvalKind::Hce {
            return self.hce.static_eval(eval_pos);
        }

        let cp = match self.incremental {
            true => self.nnue.eval(eval_pos),
            false => self.nnue.eval_stateless(eval_pos.board()),
        } as i32;
        // let hce = self.hce.static_eval(eval_pos).as_i16() as i32;
        let sc = if self.eval_kind == EvalKind::Blend {
            let material = eval_pos.board().material().centipawns_as_white();
            let material = eval_pos.board().turn().chooser_wb(1, -1) * material;
            let wt = WeightOf::<i32>::new(cp, material / 3 + 5 * cp / 6); // 4/5 better
            wt.interpolate(eval_pos.board().phase(&self.hce.phaser))
        } else {
            cp
        };
        let pov = Score::from_cp(sc);

        let eg = EndGame::from_board(eval_pos.board());
        eg.endgame_score_adjust(eval_pos.board(), pov, &self.hce.endgame)
    }

    fn static_eval_explain(&self, pos: &Position) -> EvalExplain {
        if self.eval_kind == EvalKind::Hce {
            return self.hce.static_eval_explain(pos);
        }

        let mut cells = HashMap::new();
        for sq in pos.board.occupied().squares() {
            let p = pos.board().piece(sq).unwrap();
            let c = pos.board().color_of(sq).unwrap();
            let score1 = self.nnue.eval_stateless(pos.board());
            let mut bb = pos.board().clone().into_builder();
            bb.set_piece(sq, None);
            let score2 = self.nnue.eval_stateless(&bb.build());
            let cp = score1 - score2;
            let key = (7 - sq.rank_index(), sq.file_index());
            cells.insert(key, format!("\n{p:^9}\n\n{cp:^9}\n", p = p.to_char(c)));
        }
        let t = Formatting::to_table(cells, "\n\n\n\n\n");
        println!("{t}");

        use std::fmt::Write;
        let mut e = EvalExplain::default();

        let cp = self.nnue.eval(pos) as i32;
        let pov = Score::from_cp(cp);
        let sc = self.static_eval(pos);

        let eg = EndGame::from_board(pos.board());

        let mut ai = String::new();
        writeln!(&mut ai, "balance : {}", pos.board.material().balance()).unwrap();
        writeln!(&mut ai, "endgame : {}", eg).unwrap();

        let es = &self.hce.endgame;
        if let Some(winner) = eg.likely_winner(pos.board()) {
            if let Some((metric1, metric2)) = eg.metrics(winner, pos.board()) {
                writeln!(&mut ai, "ending m1: {}", metric1).unwrap();
                writeln!(&mut ai, "ending m2: {}", metric2).unwrap();
                writeln!(&mut ai, "certain win bonus awarded {es:#?}").unwrap();
            }
            writeln!(&mut ai, "regular win bonus awarded {es:#?}").unwrap();
        }

        writeln!(&mut ai, "pov nn score {pov}").unwrap();
        writeln!(&mut ai, "adj nn score {sc}").unwrap();
        writeln!(&mut ai, "\n{:#}", pos.board().to_diagram()).unwrap();
        e.additional_info = ai;
        e
    }

    fn move_material_eval(&self, mv: Move, b: &Board) -> Score {
        self.hce.move_material_eval(mv, b)
    }

    fn eval_move_see(&self, mv: Move, b: &Board) -> Score {
        self.hce.eval_move_see(mv, b)
    }

    fn piece_material_eval(&self, piece: Piece, b: &Board) -> f64 {
        self.hce.piece_material_eval(piece, b)
    }

    fn eval_draw(&self, _b: &Board, _ply: Ply) -> Score {
        Score::from_cp(0)
    }
}

#[cfg(test)]
mod tests {
    use test_log::test;

    use super::*;

    #[test]
    fn test_eval_nnue_explain() {
        let eval = Eval::default();
        let mut pos = Position::starting_pos();
        let _expl = eval.static_eval_explain(&pos);
        pos.push_moves_str("e2e4").unwrap();
        let _expl = eval.static_eval_explain(&pos);

        // println!("{expl}");
    }
}
