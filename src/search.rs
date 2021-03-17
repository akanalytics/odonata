use crate::board::makemove::MoveMaker;
use crate::board::movegen::MoveGen;
use crate::board::Board;
use crate::pvtable::PvTable;
use crate::eval::{Scorable, Score, SimpleScorer};
use crate::movelist::{Move, MoveList};
use crate::types::Color;
use std::cmp;
use std::fmt;

// CPW
//
// Obligatory
//
//   Futility pruning
//   Null move pruning
//   Transposition Table
//   Iterative Deepening
//   Aspiration Windows
//
// Selectivity
//   Quiescence Search
//     static exchange evaluation < 0
//     delta pruning
//     standing pat

//   Selectivity
//   Mate Search
//
// Scout and Friends
//   Scout
//   NegaScout
//   Principal Variation Search (=+30%?)
//
// Alpha-Beta goes Best-First
//   NegaC*
//   MTD(f)
//   Alpha-Beta Conspiracy Search
//

// taken from wikipedia
//
// function alphabeta(node, depth, α, β, maximizingPlayer) is
//     if depth = 0 or node is a terminal node then
//         return the heuristic value of node
//     if maximizingPlayer then
//         value := −∞
//         for each child of node do
//             value := max(value, alphabeta(child, depth − 1, α, β, FALSE))
//             α := max(α, value)
//             if α ≥ β then
//                 break (* β cutoff *)
//         return value
//     else
//         value := +∞
//         for each child of node do
//             value := min(value, alphabeta(child, depth − 1, α, β, TRUE))
//             β := min(β, value)
//             if β ≤ α then
//                 break (* α cutoff *)
//         return value
//
#[derive(Debug)]
pub struct Node<'b> {
    board: &'b mut Board,
    ply: u32,
    alpha: Score,
    beta: Score,
    score: Score,
    best_move: Move,
    // stats
    // leaf
}


impl Node<'_> {
    #[inline]
    fn root<'c>(board: &'c mut Board) -> Node<'c> {
        let score = if Self::is_maximizing(board) { Score::MinusInfinity } else { Score::PlusInfinity};
        Node {
            board,
            ply: 0,
            alpha: Score::MinusInfinity,
            beta: Score::PlusInfinity,
            score,
            best_move: Default::default(),
        }
    }

    #[inline]
    pub fn child<'c>(&self, _mv: &Move, board: &'c mut Board) -> Node<'c> {
        let score = if Self::is_maximizing(board) { Score::MinusInfinity } else { Score::PlusInfinity};
        Node {
            board,
            alpha: self.alpha,
            beta: self.beta,
            ply: self.ply + 1,
            score,
            best_move: Default::default(),
        }
    }

    #[inline]
    pub fn is_maximizing(board: &Board) -> bool {
        // node.ply % 2 == 0 // 0 ply looks at our moves - maximising if white
        board.color_us() == Color::White
    }
    

}





#[derive(Clone, Debug, Default)]
pub struct Search {
    max_depth: u32,
    minmax: bool,
    eval: SimpleScorer,

    // stats
    interior_nodes: u64,
    leaf_nodes: u64,  // FIXME and terminal 

    // output
    pv: PvTable,
    best_move: Option<Move>,
    score: Option<Score>,

    // Eval
    // Search config
    // Time controls
    // Transposition table
}

/// builder methods
impl Search {
    pub fn new() -> Search {
        Default::default()
    }

    pub fn depth(&mut self, max_depth: u32) -> Self {
        self.max_depth = max_depth;
        self.clone()
    }

    pub fn minmax(&mut self, minmax: bool) -> Self {
        self.minmax = minmax;
        self.clone()
    }

    pub fn eval(&mut self, eval: SimpleScorer) -> Self {
        self.eval = eval;
        self.clone()
    }
}


/// stats methods
impl Search {
    fn node_count(&self) -> u64 {
        self.interior_nodes + self.leaf_nodes // root
    }

    fn branching_factor(&self) -> f64 {
        self.leaf_nodes as f64 / self.interior_nodes as f64
    }
}


impl Search {    
    pub fn search(&mut self, mut board: Board) {
        let mut node = Node::root(&mut board);
        self.alphabeta(&mut node);
        self.best_move = Some(node.best_move);
        self.score = Some(node.score);
    }


    #[inline]
    pub fn is_leaf(&self, node: &Node) -> bool {
        node.ply == self.max_depth
    }


    pub fn alphabeta(&mut self, node: &mut Node) {
        if self.is_leaf(node) {
            node.score = node.board.eval(&self.eval);
            self.leaf_nodes += 1;
            return;
        } 
        self.interior_nodes += 1;

        let moves = node.board.legal_moves();
        if moves.is_empty() {
            node.score = node.board.eval(&self.eval); // FIXME evaluate with full checkmate logic
            return;
        }
        for (_i, mv) in moves.iter().enumerate() {
            let mut child_board = node.board.make_move(mv);
            let mut child = node.child(mv, &mut child_board);
            debug_assert!(child.alpha < child.beta || self.minmax);
            self.alphabeta(&mut child);
            // if child.ply == 1 {
            //     println!("{}. {}: score: {}", i, mv, child.score);
            // }
            let is_cut = self.process_child(&mv, node, &child);
            if is_cut {
                break;
            }
        }
        // end node
    }
   


    #[inline]
    pub fn process_child(&mut self, mv: &Move, node: &mut Node, child: &Node) -> bool {
        if Node::is_maximizing(node.board) {
            if child.score > node.score {
                node.score = child.score;
                node.best_move = *mv; // FIXME: copy size?
            }
            if child.score > node.alpha { 
                node.alpha = child.score;
                self.pv.set(child.ply, mv);
                self.pv.propagate_from(child.ply);
            }
        } else {
            if child.score < node.score {
                node.score = child.score;
                node.best_move = *mv;
            }
            if child.score < node.beta {
                node.beta = child.score;
                self.pv.set(child.ply, mv);
                self.pv.propagate_from(child.ply);
            }
        }
        node.alpha >= node.beta && !self.minmax
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::*;
    use crate::catalog::*;
    use crate::eval::*;

    fn init() {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    
    #[test]
    fn test_node() {
        // init();
        let board = Catalog::starting_position();
        let mut eval = SimpleScorer::default();
        eval.position = false;
        let mut search = Search::new().depth(3).minmax(true).eval(eval);
        search.search(board);
        assert_eq!(search.node_count(), 1 + 20 + 400 + 8902 /* + 197_281 */ );
        assert_eq!(search.branching_factor().round() as u64, 21);

        let board = Catalog::starting_position();
        let mut eval = SimpleScorer::default();
        eval.position = false;
        let mut search = Search::new().depth(4).minmax(false).eval(eval);
        search.search(board);
        assert_eq!(search.node_count(), 1757);
        assert_eq!(search.branching_factor().round() as u64 , 2);
    }


    #[test]
    fn test_shallow() {
        let board = Catalog::starting_position();
        let mut search = Search::new().depth(3).minmax(false);
        search.search(board);
    }


    #[test]
    fn test_mate_in_2() {
        let board = Catalog::mate_in_2()[0].clone();
        let mut search = Search::new().depth(3).minmax(false); 
        search.search(board);
        assert_eq!(search.pv.extract_pv().to_string(), "d5f6, g7f6, c4f7"); 
        assert_eq!(search.score.unwrap(), Score::WhiteWin); 
    }


    #[test]
    #[ignore]
    fn jons_chess_problem() {
        init();
        let board = Board::parse_fen("2r2k2/5pp1/3p1b1p/2qPpP2/1p2B2P/pP3P2/2P1R3/2KRQ3 b - - 0 1")
            .unwrap()
            .as_board();
        println!("{}", board);
        let mut eval = SimpleScorer::default();
        eval.position = false;
        let mut search = Search::new().depth(9).minmax(false).eval(eval); //9
        search.search(board);
        println!(
            "AndyFish best move: {} with score (+ve for white): {}",
            search.pv.extract_pv(),
            search.score.unwrap()
        );
        println!("Positions examined: {}   bf:{}", search.node_count(), search.branching_factor() );
    }
}


