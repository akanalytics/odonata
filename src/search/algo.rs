use crate::board::makemove::MoveMaker;
use crate::board::movegen::MoveGen;
use crate::board::Board;
use crate::eval::{Scorable, Score, SimpleScorer};
use crate::movelist::Move;
use crate::pvtable::PvTable;
use crate::search::clock::{Clock, TimingMethod};
use crate::search::stats::Stats;
use crate::movelist::MoveList;
use crate::types::Color;
use std::fmt;
use std::ops::Range;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::sync::atomic;
use crate::types::MAX_PLY;



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

    #[inline]
    pub fn is_root(&self) -> bool {
        self.ply == 0
    }


}

// type AlgoSender = mpsc::Sender<String>;

type Func = dyn FnMut(&Algo) + Send + Sync;
type Callback = Arc<Mutex<Func>>;




#[derive(Clone, Default)]
pub struct Algo {
    max_depth: u32,
    range: Range<u32>,
    minmax: bool,
    iterative_deepening: bool,
    eval: SimpleScorer,
    stats: Stats,
    pub pv: PvTable,
    current_best: Option<Move>,
    best_move: Option<Move>,
    pub score: Option<Score>,
    clock: Clock,
    method: TimingMethod,

    callback: Option<Callback>,
    // child_thread: Arc<Option<thread::JoinHandle<Algo>>>,
    child_thread: AlgoThreadHandle,
    kill: Arc<atomic::AtomicBool>,
    clock_checks: u64,


    // Eval
    // Algo config
    // Time controls
    // Transposition table
}

/// builder methods
impl Algo {
    pub fn new() -> Algo {
        Algo::default()
    }


    pub fn set_iterative_deepening(&mut self, enable: bool) -> Self {
        self.iterative_deepening = enable;
        self.clone()
    }

    pub fn set_minmax(&mut self, minmax: bool) -> Self {
        self.minmax = minmax;
        self.clone()
    }

    pub fn set_eval(&mut self, eval: SimpleScorer) -> Self {
        self.eval = eval;
        self.clone()
    }

    pub fn set_timing_method(&mut self, tm: TimingMethod) -> Self {
        self.method = tm;
        self.clone()
    }

    //pub fn add_callback(&mut self, callback: dyn FnMut(String) -> bool + Send + Sync) -> Self {
    //}

    pub fn set_callback(&mut self, callback: Callback) -> Self {
        self.callback = Some(callback);
        self.clone()
     }
}



impl fmt::Debug for Algo  {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Algo")
            // .field("pv", &self.pv.extract_pv().)
            .field("score", &self.score)
            .field("best_move", &self.best_move)
            .field("current_best", &self.current_best)
            .field("depth", &self.max_depth)
            .field("minmax", &self.minmax)
            .field("method", &self.method)
            .field("eval", &self.eval)
            .field("iterative_deepening", &self.iterative_deepening)
            .field("method", &self.method)
            .field("depth", &self.max_depth)
            .field("range", &self.range)
            .field("stats", &self.stats)
            .field("clock", &self.clock)
            .finish()
    }
}

impl fmt::Display for Algo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "pv               :{}", self.pv.extract_pv())?;
        writeln!(f, "score            :{}", self.score.unwrap_or(Score::MinusInfinity))?;
        writeln!(f, "best_move        :{}", self.best_move.unwrap_or(Move::new_null()))?;
        writeln!(f, "current_best     :{}", self.current_best.unwrap_or(Move::new_null()))?;
        writeln!(f, "depth            :{}", self.max_depth)?;
        writeln!(f, "minmax           :{}", self.minmax)?;
        writeln!(f, "iter deepening   :{}", self.iterative_deepening)?;
        writeln!(f, "timing method    :{:?}", self.method)?;
        writeln!(f, "range            :{:?}", self.range)?;
        writeln!(f, "clock_checks     :{}", self.clock_checks)?;
        writeln!(f, "kill             :{}", self.kill.load(atomic::Ordering::SeqCst))?;
        writeln!(f, "kill ref counts  :{}", Arc::strong_count(&self.kill))?;
        // writeln!(f, "callback         :{}", self.callback)?;
        write!(f, "{}", self.clock)?;
        write!(f, "{}", self.stats)?;
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
    
    
    pub fn search_async(&mut self, board: Board)  {
        const FOUR_MB: usize = 4 * 1024 * 1024;
        let name = String::from("search");
        let builder = thread::Builder::new().name(name).stack_size(FOUR_MB);
        let mut algo = self.clone();
        self.kill.store(false, atomic::Ordering::SeqCst);
        self.child_thread = AlgoThreadHandle(Some(builder.spawn(move || algo.search(board)).unwrap()));

        // let mut res = Vec::with_capacity(n);
        // for child in children {
        // res.push(child.join().unwrap());
        // }
    }

    fn invoke_callback(&self) {
        if let Some(func) = &self.callback {
            let mut func = func.lock().unwrap();
            func(self);
        }
    }


    pub fn search(&mut self, mut board: Board) -> Algo {
        self.clock.start();
        self.stats = Stats::default();
        self.current_best = None;
        self.range = if let TimingMethod::Depth(depth) = self.method {
            if self.iterative_deepening {
                1..depth+1
            } else {
                depth..depth+1
            } 
        } else {
            // regardless of iterative deeping, we apply it if no explicit depth given
            1..MAX_PLY
        };

        for depth in self.range.clone() {
            self.set_iteration_depth(depth);
            self.score = None;
            self.best_move = None;
            self.pv = PvTable::new(MAX_PLY);
            let mut root_node = Node::root(&mut board);
            if self.time_up_or_cancelled(depth, self.stats().total_nodes(), true) {
                break;
            }
            self.alphabeta(&mut root_node);
            if !self.cancelled() {
                self.score = Some(root_node.score);
                self.current_best = Some(self.pv.extract_pv()[0]);
            }
            self.invoke_callback();
        }

        self.stats.recalculate_time_stats(self.clock.elapsed());
        self.best_move = self.current_best;
        self.invoke_callback();
        self.clone()
    }

    pub fn algo_description(&self) -> String {
        format!(
            "{algo} depth:{depth}",
            algo = if self.minmax { "minmax" } else { "alphabeta" },
            depth = self.max_depth
        )
    }


    #[inline]
    pub fn set_iteration_depth(&mut self, max_depth: u32) {
        self.max_depth = max_depth;
    }

    // FIXME recalculate time stats
    #[inline]
    pub fn stats(&self) -> Stats {
        self.stats
    }

    pub fn best_move(&self) -> Option<Move> {
        self.best_move             
    }


    #[inline]
    pub fn clock(&self) -> &Clock {
        &self.clock
    }


    pub fn search_async_stop(&mut self) {
        self.cancel();
        let handle = self.child_thread.0.take();
        if let Some(handle) = handle { 
            // wait for thread to cancel 
            let algo = handle.join().unwrap();
            self.stats = algo.stats;
            self.pv = algo.pv;
            self.score = algo.score;
            self.clock = algo.clock;
        }
    }



    #[inline]
    pub fn cancel(&mut self) {
        self.kill.store(true, atomic::Ordering::SeqCst);
    }


    #[inline]
    fn cancelled(&mut self) -> bool {
        let time_up = self.kill.load(atomic::Ordering::SeqCst);
        time_up
    }

    #[inline]
    pub fn time_up_or_cancelled(&mut self, ply: u32, nodes: u64, force_check: bool) -> bool {
        self.clock_checks += 1;

        // never cancel on ply=1, this way we always have a best move, and we detect mates
        if self.max_depth == 1 {
            return false;
        }

        if self.cancelled() {
            return true;
        }

        // only do this every 128th call to avoid expensive time computation
        if !force_check && self.clock_checks % 128 != 0 {
            return false;
        }


        let time_up = match self.method {
            TimingMethod::Depth(max_ply) => ply > max_ply,
            TimingMethod::MoveTime(duration) => self.clock().elapsed() > duration,
            TimingMethod::NodeCount(max_nodes) => nodes > max_nodes,
            TimingMethod::Infinite => false,
            TimingMethod::MateIn(_) => false,
            TimingMethod::RemainingTime { our_color, wtime, btime, winc, binc, movestogo: _ } => {
                let (time, _inc) = our_color.chooser_wb((wtime, winc), (btime, binc));
                self.clock().elapsed().as_millis() > time.as_millis() / 30
            }
        };
        if time_up {
            self.cancel();
        }
        time_up
    }



    #[inline]
    pub fn is_leaf(&self, node: &Node) -> bool {
        node.ply == self.max_depth
    }


    fn order_moves(&self, node: &Node, movelist: &mut MoveList) {
        if node.is_root() {
            if let Some(current_best) = self.current_best {
                if let Some(i) = movelist.iter().position(|mv| mv == &current_best) {
                    // println!("Swapped moves on depth {}!", self.max_depth);
                    movelist.swap(0, i);
                }
            }
        }
    }

    pub fn alphabeta(&mut self, node: &mut Node) {
        debug_assert!(self.max_depth > 0);
        if self.stats.total_nodes() % 1000000 == 0 {
            self.invoke_callback();
        }

        if self.is_leaf(node) {
            node.score = node.board.eval(&self.eval);
            self.stats.leaf_nodes += 1;
            return;
        }
        self.stats.interior_nodes += 1;
        if self.max_depth > self.stats.seldepth {
            self.stats.seldepth = self.max_depth;
            self.stats.depth = self.max_depth;
            self.invoke_callback();
        }
        // bailing here means the score is +/- inf and wont be used
        if self.time_up_or_cancelled(node.ply, self.stats.total_nodes(), false) {
            return;
        }

        let mut moves = node.board.legal_moves();
        if moves.is_empty() {
            node.score = node.board.eval(&self.eval);
            return;
        }

        self.order_moves(&node, &mut moves);

        for (_i, mv) in moves.iter().enumerate() {
            let mut child_board = node.board.make_move(mv);
            let mut child = node.child(mv, &mut child_board);
            debug_assert!(child.alpha < child.beta || self.minmax);
            self.alphabeta(&mut child);
            let is_cut = self.process_child(&mv, node, &child);
            if is_cut {
                self.stats.cuts += 1; 
                break;
            }
        }
    }

    #[inline]
    pub fn process_child(&mut self, mv: &Move, parent: &mut Node, child: &Node) -> bool {
        if Node::is_maximizing(parent.board) {
            if child.score > parent.score {
                parent.score = child.score;
            }
            if child.score > parent.alpha {
                parent.alpha = child.score;
                self.pv.set(child.ply, mv);
                self.pv.propagate_from(child.ply);
                self.stats.improvements += 1; 
                if parent.is_root() {
                    self.invoke_callback();
                }
            }
        } else {
            if child.score < parent.score {
                parent.score = child.score;
            }
            if child.score < parent.beta {
                parent.beta = child.score;
                self.pv.set(child.ply, mv);
                self.pv.propagate_from(child.ply);
                self.stats.improvements += 1; 
                if parent.is_root() {
                    self.invoke_callback();
                }
            }
        }
        parent.alpha >= parent.beta && !self.minmax
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::*;
    use crate::catalog::*;
    use crate::eval::*;
    use crate::movelist::MoveValidator;
    use crate::comms::uci::Uci;
    use std::time;

    fn init() {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    #[test]
    fn test_node() {
        // init();
        let board = Catalog::starting_position();
        let eval = SimpleScorer::new().set_position(false);
        let mut search = Algo::new().set_timing_method(TimingMethod::Depth(3)).set_minmax(true).set_eval(eval);
        search.search(board);
        assert_eq!(search.stats().total_nodes(), 1 + 20 + 400 + 8902 /* + 197_281 */);
        assert_eq!(search.stats().branching_factor().round() as u64, 21);

        let board = Catalog::starting_position();
        let eval = SimpleScorer::new().set_position(false);
        let mut search = Algo::new().set_timing_method(TimingMethod::Depth(4)).set_minmax(false).set_eval(eval);
        search.search(board);
        assert_eq!(search.stats().total_nodes(), 1757);
        assert_eq!(search.stats().branching_factor().round() as u64, 2);
    }

    #[test]
    fn test_black_opening() {
        let mut board = Catalog::starting_position();
        board.set_turn(Color::Black);
        let mut search = Algo::new().set_timing_method(TimingMethod::Depth(1)).set_minmax(false);
        search.search(board);
        println!("{}", search);
        assert_eq!(search.pv.extract_pv()[0].uci(), "d7d5");
    }

    #[test]
    fn test_mate_in_2() {
        let board = Catalog::mate_in_2()[0].clone();
        let mut search = Algo::new().set_timing_method(TimingMethod::Depth(3)).set_minmax(false);
        search.search(board);
        assert_eq!(search.pv.extract_pv().to_string(), "d5f6, g7f6, c4f7");
        assert_eq!(search.score.unwrap(), Score::WhiteWin { minus_ply: -3 });
        println!("{}", search);
    }

    #[test]
    fn test_mate_in_2_async() {
        let board = Catalog::mate_in_2()[0].clone();
        let mut algo = Algo::new().set_timing_method(TimingMethod::Depth(3)).set_minmax(true);
        algo.search(board.clone());
        let nodes = algo.stats().total_nodes();
        let millis = time::Duration::from_millis(20);
        thread::sleep(millis);

        assert_eq!(nodes, 66234);
        assert_eq!(algo.pv.extract_pv().to_string(), "d5f6, g7f6, c4f7");
        assert_eq!(algo.score.unwrap(), Score::WhiteWin { minus_ply: -3 });
        println!("{}\n\nasync....", algo);
    }

    #[test]
    fn test_mate_in_2_async_stopped() {
        let board = Catalog::mate_in_2()[0].clone();
        let mut algo2 = Algo::new().set_timing_method(TimingMethod::Depth(3)).set_minmax(true);
        let clos = |algo :&Algo| { println!("nps {}", algo.stats().knps()); };
        let am = Arc::new(Mutex::new(clos));
        algo2.set_callback( am );
        algo2.search_async(board.clone());
        let millis = time::Duration::from_millis(200);
        thread::sleep(millis);
        algo2.search_async_stop();
        println!("{}", algo2);
        // println!("after stop clock:\n{}", algo.clock);
        let nodes = algo2.stats().total_nodes();
        assert!(nodes > 10 && nodes < 66234);
    }

    #[test]
    #[ignore]
    fn test_mate_in_3_sync() {
        let board = Catalog::mate_in_3()[0].clone();
        let mut search = Algo::new().set_timing_method(TimingMethod::Depth(5)).set_minmax(false);
        search.search(board.clone());
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
        let eval = SimpleScorer::new().set_position(false);
        let mut search = Algo::new().set_timing_method(TimingMethod::Depth(9)).set_minmax(false).set_eval(eval); //9
        search.search(board);
        println!("{}", search);
    }


    #[test]
    fn debug_arena_issue() {
        let board = Board::parse_fen("r1bqkbnr/pppppppp/2n5/8/3P4/4P3/PPP2PPP/RNBQKBNR b KQkq - 0 2").unwrap();
        let method = TimingMethod::RemainingTime{ 
                wtime:time::Duration::from_millis(141516), 
                btime:time::Duration::from_millis(127990),
                winc: time::Duration::from_millis(12000), 
                binc:time::Duration::from_millis(12000), 
                movestogo:0, 
                our_color: Color::Black };
        let mut search = Algo::new().set_timing_method(method).set_minmax(false).set_iterative_deepening(true).set_callback(Arc::new(Mutex::new(Uci::uci_info)));
        println!("{}", search);
        println!("{}", board);
        search.search(board);
        println!("{}", search);
    }
}