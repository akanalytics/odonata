


    fn init() {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }




insufficient_material


// static STATIC_INSTANCE: Lazy<Hyperbola> = Lazy::new(|| Hyperbola::new());



static mut STATIC_INSTANCE: *const Hyperbola = std::ptr::null();


#[ctor]
fn init_module() {
    Hyperbola::init();
}


#[derive(Copy, Clone, Debug, Default)]
struct HyperbolaMask {
    diag: Bitboard,
    anti_diag: Bitboard,
    file: Bitboard,
    // rank: Bitboard,
}

#[derive(Clone, Debug)]
pub struct Hyperbola {
    mask: [HyperbolaMask; 64],
    rank_attacks: [[Bitboard; 8]; 64], // for perm of 6 bit-occupancy (64) and for each rook square (8)
    king_moves: [Bitboard; 64],
    knight_moves: [Bitboard; 64],
    strictly_between: [[Bitboard; 64]; 64],
    line: [[Bitboard; 64]; 64],
}

impl Hyperbola {

    pub fn init() {
        let hyperbola = Hyperbola::new();
        unsafe {
            // leak the value, so it will never be dropped or freed
            STATIC_INSTANCE = Box::leak(hyperbola) as *const Hyperbola;
        }
    }
    
    // doesnt impl Default as too large to copy by value
    // #[inline]
    // pub fn default() -> &'static Self {
    //     &STATIC_INSTANCE
    // }

    #[inline]
    pub fn default() -> &'static Self {
        unsafe {
            &*STATIC_INSTANCE
        }
    }


    fn new() -> Box<Self> {
        let mut me = Self {


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
            if child.score > Score::from_cp(1000) {
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

pub trait Component {

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


use crate::config::{Config, Component};
use crate::eval::score::Score;
use crate::{debug, logger::LogInit};
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
pub enum TtNode {
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

impl TtNode {
    pub fn hash(&self) -> Option<Hash> {
        match self {
            TtNode::Unused => None,
            TtNode::PvNodeExact { hash, .. }
            | TtNode::CutNodeLowerBound { hash, .. }
            | TtNode::AllNodeUpperBound { hash, .. } => Some(*hash),
        }
    }
    pub fn depth(&self) -> Ply {
        match self {
            TtNode::Unused => 0,
            TtNode::PvNodeExact { depth, .. }
            | TtNode::CutNodeLowerBound { depth, .. }
            | TtNode::AllNodeUpperBound { depth, .. } => *depth,
        }
    }
}

impl Default for TtNode {
    fn default() -> Self {
        Self::Unused
    }
}

#[derive(Clone)]
pub struct TranspositionTable {
    table: Vec<TtNode>,

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
            table: vec![TtNode::default(); 100_000],
            enabled: false,
            capacity: 100_000,
            hits: Stat::new("TT.HITS"),
            misses: Stat::new("TT.MISSES"),
            collisions: Stat::new("TT.COLLISIONS"),
            inserts: Stat::new("TT.INSERTS"),
        }
    }
}

impl Component for TranspositionTable {
    fn settings(&self, c: &mut Config) {
        c.set("tt.enabled", "type check default true");
        c.set("tt.capacity", "type spin default 10 min 0 max 200000");
    }
    fn configure(&mut self, c: &Config) {
        debug!("tt.configure");
        self.enabled = c.bool("tt.enabled").unwrap_or(self.enabled);
        self.capacity = c.int("tt.capacity").unwrap_or(self.capacity as i64) as usize;
    }
}

impl TranspositionTable {
    pub fn new_in_mb(mb: usize) -> Self {
        let entries = 1_000_000 * mb / mem::size_of::<TtNode>();
        Self::new(entries)
    }

    pub fn new(capacity: usize) -> Self {
        TranspositionTable { table: vec![TtNode::default(); capacity], capacity, ..Self::default() }
        // tt.table.resize(size, TtNode::default());
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

    pub fn insert(&mut self, new: &TtNode) {
        if !self.enabled {
            return;
        }
        self.inserts.increment();
        match new {
            TtNode::Unused => return,
            TtNode::PvNodeExact { hash, depth, .. }
            | TtNode::CutNodeLowerBound { hash, depth, .. }
            | TtNode::AllNodeUpperBound { hash, depth, .. } => {
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

    pub fn get(&self, h: Hash) -> &TtNode {
        let entry = &self.table[self.index(h)];
        match entry {
            TtNode::Unused => {
                self.misses.increment();
                entry
            }
            TtNode::PvNodeExact { hash, .. }
            | TtNode::CutNodeLowerBound { hash, .. }
            | TtNode::AllNodeUpperBound { hash, .. } => {
                if h == *hash {
                    self.hits.increment();
                    entry
                } else {
                    self.collisions.increment();
                    &TtNode::Unused
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
            TtNode::PvNodeExact { hash: 123, score: Score::from_cp(300), depth: 2, bm: Move::new_null() };

        let entry456 =
            TtNode::PvNodeExact { hash: 456, score: Score::from_cp(200), depth: 3, bm: Move::new_null() };

        let entry456b =
            TtNode::PvNodeExact { hash: 456, score: Score::from_cp(201), depth: 4, bm: Move::new_null() };

        let mut tt = TranspositionTable::new_in_mb(10);
        assert_eq!(tt.capacity(), 178_571);
        assert!(*tt.get(123) == TtNode::Unused);
        tt.insert(&entry123);
        tt.insert(&entry456);
        assert_eq!(tt.get(123), &entry123);
        assert_eq!(tt.get(124), &TtNode::Unused);
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



    #[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct MoveExt {
    pub p1: Piece,
    pub f1: Bitboard,
    pub t1: Bitboard,

    // them
    pub p2: Piece,
    pub f2: Bitboard,

    // us
    pub p3: Piece,
    pub t3: Bitboard,

    pub p4: Piece,
    pub f4: Bitboard,

    pub castle: CastlingRights,
    pub ep_square: Bitboard,
}

impl MoveExt {
    #[inline]
    pub fn new_quiet(p: Piece, from: Bitboard, to: Bitboard) -> Move {
        Move {
            from,
            to,
            mover: p,
            ..Default::default()
        }
    }

    #[inline]
    pub fn new_double_push(from: Bitboard, to: Bitboard, ep_square: Bitboard) -> Move {
        Move {
            from,
            to,
            ep: ep_square,
            mover: Piece::Pawn,
            ..Default::default()
        }
    }

    #[inline]
    pub fn new_capture(p: Piece, from: Bitboard, to: Bitboard, captured: Piece) -> Move {
        Move {
            from,
            to,
            mover: p,
            capture: captured,
            ..Default::default()
        }
    }

    #[inline]
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

    #[inline]
    pub fn new_promo(from: Bitboard, to: Bitboard, promo: Piece) -> Move {
        Move {
            from,
            to,
            promo,
            mover: Piece::Pawn,
            ..Default::default()
        }
    }

    #[inline]
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

    #[inline]
    pub fn new_castle(
        king_from: Bitboard,
        king_to: Bitboard,
        _rook_from: Bitboard,
        _rook_to: Bitboard,
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

    // pub fn as_move(&self) {
    //     if p2 == Piece::None && p3 == Piece::None && p4 == Piece::None {
            
    //     }

    // }
}

impl fmt::Display for MoveExt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.uci())?;
        Ok(())
    }
}


impl MoveExt {
    pub fn is_promo(&self) -> bool {
        self.p1 == Piece::Pawn && self.p3 != Piece::None
    }

    pub fn is_capture(&self) -> bool {
        self.p2 != Piece::None
    }

    pub fn uci(&self) -> String {
        // if self.is_null() {
        //     return String::from('-');
        // }
        let mut res = String::new();
        res.push_str(&self.f1.uci());
        res.push_str(&self.t1.uci());
        if self.is_promo() {
            res.push_str(&self.t3.uci());
            res.push(self.p3.to_char(Some(Color::Black)));
        }
        res
    }
}

pub fn legal_moves2(&self) -> MoveList {
    counts::LEGAL_MOVE_COUNT.increment();
    let mut moves = MoveList::new();
    let b = self;
    Rules::pawn_captures_incl_promo(b, &mut moves);
    Rules::pawn_promos(b, &mut moves);
    Rules::pawn_push(b, &mut moves);
    Rules::non_pawn(Piece::Knight, b, &mut moves);
    Rules::non_pawn(Piece::Bishop, b, &mut moves);
    Rules::non_pawn(Piece::Rook, b, &mut moves);
    Rules::non_pawn(Piece::Queen, b, &mut moves);

    Rules::king_legal(b, &mut moves);
    Rules::castles(b, &mut moves);
    moves.retain(|m| self.is_legal_move(m));
    moves
}


pub fn pseudo_legal_moves(&self) -> MoveList {
    let mut moves = MoveList::new();
    Rules::pseudo_legals(self, &mut moves);
    moves
}


pub fn has_legal_moves(&self) -> bool {
    self.pseudo_legal_moves()
        .iter()
        .rev()
        .any(|m| m.is_known_legal() || self.is_legal_move(m))
}

// fn is_in_check(&self, c: Color) -> bool {
//     let king = self.kings() & self.color(c);
//     king.intersects(self.threats_to(c))
// }






// moves: ArrayVec<Move,128>,
// moves: ArrayVec::new(),
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoveList {
    moves: ArrayVec<Move, MAX_LEGAL_MOVES>,
}

// pub struct MoveList(ArrayVec::<[Move; 384]>);
// impl Default for MoveList {
//     fn default() -> MoveList { MoveList::new() }
// }

impl Default for MoveList {
    #[inline]
    fn default() -> Self {
        // Self {
        //     moves: Move::Vec::with_capacity(60),
        // }
        Self {
            moves: ArrayVec::new(),
        }
    }
}


impl std::iter::FromIterator<Move> for MoveList {
    #[inline]
    fn from_iter<I: IntoIterator<Item = Move>>(iter: I) -> Self {
        let mut ml = MoveList::new();
        for mv in iter {
            ml.push(mv);
        }
        ml
    }
}

impl MoveList {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn sort(&mut self) -> &mut Self {
        self.moves.sort_by_key(|m| m.to_string());
        self
    }

    #[inline]
    pub fn contains(&self, m: &Move) -> bool {
        self.moves.contains(m)
    }

    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, Move> {
        //    pub fn iter(&self) -> impl Iterator<Item = &Move> {
        self.moves.iter()
    }

    #[inline]
    pub fn push(&mut self, mv: Move) {
        debug_assert!(self.size < 150);
        unsafe {
            *self.moves.push_unchecked(mv);
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.moves.clear();
    }

    #[inline]
    pub fn swap(&mut self, i: usize, j: usize) {
        self.moves.swap(i, j);
    }

    #[inline]
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&Move) -> bool,
    {
        self.moves.retain(f);
    }

    #[inline]
    pub fn sort_unstable_by_key<K, F>(&mut self, f: F)
    where
        F: FnMut(&Move) -> K,
        K: Ord,
    {
        self.moves.sort_unstable_by_key(f)
    }

    #[inline]
    pub fn reverse(&mut self) {
        self.moves.reverse();
    }

    #[inline]
    pub fn extend<T: IntoIterator<Item = Move>>(&mut self, iter: T) {
        for m in iter {
            self.push(m);
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.moves.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }

    pub fn uci(&self) -> String {
        self.iter().map(|mv| mv.uci()).collect::<Vec<String>>().join(" ")
    }
}

impl std::ops::Index<usize> for MoveList {
    type Output = Move;

    #[inline]
    fn index(&self, i: usize) -> &Self::Output {
        debug_assert!(i < self.size);
        &(self.moves[..self.size])[i]
    }
}

impl fmt::Display for MoveList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            for mv in self.iter() {
                writeln!(f, "{:#}", mv)?;
            }
        } else {
            let strings: Vec<String> = self.iter().map(Move::to_string).collect();
            f.write_str(&strings.join(", "))?
        }
        Ok(())
    }
}





MOVELIST ARRAY BASED




// moves: ArrayVec<Move,128>,
// moves: ArrayVec::new(),
#[derive(Debug, PartialEq, Eq)]
pub struct MoveList {
    moves: [Move; MAX_LEGAL_MOVES],
    size: usize,
}

// pub struct MoveList(ArrayVec::<[Move; 384]>);
// impl Default for MoveList {
//     fn default() -> MoveList { MoveList::new() }
// }

impl Default for MoveList {
    #[inline]
    fn default() -> Self {
        // Self {
        //     moves: Move::Vec::with_capacity(60),
        // }
        Self {
            moves: unsafe { std::mem::MaybeUninit::uninit().assume_init() },
            size: 0,
        }
    }
}

impl Clone for MoveList {
    fn clone(&self) -> Self {
        let mut cl = MoveList::default();
        for &mv in self.iter() {
            cl.push(mv);
        }
        cl
    }
}

impl std::iter::FromIterator<Move> for MoveList {
    #[inline]
    fn from_iter<I: IntoIterator<Item = Move>>(iter: I) -> Self {
        let mut ml = MoveList::new();
        for mv in iter {
            ml.push(mv);
        }
        ml
    }
}

impl MoveList {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn sort(&mut self) -> &mut Self {
        self.moves[..self.size].sort_by_key(|m| m.to_string());
        self
    }

    #[inline]
    pub fn contains(&self, m: &Move) -> bool {
        self.moves[..self.size].contains(m)
    }

    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, Move> {
        //    pub fn iter(&self) -> impl Iterator<Item = &Move> {
        (self.moves[..self.size]).iter()
    }

    #[inline]
    pub fn push(&mut self, mv: Move) {
        debug_assert!(self.size < 150);
        unsafe {
            *self.moves.get_unchecked_mut(self.size) = mv;
        }
        self.size += 1;
    }

    #[inline]
    pub fn clear(&mut self) {
        // self.moves.clear();
        self.size = 0;
    }

    #[inline]
    pub fn swap(&mut self, i: usize, j: usize) {
        self.moves[..self.size].swap(i, j);
    }

    #[inline]
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&Move) -> bool,
    {
        let mut v = Vec::<Move>::new();
        v.extend(self.iter());
        v.retain(f);
        for i in 0..v.len() {
            self.moves[i] = v[i];
        }
        self.size = v.len();
    }

    #[inline]
    pub fn sort_unstable_by_key<K, F>(&mut self, f: F)
    where
        F: FnMut(&Move) -> K,
        K: Ord,
    {
        self.moves[..self.size].sort_unstable_by_key(f)
    }

    #[inline]
    pub fn reverse(&mut self) {
        self.moves[..self.size].reverse();
    }

    #[inline]
    pub fn extend<T: IntoIterator<Item = Move>>(&mut self, iter: T) {
        for m in iter {
            self.push(m);
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.size
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn uci(&self) -> String {
        self.iter().map(|mv| mv.uci()).collect::<Vec<String>>().join(" ")
    }
}

impl std::ops::Index<usize> for MoveList {
    type Output = Move;

    #[inline]
    fn index(&self, i: usize) -> &Self::Output {
        debug_assert!(i < self.size);
        &(self.moves[..self.size])[i]
    }
}

impl fmt::Display for MoveList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            for mv in self.iter() {
                writeln!(f, "{:#}", mv)?;
            }
        } else {
            let strings: Vec<String> = self.iter().map(Move::to_string).collect();
            f.write_str(&strings.join(", "))?
        }
        Ok(())
    }
}



impl Ord for Score {
    fn cmp(&self, other: &Score) -> cmp::Ordering {
        if let Score::from_cp(_s1) = self {
            if let Score::from_cp(_s2) = other {
                return PartialOrd::partial_cmp(&self.win_probability(), &other.win_probability()).unwrap();
            }
        } 
        PartialOrd::partial_cmp(self, other).unwrap()
    }
}  



// macro_rules! debug {
//     ($($arg:tt)*) => (if cfg!(debug_assertions) { log!(::log::DEBUG, $($arg)*) })
// }

/// Error level log message
// #[macro_export]
// macro_rules! error {
//     ($($args:tt)*) => {
//         __impl_log!(Level::Error, format_args!($($args)*))
//     }
// }

// impl log::Log for SimpleLogger {

//     #[inline]
//     fn enabled(&self, metadata: &Metadata) -> bool {
//         metadata.level() <= Level::Debug
//     }

//     #[inline]
//     fn log(&self, record: &Record) {

//         if self.enabled(record.metadata()) {
//             eprintln!("{} - {}", record.level(), record.args());
//         }
//     }

//     #[inline]
//     fn flush(&self) {}
// }

// A helper macro to log the message.
// #[doc(hidden)]
// #[macro_export]
// macro_rules! __impl_log {
//     ($level:expr,  $($arg:tt)*) => {{
//         LOGGER; log!($level,  $($arg)*)
//     }}
// }

/// Warn level log message

// macro_rules! debug {
//     ($($arg:tt)*) => (if cfg!(debug_assertions) { log!(::log::DEBUG, $($arg)*) })
// }


// macro_rules! log {
//     (target: $target:expr, $lvl:expr, $($arg:tt)+) => ({
//         let lvl = $lvl;
//             andy_log(
//                 __log_format_args!($($arg)+),
//                 lvl,
//                 &($target, __log_module_path!(), __log_file!(), __log_line!()),
//             );
//     });
//     ($lvl:expr, $($arg:tt)+) => (log!(target: __log_module_path!(), $lvl, $($arg)+))
// }

// // WARNING: this is not part of the crate's public API and is subject to change at any time
// #[doc(hidden)]
// pub fn andy_log(
//     args: fmt::Arguments,
//     level: Level,
//     &(target, module_path, file, line): &(&str, &'static str, &'static str, u32),
// ) {
//     println!("Hello andy");
//     init();
//     logger().log(
//         &Record::builder()
//             .args(args)
//             .level(level)
//             .target(target)
//             .module_path_static(Some(module_path))
//             .file_static(Some(file))
//             .line(Some(line))
//             .build(),
//     );
// }

// pub fn init() -> Result<(), SetLoggerError> {
//     log::set_logger(&LOGGER)
//         .map(|()| log::set_max_level(LevelFilter::Debug))
// }

// #[cfg(not(feature = "slim"))]
// macro_rules! debug {
//     ($($arg: tt)*) => { debug!($($arg)*) }
// }

// #[cfg(feature = "slim")]
// macro_rules! debug {
//     ($($arg: tt)*) => { }
//}




const MAX_LEGAL_CAPTURES: usize  = MAX_LEGAL_MOVES-128;

// PKBRQK   5x6 = 30 

// moves: ArrayVec<Move,128>,
// moves: ArrayVec::new(),
#[derive(Debug, PartialEq, Eq)]
pub struct MoveList {
    pub captures: [Move;64],
    pub occupied_mask: u64,
    pub quiets: ArrayVec<Move, MAX_LEGAL_CAPTURES>,
}


// pub const MVALVV : usize[7][7] = {
//     }

const MOVES: [Move; 64] = [Move::NULL_MOVE; 64];


impl MoveList {
    #[inline]
    pub fn index(mv: &Move) -> usize {
        (49 - mv.capture_piece().index() * 7) + mv.mover_piece().index()
    }


    #[inline]
    pub fn next_free(occupied: u64, i: usize) -> usize {
        // let bits = occupied >> i;
        // let j = bits.trailing_zeros();
        // j as usize
        let occupied = Bitboard::from_u64(occupied);
        let sq = Square::from_u32(i as u32);
        if !sq.is_in(occupied) {
            i
        } else {
            occupied.last_square_from(sq).index() + 1
        }
    }



    #[inline]
    pub fn insert(&mut self, mv: Move) {
        if mv.is_capture() {
            let i = Self::index(&mv); 
            let j = Self::next_free(self.occupied_mask, i);
            self.occupied_mask |= 1 << j;
            // if j >= self.captures.len() {
            //     self.captures.try_extend_from_slice(&MOVES[0..=(j-self.captures.len())]).unwrap();
            // }
            self.captures[j] = mv;
        } else {
            self.quiets.push(mv);
        }
    }

}




impl Default for MoveList {
    #[inline]
    fn default() -> Self {
        let mut me = Self {
            quiets: ArrayVec::new(),
            occupied_mask: 0,
             captures: [Move::NULL_MOVE; 64],
        };
        me.captures.copy_from_slice(&MOVES);
        me
    }
}

impl Clone for MoveList {
    #[inline]
    fn clone(&self) -> Self {
        MoveList {
            quiets: self.quiets.clone(),
            occupied_mask: self.occupied_mask,
            captures: self.captures.clone(),
        }
    }
}

impl std::iter::FromIterator<Move> for MoveList {
    #[inline]
    fn from_iter<I: IntoIterator<Item = Move>>(iter: I) -> Self {
        let mut ml = MoveList::new();
        for mv in iter {
            ml.push(mv);
        }
        ml
    }
}

impl MoveList {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn sort(&mut self) -> &mut Self {
        self.quiets.extend(self.captures.iter().filter(|mv| !mv.is_null()).cloned());
        self.quiets.sort_by_key(|m| m.to_string());
        self.captures.fill(Move::NULL_MOVE);
        self.occupied_mask = 0;
        self
    }

    #[inline]
    pub fn contains(&self, m: &Move) -> bool {
        self.quiets.contains(m)
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Move> + '_ {
        self.captures.iter().filter(|mv| !mv.is_null()).chain(self.quiets.iter())
        // self.quiets.iter()
    }

    #[inline]
    pub fn push(&mut self, mv: Move) {
        debug_assert!(self.len() < MAX_LEGAL_MOVES);
        if mv.is_capture() {
            self.insert(mv);
            return;
        }
        unsafe {
            self.quiets.push_unchecked(mv);
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.quiets.clear();
        self.occupied_mask = 0;
        self.captures.fill(Move::NULL_MOVE);
    }

    #[inline]
    pub fn swap(&mut self, i: usize, j: usize) {
        // self.quiets.swap(i, j);
    }

    #[inline]
    pub fn retain<F>(&mut self, mut f: F)
    where
         F: FnMut(&mut Move) -> bool
    {
        for (i, m) in self.captures.iter_mut().enumerate() {
            if !f(m) {
                self.occupied_mask -= 1 << i;
                *m = Move::NULL_MOVE;
            }
        }
        self.quiets.retain(f);
    }

    #[inline]
    pub fn sort_unstable_by_key<K, F>(&mut self, f: F)
    where
        F: FnMut(&Move) -> K,
        K: Ord,
    {
        // self.quiets.sort_unstable_by_key(f)
    }

    #[inline]
    pub fn reverse(&mut self) {
        // self.quiets.reverse();
    }

    #[inline]
    pub fn extend<T: IntoIterator<Item = Move>>(&mut self, iter: T) {
        self.quiets.extend(iter);
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.quiets.len() + self.occupied_mask.count_ones() as usize
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.quiets.is_empty() && self.occupied_mask == 0
    }

    pub fn uci(&self) -> String {
        self.iter().map(|mv| mv.uci()).collect::<Vec<String>>().join(" ")
    }

    pub fn get(&self, i: usize) -> Option<&Move> {
        self.iter().nth(i)
    }
}



impl std::ops::Index<usize> for MoveList {
    type Output = Move;

    #[inline]
    fn index(&self, i: usize) -> &Self::Output {
        self.get(i).unwrap()
        // let cap_count = self.occupied_mask.count_ones();
        // if i >= cap_count {
        //     return &self.quiets[i-cap_count as usize];
        // }

        // //&self.quiets[i]
    }
}

impl fmt::Display for MoveList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            // for mv in self.captures.iter() {
            //     if mv.is_null() {
            //         write!(f, ".")?;
            //     } else {
            //         write!(f, " {} [{}] ", mv, mv.mvv_lva_score())?;
            //     }
            // }
            let strings : Vec<String> = self.quiets.iter().map(Move::to_string).collect();
            write!(f, " - {}", &strings.join(", "))?;
        } else {
            let strings: Vec<String> = self.iter().map(Move::to_string).collect();
            f.write_str(&strings.join(", "))?
        }
        Ok(())
    }
}

// pub trait MoveValidator {
//     fn parse_uci_move(&self, mv: &str) -> Result<Move, String>;
//     fn parse_uci_choices(&self, moves: &str) -> Result<MoveList, String>;
//     fn parse_uci_moves(&self, moves: &str) -> Result<Variation, String>;

//     fn parse_san_move(&self, mv: &str) -> Result<Move, String>;
//     fn parse_san_choices(&self, moves: &str) -> Result<MoveList, String>;
//     fn parse_san_moves(&self, moves: &str) -> Result<Variation, String>;

//     fn to_san(&self, mv: &Move) -> String;
//     fn to_san_moves(&self, moves: &Variation, vec_tags: Option<&Vec<Tags>>) -> String;
// }

impl Board {
    pub fn parse_uci_move(&self, mv: &str) -> Result<Move, String> {
        let moves = self.legal_moves();
        for &m in moves.iter() {
            if m.uci() == mv {
                return Ok(m);
            }
        }
        Err(format!("Move {} is not legal for board {}", mv, self.to_fen()))
    }

    pub fn parse_uci_choices(&self, s: &str) -> Result<MoveList, String> {
        let mut moves = MoveList::new();
        let s = s.replace(",", " ");
        let s = strip_move_numbers(&s);
        for mv in s.split_ascii_whitespace() {
            moves.push(self.parse_uci_move(mv)?);
        }
        Ok(moves)
    }

    pub fn parse_uci_moves(&self, s: &str) -> Result<Variation, String> {
        let mut board = self.clone();
        let mut moves = Variation::new();
        let s = s.replace(",", " ");
        let s = strip_move_numbers(&s);
        for mv in s.split_ascii_whitespace() {
            let mv = board.parse_uci_move(mv)?;
            moves.push(mv);
            board = board.make_move(&mv);
        }
        Ok(moves)
    }

    pub fn parse_san_move(&self, mv: &str) -> Result<Move, String> {
        Parse::move_san(mv, self)
    }

    pub fn parse_san_choices(&self, s: &str) -> Result<MoveList, String> {
        let mut moves = MoveList::new();
        let s = s.replace(",", " ");
        let s = strip_move_numbers(&s);
        for mv in s.split_ascii_whitespace() {
            moves.push(self.parse_san_move(mv)?);
        }
        Ok(moves)
    }

    pub fn parse_san_moves(&self, s: &str) -> Result<Variation, String> {
        let mut board = self.clone();
        let mut moves = Variation::new();
        let s = s.replace(",", " ");
        let s = strip_move_numbers(&s);
        for mv in s.split_ascii_whitespace() {
            let mv = board.parse_san_move(mv)?;
            moves.push(mv);
            board = board.make_move(&mv);
        }
        Ok(moves)
    }

    pub fn to_san(&self, mv: &Move) -> String {
        if mv.is_castle() {
            if mv.castling_side().is_king_side() {
                return String::from("O-O");
            } else {
                return String::from("O-O-O");
            }
        }

        let mut s = String::new();
        if mv.mover_piece() != Piece::Pawn {
            s += &mv.mover_piece().to_upper_char().to_string();
        }
        // ambiguity resolution
        let mut pieces = 0;
        let mut file_pieces = 0;
        let mut rank_pieces = 0;
        for lm in self.legal_moves().iter() {
            if lm.to() == mv.to() && lm.mover_piece() == mv.mover_piece() {
                pieces += 1;
                if lm.from().file_char() == mv.from().file_char() {
                    file_pieces += 1;
                }
                if lm.from().rank_char() == mv.from().rank_char() {
                    rank_pieces += 1;
                }
            }
        }
        if pieces > 1 || (mv.mover_piece() == Piece::Pawn && mv.is_capture()) {
            // need to resolve ambiguity
            if file_pieces == 1 {
                s.push(mv.from().file_char());
            } else if rank_pieces == 1 {
                s.push(mv.from().rank_char());
            } else {
                s += &mv.from().uci();
            }
        }

        if mv.is_capture() {
            s.push('x');
        }
        s += &mv.to().uci();
        if mv.is_ep_capture() {
            s += " e.p.";
        }
        if mv.is_promo() {
            s.push('=');
            s.push(mv.promo_piece().to_upper_char());
        }
        if self.will_check_them(mv) {
            s.push('+');
        }
        s
    }

    pub fn to_san_moves(&self, moves: &Variation, vec_tags: Option<&Vec<Tags>>) -> String {
        let mut s = String::new();
        let mut board = self.clone();
        for (i, mv) in moves.iter().enumerate() {
            debug_assert!(
                board.is_legal_move(mv),
                "mv {} is illegal for board {}",
                mv,
                board.to_fen()
            );
            if i % 2 == 0 {
                if i != 0 {
                    s += "\n";
                }
                s += &board.fullmove_number().to_string();
                s += ".";
            }
            if i == 0 && board.color_us() == Color::Black {
                s += "..";
            }
            s += " ";
            s += &board.to_san(mv);
            if let Some(vec) = vec_tags {
                let tags = &vec[i];
                s += &tags.to_pgn();
            }

            board = board.make_move(mv);
        }
        s
    }
}

static REGEX_MOVE_NUMBERS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?x)         # x flag to allow whitespace and comments
    (\d)+\.(\s)*(\.\.)?(\s)?      # digits a '.' and then whitespace and optionally ".."
    "#,
    )
    .unwrap()
});

fn strip_move_numbers(s: &str) -> String {
    REGEX_MOVE_NUMBERS.replace_all(&s, "").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::*;
    use crate::catalog::Catalog;
    use crate::globals::constants::*;

    #[test]
    fn test_movelist_captures() -> Result<(), String> {
        let capture_a1b2_pxq = Move {
            from: a1.square(),
            to: b2.square(),
            mover: Piece::Pawn,
            capture: Piece::Queen,
            ..Default::default()
        };
        let capture_a1b3_pxq = Move {
            from: a1.square(),
            to: b3.square(),
            mover: Piece::Pawn,
            capture: Piece::Queen,
            ..Default::default()
        };
        let capture_b1b3_bxr = Move {
            from: b1.square(),
            to: b3.square(),
            mover: Piece::Bishop,
            capture: Piece::Rook,
            ..Default::default()
        };
        let promo_a7a8_pq = Move {
            from: a7.square(),
            to: a8.square(),
            promo: Piece::Queen,
            ..Default::default()
        };

        let mut moves = MoveList::new();
        // let mv = capture_a1b2_pxq;
        // let i = MoveList::index(&mv); 
        // let j = MoveList::next_free(moves.occupied_mask, i);
        // println!("{} {}", i, j);
        // let occupied_mask = 1u64 << j;
        moves.insert(capture_a1b2_pxq);
        // let mv = capture_b1b3_bxr;
        // let i = MoveList::index(&mv); 
        // let j = MoveList::next_free(moves.occupied_mask, i);
        // println!("{} {}", i, j);
        moves.insert(capture_b1b3_bxr);
        moves.insert(capture_a1b3_pxq);
        moves.insert(promo_a7a8_pq);
        println!("{}", moves);
        println!("{:#}", moves);

        Ok(())
    }


    #[test]
    fn test_movelist() -> Result<(), String> {
        let move_a1b2 = Move {
            from: a1.square(),
            to: b2.square(),
            ..Default::default()
        };
        let promo_a7a8 = Move {
            from: a7.square(),
            to: a8.square(),
            promo: Piece::Queen,
            ..Default::default()
        };

        let mut moves = MoveList::new();
        assert_eq!(moves.iter().count(), 0);
        moves.push(move_a1b2);
        assert_eq!(moves.contains(&promo_a7a8), false);
        moves.reverse();
        assert_eq!(moves.iter().count(), 1);

        moves.push(promo_a7a8);
        assert_eq!(moves.contains(&move_a1b2), true);

        assert_eq!(moves.to_string(), "a1b2, a7a8q");

        let mut moves = Variation::new();
        moves.set_last_move(1, &move_a1b2);
        assert_eq!(moves.to_string(), "a1b2");
        moves.set_last_move(1, &promo_a7a8);
        assert_eq!(moves.to_string(), "a7a8q");

        moves.set_last_move(0, &promo_a7a8);
        assert_eq!(moves.to_string(), "");

        moves.set_last_move(1, &move_a1b2);
        moves.set_last_move(2, &promo_a7a8);
        assert_eq!(moves.to_string(), "a1b2, a7a8q");

        moves.set_last_move(0, &promo_a7a8);
        moves.set_last_move(2, &move_a1b2);
        assert_eq!(moves.to_string(), "a1b2, a1b2");

        let s = strip_move_numbers("1. .. c4c5 2. c6c7 3.");
        assert_eq!(s, "c4c5 c6c7 ");

        let s = strip_move_numbers("1... c4c5 2. c6c7 3.");
        assert_eq!(s, "c4c5 c6c7 ");

        let s = strip_move_numbers("1. c1c2 c4c5 2. c6c7 3.");
        assert_eq!(s, "c1c2 c4c5 c6c7 ");

        let board = Catalog::starting_position();

        let list = board.parse_uci_choices("a2a3, b2b3  c2c4  ")?;
        assert_eq!(list.to_string(), "a2a3, b2b3, c2c4");

        let list = board.parse_uci_choices("1. a2a3, 2. b2b3  c2c4  ")?;
        assert_eq!(list.to_string(), "a2a3, b2b3, c2c4");

        let list = board.parse_uci_moves("1. a2a3 h7h6 2. b2b3 h6h5")?;
        assert_eq!(list.to_string(), "a2a3, h7h6, b2b3, h6h5");

        let mv = board.parse_uci_move("a2a3")?;
        let board2 = board.make_move(&mv);
        let list = board2.parse_uci_moves("1. .. h7h6 2. b2b3 h6h5")?;

        assert_eq!(list.to_string(), "h7h6, b2b3, h6h5");

        let list = board.parse_san_choices("Nc3, c3  Pc2c3")?;
        assert_eq!(list.to_string(), "b1c3, c2c3, c2c3");

        let san = r"
            1. d4 c6 2. Bf4 d6 3. Nd2 h6 
            4. Ngf3 g5 5. Bg3 Qb6 6. Nc4 Qb4+ 

            7. Nfd2 Be6 8. c3 Qb5 9. e3 Bxc4 
            10. Nxc4 Qd5 11. Qf3 Qxf3 12. gxf3 Nd7 

            13. h4 Bg7 14. e4 Ngf6 15. Bd3 Nh5 
            16. hxg5 Nxg3 17. fxg3 hxg5 18. Rxh8+ Bxh8 

            19. Kd2 O-O-O 20. Ne3 e6 21. Rh1 b5";

        let mut s = String::new();
        s += "d2d4, c7c6, c1f4, d7d6, b1d2, h7h6, ";
        s += "g1f3, g7g5, f4g3, d8b6, d2c4, b6b4, ";

        s += "f3d2, c8e6, c2c3, b4b5, e2e3, e6c4, ";
        s += "d2c4, b5d5, d1f3, d5f3, g2f3, b8d7, ";

        s += "h2h4, f8g7, e3e4, g8f6, f1d3, f6h5, ";
        s += "h4g5, h5g3, f2g3, h6g5, h1h8, g7h8, ";

        s += "e1d2, e8c8, c4e3, e7e6, a1h1, b7b5";
        assert_eq!(board.parse_san_moves(san)?.to_string(), s);
        let s1: String = board
            .to_san_moves(&board.parse_san_moves(san)?, None)
            .split_whitespace()
            .collect();
        let s2: String = san.split_whitespace().collect();
        assert_eq!(s1, s2);

        let board =
            Board::parse_fen("rnbqkbnr/pp2ppp1/2pp3p/8/3P1B2/8/PPPNPPPP/R2QKBNR w KQkq - 0 4").unwrap();
        println!("{}", board.legal_moves());
        let mv = board.parse_uci_move("g1f3")?;
        assert_eq!(board.to_san(&mv), "Ngf3");
        Ok(())
    }

 
}
