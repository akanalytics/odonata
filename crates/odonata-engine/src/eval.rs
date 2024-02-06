use odonata_base::{domain::staticeval::StaticEval, infra::config::Config};
use odonata_hce::Hce;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};
pub mod recognizer;

#[cfg(feature = "nnue")]
use odonata_nnue::{MaterialEval, Nnue};

use serde::Serialize;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Default, Clone, Serialize)]
pub enum Eval {
    #[default]
    Uninitialised,
    Hce(Hce),

    #[cfg(feature = "nnue")]
    MaterialEval(MaterialEval),
    #[cfg(feature = "nnue")]
    Nnue(Nnue),
}


// settings: [[eval, nnue]]
impl Eval {
    pub fn kind(&self) -> String {
        match self {
            Eval::Uninitialised => "Uninitialised",
            Eval::Hce(_) => "HCE",
            #[cfg(feature = "nnue")]
            Eval::MaterialEval(_) => "Material",
            #[cfg(feature = "nnue")]
            Eval::Nnue(_) => "NNUE",
        }
        .to_string()
    }

    pub fn configure(mut settings: HashMap<String, String>) -> anyhow::Result<Self> {
        let tag = settings.remove("eval").unwrap_or("hce".to_string());
        match tag.as_str() {
            "hce" => {
                let hce = Config::new()
                    .resource("eval.hce.toml")
                    .props(settings)
                    .env_var_props("ODONATA")
                    .allow_override_files()
                    .deserialize_node("eval")?;
                Ok(Eval::Hce(hce))
            }

            #[cfg(feature = "nnue")]
            "nnue" => {
                let nnue = Config::new()
                    .resource("eval.hce.toml")
                    .props(settings)
                    .env_var_props("ODONATA")
                    .allow_override_files()
                    .deserialize_node("eval")?;
                Ok(Eval::Nnue(nnue))
            }

            #[cfg(feature = "nnue")]
            "me" => {
                let me = Config::new()
                    .resource("eval.me.toml")
                    .props(settings)
                    .env_var_props("ODONATA")
                    .allow_override_files()
                    .deserialize_node("eval")?;
                Ok(Eval::MaterialEval(me))
            }
            eval => unreachable!("bad eval in configuration {eval}"),
        }
    }
}

impl Deref for Eval {
    type Target = dyn StaticEval;

    fn deref(&self) -> &Self::Target {
        match self {
            Eval::Hce(hce) => hce,
            #[cfg(feature = "nnue")]
            Eval::MaterialEval(me) => me,
            #[cfg(feature = "nnue")]
            Eval::Nnue(nnue) => nnue,
            Eval::Uninitialised => unreachable!("deref of uninitialized"),
        }
    }
}

impl DerefMut for Eval {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Eval::Hce(hce) => hce,
            #[cfg(feature = "nnue")]
            Eval::MaterialEval(me) => me,
            #[cfg(feature = "nnue")]
            Eval::Nnue(nnue) => nnue,
            Eval::Uninitialised => unreachable!("deref of uninitialized"),
        }
    }
}

// #[inline]
// pub fn eval_with_outcome(eval: &dyn StaticEval, b: &Board, nd: &Node) -> Score {
//     let outcome = b.outcome();
//     if outcome.is_game_over() {
//         if outcome.is_draw() {
//             return eval.eval_draw(b, nd.ply);
//         }
//         if let Some(c) = outcome.winning_color() {
//             if b.color_us() == c {
//                 return Score::we_win_in(nd.ply);
//             } else {
//                 return Score::we_lose_in(nd.ply);
//             }
//         }
//     }
//     eval.static_eval(b)
// }
// impl Board {
//     #[inline]
//     pub fn eval_draw(&self, _eval: &impl StaticEval, _nd: &Node) -> Score {
//         Score::from_cp(0)
//         // self.pov_score(eval.w_eval_draw(self, nd))
//         // pub fn w_eval_draw(&self, board: &Board, node: &Node) -> WhiteScore {
//         //     // draw score is +ve for playing a stronger opponent (we want a draw), neg for weaker
//         //     //
//         //     //  Engine Col   |  search ply   |  value to searcher   | Score to white
//         //     //     W               0                   +ve               +ve
//         //     //     B               0                   +ve               -ve
//         //     //     W               1 (oppo B)          -ve               +ve (a bonus to white opponet)
//         //     //     B               1 (oppo W)          -ve               -ve
//         //     // board.color_us() == Color::White => maximising
//         //     // +ve contempt => +ve score => aim for draw => opponent stronger than us
//         //     // board.color_us() == Color::Black => minimising
//         //     // +ve contempt => -ve score => aim for draw => opponent stronger than us
//         //     let contempt_weight = self.weight(&Attr::ContemptPenalty.into());
//         //     let mut contempt_pov =
//         //         Score::from_f32(contempt_weight.interpolate(board.phase(&self.phaser)));
//         //     if (node.ply % 2) == 1 {
//         //         contempt_pov = -contempt_pov;
//         //     }
//         //     board.white_score(contempt_pov)
//         // }
//     }

//     #[inline]
//     pub fn eval_move_see(&self, see: &See, mv: Move) -> Score {
//         Score::from_cp(see.eval_move_see(self, mv))
//     }

//     #[inline]
//     pub fn eval_move_material(&self, eval: &impl StaticEval, mv: Move) -> Score {
//         // FIXME! far too slow (-7 ELO)
//         eval.move_material_eval(mv, self).pov_score()
//     }

//     #[inline]
//     pub fn static_eval(&self, eval: &impl StaticEval) -> Score {
//         eval.static_eval(self).pov_score()
//     }
// }
