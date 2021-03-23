


    fn init() {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }




insufficient_material



// macro_rules! log {
//     ($msg:expr) => {
//         LOGGER.with(|logger| {
//             if let Some(logger) = logger.borrow().last() {
//                 logger.log($msg)
//             }
//         })
//     }
// }

// fn main() {
//     log!("a");
//     push_context(NetworkLogger, || {
//         log!("b");
//     });
//     log!("c");
// }

//     pub fn make_move_enum(&self, m: MoveEnum) -> Board {
//         let mut board = Board {
//             en_passant: Bitboard::EMPTY,
//             turn: self.turn.opposite(),
//             fullmove_count: if self.turn == Color::Black { self.fullmove_count + 1 } else { self.fullmove_count },
//             fifty_clock: self.fifty_clock + 1,
//             .. *self
//         };
//         match m {
//             MoveEnum::Quiet{ dest, src, mover } => {
//                 // clear one bit and set another for the move
//                 let from_to_bits = dest ^ src;
//                 board.pieces[mover.index()] ^= from_to_bits;
//                 board.colors[self.turn.index] ^= from_to_bits;
//             },
//             MoveEnum::Push { dest, src } => {
//                 board.fifty_clock = 0;
//                 let from_to_bits = dest ^ src;
//                 board.pieces[Piece::Pawn.index()] ^= from_to_bits;
//                 board.colors[self.turn.index] ^= from_to_bits;
//             }
//             MoveEnum::Null() => {
//             },
//             MoveEnum::Capture { dest, src, mover, capture } => {
//                 board.fifty_clock = 0;
//                 board.pieces[capture.index()].remove(dest);
//                 board.colors[board.turn.index].remove(dest);

//                 let from_to_bits = dest ^ src;
//                 board.pieces[mover.index()] ^= from_to_bits;
//                 board.colors[self.turn.index] ^= from_to_bits;
//             },
//             MoveEnum::Promo { dest, src, promo } => {
//                 board.fifty_clock = 0;
//                 let from_to_bits = dest ^ src;
//                 board.pieces[Piece::Pawn.index()].remove(src);
//                 board.pieces[promo.index()].insert(dest);
//                 board.colors[self.turn.index] ^= from_to_bits;
//             },
//             MoveEnum::PromoCapture { dest, src, promo, capture } => {
//                 board.fifty_clock = 0;
//                 board.pieces[capture.index()].remove(dest);
//                 board.colors[board.turn.index].remove(dest);

//                 let from_to_bits = dest ^ src;
//                 board.pieces[Piece::Pawn.index()].remove(src);
//                 board.pieces[promo.index()].insert(dest);
//                 board.colors[self.turn.index] ^= from_to_bits;
//             },
//             MoveEnum::EnPassant { dest, src, capture_sq } => {
//                 board.fifty_clock = 0;
//                 board.pieces[Piece::Pawn.index()].remove(capture_sq);
//                 board.colors[board.turn.index].remove(capture_sq);

//                 let from_to_bits = dest ^ src;
//                 board.pieces[Piece::Pawn.index()] ^= from_to_bits;
//                 board.colors[self.turn.index] ^= from_to_bits;
//             }
//             MoveEnum::Castle { king_dest, king_src, rook_dest, rook_src, right } => {
//                 // NO reset of fifty clock. MoveEnum king then move rook.
//                 board.castling.remove(right);
//                 let from_to_bits = king_dest ^ king_src;
//                 board.pieces[Piece::King.index()] ^= from_to_bits;
//                 board.colors[self.turn.index] ^= from_to_bits;

//                 let from_to_bits = rook_dest ^ rook_src;
//                 board.pieces[Piece::King.index()] ^= from_to_bits;
//                 board.colors[self.turn.index] ^= from_to_bits;
//             }
 

//         }
//         board
//     }
// }


// #[derive(Debug)]
// pub enum MoveEnum {
//     Promo { dest: Bitboard, src: Bitboard, promo: Piece },
//     PromoCapture { dest: Bitboard, src: Bitboard, promo: Piece, capture: Piece },
//     EnPassant { dest: Bitboard, src: Bitboard, capture_sq: Bitboard },
//     Push { dest: Bitboard, src: Bitboard },
//     Castle { king_dest: Bitboard, king_src: Bitboard, rook_dest: Bitboard, rook_src: Bitboard, right: CastlingRights },
//     Quiet { dest: Bitboard, src: Bitboard, mover: Piece },
//     Capture { dest: Bitboard, src: Bitboard, mover: Piece, capture: Piece },
//     Null(),
//     // DropAdd { dest: Bitboard, piece: Piece },
//     // DropRemove { dest: Bitboard, piece: Piece },
// }



use crate::board::{Board, Move};
use crate::board::makemove::MoveMaker;
use crate::eval::{Scorable, Score};
use crate::board::movegen::MoveGen;
use std::cmp;

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
#[derive(Debug)]
pub struct Node<'b> {
    parent: Option<&'b Node<'b>>,
    board: &'b Board,
    ply: u32,
    alpha: Score,
    beta: Score,
    score: Score,
    best_move: Move,
    // stats
    // leaf
    // pv
}

#[derive(Debug, Default)]
pub struct Search {
    max_depth: u32,
    node_count: u64, 
}


impl Search {


    #[inline]
    pub fn is_maximizing(&self, node: &Node) -> bool {
        node.ply % 2 == 0  // 0 ply looks at our moves - maximising
    }

    #[inline]
    pub fn is_leaf(&self, node: &Node) -> bool {
        node.ply == self.max_depth
    }

    #[inline]
    pub fn new_child<'c>(&mut self, parent: &'c mut Node<'c>, _mv: &Move, board: &'c Board) -> Node<'c> {
        let child = Node {
            parent: Some(parent),
            board, 
            alpha: parent.alpha, 
            beta: parent.beta, 
            ply: parent.ply + 1, 
            score: if self.is_maximizing(parent) { Score::PlusInfinity } else { Score::MinusInfinity },  // parent maximising => child isnt
            best_move: Default::default()
        };
        debug_assert!(child.alpha < child.beta);
        self.node_count += 1;
        child
    }


    pub fn alphabeta<'c>(&mut self, node: &'c mut Node<'c>) {
        if self.is_leaf(node) { 
            node.score = node.board.evaluate().negate();
            return;
        }
        for mv in node.board.legal_moves().iter() {
            let board2 = node.board.make_move(mv);
            let mut child = self.new_child(node, mv, &board2);
            self.alphabeta(&mut child);
            if child.score > Score::Millipawns(1000) {
                // info!("Ply:{}  Move:{} score:{} bm:{} a:{} b:{}", child.ply, mv.uci(), child.score, self.best_move, self.alpha, self.beta);
            }
            let is_cut = self.process_child(&mv, &child);
            if is_cut {
                break
            }
        }
        // end node
    }
    
    
    #[inline]
    pub fn process_child(&self, mv: &Move, child: &Node) -> bool {
        // let node = child.parent.unwrap();
        if self.is_maximizing(&node) {
            if child.score > node.score {
                node.score = child.score;
                node.best_move = *mv;  // FIXME: copy size?
            } 
            node.alpha = cmp::max(node.alpha, child.score);
        } else {
            if child.score < node.score {
                node.score = child.score;
                node.best_move = *mv;
            } 
            node.beta = cmp::min(node.beta, child.score);
        }
        node.alpha >= node.beta
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
    use crate::catalog::*;
    use crate::eval::*;
    use crate::board::boardbuf::*;

    fn init() {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }


    #[test]
    fn test_node() {
        init();
        let board = &Catalog::starting_position();
        let mut node = Node { parent: None, board, ply: 0, alpha: Score::MinusInfinity, beta: Score::PlusInfinity, score: Score::MinusInfinity, best_move: Default::default() };
        let mut s = Search{ max_depth: 6, ..Default::default() };
        s.alphabeta(&mut node);
        assert_eq!( format!("{:?}", node), "");
    }

    #[test]
    fn jons_chess_problem() {
        init();
        let board = &Board::parse_fen("2r2k2/5pp1/3p1b1p/2qPpP2/1p2B2P/pP3P2/2P1R3/2KRQ3 b - - 0 1").unwrap().as_board();
        println!("{}", board);
        let mut node = Node { parent: None,  board, ply: 0, alpha: Score::MinusInfinity, beta: Score::PlusInfinity, score: Score::MinusInfinity, best_move: Default::default() };
        let mut s = Search{ max_depth: 9, ..Default::default() };
        s.alphabeta(&mut node);
        println!("AndyFish best move: {} with score (+ve for white): {}", node.best_move, node.score);
        println!("Positions examined: {}", s.node_count);
    }
}


// impl Search {

//     pub fn new() -> Search {
//         Search
//     }




//     pub fn abort(&mut self) {

//     }

// }





#[derive(Clone, Debug, Default)]
pub struct Algo {
    max_depth: u32,
    minmax: bool,
    eval: SimpleScorer,
    stats: Stats,
    pub pv: PvTable,
    score: Option<Score>,
    clock: Clock,
    // child_thread: Arc<Option<thread::JoinHandle<Algo>>>,
    child_thread: AlgoThreadHandle,
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
        writeln!(f, "score            :{}", self.score.unwrap())?;
        writeln!(f, "depth            :{}", self.max_depth)?;
        writeln!(f, "minmax           :{}", self.minmax)?;
        write!(f, "{}", self.clock)?;
        write!(f, "{}", self.stats())?;
        Ok(())
    }
}


#[derive(Debug, Default)]
struct AlgoThreadHandle(Option<thread::JoinHandle<Algo>>);

impl Clone for AlgoThreadHandle {
    fn clone(&self) -> Self {
        Self(None)
    }
}

impl Algo {
    pub fn search_async(&mut self, board: Board) {
        debug_assert!(self.max_depth > 0);

        const FOUR_MB: usize = 4 * 1024 * 1024;
        let name = String::from("search");
        let builder = thread::Builder::new().name(name).stack_size(FOUR_MB);
        let mut algo = self.clone();
        self.child_thread = AlgoThreadHandle(Some(builder.spawn(move || algo.search(board)).unwrap()));

        // let mut res = Vec::with_capacity(n);
        // for child in children {
        // res.push(child.join().unwrap());
        // }
    }

    pub fn search_async_stop(&mut self) {
        self.clock.set_time_up();
        let mut option_thread = self.child_thread.0.take();
        let handle = option_thread.take().unwrap();
        let algo = handle.join().unwrap();
        self.stats = algo.stats;
        self.pv = algo.pv;
        self.score = algo.score;
        self.clock = algo.clock;
    }

    pub fn search(&mut self, mut board: Board) -> Algo {
        debug_assert!(self.max_depth > 0);
        self.clock.start();
        println!("start search\n{}", self.clock);
        let mut node = Node::root(&mut board);
        self.alphabeta(&mut node);
        self.stats.elapsed = self.clock.elapsed();
        self.score = Some(node.score);
        println!("end start search\n{}", self.clock);
        self.clone()
    }
