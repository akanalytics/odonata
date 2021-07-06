use crate::board::boardbuf::BoardBuf;
use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::catalog::Catalog;
use crate::config::{Config, Component};
use crate::mv::Move;
use crate::clock::Clock;
use crate::variation::Variation;
use crate::perft::Perft;
use crate::search::algo::Engine;
use crate::search::node::Node;
use crate::comms::json_rpc::JsonRpc;
use crate::eval::eval::SimpleScorer;
use crate::search::searchprogress::SearchProgress;
use crate::search::timecontrol::TimeControl;
use crate::types::Ply;
use crate::version::Version;
use std::fmt;
use std::io::{self, Write};
use std::time::{Duration, Instant};
use crate::logger::LogInit;
use std::thread;


//  see https://www.chessprogramming.org/CPW-Engine_com
//
//  comments are from excellent article at
//  see http://wbec-ridderkerk.nl/html/UCIProtocol.html
//
//  Stockfish can display a diagram of the current Board
//  Type 'd' into command line
//
//  Stockfish can display a static eval (and breakdown of the Board)
//  Type 'eval' into command line
//
//  Benchmark Testing
//  type 'bench'
//
//  PERFT
//  type 'go perft [ply/depth]'
//
//  Move format:
//  ------------
//  The move format is in long algebraic notation.
//  A nullmove from the Engine to the GUI should be send as 0000.
//  Examples:  e2e4, e7e5, e1g1 (white short castling), e7e8q (for promotion)

// PONDER ON
//
// GUI -> engine1: position startpos
// GUI -> engine1: go wtime 100000 winc 1000 btime 100000 binc 1000
// engine1 -> GUI: bestmove e2e4 ponder e7e6
// GUI -> engine1: position startpos moves e2e4 e7e6
// GUI -> engine1: go ponder wtime 98123 winc 1000 btime 100000 binc 1000
// [user or other engine plays the expected e7e6 move]
// GUI -> engine1: ponderhit
// [engine keeps thinking]
// engine1 -> GUI: bestmove d2d4 ponder d7d5

// Ponderhit example:

// gui -> engine: position p1 [initial position]
// gui -> engine: go wtime xxx btime yyy [engine starts searching]
// ... time passes
// gui <- engine: bestmove a2a3 ponder a7a6 [engine stops]
// gui -> engine: position p1 moves a2a3 a7a6 [position after ponder move]
// gui -> engine: go ponder wtime xxx btime yyy [engine starts searching]
// ... time passes (engine does not stop searching until 'stop' or 'ponderhit' is received)
// gui -> engine: ponderhit [engine may or may not continue searching depending on time management]
// ... time passes (or not, engine is free to reply instantly)
// gui <- engine: bestmove a3a4 ponder a6a5

// Pondermiss example:

// gui -> engine: position p1
// gui -> engine: go wtime xxx btime yyy [engine starts searching]
// ... time passes
// gui <- engine: bestmove a2a3 ponder a7a6 [engine stops]
// gui -> engine: position p1 moves a2a3 a7a6
// gui -> engine: go ponder wtime xxx btime yyy [engine starts searching]
// ... time passes (engine does not stop until 'stop' or 'ponderhit' is received)
// gui -> engine: stop [engine stops searching]
// gui <- engine: bestmove m1 ponder m2 [this is discarded by gui -]
// gui -> engine: position p1 moves a2a3 b7b6... [- because engine2 played a different move]
// gui -> engine: go...
//
#[derive(Debug, Default)]
pub struct Uci {
    preamble: Vec<String>,
    running: bool,
    board: Board,
    engine: Engine,
    debug: bool,
    json_rpc: JsonRpc,
}

impl Component for Uci {
    fn settings(&self, c: &mut Config) {
        c.set("uci.debug", "type check default false");
        c.set("Ponder", "type check default false");
        c.set("Clear Hash", "type button");
        self.engine.settings(c);
    }

    fn configure(&mut self, c: &Config) {
        if let Some(b) = c.bool("uci.debug") {
            self.debug = b;
        }
        if c.string("clear_cache").is_some() || c.string("Clear Hash").is_some() {
            let _res = self.uci_newgame();
        }

        self.engine.configure(&c);
    }

    fn new_game(&mut self) {
        self.engine.new_game();
    }

    fn new_search(&mut self) {
    }    
}

impl Uci {
    pub fn new() -> Uci {
        let mut uci = Uci::default();
        uci.board = Catalog::starting_position();
        uci.engine.algo.set_callback(|sp| Self::uci_info(sp));
    
        uci
    }


    fn recv(receive: &str) {
        info!("<< {}", receive);
    }

    fn send(send: &str) {
        info!(">> {}", send);
        println!("{}", send);
    }

    pub fn run(&mut self) {
        self.running = true;
        self.preamble.insert(0, "uci".to_string());
        while self.running {
            self.readline_and_execute();
        }
        Self::send("info string exiting...");
        io::stdout().flush().ok();
    }


    fn is_json_request(&self, line: &str ) -> bool {
        line.starts_with(r#"{"jsonrpc":"#)
    }

    fn readline_and_execute(&mut self) {
        let mut input = String::new();
        if !self.preamble.is_empty() {
            input = self.preamble.remove(0);
        } else {
            io::stdin().read_line(&mut input).unwrap();
        }
        let words: Vec<&str> = input.split_whitespace().collect();
        if words.is_empty() {
            return;
        }
        Self::recv(&input);
        let res = match words[0] {
            "uci" => self.uci_uci(),
            "isready" => self.uci_isready(),
            "debug" => self.uci_debug(&words[1..]),
            "setoption" => self.uci_setoption(&Args::parse(&input)),
            "ucinewgame" => self.uci_newgame(),
            "position" => self.uci_position(&Args::parse(&input)),
            "go" => self.uci_go(&Args::parse(&input)),
            "stop" => self.uci_stop(),
            "quit" => self.uci_quit(),
            "ponderhit" => self.uci_ponder_hit(),
            // "register" => self.uci_unknown(&words),

            // extensions
            "ext:legal_moves" => self.ext_uci_legal_moves(&Args::parse(&input)),
            "ext:make_moves" => self.ext_uci_make_moves(&Args::parse(&input)),
            "ext:version" => self.ext_uci_version(&Args::parse(&input)),
            "ext:static_eval" => self.ext_uci_static_eval(&Args::parse(&input)),
            "ext:move_attributes" => self.ext_uci_move_attributes(&Args::parse(&input)),
//            "ext:catalog" => self.ext_uci_catalog(&Args::parse(&input)),
            "sleep" => self.uci_sleep(&words[1..]),
            "perft" => self.uci_perft(&words[1..]),
            "display" | "d" => self.uci_display(),
            "board" | "b" => self.uci_board(),

            _ if self.is_json_request(&input) => self.json_method(&input),

            // "tune" => self.uci_unknown(&words),
            // "eval" => self.uci_unknown(&words),
            // "bench" => self.uci_unknown(&words),
            _ => self.uci_unknown(&words),
        };
        if let Err(s) = res {
            Self::send(&format!("info string error '{}'", s));
        }
        io::stdout().flush().ok();
    }

    fn uci_unknown(&mut self, words: &[&str]) -> Result<(), String> {
        Err(format!("unknown command {:?}", words))
    }

    fn uci_debug(&mut self, words: &[&str]) -> Result<(), String> {
        self.debug = match words.first().copied() {
            Some("on") => {
                Self::send("info string debug on");
                true
            }
            Some("off") => {
                Self::send("info string debug off");
                false
            }
            _ => return Err("unknown debug option".into()),
        };
        Ok(())
    }

    fn uci_isready(&mut self) -> Result<(), String> {
        Self::send("readyok");
        Ok(())
    }

    fn uci_newgame(&mut self) -> Result<(), String> {
        // clear the transposition tables/eval caches and repetition counts before the new game
        self.new_game();
        Ok(())
    }

    fn uci_quit(&mut self) -> Result<(), String> {
        Self::send("info string quitting...");
        self.engine.search_stop();
        Self::send("info string stopped...");
        self.running = false;
        // info!("{}", self.algo);
        Ok(())
    }

    fn uci_uci(&mut self) -> Result<(), String> {
        Self::send(&format!("id name {} v{}", Version::NAME, Version::VERSION));
        Self::send(&format!("id author {}", Version::AUTHORS));
        self.uci_show_options();
        Self::send("uciok");
        Ok(())
    }

    fn uci_sleep(&mut self, words: &[&str]) -> Result<(), String> {
        let time = words.first().ok_or("Must specify a sleep time")?;
        let time = time
            .parse::<u64>()
            .or(Err(format!("Sleep time {} must be numeric", time)))?;
        let millis = Duration::from_millis(time);
        thread::sleep(millis);
        Ok(())
    }

    fn uci_perft(&mut self, words: &[&str]) -> Result<(), String> {
        self.engine.search_stop();
        let depth = words.first().ok_or("Must specify a depth")?;
        let depth = depth
            .parse::<u32>()
            .or(Err(format!("Depth {} must be numeric", depth)))?;
        let mut board = Catalog::starting_position();
        for d in 1..=depth {
            let t = Instant::now();
            let p = Perft::perft(&mut board, d);
            Self::send(&format!("info string perft({}) = {:<12} in {}", d, p, Clock::format(t.elapsed())));
        }
        Ok(())
    }

    // ['from', 'to', 'capture', 'ep', 'legal', 'pseudo_legal', 'san', 'rook_move', 'is_ep', 'is_castle']:
    fn ext_uci_move_attributes(&mut self, arg: &Args) -> Result<(), String> {
        let mut b = Board::new_empty();
        Self::parse_fen(arg, &mut b)?;
        let var = Self::parse_variation(arg, &b)?;
        if let Some(mv) = var.first() {
            Self::send(&format!("result:from {from} to {to} capture {capture} ep {ep} san {san} rook_move {rook_move} is_ep {is_ep} is_castle {is_castle}", 
                from=mv.from().uci(), 
                to=mv.to().uci(), 
                capture=mv.capture_square().uci(), 
                ep=mv.ep().uci(), 
                // pseudo_legal=b.is_pseudo_legal_move(&mv),
                // legal=b.is_legal_move(&mv),
                san=b.to_san(&mv),
                rook_move=mv.rook_move().uci(),
                is_ep=mv.is_ep_capture(),
                is_castle=mv.is_castle()));
        } else {
            return Err("Empty variation. Move not specificed".into());
        }
        Ok(())
    }

    // fn ext_uci_catalog(&mut self, arg: &Args) -> Result<(), String> {
    //     let name = 
    //     let positions = Catalog::from_name(name);
    //     for p in positions {
    //         print!("{} {}", p.board().to_fen(),"\t");
    //         for kv in p.
    //         println!();
    //     }
    //     println!();
    //     Ok(())
    // }

    fn ext_uci_static_eval(&mut self, arg: &Args) -> Result<(), String> {
        let mut b = Board::new_empty();
        Self::parse_fen(arg, &mut b)?;
        let mut eval = SimpleScorer::new();
        let score = b.eval(&mut eval, &Node::root());
        Self::send(&format!("result:{}", score));
        Ok(())
    }

    fn json_method(&mut self, request: &str) -> Result<(), String> {
        let response = self.json_rpc.invoke(request);
        Self::send(&format!("{}", response.unwrap()));
        Ok(())
    }

    fn ext_uci_version(&mut self, _arg: &Args) -> Result<(), String> {
        Self::send(&format!("result:{}", Version::VERSION));
        Ok(())
    }

    fn ext_uci_make_moves(&mut self, arg: &Args) -> Result<(), String> {
        let mut b = Board::new_empty();
        Self::parse_fen(arg, &mut b)?;
        let var = Self::parse_variation(arg, &b)?;
        Self::send(&format!("result:{}", b.make_moves(&var).to_fen()));
        Ok(())
    }


    fn ext_uci_legal_moves(&mut self, arg: &Args) -> Result<(), String> {
        let mut b = Board::new_empty();
        Self::parse_fen(arg, &mut b)?;
        let moves = b.legal_moves();
        Self::send(&format!("result:{}", moves.uci()));
        Ok(())
    }

    fn uci_position(&mut self, arg: &Args) -> Result<(), String> {
        self.engine.search_stop();
        self.engine.algo.repetition.new_game();
        Self::parse_fen(arg, &mut self.board)?;
        let moves = Self::parse_variation(arg, &self.board)?;
        self.engine.algo.repetition.push_variation(&moves, &self.board);
        self.board = self.board.make_moves(&moves);
        Ok(())
    }

    fn parse_fen(arg: &Args, b: &mut Board) -> Result<(), String> {
        let fen = arg.words.get(1);
        if let Some(fen) = fen {
            if fen == "startpos" {
                *b = Catalog::starting_position();
            } else if fen == "fen" {
                // expect pos, b/w, castling, ep and 2 x counts
                let fen = arg.words.get(2..8);
                if let Some(fen) = fen {
                    *b = Board::parse_fen(&fen.join(" "))?;
                } else {
                    return Err("Fen or parts of fen are missing".into());
                }
            } else {
                return Err("Must specify fen or startpos after position command".into());
            }
        } else {
            return Err("Must specify a fen position or startpos".into())
        }
        Ok(())
    }

    fn parse_variation(args: &Args, board: &Board) -> Result<Variation, String> {
        let mut variation = Variation::new();
        let index = args.index_of("moves");
        let mut b = board.clone();
        if let Some(index) = index {
            for mv in args.words[(index + 1)..].iter() {
                let mv = b.parse_uci_move(&mv)?;
                b = b.make_move(&mv);
                variation.push(mv)
            }
        }
        Ok(variation)
    }

    // fn parse_movelist(args: &Args, b: &mut Board) -> Result<MoveList, String> {
    //     let mut movelist = MoveList::new();
    //     let index = args.index_of("moves");
    //     if let Some(index) = index {
    //         for mv in args.words[(index + 1)..].iter() {
    //             let mv = b.parse_uci_move(mv)?;
    //             movelist.push(mv)
    //         }
    //     }
    //     Ok(movelist)
    // }

    fn uci_go(&mut self, args: &Args) -> Result<(), String> {
        let ponder = args.contain("ponder");

        //  search x ply only
        let depth = args.int_after("depth");

        // white/black has x msec left on the clock
        let wtime = args.int_after("wtime");
        let btime = args.int_after("btime");

        // white & black increment per move in mseconds if x > 0 (fisher)
        let winc = args.int_after("winc");
        let binc = args.int_after("binc");

        // there are x moves to the next time control, this will only be sent if x > 0,
        // if you don't get this and get the wtime and btime it's sudden death
        let movestogo = args.int_after("movestogo");
        //search x nodes only
        let nodes = args.int_after("nodes");
        // search for a mate in x moves
        let mate = args.int_after("mate");

        // search for exactly x millis
        let movetime = args.int_after("movetime");
        // search until the "stop" command. Do not exit the search without being told so in this mode!
        let infinite = args.contain("infinite");

        let tc = if let Some(wtime) = wtime {
            let btime = btime.unwrap_or(0) as u64;
            let winc = winc.unwrap_or(0) as u64;
            let binc = binc.unwrap_or(0) as u64;
            let movestogo = movestogo.unwrap_or(0) as u16;
            TimeControl::RemainingTime {
                our_color: self.board.color_us(),
                wtime: Duration::from_millis(wtime as u64),
                btime: Duration::from_millis(btime),
                winc: Duration::from_millis(winc),
                binc: Duration::from_millis(binc),
                movestogo,
            }
        } else if infinite {
            TimeControl::Infinite
        } else if let Some(depth) = depth {
            TimeControl::Depth(depth as Ply)
        } else if let Some(nodes) = nodes {
            TimeControl::NodeCount(nodes as u64)
        } else if let Some(movetime) = movetime {
            TimeControl::MoveTime(Duration::from_millis(movetime as u64))
        } else if let Some(mate) = mate {
            TimeControl::MateIn(mate as u32)
        } else {
            TimeControl::default()
        };

        self.engine.algo.set_timing_method(tc);
        self.engine.algo.mte.set_shared_ponder(ponder);
        // restrict search to this moves only
        // Example: After "position startpos" and "go infinite searchmoves e2e4 d2d4"
        // the engine should only search the two moves e2e4 and d2d4 in the initial position
        let _searchmoves = args.string_after("searchmoves");
        // self.log_debug_message("starting search with configuration ...");
        // self.log_debug_message(&format!("{}", self.engine.algo));
        // self.log_debug_message(&format!("{}", self.board));
        info!("odonata: searching {} on tc {}", self.board.to_fen(), tc);
        self.engine.algo.board = self.board.clone();
        self.engine.search_start();
        Ok(())
    }

    fn uci_setoption(&mut self, args: &Args) -> Result<(), String> {
        let name = args.string_after("name");
        let value = args.string_after("value").or(Some("".to_string()));
        if let Some(name) = name {
            if let Some(value) = value {
                let c = Config::new().set(&name, &value);
                self.configure(&c);
            }
        }
        Ok(())
    }

    fn uci_show_options(&self) {
        let mut c = Config::new();
        self.settings(&mut c);
        for (name, value) in c.iter() {
            Self::send(&format!("option name {} {}", name, value));
        }
    }

    fn uci_display(&mut self) -> Result<(), String> {
        self.engine.search_stop();
        self.uci_info_string("display");
        self.uci_info_string(&format!("{}", self.board));
        self.uci_info_string(&format!("{}", self.engine.algo));
        Ok(())
    }

    fn uci_board(&mut self) -> Result<(), String> {
        self.engine.search_stop();
        self.uci_info_string("board");
        self.uci_info_string(&format!("{}", self.board));
        self.uci_info_string(&format!("outcome {}", self.board.outcome()));
        self.uci_info_string(&format!("legal moves:{}", self.board.legal_moves().uci()));
        Ok(())
    }

    fn uci_stop(&mut self) -> Result<(), String> {
        self.engine.search_stop();
        // Self::print_bm_and_ponder(&self.algo.bm(), &self.algo.pv() );
        Ok(())
    }

    fn uci_ponder_hit(&mut self) -> Result<(), String> {
        self.engine.ponder_hit();
        Ok(())
    }

    pub fn uci_info(search_progress: &SearchProgress) {
        Self::send(&format!("info {}", UciInfo(search_progress)));
        if let Some(bm) = search_progress.bestmove {
            Self::print_bm_and_ponder(&bm, &search_progress.pv.as_ref().unwrap_or(&Variation::default()) );
        }
    }

    fn uci_info_string(&self, str: &str) {
        Self::send(&format!("info string {}", str.replace("\n", "\ninfo string ")));
    }

    fn print_bm_and_ponder(bm: &Move, var: &Variation) {
        let mut output = format!("bestmove {}", bm.uci());
        if var.len() > 1 {
            output = format!("{} ponder {}", output, var[1].uci());
        }
        Self::send(&output)
    }

    // fn log_debug_message(&self, str: &str) {
    //     if self.debug {
    //         // replace "\n" with "info string "
    //         Self::send(format!("info string {}", str.replace("\n", "\ninfo string ")));
    //     }
    // }
}

struct UciInfo<'a>(&'a SearchProgress);

impl<'a> fmt::Display for UciInfo<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(depth) = self.0.depth {
            write!(f, "depth {} ", depth)?;
            if let Some(seldepth) = self.0.seldepth {
                write!(f, "seldepth {} ", seldepth)?;
            }
        }
        if let Some(nodes) = self.0.nodes {
            write!(f, "nodes {} ", nodes)?;
        }
        if let Some(nps) = self.0.nps {
            write!(f, "nps {} ", nps)?;
        }
        if let Some(score) = self.0.score {
            write!(f, "score {} ", score.uci(self.0.pov))?;
        }
        if let Some(currmovenumber) = self.0.currmovenumber_from_1 {
            write!(f, "currmovenumber {} ", currmovenumber)?;
        }
        if let Some(currmove) = self.0.currmove {
            write!(f, "currmove {} ", currmove)?;
        }
        if let Some(hashfull) = self.0.hashfull_per_mille {
            write!(f, "hashfull {} ", hashfull)?;
        }
        if let Some(tbhits) = self.0.tbhits {
            write!(f, "tbhits {} ", tbhits)?;
        }
        if let Some(cpuload) = self.0.cpuload_per_mille {
            write!(f, "cpuload {} ", cpuload)?;
        }
        if let Some(time_millis) = self.0.time_millis {
            write!(f, "time {} ", time_millis)?;
            if let Some(pv) = &self.0.pv {
                let strings: Vec<String> = pv.iter().map(Move::to_string).collect();
                write!(f, "pv {}", strings.join(" "))?;
            }
        }
        Ok(())
    }
}

struct Args {
    // FIXME:
    // line: String,
    pub words: Vec<String>,
}

impl Args {
    pub fn parse(s: &str) -> Args {
        Args {
            /* line: String::from(s), */ words: s.split_whitespace().map(|s| s.to_string()).collect(),
        }
    }

    pub fn contain(&self, s: &str) -> bool {
        self.words.contains(&s.into())
    }

    pub fn index_of(&self, s: &str) -> Option<usize> {
        self.words.iter().position(|x| x == s)
    }

    /// if then n-th word is 's' then return the (n+1)th word  
    pub fn string_after(&self, s: &str) -> Option<String> {
        let i = self.words.iter().position(|x| x == s)?;
        self.words.get(i + 1).cloned()
    }

    pub fn int_after(&self, s: &str) -> Option<i64> {
        let s = self.string_after(s)?;
        s.parse::<i64>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Piece;
    use std::thread;

    #[test]
    fn test_args() {
        let s = "go depth 3";
        let args = Args::parse(s);
        assert!(args.contain("depth"));
        let s = args.string_after("depth");
        assert_eq!(s, Some(String::from("3")));

        let s = "option name reset";
        let args = Args::parse(s);
        assert!(args.contain("reset"));
        let s = args.string_after("reset");
        assert_eq!(s, None);
    }

    #[test]
    fn test_uci() {
        let mut uci = Uci::new();
        uci.preamble.push("isready".into());
        // uci.preamble.push("debug on".into());
        uci.preamble.push("debug off".into());
        uci.preamble.push("debug junk".into());
        uci.preamble.push("quit".into());
        uci.run();
    }

    #[test]
    fn test_uci_perft() {
        let mut uci = Uci::new();
        uci.preamble.push("perft 1".into());
        uci.preamble.push("quit".into());
        uci.run();
    }

    #[test]
    fn test_uci_setoption() {
        let mut uci = Uci::new();
        uci.preamble.push("setoption name eval.b value 700".into());
        uci.preamble
            .push("setoption name eval.position value false".into());
        uci.preamble.push("quit".into());
        uci.run();
        assert_eq!(uci.engine.algo.eval.material_scores[Piece::Bishop], 700);
        assert_eq!(uci.engine.algo.eval.material_scores[Piece::Pawn], 100);
        assert_eq!(uci.engine.algo.eval.position, false);
    }

    #[test]
    fn test_uci_position() {
        let mut uci = Uci::new();
        //uci.preamble.push("debug on".into());
        uci.preamble.push("position startpos".into());
        uci.preamble.push("display".into());
        uci.preamble.push("quit".into());
        uci.run();
        assert_eq!(uci.board, Catalog::starting_position());

        let mut uci = Uci::new();
        uci.preamble
            .push("position fen k7/8/8/8/8/8/8/7k w - - 0 2".into());
        uci.preamble.push("quit".into());
        uci.run();
        assert_eq!(
            uci.board,
            Board::parse_fen("k7/8/8/8/8/8/8/7k w - - 0 2").unwrap()
        );

        let mut uci = Uci::new();
        uci.preamble.push("position startpos moves a2a3 a7a6".into());
        uci.preamble.push("quit".into());
        uci.run();
        assert_eq!(
            uci.board.to_fen(),
            Board::parse_fen("rnbqkbnr/1ppppppp/p7/8/8/P7/1PPPPPPP/RNBQKBNR w KQkq - 0 2").unwrap().to_fen()
        );

        let mut uci = Uci::new();
        uci.preamble.push(
            "position fen rnbqkbnr/1ppppppp/p7/8/8/P7/1PPPPPPP/RNBQKBNR w KQkq - 0 1 moves h2h3 h7h6".into(),
        );
        uci.preamble.push("quit".into());
        uci.run();
        assert_eq!(
            uci.board.to_fen(),
            Board::parse_fen("rnbqkbnr/1pppppp1/p6p/8/8/P6P/1PPPPPP1/RNBQKBNR w KQkq - 0 2").unwrap().to_fen()
        );
    }

    #[test]
    fn test_uci_go1() {
        let mut uci = Uci::new();
        // uci.preamble.push("debug on".into());
        uci.preamble.push("position startpos moves d2d4".into());
        uci.preamble.push("go depth 1".into());
        uci.preamble.push("quit".into());
        uci.run();
        thread::sleep(Duration::from_millis(600));
    }

    #[test]
    fn test_uci_go2() {
        let mut uci = Uci::new();
        uci.preamble.push("debug on".to_string());
        uci.preamble.push("position startpos moves d2d4".to_string());
        uci.preamble.push("go wtime 10000 btime 10000".to_string());
        uci.preamble.push("sleep 1100".to_string());
        uci.preamble.push("ucinewgame".to_string());
        uci.preamble.push("position startpos moves d2d4".to_string());
        uci.preamble.push("go movetime 300".to_string());
        uci.preamble.push("sleep 500".to_string());
        uci.preamble.push("quit".to_string());
        uci.run();
        // println!("pvtable:\n{}", uci.algo.pv);
        // assert_eq!(uci.board, Catalog::starting_position());
    }

    #[test]
    fn test_ponder() {
        let mut uci = Uci::new();
        uci.preamble.push("debug on".to_string());
        uci.preamble.push("position startpos".to_string());
        uci.preamble.push("go ponder movetime 1000".to_string());
        uci.preamble.push("sleep 300".to_string());
        uci.preamble.push("ponderhit".to_string());
        uci.preamble.push("sleep 1100".to_string());
        uci.preamble.push("quit".to_string());
        uci.run();
        println!("\n{}", uci.engine.algo);
    }

}
