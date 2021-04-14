use crate::board::movegen::MoveGen;
use crate::board::Board;
use crate::material::Material;
use crate::types::{Color, ScoreWdl};
use std::fmt;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    InProgress,
    WinWhite,
    WinBlack,
    WinOnTimeWhite,
    WinOnTimeBlack,
    DrawByAgreement,
    DrawStalemate,
    DrawRepitition3,
    DrawRepitition5,
    DrawInsufficientMaterial,
    DrawRule50,
    DrawRule75,
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Outcome {
    #[inline]
    pub fn is_draw(self) -> bool {
        !matches!(
            self,
            Self::InProgress | Self::WinWhite | Self::WinBlack | Self::WinOnTimeWhite | Self::WinOnTimeBlack
        )
    }

    pub fn reversed(self) -> Outcome {
        match self {
            Self::WinWhite => Self::WinBlack,
            Self::WinBlack => Self::WinWhite,
            Self::WinOnTimeWhite => Self::WinOnTimeBlack,
            Self::WinOnTimeBlack => Self::WinOnTimeWhite,
            _ => self,
        }
    }

    #[inline]
    pub fn winning_color(self) -> Option<Color> {
        match self {
            Self::WinWhite | Self::WinOnTimeWhite => Some(Color::White),
            Self::WinBlack | Self::WinOnTimeBlack => Some(Color::Black),
            _ => None,
        }
    }

    #[inline]
    pub fn is_game_over(self) -> bool {
        self != Self::InProgress
    }

    pub fn as_wdl(self) -> ScoreWdl {
        if self.is_draw() {
            return ScoreWdl::new(0, 1, 0);
        } else if self.winning_color() == Some(Color::White) {
            return ScoreWdl::new(1, 0, 0);
        } else if self.winning_color() == Some(Color::Black) {
            return ScoreWdl::new(0, 0, 1);
        }
        ScoreWdl::new(0, 0, 0)
    }

    pub fn as_pgn(self) -> String {
        if self.is_draw() {
            return String::from("1/2-1/2");
        } else if self.winning_color() == Some(Color::White) {
            return String::from("1-0");
        } else if self.winning_color() == Some(Color::Black) {
            return String::from("0-1");
        }
        String::from("*")
    }
}

pub trait GameEnd {
    fn cursory_outcome(&self) -> Option<Outcome>;
    fn outcome(&self) -> Outcome;
    fn outcome_with_claim_draw(&self) -> Outcome;
    fn position_repitition_count(&self) -> u32;
}

impl GameEnd for Board {
    fn outcome(&self) -> Outcome {
        if let Some(outcome) = self.cursory_outcome() {
            return outcome;
        }
        let legal_moves = self.legal_moves();
        let color_to_play = self.color_us();
        if legal_moves.len() == 0 {
            if self.is_in_check(color_to_play) {
                // white to play and in check with no moves => black win
                return color_to_play.chooser_wb(Outcome::WinBlack, Outcome::WinWhite);
            } else {
                return Outcome::DrawStalemate;
            }
        }
        Outcome::InProgress
    }

    fn cursory_outcome(&self) -> Option<Outcome> {
        // X InProgress = 0,
        // X WinWhite,
        // X WinBlack,
        // WinOnTimeWhite,
        // WinOnTimeBlack,
        // DrawByAgreement,
        // X DrawStalemate,
        // DrawRepitition3,
        // DrawRepitition5,
        // X DrawInsufficientMaterial,
        // DrawRule50,
        // X DrawRule75,
        if self.fifty_halfmove_clock() >= 75 {
            return Some(Outcome::DrawRule75);
        }

        if self.fifty_halfmove_clock() >= 75 {
            return Some(Outcome::DrawRule75);
        }

        if Material::from_board(self).is_insufficient() {
            return Some(Outcome::DrawInsufficientMaterial);
        }
        if self.position_repitition_count() >= 5 {
            return Some(Outcome::DrawRepitition5);
        }
        None
    }

    fn position_repitition_count(&self) -> u32 {
        // FIXME
        2
    }

    fn outcome_with_claim_draw(&self) -> Outcome {
        if self.fifty_halfmove_clock() >= 50 {
            return Outcome::DrawRule50;
        }
        if self.position_repitition_count() >= 3 {
            return Outcome::DrawRepitition3;
        }
        Outcome::InProgress
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::*;

    #[test]
    fn test_outcome() {
        assert_eq!(Outcome::InProgress.to_string(), "InProgress");
        assert_eq!(format!("{}", Outcome::InProgress), "InProgress");
        assert_eq!(format!("{}", Outcome::InProgress), "InProgress");
        assert_eq!(Outcome::WinBlack.is_draw(), false);
        assert_eq!(Outcome::WinBlack.winning_color(), Some(Color::Black));
        assert_eq!(Outcome::WinBlack.winning_color(), Some(Color::Black));
        assert_eq!(Outcome::DrawRule50.winning_color(), None);
    }

    #[test]
    fn test_checkmate() {
        assert!(Catalog::checkmates()[0].outcome() == Outcome::WinWhite);
        assert!(Catalog::checkmates()[1].outcome() == Outcome::WinBlack);
    }

    #[test]
    fn test_stalemate() {
        assert!(Catalog::stalemates()[0].outcome() == Outcome::DrawStalemate);
        assert!(Catalog::stalemates()[1].outcome() == Outcome::DrawStalemate);
    }
}
