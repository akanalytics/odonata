use crate::bitboard::square::Square;
use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::eval::score::Score;
use crate::movelist::MoveList;
use crate::search::algo::Algo;
use crate::search::node::Node;
use crate::types::Ply;



impl Algo {
 

    pub fn qsearch_sq(
        &mut self,
        sq: Square,
        ply: Ply,
        board: &mut Board,
        mut alpha: Score,
        beta: Score,
    ) -> Score {
        self.report_progress();

        if self.time_up_or_cancelled(ply, false) {
            return alpha;
        }

        // this will handle mates too at quiescent node
        let standing_pat;
        if self.is_leaf(ply) {
            standing_pat = board.eval(&mut self.eval, &Node {ply, alpha, beta});
        } else {
            // in qsearch a mate score might mean a queen sacrifice. But in reality
            // opponent would just play some other move
            standing_pat = board.eval_qsearch(&mut self.eval, &Node {ply, alpha, beta});
        }

        // if standing_pat.is_mate() {
        //     return standing_pat;
        // }
        if standing_pat > alpha {
            if standing_pat >= beta {
                self.search_stats.inc_q_leaf_nodes(ply);
                return beta;
            }
            alpha = standing_pat;
        }

        // coarse delta pruning
        if standing_pat < alpha - self.qsearch.coarse_delta_prune {
            self.search_stats.inc_q_leaf_nodes(ply);
            return alpha;
        }

        let moves = board.legal_capture_moves();
        let mut moves: MoveList = moves
            .iter()
            .filter(|mv| mv.to() == sq)
            .cloned()
            .collect();

        if moves.is_empty() {
            self.search_stats.inc_q_leaf_nodes(ply);
        } else {
            self.search_stats.inc_q_interior_nodes(ply);
        }

        self.order_moves(ply, &mut moves, &None);

        for mv in moves.iter() {
            let mut child = board.make_move(mv);
            let score = -self.qsearch_sq(sq, ply + 1, &mut child, -beta, -alpha);
            debug_assert!(!score.is_mate()); // no mate scores except on leaf ply
            board.undo_move(mv);
            if score > beta {
                return score;
            }
            if score > alpha {
                alpha = score;
            }
        }
        alpha
    }
}
