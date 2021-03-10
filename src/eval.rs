use crate::board::{Board};
use crate::material::{Material};
use crate::types::{Piece, Color};





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

#[derive(Copy, Clone, Default)]
pub struct Score {
    total: i32,   // millipawns, +ve = white advantage
}

// score config needs to be by colour and by MG/EG
// option to have minimizing nodes use different config
// what can we cache
// pass in alpha beta so eval can short circuit
// some human-like tweaks: aggresive/defensive, open/closed preference, test an opening, lay traps, complicate the position, 

impl Score {

    pub const MATERIAL_SCORES: [i32; Piece::ALL.len()] = [1000, 3250, 3500, 5000, 9000, 0 ]; 
    
    pub fn new(board: &Board) -> Score {
        let mut score: Score = Default::default();
        let mat = Material::count_from(board);
        score.evaluate_material(&mat);
        score
    }


    // always updated
    pub fn mobility(_board: &Board) -> Score {
       panic!("Not implmented");        
    }


    // piece positions, king safety, centre control
    // only updated for the colour thats moved - opponents(blockes) not relevant
    pub fn position(_board: &Board) -> Score {
        panic!("Not implmented");        
    }

    // updated on capture & promo
    pub fn evaluate_material(&mut self, mat: &Material) {
        for &p in &Piece::ALL {
            self.total += Self::MATERIAL_SCORES[p.index()] * (mat.counts(Color::White, p) - mat.counts(Color::Black, p));
        }
    }

    // static_exchangce_evaluation()
    // least_valuable_piece()
}



pub trait Scorable {
    fn evaluate(&self) -> Score;
}

impl Scorable for Board {
    fn evaluate(&self) -> Score {
        Score::new(self)
    }
}




#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;


    #[test]
    fn score_material() {
        let board = Catalog::starting_position();
        assert_eq!(Score::new(&board).total, 0);

        let starting_pos_score = 8 * 1000 + 2 * 3250 + 2 * 3500 + 2 * 5000 + 9000;
        let board = Catalog::white_starting_position();
        assert_eq!(Score::new(&board).total, starting_pos_score);

        let board = Catalog::black_starting_position();
        assert_eq!(Score::new(&board).total, -starting_pos_score);
    }

}