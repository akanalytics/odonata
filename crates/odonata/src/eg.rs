pub mod endgame;
pub mod zugzwang;

pub use crate::eg::{
    endgame::{EndGame, LikelyOutcome},
    zugzwang::Zugzwang,
};
