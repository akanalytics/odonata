use crate::board::Board;
use crate::movelist::Move;
use crate::eval::eval::SimpleScorer;
use std::cmp;

impl SimpleScorer {

    // a rusty version of https://www.chessprogramming.org/SEE_-_The_Swap_Algorithm
    //
    pub fn eval_move_see(&self, board: &Board, mv: &Move) -> i32 {
        let mut gain: [i32;32] = [0;32]; 
        let mut d = 0;
        //let mayXray = board.pawns() | board.bishops() | board.rooks() | board.queens();
        let mut from = mv.from();
        let mut occ = board.black() | board.white();
        let mut attacker_color = board.color_us();
        let mut attackers = board.attacked_by(mv.to());  // will include the current 'mv' attacker
        gain[0] = self.material_scores[mv.capture_piece()];
        while !attackers.is_empty() {
            let mover = board.piece_at(from);
            d += 1; 
            gain[d]  = self.material_scores[mover.index()] - gain[d-1]; // what you are taking less what opp has
            eprintln!("{}\n{}: mover: {} from: {:?} for spec gain {:?}\n{}",board.to_fen(), d, mover, from, gain, attackers);
            // if cmp::max(-gain[d-1], gain[d]) < 0 {
            //     break; // safely prune as from here on its zero
            // } 
            
            attackers -= from; // reset bit in set to traverse
            occ -= from; // reset bit in temporary occupancy (for x-Rays)
            
            // if ( fromSet & mayXray )
            //     attadef |= considerXrays(occ, ..);
            attacker_color = attacker_color.opposite();
            from = board.least_valuable_piece(attackers & board.color(attacker_color));
        } 
       

        // so  1=wp:  bn x p, b x n,  r x b, q x r
        //   0=pawn = 1
        //   1=n    = 3-1 = 2
        //   2=b    = 3 - 2 = 1  
        //   3=r    = 5 - 1 = 4
        //   4=q    = 9 - 4 = 5 
        
        while d >= 2 {
            gain[d-2] = -cmp::max(-gain[d-2], gain[d-1]);
            d -= 1;
        }
        gain[0]
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::BoardBuf;
    use crate::catalog::Catalog;
    use crate::globals::constants::*;
    use crate::movelist::MoveValidator;

    #[test]
    fn test_see() {
        let b = Board::parse_fen("7k/8/8/8/8/q7/8/R6K w - - 0 1").unwrap();  // R v q
        let eval = SimpleScorer::new();
        let mv = b.parse_uci_move("a1a3").unwrap();
        assert_eq!(eval.eval_move_see(&b, &mv), 900);

        let b = Board::parse_fen("7k/8/8/8/1p6/q7/8/R6K w - - 0 1").unwrap();  //R v qp
        let mv = b.parse_uci_move("a1a3").unwrap();
        assert_eq!(eval.eval_move_see(&b, &mv), 400);
        

        let b = Board::parse_fen("7k/8/8/8/1p6/q7/2N5/R6K w - - 0 1").unwrap();  //RN v qp
        let mv = b.parse_uci_move("a1a3").unwrap();
        assert_eq!(eval.eval_move_see(&b, &mv), 500);  // +q+p -R = 500
 
        let b = Board::parse_fen("7k/8/8/8/1q6/p7/2N5/R6K w - - 0 1").unwrap();  //RN v pq
        let mv = b.parse_uci_move("a1a3").unwrap();
        assert_eq!(eval.eval_move_see(&b, &mv), 100);  // +p  = +100 (retake by queen doesnt occur)
    }
}