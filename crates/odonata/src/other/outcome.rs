use crate::{boards::Board, piece::Color};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Copy, Hash, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    // see http://www.saremba.de/chessgml/standards/pgn/pgn-complete.htm#c9.8.1
    Unterminated,

    WinOther(Color),
    WinByCheckmate(Color),
    WinOnTime(Color),
    WinByAdjudication(Color),
    WinByResignation(Color),
    WinByRulesInfraction(Color),

    DrawByAdjudication,
    DrawByAgreement,
    DrawStalemate,
    DrawRepetition3,
    DrawRepetition5,
    DrawInsufficientMaterial,
    DrawRule50,
    DrawRule75,
    DrawOther,
    Abandoned,
}

impl Default for Outcome {
    fn default() -> Self {
        Self::Unterminated
    }
}

/// formatting with sign_minus, eg {outcome:-}, shows loss rather than win
impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match (self, f.sign_minus()) {
            (Self::Unterminated, _) => "unterminated",

            (Self::WinOther(_), false) => "win (other)",
            (Self::WinByCheckmate(_), false) => "checkmate",
            (Self::WinOnTime(_), false) => "win on time",
            (Self::WinByAdjudication(_), false) => "win by adjudication",
            (Self::WinByResignation(_), false) => "win by resignation",
            (Self::WinByRulesInfraction(_), false) => "win by illegal move",

            (Self::WinOther(_), true) => "loss (other)",
            (Self::WinByCheckmate(_), true) => "loss by checkmate",
            (Self::WinOnTime(_), true) => "loss on time",
            (Self::WinByAdjudication(_), true) => "loss by adjudication",
            (Self::WinByResignation(_), true) => "loss by resignation",
            (Self::WinByRulesInfraction(_), true) => "loss by illegal move",

            (Self::DrawByAgreement, _) => "draw by agreement",
            (Self::DrawByAdjudication, _) => "draw by adjudication",
            (Self::DrawStalemate, _) => "draw stalemate",
            (Self::DrawRepetition3, _) => "draw repetition 3",
            (Self::DrawRepetition5, _) => "draw repetition 5",
            (Self::DrawInsufficientMaterial, _) => "draw insufficient material",
            (Self::DrawRule50, _) => "draw rule 50",
            (Self::DrawRule75, _) => "draw rule 75",
            (Self::DrawOther, _) => "draw (other)",
            (Self::Abandoned, _) => "abandoned",
        };
        f.write_str(s)?;
        Ok(())
    }
}

impl Outcome {
    /// from Color's point of view
    pub fn to_string_pov(&self, c: Color) -> String {
        match (self, self.winning_color().map(|wc| wc == c)) {
            (_, None) => self.to_string(),           // draw things
            (_, Some(true)) => self.to_string(),     // we won
            (_, Some(false)) => format!("{self:-}"), // we lost
        }
    }

    #[inline]
    pub fn is_draw(self) -> bool {
        !matches!(
            self,
            Self::Unterminated
                | Self::WinOther(_)
                | Self::WinByCheckmate(_)
                | Self::WinOnTime(_)
                | Self::WinByResignation(_)
                | Self::WinByAdjudication(_)
                | Self::WinByRulesInfraction(_)
        )
    }

    pub fn reversed(self) -> Outcome {
        match self {
            Self::WinOther(c) => Self::WinOther(c.opposite()),
            Self::WinByCheckmate(c) => Self::WinByCheckmate(c.opposite()),
            Self::WinOnTime(c) => Self::WinOnTime(c.opposite()),
            Self::WinByResignation(c) => Self::WinByResignation(c.opposite()),
            Self::WinByAdjudication(c) => Self::WinByAdjudication(c.opposite()),
            Self::WinByRulesInfraction(c) => Self::WinByRulesInfraction(c.opposite()),
            _ => self,
        }
    }

    #[inline]
    pub fn winning_color(self) -> Option<Color> {
        match self {
            Self::WinOther(c) => Some(c),
            Self::WinByCheckmate(c) => Some(c),
            Self::WinOnTime(c) => Some(c),
            Self::WinByResignation(c) => Some(c),
            Self::WinByAdjudication(c) => Some(c),
            Self::WinByRulesInfraction(c) => Some(c),
            _ => None,
        }
    }

    #[inline]
    pub fn is_game_over(self) -> bool {
        self != Self::Unterminated
    }

    #[must_use]
    pub fn refine_from_pgn(
        &self,
        termination_header: Option<&str>,
        last_comment: Option<&str>,
    ) -> Outcome {
        use Outcome::*;
        let s =
            termination_header.unwrap_or_default().to_string() + last_comment.unwrap_or_default();
        let s = s.to_lowercase();
        match self {
            WinOther(c) if s.contains("mate") || s.contains("checkmate") => WinByCheckmate(*c),
            WinOther(c) if s.contains("time") => WinOnTime(*c),
            WinOther(c) if s.contains("adjudication") => WinByAdjudication(*c),
            WinOther(c) if s.contains("resign") => WinByResignation(*c),
            WinOther(c) if s.contains("illegal move") => WinByRulesInfraction(*c),
            WinOther(c) if s.contains("infraction") => WinByRulesInfraction(*c),

            DrawOther if s.contains("stalemate") => DrawStalemate,
            DrawOther if s.contains("material") || s.contains("insufficient") => {
                DrawInsufficientMaterial
            }
            DrawOther if s.contains("agreement") => DrawByAgreement,
            DrawOther if s.contains("adjudication") => DrawByAdjudication,
            DrawOther if s.contains("repetition") || s.contains("rep") || s.contains("3 fold") => {
                DrawRepetition3
            }
            DrawOther if s.contains(" 75") => DrawRule75,
            DrawOther if s.contains(" 50") || s.contains("fifty") => DrawRule50,
            _ => *self,
        }
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

    pub fn as_win_fraction(self) -> f32 {
        if self.is_draw() {
            return 0.5;
        } else if self.winning_color() == Some(Color::White) {
            return 1.0;
        } else if self.winning_color() == Some(Color::Black) {
            return 0.0;
        }
        unreachable!("Expected win, draw or loss, but found in progress outcome")
    }

    pub fn try_from_pgn(s: &str) -> Result<Outcome> {
        match s {
            "1/2-1/2" => Ok(Outcome::DrawOther),
            "1-0" => Ok(Outcome::WinOther(Color::White)),
            "0-1" => Ok(Outcome::WinOther(Color::Black)),
            "*" => Ok(Outcome::Unterminated),
            _ => Err(anyhow!("Unknown outcome token '{}'", s)),
        }
    }
}

// does not detect repetition counts
impl Board {
    pub fn outcome(&self) -> Outcome {
        if let Some(outcome) = self.draw_outcome() {
            return outcome;
        }
        let color_to_play = self.color_us();
        if self.is_in_check(color_to_play) {
            if !self.has_legal_moves() {
                // white to play and in check with no moves => black win
                return Outcome::WinByCheckmate(color_to_play.opposite());
            }
            return Outcome::Unterminated;
        } else if !self.has_legal_moves() {
            return Outcome::DrawStalemate;
        }
        Outcome::Unterminated
    }

    pub fn draw_outcome(&self) -> Option<Outcome> {
        // X InProgress = 0,
        // X WinWhite,
        // X WinBlack,
        // WinOnTimeWhite,
        // WinOnTimeBlack,
        // DrawByAgreement,
        // X DrawStalemate,
        // DrawRepetition3,
        // DrawRepetition5,
        // X DrawInsufficientMaterial,
        // DrawRule50,
        // X DrawRule75,
        if self.halfmove_clock() >= 2 * 50 {
            return Some(Outcome::DrawRule50);
        }

        // if self.fifty_halfmove_clock() >= 2 * 75 {
        //     return Some(Outcome::DrawRule75);
        // }

        if self.repetition_count().in_total >= 5 {
            return Some(Outcome::DrawRepetition5);
        }
        // if self.repetition_count().in_search >= 2 || self.repetition_count().in_total >= 2 {
        //     return Some(Outcome::DrawRepetition3);
        // }

        // either a repeat 3 times in the game OR
        // were in search and theres a repeat (in addition to itself)
        if self.repetition_count().in_game() >= 3
            || (self.repetition_count().in_search >= 1
                && self.repetition_count().in_game() + self.repetition_count().in_search >= 2)
        {
            return Some(Outcome::DrawRepetition3);
        }
        if self.material().is_insufficient() {
            return Some(Outcome::DrawInsufficientMaterial);
        }
        None
    }

    pub fn outcome_with_claim_draw(&self) -> Outcome {
        if self.halfmove_clock() >= 2 * 50 {
            return Outcome::DrawRule50;
        }
        Outcome::Unterminated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::*;

    #[test]
    fn test_outcome() {
        use Outcome::*;
        assert_eq!(Unterminated.to_string(), "unterminated");
        assert_eq!(Unterminated.to_string_pov(Color::Black), "unterminated");
        assert_eq!(Unterminated.to_string_pov(Color::White), "unterminated");
        assert_eq!(
            WinByCheckmate(Color::White).to_string_pov(Color::White),
            "checkmate"
        );
        assert_eq!(
            WinByCheckmate(Color::White).to_string_pov(Color::Black),
            "loss by checkmate"
        );
        assert_eq!(
            WinByCheckmate(Color::Black).to_string_pov(Color::White),
            "loss by checkmate"
        );

        assert_eq!(format!("{}", Unterminated), "unterminated");
        assert_eq!(format!("{}", WinByCheckmate(Color::Black)), "checkmate");
        assert_eq!(WinByCheckmate(Color::Black).is_draw(), false);
        assert_eq!(
            WinByCheckmate(Color::Black).winning_color(),
            Some(Color::Black)
        );
        assert_eq!(
            WinByCheckmate(Color::White).winning_color(),
            Some(Color::White)
        );
        assert_eq!(
            WinByResignation(Color::White).winning_color(),
            Some(Color::White)
        );
        assert_eq!(DrawRule50.winning_color(), None);
        assert_eq!(
            DrawOther.refine_from_pgn(Some("Repetion"), None),
            DrawRepetition3
        );
        assert_eq!(
            DrawStalemate.refine_from_pgn(Some("Repetion"), None),
            DrawStalemate
        );
        assert_eq!(
            DrawOther.refine_from_pgn(None, Some("Repetion")),
            DrawRepetition3
        );
        assert_eq!(
            Outcome::try_from_pgn("1-0")
                .unwrap()
                .refine_from_pgn(None, Some("mate")),
            WinByCheckmate(Color::White)
        );
    }

    #[test]
    fn test_checkmate() {
        assert_eq!(
            Catalog::checkmates()[0].board().outcome(),
            Outcome::WinByCheckmate(Color::White)
        );
        assert_eq!(
            Catalog::checkmates()[1].board().outcome(),
            Outcome::WinByCheckmate(Color::Black)
        );
    }

    #[test]
    fn test_stalemate() {
        assert_eq!(Catalog::stalemates()[0].outcome(), Outcome::DrawStalemate);
        assert_eq!(Catalog::stalemates()[1].outcome(), Outcome::DrawStalemate);
    }
}
