use crate::board::makemove::MoveMaker;
use crate::board::movegen::MoveGen;
use crate::board::Board;
use crate::eval::{Scorable, Score};
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
//   Principal Variation Search
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
#[derive(Debug, Clone)]
pub struct Node<'b> {
    board: &'b Board,
    ply: u32,
    alpha: Score,
    beta: Score,
    score: Score,
    best_move: Move,
    // stats
    // leaf
}

const MAX_PLY: usize = 128;

#[derive(Debug, Clone)]
pub struct PvTable {
    matrix: Vec<Vec<Move>>,
    size: usize,
}

impl PvTable {
    pub fn new() -> PvTable {
        let mut pvc = PvTable { matrix: vec![Vec::new(); MAX_PLY], size: 0 };
        for (r, row) in pvc.matrix.iter_mut().enumerate() {
            row.resize_with(MAX_PLY - r, Move::new_null)
            // row.extend( vec![Move::new(); r+1] );
        }
        pvc
    }
    pub fn set(&mut self, ply: usize, m: &Move) {
        self.matrix[ply][0] = m.clone();
        if self.size <= ply {
            self.size = ply + 1;
        }
    }

    pub fn propagate_from(&mut self, from_ply: usize) {
        // copy up one ply and accross one
        debug_assert!(from_ply > 0, "PV propagation from ply=0");
        let (top, bottom) = self.matrix.split_at_mut(from_ply);
        let len = self.size - from_ply;
        let dst = &mut top[from_ply-1][1..=len];
        let src = &mut bottom[0][0..len];
        dst.clone_from_slice(src);
    }
}

impl fmt::Display for PvTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for i in 0..self.size {
            write!(f, "{:>3}: ", i)?;
            for j in 0..self.size - i {
                write!(f, "{:>6}", self.matrix[i][j].uci())?;
            }
            writeln!(f)?
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Search {
    max_depth: u32,
    minmax: bool,

    // stats
    node_count: u64,

    // output
    best_move: Option<Move>,
    score: Option<Score>,
    //variations: Variations,
}

impl Search {
    pub fn new() -> Search {
        Default::default()
    }

    pub fn depth(&mut self, max_depth: u32) -> Self {
        self.max_depth = max_depth;
        *self
    }

    pub fn minmax(&mut self, minmax: bool) -> Self {
        self.minmax = minmax;
        *self
    }

    pub fn search(&mut self, board: &Board) {
        let mut node = Node {
            board,
            ply: 0,
            alpha: Score::MinusInfinity,
            beta: Score::PlusInfinity,
            score: Score::MinusInfinity,
            best_move: Default::default(),
        };
        self.alphabeta(&mut node);
        self.best_move = Some(node.best_move);
        self.score = Some(node.score);
    }

    #[inline]
    pub fn is_maximizing(&self, node: &Node) -> bool {
        // node.ply % 2 == 0 // 0 ply looks at our moves - maximising if white
        node.board.color_us() == Color::White
    }

    #[inline]
    pub fn is_leaf(&self, node: &Node) -> bool {
        node.ply == self.max_depth
    }

    #[inline]
    pub fn new_child<'c>(&mut self, _mv: &Move, parent: &Node, board: &'c Board) -> Node<'c> {
        let child = Node {
            board,
            alpha: parent.alpha,
            beta: parent.beta,
            ply: parent.ply + 1,
            score: if self.is_maximizing(parent) { Score::PlusInfinity } else { Score::MinusInfinity }, // parent maximising => child isnt
            best_move: Default::default(),
        };
        debug_assert!(child.alpha < child.beta || self.minmax);
        self.node_count += 1;
        child
    }

    pub fn alphabeta(&mut self, node: &mut Node) {
        if self.is_leaf(node) {
            node.score = node.board.evaluate();
            return;
        }
        for (i, mv) in node.board.legal_moves().iter().enumerate() {
            let child_board = node.board.make_move(mv);
            let mut child = self.new_child(mv, node, &child_board);
            self.alphabeta(&mut child);
            if node.ply == 0 {
                println!("{}. {}: score: {}", i, mv, child.score);
            }
            let is_cut = self.process_child(&mv, node, &child);
            if is_cut {
                break;
            }
        }
        // end node
    }
    #[inline]
    pub fn process_child(&self, mv: &Move, node: &mut Node, child: &Node) -> bool {
        if self.is_maximizing(&node) {
            node.alpha = cmp::max(node.alpha, child.score);
            if child.score > node.score {
                node.score = child.score;
                node.best_move = *mv; // FIXME: copy size?
            }
        } else {
            node.beta = cmp::min(node.beta, child.score);
            if child.score < node.score {
                node.score = child.score;
                node.best_move = *mv;
            }
        }
        node.alpha >= node.beta && !self.minmax
    }
}

// pub struct Search {
//     // Eval
//     // Search config
//     // Time controls
//     // Transposition table
// }

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
    fn test_pv_table() {
        let mut pvc = PvTable::new();
        pvc.set(0, &Move::parse("a1h1").unwrap());
        pvc.set(1, &Move::parse("b1h1").unwrap());
        pvc.set(2, &Move::parse("c1h1").unwrap());
        println!("{}", pvc);

        pvc.propagate_from(1);
        println!("{}", pvc);

        pvc.propagate_from(2);
        pvc.set(2, &Move::parse("c1h2").unwrap());
        println!("{}", pvc);

        // assert_eq!( format!("{:?}", pvc), "" );
        assert_eq!(format!("{}", pvc), "  0:   a2a3     -\n  1:   a1a4\n");
    }

    #[test]
    fn test_node() {
        // init();
        let board = &Catalog::starting_position();
        let mut search = Search::new().depth(4).minmax(true);
        search.search(board);
        assert_eq!(search.node_count, 20 + 400 + 8902 + 197_281);

        let board = &Catalog::starting_position();
        let mut search = Search::new().depth(4).minmax(false);
        search.search(board);
        assert_eq!(search.node_count, 1756);
    }

    #[test]
    #[ignore]
    fn jons_chess_problem() {
        init();
        let board = &BoardBuf::parse_fen("2r2k2/5pp1/3p1b1p/2qPpP2/1p2B2P/pP3P2/2P1R3/2KRQ3 b - - 0 1")
            .unwrap()
            .as_board();
        println!("{}", board);
        let mut search = Search::new().depth(9).minmax(false);
        search.search(board);
        println!(
            "AndyFish best move: {} with score (+ve for white): {}",
            search.best_move.unwrap(),
            search.score.unwrap()
        );
        println!("Positions examined: {}", search.node_count);
    }
}

// impl Search {

//     pub fn new() -> Search {
//         Search
//     }

//     pub fn abort(&mut self) {

//     }

// }
