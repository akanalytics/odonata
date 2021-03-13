use crate::board::Board;
use crate::material::Material;
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
#[rustfmt::skip]
const PAWN_PST: [i32; 64] = [
0,  0,  0,  0,  0,  0,  0,  0,
50, 50, 50, 50, 50, 50, 50, 50,
10, 10, 20, 30, 30, 20, 10, 10,
 5,  5, 10, 25, 25, 10,  5,  5,
 0,  0,  0, 20, 20,  0,  0,  0,
 5, -5,-10,  0,  0,-10, -5,  5,
 5, 10, 10,-20,-20, 10, 10,  5,
 0,  0,  0,  0,  0,  0,  0,  0];














pub trait Scorable<Strategy> {
    fn evaluate(&self) -> Score;
}


#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Score {
    MinusInfinity,
    Loss,
    Millipawns(i32),
    Win,
    PlusInfinity,
}

impl Score {
    #[inline]
    pub fn negate(self) -> Score {
        match self {
            Self::MinusInfinity => Self::PlusInfinity,
            Self::Loss => Self::Win,
            Self::Millipawns(s) => Self::Millipawns(-s),
            Self::Win => Self::Loss,
            Self::PlusInfinity => Self::MinusInfinity,
        }
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


pub struct SimpleScorer;

impl SimpleScorer {
    pub const MATERIAL_SCORES: [i32; Piece::ALL.len()] = [1000, 3250, 3500, 5000, 9000, 0];

    pub fn evaluate_board(board: &Board) -> Score {
        let mat = Material::from_board(board);
        let s = Self::evaluate_material(&mat);
        // let p = Self::evaluate_position(board);
        Score::Millipawns(s)
    }




    // always updated
    pub fn mobility(_board: &Board) -> Score {
        panic!("Not implmented");
    }

    // piece positions, king safety, centre control
    // only updated for the colour thats moved - opponents(blockes) not relevant
    pub fn evaluate_position(board: &Board) -> i32 {
        panic!("Not implmented");
    }

    // updated on capture & promo
    pub fn evaluate_material(mat: &Material) -> i32 {
        let mut total = 0_i32;
        for &p in &Piece::ALL {
            total += Self::MATERIAL_SCORES[p.index()]
                * (mat.counts(Color::White, p) - mat.counts(Color::Black, p));
        }
        total
    }

    // static_exchangce_evaluation()
    // least_valuable_piece()
}


impl Scorable<SimpleScorer> for Board {
    fn evaluate(&self) -> Score {
        SimpleScorer::evaluate_board(self)
    }
}




#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;

    #[test]
    fn score_material() {

        assert_eq!( Score::Millipawns(1).negate(), Score::Millipawns(-1));
        assert_eq!( Score::Win.negate(), Score::Loss);
        assert_eq!( Score::Loss.negate(), Score::Win);
        assert_eq!( Score::MinusInfinity.negate(), Score::PlusInfinity);
        assert!( Score::MinusInfinity < Score::PlusInfinity);
        assert!( Score::Millipawns(-5) < Score::Millipawns(5));
        assert!( Score::Millipawns(5) < Score::Win);
        assert!( Score::Millipawns(1000) > Score::Millipawns(0));
        assert!( Score::Win < Score::PlusInfinity);
        assert!( Score::Win == Score::Win);


        let board = Catalog::starting_position();
        assert_eq!(board.evaluate(), Score::Millipawns(0));

        let starting_pos_score = 8 * 1000 + 2 * 3250 + 2 * 3500 + 2 * 5000 + 9000;
        let board = Catalog::white_starting_position();
        assert_eq!(board.evaluate(), Score::Millipawns(starting_pos_score));

        let board = Catalog::black_starting_position();
        assert_eq!(board.evaluate(), Score::Millipawns(starting_pos_score).negate());
    }
}
