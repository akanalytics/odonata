use crate::board::{Board};





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


pub struct Score;

// score config needs to be by colour and by MG/EG
// option to have minimizing nodes use different config
// what can we cache
// some human-like tweaks: aggresive/defensive, open/closed preference, test an opening, lay traps, complicate the position, 

impl Score {
    
    
    total(&board: Board) -> Score; 


    // always updated
    mobility(&board: Board) -> Score;


    // piece positions, king safety, centre control
    // only updated for the colour thats moved - opponents(blockes) not relevant
    position(&board: Board) -> Score;

    // updated on capture & promo
    material( /* material*/ ) -> Score;

    // static_exchangce_evaluation()
    // least_valuable_piece()
}



pub trait Evaluation {

}

impl Evaluation for Board {

}
