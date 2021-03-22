use crate::board::makemove::MoveMaker;
use crate::board::movegen::MoveGen;
use crate::board::Board;
use crate::eval::{Scorable, Score, SimpleScorer};
use crate::movelist::Move;
use crate::pvtable::PvTable;
use crate::search::stats::Stats;
use crate::search::clock::Clock;
use crate::types::Color;
use std::fmt;
use std::thread;


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
//   Quiescence Algo
//     static exchange evaluation < 0
//     delta pruning
//     standing pat

//   Selectivity
//   Mate Algo
//
// Scout and Friends
//   Scout
//   NegaScout
//   Principal Variation Algo (=+30%?)
//
// Alpha-Beta goes Best-First
//   NegaC*
//   MTD(f)
//   Alpha-Beta Conspiracy Algo
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
    fn root(board: &mut Board) -> Node {
        let score = if Self::is_maximizing(board) { Score::MinusInfinity } else { Score::PlusInfinity };
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
        let score = if Self::is_maximizing(board) { Score::MinusInfinity } else { Score::PlusInfinity };
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
pub struct Algo {
    max_depth: u32,
    minmax: bool,
    eval: SimpleScorer,
    stats: Stats,
    pub pv: PvTable,
    score: Option<Score>,
    clock: Clock,
    // Eval
    // Algo config
    // Time controls
    // Transposition table
}

/// builder methods
impl Algo {
    pub fn new() -> Algo {
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

impl fmt::Display for Algo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "pv               :{}", self.pv.extract_pv())?;
        writeln!(f, "depth            :{}", self.max_depth)?;
        writeln!(f, "minmax           :{}", self.minmax)?;
        writeln!(f, "pv               :{}", self.pv.extract_pv())?;
        writeln!(f, "score            :{}", self.score.unwrap())?;
        writeln!(f, "{}", self.stats())?;
        Ok(())
    }
}

impl Algo {
    pub fn algo_description(&self) -> String {
        format!(
            "{algo} depth:{depth}",
            algo = if self.minmax { "minmax" } else { "alphabeta" },
            depth = self.max_depth
        )
    }

    pub fn search_async(&mut self, board: Board) {
        debug_assert!(self.max_depth > 0);
        
        const FOUR_MB : usize = 4 * 1024 * 1024;
        let name = String::from("search");
        let builder = thread::Builder::new().name(name).stack_size(FOUR_MB);
        let mut algo = self.clone();
        let _child = builder.spawn(move || { algo.search(board) }).unwrap();
        

        // let mut res = Vec::with_capacity(n);
        // for child in children {
        // res.push(child.join().unwrap());
        // }

        let algo = _child.join().unwrap();
        self.stats = algo.stats;
        self.pv = algo.pv;
        self.score = algo.score;
        self.clock = algo.clock;
    }



    pub fn search(&mut self, mut board: Board) -> Algo {
        debug_assert!(self.max_depth > 0);
        self.clock.start();
        let mut node = Node::root(&mut board);
        self.alphabeta(&mut node);
        self.stats.elapsed = self.clock.elapsed();
        self.score = Some(node.score);
        self.clone()
    }

    pub fn stats(&self) -> Stats {
        self.stats
    }

    #[inline]
    pub fn is_leaf(&self, node: &Node) -> bool {
        node.ply == self.max_depth
    }

    pub fn alphabeta(&mut self, node: &mut Node) {
        if self.is_leaf(node) {
            node.score = node.board.eval(&self.eval);
            self.stats.leaf_nodes += 1;
            return;
        }
        self.stats.interior_nodes += 1;

        // bailing here means the score is +/- inf and wont be used
        if self.clock.time_up() {
            return;
        }
        let moves = node.board.legal_moves();
        if moves.is_empty() {
            node.score = node.board.eval(&self.eval); 
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
    }

    #[inline]
    pub fn process_child(&mut self, mv: &Move, node: &mut Node, child: &Node) -> bool {
        if Node::is_maximizing(node.board) {
            if child.score > node.score {
                node.score = child.score;
            }
            if child.score > node.alpha {
                node.alpha = child.score;
                self.pv.set(child.ply, mv);
                self.pv.propagate_from(child.ply);
            }
        } else {
            if child.score < node.score {
                node.score = child.score;
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
    use crate::movelist::MoveValidator;

    fn init() {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    #[test]
    fn test_node() {
        // init();
        let board = Catalog::starting_position();
        let mut eval = SimpleScorer::default();
        eval.position = false;
        let mut search = Algo::new().depth(3).minmax(true).eval(eval);
        search.search(board);
        assert_eq!(search.stats().total_nodes(), 1 + 20 + 400 + 8902 /* + 197_281 */);
        assert_eq!(search.stats().branching_factor().round() as u64, 21);

        let board = Catalog::starting_position();
        let mut eval = SimpleScorer::default();
        eval.position = false;
        let mut search = Algo::new().depth(4).minmax(false).eval(eval);
        search.search(board);
        assert_eq!(search.stats().total_nodes(), 1757);
        assert_eq!(search.stats().branching_factor().round() as u64, 2);
    }

    #[test]
    fn test_black_opening() {
        let mut board = Catalog::starting_position();
        board.set_turn(Color::Black);
        let mut search = Algo::new().depth(1).minmax(false);
        search.search(board);
        println!("{}", search);
        assert_eq!(search.pv.extract_pv()[0].uci(), "d7d5");
    }

    #[test]
    fn test_mate_in_2() {
        let board = Catalog::mate_in_2()[0].clone();
        let mut search = Algo::new().depth(3).minmax(false);
        search.search(board);
        assert_eq!(search.pv.extract_pv().to_string(), "d5f6, g7f6, c4f7");
        assert_eq!(search.score.unwrap(), Score::WhiteWin { minus_ply: -3 });
        println!("{}", search);
    }

    #[test]
    #[ignore]
    fn test_mate_in_3_sync() {
        let board = Catalog::mate_in_3()[0].clone();
        let mut search = Algo::new().depth(5).minmax(false);
        search.search(board.clone());
        let san = board.to_san_moves(&search.pv.extract_pv()).replace("\n", " ");
        println!("{}", search);
        assert_eq!(san, "1. Bb5+ c6 2. Qe6+ Qe7 3. Qxe7+");
        assert_eq!(search.score.unwrap(), Score::WhiteWin { minus_ply: -3 });
    }

    #[test]
    #[ignore]
    fn test_mate_in_3_async() {
        let board = Catalog::mate_in_3()[0].clone();
        let mut search = Algo::new().depth(5).minmax(false);
        search.search_async(board.clone());
        let san = board.to_san_moves(&search.pv.extract_pv()).replace("\n", " ");
        println!("{}", search);
        assert_eq!(san, "1. Bb5+ c6 2. Qe6+ Qe7 3. Qxe7+");
        assert_eq!(search.score.unwrap(), Score::WhiteWin { minus_ply: -3 });
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
        let mut search = Algo::new().depth(9).minmax(false).eval(eval); //9
        search.search(board);
        println!("{}", search);
    }
}
