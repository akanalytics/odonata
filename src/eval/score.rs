use std::fmt;
use crate::types::{Color, Ply};
use crate::outcome::Outcome;



#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Score {
    MinusInf,
    WhiteLoss { ply: i32 }, // WhiteLoss(1) < WhiteLoss(5)
    Cp(i32),
    WhiteWin { minus_ply: i32 }, // // WhiteWin(-5) < WhiteWin(-1)
    PlusInf,
}

impl Default for Score {
    #[inline]
    fn default() -> Self { Self::MinusInf }
}



impl Score {
    #[inline]
    pub fn cp(centipawn: i32) -> Score {
        Score::Cp(centipawn)
    }


    // * score
	// 	* cp 
	// 		the score from the engine's point of view in centipawns.
	// 	* mate 
	// 		mate in y moves, not plies.
	// 		If the engine is getting mated use negativ values for y.
	// 	* lowerbound
	//       the score is just a lower bound.
	// 	* upperbound
	// 	   the score is just an upper bound.
    pub fn uci(self, c: Color) -> String {
        let score = match c {
            Color::White => self,
            Color::Black => self.negate() ,
        };
        // we assume we are now from white's point of view
        match score {
            Self::MinusInf => "cp -9999".to_string(),
            Self::WhiteLoss { ply } => format!("mate {}", ply),
            Self::Cp(cp) => format!("cp {}", cp),
            Self::WhiteWin { minus_ply } => format!("mate {}", -minus_ply),
            Self::PlusInf => "cp 9999".to_string(),
        }
    }

    pub fn side_to_move_score(tempo: i32, us: Color) -> Score {
        // axiom: were white
        // white to move => advantage, black to move means white has a disadvantage
        if us == Color::White {
            Score::cp(tempo * 0)
        } else {
            Score::cp(-tempo * 0)
        }
    }


    /// Outcome must be game ending else panic
    #[inline]
    pub fn score_from_outcome(contempt: i32, o: Outcome, us: Color, total_half_moves: Ply) -> Score {
        if o.is_draw() {
            // draw score is +ve for playing a stronger opponent (we want a draw), neg for weaker
            //
            //  Engine Col   |  search ply   |  value to searcher   | Score to white
            //     W               0                   +ve               +ve
            //     B               0                   +ve               -ve
            //     W               1 (oppo B)          -ve               +ve (a bonus to white opponet) 
            //     B               1 (oppo W)          -ve               -ve  
            // board.color_us() == Color::White => maximising
            // +ve contempt => +ve score => aim for draw => opponent stronger than us
            // board.color_us() == Color::Black => minimising
            // +ve contempt => -ve score => aim for draw => opponent stronger than us
            let contempt = us.chooser_wb(contempt, -contempt); 
            return Score::Cp(contempt);
        }
        if let Some(c) = o.winning_color() {
            return c.chooser_wb(Score::WhiteWin { minus_ply: -total_half_moves }, Score::WhiteLoss { ply: total_half_moves });
        }
        panic!("Tried to final score a non-final board outcome:{}", o);
    }

    pub fn is_mate(&self) -> bool {
        matches!(self, Self::WhiteLoss { ply: _ } | Self::WhiteWin { minus_ply: _ })
    }

    #[inline]
    pub fn negate(self) -> Score {
        match self {
            Self::MinusInf => Self::PlusInf,
            Self::WhiteLoss { ply } => Self::WhiteWin { minus_ply: -ply },
            Self::Cp(s) => Self::Cp(-s),
            Self::WhiteWin { minus_ply } => Self::WhiteLoss { ply: -minus_ply },
            Self::PlusInf => Self::MinusInf,
        }
    }

    // https://www.chessprogramming.org/Pawn_Advantage,_Win_Percentage,_and_Elo
    pub fn win_probability(&self) -> f32 {
        match &self {
            Self::MinusInf => 0.0,
            Self::WhiteLoss { ply: _ } => 0.0,
            Self::Cp(cp) => {
                let k = 4_f32;
                let w = 1.0 / (1.0 + 10_f32.powf(-cp as f32/k));
                w
            },
            Self::WhiteWin { minus_ply: _ } => 1.0,
            Self::PlusInf => 1.0,
        }
    }
}

impl std::ops::Add for Score {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        if let Score::Cp(s2) = other {
            if let Score::Cp(s1) = self {
                return Score::Cp(s1 + s2);
            } else {
                return self; // if self is an infinite or mate then adding cp/mp makes no difference
            }
        }
        panic!("Can only add centipawns not {} + {}", self, other);
    }
}

impl std::ops::Mul<Score> for i32 {
    type Output = Score;

    #[inline]
    fn mul(self, other: Score) -> Score {
        match other {
            Score::Cp(s) => Score::Cp( self * s),
            _ if self > 0 => other ,
            _ if self < 0 => -other ,
            _  => Score::Cp(0),
        }
    }
}

impl std::ops::Sub for Score {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self {
        if let Score::Cp(s2) = other {
            if let Score::Cp(s1) = self {
                return Score::Cp(s1 - s2);
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
        match self {
            Self::MinusInf => f.write_str("-inf"),
            Self::WhiteLoss { ply } => write!(f, "win({})", ply),
            Self::Cp(cp) => write!(f, "{} cp", cp),
            Self::WhiteWin { minus_ply } => write!(f, "loss({})", -minus_ply),
            Self::PlusInf => f.write_str("+inf"),
        }
    }
}

// score config needs to be by colour and by MG/EG
// option to have minimizing nodes use different config
// what can we cache
// pass in alpha beta so eval can short circuit (lazy evaluation)
// some human-like tweaks: aggresive/defensive, open/closed preference, test an opening, lay traps, complicate the position,
// consider odd / even parity and tempo




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score() {
        assert_eq!(Score::Cp(1).negate(), Score::Cp(-1));
        assert_eq!(Score::WhiteWin { minus_ply: -1 }.negate(), Score::WhiteLoss { ply: 1 });
        assert_eq!(Score::WhiteLoss { ply: 1 }.negate(), Score::WhiteWin { minus_ply: -1 });
        assert_eq!(Score::MinusInf.negate(), Score::PlusInf);
        assert!(Score::MinusInf < Score::PlusInf);
        assert_eq!(Score::MinusInf.is_mate(), false);
        assert_eq!(Score::WhiteWin { minus_ply: 1 }.is_mate(), true );
        assert!(Score::Cp(-5) < Score::Cp(5));
        assert!(Score::Cp(5) < Score::WhiteWin { minus_ply: 0 });
        assert!(Score::Cp(100) > Score::Cp(0));
        assert_eq!( 2* Score::Cp(100), Score::Cp(200));
        assert_eq!( -2 * Score::Cp(200), Score::Cp(-400));
        assert_eq!( -2 * Score::MinusInf , Score::PlusInf);
        assert_eq!( -2 * Score::PlusInf , Score::MinusInf);
        assert_eq!( 1 * Score::PlusInf , Score::PlusInf);
        assert_eq!( -1 * Score::WhiteWin{ minus_ply: 2 } , Score::WhiteLoss{ply: -2});
        assert_eq!( -3 * Score::WhiteWin{ minus_ply: 2 } , Score::WhiteLoss{ply: -2});
        assert_eq!( 1 * Score::WhiteWin{ minus_ply: 2 } , Score::WhiteWin{minus_ply: 2});
        assert!(Score::WhiteWin { minus_ply: 1 } < Score::PlusInf);
        assert!(Score::WhiteWin { minus_ply: 0 } == Score::WhiteWin { minus_ply: 0 });
        assert!(Score::Cp(0).win_probability() > 0.499 );
        assert!(Score::Cp(0).win_probability() < 0.501 );
        assert!(Score::Cp(1000).win_probability() > 0.95 );
        assert!(Score::Cp(-1000).win_probability() < 0.05 );
        assert!(Score::MinusInf.win_probability() < 0.001 );

    }
}