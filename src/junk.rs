


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
use crate::eval::eval::{Scorable, Score};
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
            score: if self.is_maximizing(parent) { Score::PlusInf } else { Score::MinusInf },  // parent maximising => child isnt
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
            if child.score > Score::Cp(1000) {
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
    use crate::eval::eval::*;
    use crate::board::boardbuf::*;

    fn init() {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }


    #[test]
    fn test_node() {
        init();
        let board = &Catalog::starting_position();
        let mut node = Node { parent: None, board, ply: 0, alpha: Score::MinusInf, beta: Score::PlusInf, score: Score::MinusInf, best_move: Default::default() };
        let mut s = Search{ max_depth: 6, ..Default::default() };
        s.alphabeta(&mut node);
        assert_eq!( format!("{:?}", node), "");
    }

    #[test]
    fn jons_chess_problem() {
        init();
        let board = &Board::parse_fen("2r2k2/5pp1/3p1b1p/2qPpP2/1p2B2P/pP3P2/2P1R3/2KRQ3 b - - 0 1").unwrap().as_board();
        println!("{}", board);
        let mut node = Node { parent: None,  board, ply: 0, alpha: Score::MinusInf, beta: Score::PlusInf, score: Score::MinusInf, best_move: Default::default() };
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





    
// get
// set
// defaultmin
// max
// parse



#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Setting {
    Int { name: &'static str, min: i64, max: i64, default: i64, value: i64 },
    // Float { name: String, min: f32, max: f32, default: f32, value: f32 },
    String { name: &'static str, values: &'static [&'static str], default: &'static str, value: &'static str },
    //Boolean { name: String, default: bool, value: bool },
}

impl fmt::Display for Setting {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Setting::Int { name, min, max, default, value } => {
                if f.alternate() {
                    write!(f, "{:30} = {:<10} min = {:<10} max = {:<10} default = {:<10}", name, value, min, max, default)?;
                } else {
                    write!(f, "{}={}", name, value)?
                }
            }
            _ => {},
        }
        Ok(())
    }
}


use crate::eval::eval::SimpleScorer;

#[derive(Debug, Clone)]
pub struct TypedSettingString(Box<Setting>);

#[derive(Debug, Clone)]
pub struct TypedSettingInt(Box<Setting>);


impl TypedSettingInt {
    fn value(&self) -> i64 {
        match *self.0 {
            Setting::Int{value, min:_, name:_, default:_,max:_} => value,
            _ => panic!("Not an int"), 
        }
    }
    fn set(&mut self, v: i64) {
        match *self.0 {
            Setting::Int{value:_, min, name, default,max} => *self.0 = Setting::Int{value:v, min, name, default,max},
            _ => panic!("Not an int"), 
        }
    }
}

impl TypedSettingString {
    fn value(&self) -> &str {
        match *self.0 {
            Setting::String{value, values:_, default:_, name:_} => value,
            _ => panic!("Not a String"), 
        }
    }
}

pub trait Configurable {

} 





impl SimpleScorer {
//    const eval_position_mobility: TypedSettingInt = TypedSettingInt(int("eval.pawn.value", 0, 10000, 100));
    // const eval_bishop_mobility: TypedSettingInt = Config::int("eval.pawn.value", 0, 10000, 100);
    // const eval_mode: TypedSettingString = Config::string("eval.mode", &["end-game", "mid-game"], "mid-game");

    fn config_eval_mode() -> TypedSettingInt {
        Config::int("eval.pawn.value", 0, 10000, 100)
    }



}

// use std::cell::RefCell;
// thread_local! {
//     pub static HASHER: RefCell<Hasher> = RefCell::new(Hasher { seed: 3141592653589793, squares: [[[0; 64]; 6]; 2], side: 0, castling: [0; 4], ep: [0; 8] });
// }
// HASHER.with( |h| { move_hash = h.borrow().seed() });

// INSTANCE.get_or_init(|| Self::new(3141592653589793))



#[derive(Clone, Debug)]
pub struct Config {
//     settings: Vec<Setting>,
}



impl Default for Config {
    fn default() -> Self {
        Config::new()
    }
}


// fn global_classical_bitboard() -> &'static ClassicalBitboard {
//     static INSTANCE: OnceCell<ClassicalBitboard> = OnceCell::new();
//     INSTANCE.get_or_init(|| {
//         debug!("Initilizing classical bitboard lookup tables");
//         ClassicalBitboard::new()
//     })


impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for s in Self::settings().lock().unwrap().values() {
            if f.alternate() {
                writeln!(f, "{:#}", s)?;
            } else {
                writeln!(f, "{}", s)?;
            }
        }
        Ok(())
    }
}

const fn int( name: &'static str, min: i64, max: i64, default: i64 ) -> Setting {
    Setting::Int { name, min, max, default, value: default }
} 



impl Config {
    pub fn new()-> Self {
        const MAX:i64 = 100_000;
        Config{}
        // Config { settings: vec![
        //     int("eval.pawn.value", 0, MAX, 100),
        //     int("eval.knight.value", 0, MAX, 325),
        //     int("eval.bishop.value", 0, MAX, 350),
        //     int("eval.rook.value", 0, MAX, 500),
        //     int("eval.queen.value", 0, MAX, 900),
        // ]}
    }

    fn settings() -> &'static Mutex<HashMap<String,Box<Setting>>> {
        static SETTINGS: Lazy<Mutex<HashMap<String,Box<Setting>>>> = Lazy::new(|| Mutex::new(HashMap::new()));
        &*SETTINGS
    }


    pub fn int(name: &'static str, min: i64, max: i64, default: i64) -> TypedSettingInt {
        let map = Self::settings().lock().unwrap();
        let entry =  map.get(name);
        if let Some(entry) = entry {
            return TypedSettingInt(entry.clone());
        }
        let b = Box::new(Setting::Int{ name, default, value: default, max, min });
        Self::settings().lock().unwrap().insert(String::from(name), b.clone());
        TypedSettingInt(b)

    }


    pub fn string( name: &'static str, values: &'static [&'static str], default: &'static str) -> TypedSettingString {
        let b = Box::new(Setting::String{ name, values, default, value: default });
        Self::settings().lock().unwrap().insert(String::from(name), b.clone());
        TypedSettingString(b)
    }

    // pub fn set(&self, name: &str, value: &str) {
    //     let a = Setting::Int{ name: "", max: 0, min: 0, default: 0, value: 1 };     
    //     println!("{}", a);   
    // }
}


// eval.configure(&mut self, c: &Config) {
//     self.position = c.evaluation_position.value;
// }





#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let config = Config::new();
        println!("config={}", config);
        println!("\n");
        println!("config#={:#}", config);
        println!("end config#\n");

        println!("config_eval_mode={:?}", SimpleScorer::config_eval_mode());
        let a = SimpleScorer::config_eval_mode().value();
        println!("a={}", a);
        SimpleScorer::config_eval_mode().set(42);
        println!("after: a={}", a);
        println!("config#={:#}", config);


    }
}




OLDTT


use crate::config::{Config, Configurable};
use crate::eval::score::Score;
use crate::log_debug;
use crate::movelist::Move;
use crate::stat::{ArrayStat, Stat};
use crate::types::{Hash, Ply};
use std::fmt;
use std::mem;

// #[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
// pub enum NodeType {
//     Unused = 0,
//     UpperBound = 1,   // All node, score = upperbound ()
//     LowerBound = 2,   // Cut node, score = lowerbound (we've not looked at all possible scores)
//     Exact = 3,  // PV node. score is exact
// }

// impl Default for NodeType {
//     fn default() -> Self {
//         Self::Unused
//     }
// }

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Entry {
    Unused,
    AllNodeUpperBound {
        hash: Hash,
        alpha: Score,
        depth: Ply,
        //good_move: Move
    },
    CutNodeLowerBound {
        hash: Hash,
        beta: Score,
        depth: Ply,
        cut_move: Move,
    },
    PvNodeExact {
        hash: Hash,
        score: Score,
        depth: Ply,
        bm: Move,
    },
}

impl Entry {
    pub fn hash(&self) -> Option<Hash> {
        match self {
            Entry::Unused => None,
            Entry::PvNodeExact { hash, .. }
            | Entry::CutNodeLowerBound { hash, .. }
            | Entry::AllNodeUpperBound { hash, .. } => Some(*hash),
        }
    }
    pub fn depth(&self) -> Ply {
        match self {
            Entry::Unused => 0,
            Entry::PvNodeExact { depth, .. }
            | Entry::CutNodeLowerBound { depth, .. }
            | Entry::AllNodeUpperBound { depth, .. } => *depth,
        }
    }
}

impl Default for Entry {
    fn default() -> Self {
        Self::Unused
    }
}

#[derive(Clone)]
pub struct TranspositionTable {
    table: Vec<Entry>,

    pub enabled: bool,
    capacity: usize,
    hits: Stat<'static>,
    misses: Stat<'static>,
    collisions: Stat<'static>,
    inserts: Stat<'static>,
}

impl fmt::Debug for TranspositionTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TranspositionTable")
            // .field("pv_table", &self.pv_table.extract_pv().)
            .field("enabled", &self.enabled)
            .field("capacity", &self.capacity)
            .field("hits", &self.hits)
            .field("misses", &self.misses)
            .field("collisions", &self.collisions)
            .field("inserts", &self.inserts)
            .field("table", &self.table.len()) // dont show large table!
            .finish()
    }
}




impl fmt::Display for TranspositionTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "capacity         : {}", self.capacity())?;
        writeln!(f, "table            : {}", self.table.len())?;
        writeln!(f, "tt stats\n{}", ArrayStat(&[&self.hits, &self.misses, &self.collisions, &self.inserts]))?;
        Ok(())
    }
}

impl Default for TranspositionTable {
    fn default() -> Self {
        Self {
            table: vec![Entry::default(); 100_000],
            enabled: false,
            capacity: 100_000,
            hits: Stat::new("TT.HITS"),
            misses: Stat::new("TT.MISSES"),
            collisions: Stat::new("TT.COLLISIONS"),
            inserts: Stat::new("TT.INSERTS"),
        }
    }
}

impl Configurable for TranspositionTable {
    fn settings(&self, c: &mut Config) {
        c.set("tt.enabled", "type check default true");
        c.set("tt.capacity", "type spin default 10 min 0 max 200000");
    }
    fn configure(&mut self, c: &Config) {
        log_debug!("tt.configure with {}", c);
        self.enabled = c.bool("tt.enabled").unwrap_or(self.enabled);
        self.capacity = c.int("tt.capacity").unwrap_or(self.capacity as i64) as usize;
    }
}

impl TranspositionTable {
    pub fn new_in_mb(mb: usize) -> Self {
        let entries = 1_000_000 * mb / mem::size_of::<Entry>();
        Self::new(entries)
    }

    pub fn new(capacity: usize) -> Self {
        TranspositionTable { table: vec![Entry::default(); capacity], capacity, ..Self::default() }
        // tt.table.resize(size, Entry::default());
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn index(&self, hash: Hash) -> usize {
        hash as usize % self.capacity()
    }

    pub fn insert(&mut self, new: &Entry) {
        if !self.enabled {
            return;
        }
        self.inserts.increment();
        match new {
            Entry::Unused => return,
            Entry::PvNodeExact { hash, depth, .. }
            | Entry::CutNodeLowerBound { hash, depth, .. }
            | Entry::AllNodeUpperBound { hash, depth, .. } => {
                let i = self.index(*hash);
                let old = &mut self.table[i];
                // FIXME!
                if *depth > old.depth() || *depth == old.depth() {
                    *old = *new;
                }
            }
        }
        self.inserts.increment();
    }

    pub fn get(&self, h: Hash) -> &Entry {
        let entry = &self.table[self.index(h)];
        match entry {
            Entry::Unused => {
                self.misses.increment();
                entry
            }
            Entry::PvNodeExact { hash, .. }
            | Entry::CutNodeLowerBound { hash, .. }
            | Entry::AllNodeUpperBound { hash, .. } => {
                if h == *hash {
                    self.hits.increment();
                    entry
                } else {
                    self.collisions.increment();
                    &Entry::Unused
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tt() {
        let entry123 =
            Entry::PvNodeExact { hash: 123, score: Score::Cp(300), depth: 2, bm: Move::new_null() };

        let entry456 =
            Entry::PvNodeExact { hash: 456, score: Score::Cp(200), depth: 3, bm: Move::new_null() };

        let entry456b =
            Entry::PvNodeExact { hash: 456, score: Score::Cp(201), depth: 4, bm: Move::new_null() };

        let mut tt = TranspositionTable::new_in_mb(10);
        assert_eq!(tt.capacity(), 178_571);
        assert!(*tt.get(123) == Entry::Unused);
        tt.insert(&entry123);
        tt.insert(&entry456);
        assert_eq!(tt.get(123), &entry123);
        assert_eq!(tt.get(124), &Entry::Unused);
        assert_eq!(tt.get(456), &entry456);
        tt.insert(&entry456b);
        assert_eq!(tt.get(456), &entry456b);

        // insert fails due to ply, leaving 456b in place
        tt.insert(&entry456);
        assert_eq!(tt.get(456), &entry456b);
        println!("{:?}", tt);
        println!("{}", tt);
    }
}




use std::cell::RefCell;

#[derive(Clone)]
pub struct Tracer(RefCell<Vec<String>>);

impl Tracer {
    pub fn on() -> Option<Self> {
        Some(Tracer(RefCell::new(vec![])))
    }

    pub fn record(t: &Option<Tracer>, str: &str) {
        let t = t.as_ref();
        let s = t.unwrap();
        let u = &s.0;
        let mut v = u.borrow_mut();
        v.push(str.to_string());
    }
}

impl fmt::Debug for Tracer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Tracer").field("0", &self.0).finish()
    }
}







impl MoveExt {
    pub fn new_quiet(p: Piece, from: Bitboard, to: Bitboard) -> Move {
        Move {
            from,
            to,
            mover: p,
            ..Default::default()
        }
    }

    pub fn new_double_push(from: Bitboard, to: Bitboard, ep_square: Bitboard) -> Move {
        Move {
            from,
            to,
            ep: ep_square,
            mover: Piece::Pawn,
            ..Default::default()
        }
    }

    pub fn new_capture(p: Piece, from: Bitboard, to: Bitboard, captured: Piece) -> Move {
        Move {
            from,
            to,
            mover: p,
            capture: captured,
            ..Default::default()
        }
    }
    pub fn new_ep_capture(
        from: Bitboard,
        to: Bitboard,
        captured_sq: Bitboard,
    ) -> Move {
        Move {
            from,
            to,
            mover: Piece::Pawn,
            capture: Piece::Pawn,
            ep: captured_sq,
            ..Default::default()
        }
    }
    pub fn new_promo(from: Bitboard, to: Bitboard, promo: Piece) -> Move {
        Move {
            from,
            to,
            promo,
            mover: Piece::Pawn,
            ..Default::default()
        }
    }
    pub fn new_promo_capture(from: Bitboard, to: Bitboard, promo: Piece, capture: Piece) -> Move {
        Move {
            from,
            to,
            mover: Piece::Pawn,
            capture,
            promo,
            ..Default::default()
        }
    }
    pub fn new_castle(
        king_from: Bitboard,
        king_to: Bitboard,
        rook_from: Bitboard,
        rook_to: Bitboard,
        castle: CastlingRights,
    ) -> Move {

        Move {
            from: king_from,
            to: king_to,
            mover: Piece::King,
            castle_side: castle,
            // p3: Piece::Rook,
            // t3: rook_to,
            // p4: Piece::Rook,
            // f4: rook_from,
            is_known_legal: true,
            ..Default::default()
        }
    }

