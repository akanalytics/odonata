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
    debug: bool,
}

impl Uci {
    pub fn new() -> Uci {
        let mut uci = Uci::default();
        let callback = Arc::new(Mutex::new(Self::uci_info));
        uci.algo.set_callback(callback);
        uci.algo.set_iterative_deepening(true);
        uci
    }

    pub fn run(&mut self) {
        self.running = true;
        while self.running {
            self.readline_and_execute();
        }
        println!("info string exiting...")
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
            "ucinewgame" => self.uci_newgame(),
            "position" => self.uci_position(&Args::parse(&input)),
            "go" => self.uci_go(&Args::parse(&input)),
            "stop" => self.uci_stop(),
            "quit" => self.uci_quit(),
            // "ponderhit" => self.uci_unknown(&words),
            // "register" => self.uci_unknown(&words),

            // extensions
            "perft" => self.uci_perft(&words[1..]),
            "display" | "d" => self.uci_display(),
            // "tune" => self.uci_unknown(&words),
            // "eval" => self.uci_unknown(&words),
            // "bench" => self.uci_unknown(&words),
            _ => self.uci_unknown(&words),
        };
        if let Err(s) = res {
            println!("info string error '{}'", s);
        }
        io::stdout().flush().ok();
    }

    fn uci_unknown(&mut self, words: &[&str]) -> Result<(), String> {
        Err(format!("unknown command {:?}", words))
    }

    fn uci_debug(&mut self, words: &[&str]) -> Result<(), String> {
        self.debug = match words.first().copied() {
            Some("on") => { println!("info string debug on"); true },
            Some("off") => { println!("info string debug off"); false },
            _ => return Err("unknown debug option".into()),
        };
        Ok(())
    }

    //
    // * isready
    //     this is used to synchronize the engine with the GUI. When the GUI has sent a command or
    //     multiple commands that can take some time to complete,
    //     this command can be used to wait for the engine to be ready again or
    //     to ping the engine to find out if it is still alive.
    //     E.g. this should be sent after setting the path to the tablebases as this can take some time.
    //     This command is also required once before the engine is asked to do any search
    //     to wait for the engine to finish initializing.
    //     This command must always be answered with "readyok" and can be sent also when the engine is calculating
    //     in which case the engine should also immediately answer with "readyok" without stopping the search.
    // 
    fn uci_isready(&mut self) -> Result<(), String> {
        println!("readyok");
        Ok(())
    }

    //
    // * ucinewgame
    //    this is sent to the engine when the next search (started with "position" and "go") will be from
    //    a different game. This can be a new game the engine should play or a new game it should analyse but
    //    also the next position from a testsuite with positions only.
    //    If the GUI hasn't sent a "ucinewgame" before the first "position" command, the engine shouldn't
    //    expect any further ucinewgame commands as the GUI is probably not supporting the ucinewgame command.
    //    So the engine should not rely on this command even though all new GUIs should support it.
    //    As the engine's reaction to "ucinewgame" can take some time the GUI should always send "isready"
    //    after "ucinewgame" to wait for the engine to finish its operation.
    //
    fn uci_newgame(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn uci_quit(&mut self) -> Result<(), String> {
        println!("info string quitting...");
        self.running = false;
        Ok(())
    }

    fn uci_uci(&mut self) -> Result<(), String> {
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
            println!("info string perft({}) = {}", d, Perft::perft(&board, d));
        }
        Ok(())
    }

    fn uci_position(&mut self, arg: &Args) -> Result<(), String> {
        let fen = arg.words.get(1);
        let moves;
        if let Some(fen) = fen {
            if fen == "startpos" {
                self.board = Catalog::starting_position();
                moves = 2;
            } else if fen == "fen" {
                // expect pos, b/w, castling, ep and 2 x counts
                let fen = arg.words.get(2..8);
                if let Some(fen) = fen {
                    self.board = Board::parse_fen(&fen.join(" "))?;
                    moves = 8;
                } else {
                    return Err("Fen or parts of fen are missing".into());
                }
            } else {
                return Err("Must specify fen or startpos after position command".into());
            }
            if let Some(word) = arg.words.get(moves) {
                if word != "moves" {
                    return Err(format!("Token after startpos/fen must be 'moves' not '{}'", word));
                }
                for mv in arg.words[(moves+1)..].iter() {
                    let mv = self.board.parse_uci_move(mv)?;
                    self.board = self.board.make_move(&mv);
                }
            }
            Ok(())
        } else {
            Err("Must specify a fen position or startpos".into())
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
        let _ponder = args.contain("ponder");


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
        self.debug(&format!("{}", self.algo));
        self.debug(&format!("{}", self.board));
        self.algo.search_async(self.board.clone());
        Ok(())
    }

    fn uci_setoption(&mut self, _words: &[&str]) -> Result<(), String> {
        Ok(())
    }

    fn uci_display(&mut self) -> Result<(), String> {
        self.debug(&format!("fen {}", self.board.to_fen()));
        self.debug(&format!("{}", self.algo));
        Ok(())
    }


    fn uci_stop(&mut self) -> Result<(), String> {
        self.algo.search_async_stop();
        // Self::uci_info(&self.algo);
        self.debug("stopped");
        Ok(())
    }


    // * depth 
    // * seldepth 
    //     selective search depth in plies,
    //     if the engine sends seldepth there must also a "depth" be present in the same string.
    // * time 
    //     the time searched in ms, this should be sent together with the pv.
    // * nodes 
    //     x nodes searched, the engine should send this info regularly
    // * pv  ... 
    //     the best line found
    // * multipv 
    //     this for the multi pv mode.
    //     for the best move/pv add "multipv 1" in the string when you send the pv.
    //     in k-best mode always send all k variants in k strings together.
    // * score
    //     * cp 
    //         the score from the engine's point of view in centipawns.
    //     * mate 
    //         mate in y moves, not plies.
    //         If the engine is getting mated use negativ values for y.
    //     * lowerbound
    //       the score is just a lower bound.
    //     * upperbound
    //        the score is just an upper bound.
    // * currmove 
    //     currently searching this move
    // * currmovenumber 
    //     currently searching move number x, for the first move x should be 1 not 0.
    // * hashfull 
    //     the hash is x permill full, the engine should send this info regularly
    // * nps 
    //     x nodes per second searched, the engine should send this info regularly
    // * tbhits 
    //     x positions where found in the endgame table bases
    // * cpuload 
    //     the cpu usage of the engine is x permill.
    // 
    pub fn uci_info(algo: &Algo) {
        let mut stats = algo.stats();
        stats.recalculate_time_stats(algo.clock().elapsed());
        let moves = algo.pv.extract_pv().iter().map(|m| m.uci()).collect::<Vec<String>>().join(" ");
        println!("info time {time} depth {depth} nodes {nodes} nps {nps} pv {pv}", 
            time = stats.elapsed.as_millis(), 
            depth = stats.depth, 
            nodes = stats.total_nodes(), 
            nps = stats.knps()*1000,
            pv = moves);
        if let Some(best_move) = algo.best_move() {
            println!("bestmove {}", best_move.uci());
        }
    }

    fn debug(&self, str: &str) {
        if self.debug {
            // replace "\n" with "info string "
            println!("info string {}", str.replace("\n", "\ninfo string "));
        }
    }

}

struct Args {
    // FIXME:
    // line: String,
    pub words: Vec<String>,
}

impl Args {
    pub fn parse(s: &str) -> Args {
        Args { /* line: String::from(s), */ words: s.split_whitespace().map(|s| s.to_string()).collect() }
    }

    pub fn contain(&self, s: &str) -> bool {
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
    use std::thread;

    #[test]
    fn test_args() {
        let s = "go depth 3";
        let args = Args::parse(s);
        assert!(args.contain("depth"));
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
        uci.preamble.push("debug on".into());
        uci.preamble.push("position startpos".into());
        uci.preamble.push("display".into());
        uci.preamble.push("quit".into());
        uci.run();
        assert_eq!(uci.board, Catalog::starting_position());

        let mut uci = Uci::new();
        uci.preamble.push("position fen k7/8/8/8/8/8/8/7k w - - 0 2".into());
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

        let mut uci = Uci::new();
        uci.preamble.push("position fen rnbqkbnr/1ppppppp/p7/8/8/P7/1PPPPPPP/RNBQKBNR w KQkq - 0 1 moves h2h3 h7h6".into());
        uci.preamble.push("quit".into());
        uci.run();
        assert_eq!(
            uci.board,
            Board::parse_fen("rnbqkbnr/1pppppp1/p6p/8/8/P6P/1PPPPPP1/RNBQKBNR w KQkq - 0 2").unwrap()
        );
    }

    #[test]
    fn test_uci_go1() {
        let mut uci = Uci::new();
        uci.preamble.push("debug on".into());
        uci.preamble.push("position startpos moves d2d4".into());
        uci.preamble.push("go depth 1".into());
        uci.preamble.push("quit".into());
        uci.run();
        thread::sleep(Duration::from_millis(600));
    }

    #[test]
    fn test_uci_go2() {
        let mut uci = Uci::new();
        uci.preamble.push("debug on".into());
        uci.preamble.push("position startpos moves d2d4".into());
        uci.preamble.push("go wtime 10000 btime 10000".into());
        uci.preamble.push("quit".into());
        uci.run();
        // println!("pvtable:\n{}", uci.algo.pv);
        // assert_eq!(uci.board, Catalog::starting_position());
        thread::sleep(Duration::from_millis(600));
    }
}
