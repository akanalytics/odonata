use crate::board::boardbuf::BoardBuf;
use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::catalog::Catalog;
use crate::movelist::MoveValidator;
use crate::perft::Perft;
use crate::search::algo::Algo;
use crate::version::Version;
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use crate::search::clock::TimingMethod;


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

#[derive(Debug, Default)]
pub struct Uci {
    preamble: Vec<String>,
    running: bool,
    board: Board,
    algo: Algo,
}

impl Uci {
    pub fn new() -> Uci {
        let mut uci = Uci {
            preamble: vec![String::from("version")],
            running: false,
            board: Board::default(),
            algo: Algo::default(),
        };
        uci.algo.set_depth(5);
        let callback = Arc::new(Mutex::new(Self::uci_info));
        uci.algo.set_callback(callback);
        uci
    }

    pub fn run(&mut self) {
        self.running = true;
        while self.running {
            self.readline_and_execute();
        }
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

        let res = match words[0] {
            "uci" => self.uci_uci(),
            "isready" => self.uci_isready(),
            "debug" => self.uci_debug(&words[1..]),
            "setoption" => self.uci_setoption(&words[1..]),
            // "register" => self.uci_unknown(&words),
            "ucinewgame" => self.uci_unknown(&words),
            "position" => self.uci_position(&words[1..]),
            "go" => self.uci_go(&Args::parse(&input)),
            "stop" => self.uci_stop(),
            // "ponderhit" => self.uci_unknown(&words),
            "quit" => self.uci_quit(),

            // extensions
            "version" => self.uci_version(),
            "perft" => self.uci_perft(&words[1..]),
            // "tune" => self.uci_unknown(&words),
            "display" => self.uci_display(),
            "d" => self.uci_display(),
            // "eval" => self.uci_unknown(&words),
            // "bench" => self.uci_unknown(&words),
            _ => self.uci_unknown(&words),
        };
        if let Err(s) = res {
            println!("{}", s);
        }
        io::stdout().flush().ok();
    }

    fn uci_unknown(&mut self, words: &[&str]) -> Result<(), String> {
        Err(format!("unknown command {:?}", words))
    }

    fn uci_debug(&mut self, words: &[&str]) -> Result<(), String> {
        println!("command {:?}", words);
        match words.first().copied() {
            Some("on") => println!("--on"),
            Some("off") => println!("--off"),
            _ => return Err("unknown debug option".into()),
        }
        Ok(())
    }

    fn uci_uci(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn uci_isready(&mut self) -> Result<(), String> {
        println!("isready");
        Ok(())
    }

    fn uci_quit(&mut self) -> Result<(), String> {
        self.running = false;
        Ok(())
    }

    fn uci_version(&self) -> Result<(), String> {
        println!("id name {} v{}", Version::NAME, Version::VERSION);
        println!("id author {}", Version::AUTHORS);
        println!("option name depth type spin default 7 min 1 max 10");
        println!("uciok");
        Ok(())
    }

    fn uci_perft(&self, words: &[&str]) -> Result<(), String> {
        let depth = words.first().ok_or("Must specify a depth")?;
        let depth = depth.parse::<u32>().or(Err(format!("Depth {} must be numeric", depth)))?;
        let board = Catalog::starting_position();
        for d in 1..=depth {
            println!("perft({}) = {}", d, Perft::perft(&board, d));
        }
        Ok(())
    }

    fn uci_position(&mut self, words: &[&str]) -> Result<(), String> {
        match words.first().copied() {
            None => Err("Must specify a fen position or startpos".into()),
            Some("startpos") => {
                self.board = Catalog::starting_position();
                if words.get(1) != Some(&"moves") {
                    return Err("Token after startpos must be 'moves'".into());
                }
                for mv in words[2..].iter() {
                    let mv = self.board.parse_uci_move(mv)?;
                    self.board = self.board.make_move(&mv);
                }
                Ok(())
            }
            _ => {
                let fen = words[0..].join(" ");
                self.board = Board::parse_fen(&*fen)?;
                Ok(())
            }
        }
    }

    fn uci_go(&mut self, args: &Args) -> Result<(), String> {
        //  start searching in pondering mode.
        // 	Do not exit the search in ponder mode, even if it's mate!
        // 	This means that the last move sent in in the position string is the ponder move.
        // 	The engine can do what it wants to do, but after a "ponderhit" command
        // 	it should execute the suggested move to ponder on. This means that the ponder move sent by
        // 	the GUI can be interpreted as a recommendation about which move to ponder. However, if the
        // 	engine decides to ponder on a different move, it should not display any mainlines as they are
        // 	likely to be misinterpreted by the GUI because the GUI expects the engine to ponder
        //  on the suggested move.
        let _ponder = args.contains("ponder");


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
        let infinite = args.contains("infinite");

        let tm = if let Some(wtime) = wtime {
            let btime = btime.unwrap_or(0) as u64;
            let winc =  winc.unwrap_or(0) as u64;
            let binc = binc.unwrap_or(0) as u64;
            let movestogo = movestogo.unwrap_or(0) as u16;
            TimingMethod::RemainingTime{
                    our_color: self.board.color_us(),
                    wtime: Duration::from_millis(wtime as u64),
                    btime: Duration::from_millis(btime), 
                    winc: Duration::from_millis(winc),
                    binc: Duration::from_millis(binc),
                    movestogo}
        } else if infinite {
            TimingMethod::Infinite
        } else if let Some(depth) = depth {
            TimingMethod::Depth(depth as u32)
        } else if let Some(nodes) = nodes {
            TimingMethod::NodeCount(nodes as u64)
        } else if let Some(movetime) = movetime {
            TimingMethod::MoveTime(Duration::from_millis(movetime as u64)) 
        } else if let Some(mate) = mate {
            TimingMethod::MateIn(mate as u32)
        } else {
            TimingMethod::default()
        };

        self.algo.set_timing_method(tm); 
        // restrict search to this moves only
        // Example: After "position startpos" and "go infinite searchmoves e2e4 d2d4"
        // the engine should only search the two moves e2e4 and d2d4 in the initial position
        let _searchmoves = args.string_after("searchmoves");

        if let Some(depth) = depth {
            self.algo.set_depth(depth as u32);
        }
        self.algo.search_async(self.board.clone());
        // println!("{}", self.algo);
        Ok(())
    }

    fn uci_setoption(&mut self, _words: &[&str]) -> Result<(), String> {
        Ok(())
    }

    fn uci_display(&mut self) -> Result<(), String> {
        println!("{}", self.board);
        println!("{}", self.algo);
        Ok(())
    }


    fn uci_stop(&mut self) -> Result<(), String> {
        self.algo.search_async_stop();
        // Self::uci_info(&self.algo);
        Ok(())
    }

    fn uci_info(algo: &Algo) {
        let mut stats = algo.stats();
        stats.recalculate_time_stats(algo.clock().elapsed());
        println!("info nodes {} nps {}", stats.total_nodes(), stats.knps()*1000);
        if algo.score.is_some() {
            println!("bestmove {}", algo.pv.extract_pv()[0].uci());
        }
    }

}

struct Args {
    // FIXME:
    // line: String,
    words: Vec<String>,
}

impl Args {
    pub fn parse(s: &str) -> Args {
        Args { /* line: String::from(s), */ words: s.split_whitespace().map(|s| s.to_string()).collect() }
    }

    pub fn contains(&self, s: &str) -> bool {
        self.words.contains(&s.into())
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

    #[test]
    fn test_args() {
        let s = "go depth 3";
        let args = Args::parse(s);
        assert!(args.contains("depth"));
        let s = args.string_after("depth");
        assert_eq!(s, Some(String::from("3")));
    }

    #[test]
    fn test_uci() {
        let mut uci = Uci::new();
        uci.preamble.push("isready".into());
        uci.preamble.push("debug on".into());
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
    fn test_uci_position() {
        let mut uci = Uci::new();
        uci.preamble.push("position startpos".into());
        uci.preamble.push("quit".into());
        uci.run();
        assert_eq!(uci.board, Catalog::starting_position());

        let mut uci = Uci::new();
        uci.preamble.push("position k7/8/8/8/8/8/8/7k w - - 0 2".into());
        uci.preamble.push("quit".into());
        uci.run();
        assert_eq!(uci.board, Board::parse_fen("k7/8/8/8/8/8/8/7k w - - 0 2").unwrap());

        let mut uci = Uci::new();
        uci.preamble.push("position startpos moves a2a3 a7a6".into());
        uci.preamble.push("quit".into());
        uci.run();
        assert_eq!(
            uci.board,
            Board::parse_fen("rnbqkbnr/1ppppppp/p7/8/8/P7/1PPPPPPP/RNBQKBNR w KQkq - 0 2").unwrap()
        );
    }

    #[test]
    fn test_uci_go() {
        let mut uci = Uci::new();
        uci.preamble.push("position startpos moves d2d4".into());
        uci.preamble.push("go depth 2".into());
        uci.preamble.push("quit".into());
        uci.run();
        // println!("pvtable:\n{}", uci.algo.pv);
        // assert_eq!(uci.board, Catalog::starting_position());
    }
}
