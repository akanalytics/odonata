use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context};
use odonata_base::catalog::Catalog;
use odonata_base::domain::info::{Info, InfoKind};
use odonata_base::domain::staticeval::StaticEval;
use odonata_base::epd::Epd;
use odonata_base::infra::component::{Component, State};
use odonata_base::infra::utils::{Formatting, Uci};
use odonata_base::infra::value::Stats;
use odonata_base::infra::version::Version;
use odonata_base::other::Perft;
use odonata_base::prelude::*;

use crate::search::engine::ThreadedSearch;

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
#[derive(Debug)]
pub struct UciServer {
    pub prelude:               Vec<String>,
    pub strict_error_handling: bool,
    running:                   bool,
    board:                     Board,
    engine:                    Arc<Mutex<ThreadedSearch>>,
    debug:                     bool,
}

impl Component for UciServer {
    fn new_game(&mut self) {
        self.engine.lock().unwrap().set_state(State::NewGame);
    }

    fn new_position(&mut self) {}
}

#[allow(clippy::useless_format)]
impl UciServer {
    pub fn configure(settings: HashMap<String, String>) -> anyhow::Result<UciServer> {
        let mut eng = ThreadedSearch::with_threads(1);
        eng.configure(settings)?;
        let engine = Arc::new(Mutex::new(eng));
        let uci = UciServer {
            board:                 Catalog::starting_board(),
            engine:                Arc::clone(&engine),
            debug:                 false,
            running:               false,
            prelude:               Vec::default(),
            strict_error_handling: false,
        };
        {
            let mut eng = uci.engine.lock().unwrap();
            eng.set_position(Epd::from_board(uci.board.clone()));
            eng.set_callback(Self::uci_info);
        }
        Ok(uci)
    }

    pub fn new() -> Result<UciServer> {
        Self::configure(HashMap::new())
    }

    pub fn add_prelude(mut self, s: &str) -> Self {
        self.prelude.extend(Itertools::intersperse(s.split(';'), "isready").map(String::from));
        self
    }

    fn recv(receive: &str) {
        info!(">> {receive}");
        debug!(target: "uci", ">> {receive}");
        log::logger().flush();
    }

    fn print_info_string(msg: &str) {
        let send = format!("info string {}", msg.replace('\n', "\ninfo string "));
        info!("<< {send}");
        println!("{send}");
        debug!(target: "uci", "<< {send}");
    }

    fn print(send: &str) {
        for line in send.lines() {
            println!("{}", line);
            info!("<< {}", line);
            debug!(target: "uci", "<< {line}");
            log::logger().flush();
        }
    }

    pub fn uci_compiler(&self) -> Result<()> {
        Self::print(&format!("{} {}", Version::prog_name(), Version::VERSION_NUMBER));
        Self::print(&Version::small_splash());
        Self::print("");
        let mode = &self.engine.lock().unwrap().search.eval.eval_kind;
        Self::print(&format!("eval mode: {mode}"));
        Self::print(&format!("Please see {} for updates,", Version::HOMEPAGE));
        Self::print("releases and licence details.");
        Self::print("Commands...");
        Self::print(&format!("{:<10} enter uci protocol mode", "uci"));
        Self::print(&format!("{:<10} quit the program", "quit"));
        Ok(())
    }

    pub fn run(&mut self) {
        self.running = true;
        while self.running {
            self.readline_and_execute();
        }
        // Self::print("exiting...");
        // io::stdout().flush().ok();
    }

    fn is_json_request(&self, line: &str) -> bool {
        line.starts_with(r#"{"jsonrpc":"#)
    }

    fn readline_and_execute(&mut self) {
        let mut input = String::new();
        if !self.prelude.is_empty() {
            input = self.prelude.remove(0);
        } else {
            let bytes_read = io::stdin().read_line(&mut input).unwrap();
            if bytes_read == 0 {
                // exit without printing as the pipe is broken
                // self.uci_quit().unwrap();
                self.engine.lock().unwrap().search_stop();
                self.running = false;
                return;
            }
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
            "setoption" => self.uci_setoption(&input),
            "ucinewgame" => self.uci_newgame(),
            "position" => self.uci_position(&Args::parse(&input)),
            "go" => self.uci_go(&input),
            "stop" => self.uci_stop(),
            "quit" => self.uci_quit(),
            "ponderhit" => self.uci_ponder_hit(),
            // "register" => self.uci_unknown(&words),

            // extensions
            "ext:legal_moves" => self.ext_uci_legal_moves(&Args::parse(&input)),
            "ext:make_moves" => self.ext_uci_make_moves(&Args::parse(&input)),
            "ext:version" => self.ext_uci_version(&Args::parse(&input)),
            // "ext:move_attributes" => self.ext_uci_move_attributes(&Args::parse(&input)),
            "sleep" => self.uci_sleep(&words[1..]),
            "perft" => self.uci_perft(&words[1..]),
            "perft_cat" => self.uci_perft_cat(&words[1..]),
            "board" | "d" => self.uci_board(),
            "bench" => self.ext_uci_bench(),
            "compiler" => self.uci_compiler(),
            "show_options" => self.uci_show_options(),
            "metrics" => self.uci_metrics(&Args::parse(&input)),
            "show_config" => self.ext_uci_show_config(),
            "eval" | "." => self.ext_uci_explain_eval(),
            "explain_last_search" | "?" => self.uci_explain_last_search(),

            _ => self.uci_unknown(&words),
        };
        if let Err(s) = &res {
            warn!("uci error '{:#?}' processing '{input}'", res);
            warn!(target: "uci", "!! uci error '{s:#}'");
            Self::print_info_string(&format!("error '{:#}'", s));
            if self.strict_error_handling {
                self.uci_quit().unwrap();
            }
        }
        io::stdout().flush().ok();
    }

    fn uci_unknown(&mut self, words: &[&str]) -> Result<()> {
        bail!("unknown command {:?}", words)
    }

    fn uci_debug(&mut self, words: &[&str]) -> Result<()> {
        let is_debug = match words.first().copied() {
            Some("on") => true,
            Some("off") => false,
            _ => bail!("unknown debug option"),
        };
        self.debug = is_debug;
        // let mut eng = self.engine.lock().unwrap();
        if is_debug {
            Self::print("setting debug mode on");
        } else {
            Self::print("setting debug mode off");
        }
        // eng.configment("debug", if self.debug { "true" } else { "false" })?;
        Ok(())
    }

    fn uci_isready(&mut self) -> Result<()> {
        self.engine.lock().unwrap().wait();
        Self::print("readyok");
        Ok(())
    }

    fn uci_newgame(&mut self) -> Result<()> {
        // clear the transposition tables/eval caches and repetition counts before the new game
        self.set_state(State::NewGame);
        Ok(())
    }

    fn uci_quit(&mut self) -> Result<()> {
        Self::print_info_string("quitting...");
        let mut eng = self.engine.lock().unwrap();
        eng.search_stop();
        Self::print_info_string("stopped...");
        self.running = false;
        // info!("{}", self.algo);
        // warn!("{}", EndGame::counts_to_string());
        // if eng.search.explainer.show_metrics_on_exit {
        //     warn!("{}", *METRICS_TOTAL.read());
        // }
        Ok(())
    }

    fn uci_uci(&mut self) -> Result<()> {
        Self::print(&format!("id name {}", Version::name_and_version()));
        Self::print(&format!("id author {}", Version::AUTHORS));
        self.uci_show_options()?;
        Self::print("uciok");
        Ok(())
    }

    fn uci_sleep(&mut self, words: &[&str]) -> Result<()> {
        let time = words.first().ok_or(anyhow!("Must specify a sleep time"))?;
        let time = time
            .parse::<u64>()
            .map_err(|_| anyhow!("Sleep time {} must be numeric", time))?;
        let millis = Duration::from_millis(time);
        thread::sleep(millis);
        Ok(())
    }

    fn uci_perft(&mut self, words: &[&str]) -> Result<()> {
        self.engine.lock().unwrap().search_stop();
        let depth = words.first().ok_or(anyhow!("Must specify a depth"))?;
        let depth = depth
            .parse::<u32>()
            .map_err(|_| anyhow!("Depth {} must be numeric", depth))?;
        for d in 1..=depth {
            let t = Instant::now();
            let p = Perft::<true>::count(&self.board, d);
            let elapsed = t.elapsed();
            Self::print(&format!(
                "perft({}) = {:<12} in {}",
                d,
                p,
                Formatting::duration(elapsed)
            ));
            if d == depth {
                Self::print(&format!(
                    "\n{} nodes/sec",
                    Formatting::f64(p as f64 / elapsed.as_secs_f64())
                ));
            }
        }
        Ok(())
    }

    pub fn uci_perft_cat(&mut self, words: &[&str]) -> Result<()> {
        self.engine.lock().unwrap().search_stop();
        let depth = words.first().ok_or(anyhow!("Must specify a depth"))?;
        let depth = depth
            .parse::<u32>()
            .map_err(|_| anyhow!("Depth {} must be numeric", depth))?;
        Self::print(&format!(
            "{:>14}{:>14}{:>14}{:>14}{:>14}{:>14}{:>14}",
            "depth", "total", "captures", "en passant", "castles", "promos", "time"
        ));
        for d in 1..=depth {
            let mut p = Perft::<true>::default();
            let t = Instant::now();
            let total = p.perft_cat(&mut self.board, d);
            let elapsed = t.elapsed();
            let time = Formatting::duration(elapsed);
            Self::print(&format!(
                "{:>14}{:>14}{:>14}{:>14}{:>14}{:>14}{:>14}",
                d, total, p.captures, p.en_passant, p.castles, p.promos, time
            ));
            if d == depth {
                Self::print(&format!(
                    "\n{} nodes/sec",
                    Formatting::f64(total as f64 / elapsed.as_secs_f64())
                ));
            }
        }
        Ok(())
        // println!("\nstatistics\n{}", counts::GLOBAL_COUNTS);
    }

    // // ['from', 'to', 'capture', 'ep', 'legal', 'pseudo_legal', 'san', 'rook_move', 'is_ep', 'is_castle']:
    // fn ext_uci_move_attributes(&mut self, arg: &Args) -> Result<()> {
    //     let mut b = Board::new_empty();
    //     Self::parse_fen(arg, &mut b)?;
    //     let var = Self::parse_moves(arg, &b);
    //     if let Ok(var) = var {
    //         if let Some(mv) = var.first() {
    //             let from = mv.from().uci();
    //             let to = mv.to().uci();
    //             let capture = mv.capture_square(&b).uci();
    //             let ep = mv.double_push_en_passant_square().uci();
    //             let legal = mv.is_valid(&b);
    //             let san = if legal {
    //                 b.to_san(mv)
    //             } else {
    //                 "???".to_string()
    //             };
    //             let rook_move = mv.rook_move(&b).to_uci();
    //             let is_ep = mv.is_ep_capture(&b);
    //             let is_castle = mv.is_castle(&b);
    //             Self::print(&format!("result:from {from} to {to} capture {capture} ep {ep} legal {legal} san {san} rook_move {rook_move} is_ep {is_ep} is_castle {is_castle}",
    //                 from = from,
    //                 to = to,
    //                 capture = capture,
    //                 ep = ep,
    //                 // pseudo_legal,
    //                 legal = legal,
    //                 san = san,
    //                 rook_move = rook_move,
    //                 is_ep = is_ep,
    //                 is_castle = is_castle));
    //         } else {
    //             bail!("Empty variation. Move not specificed");
    //         }
    //     } else {
    //         Self::print_info_string("result:from 00 to 00 capture 00 ep - legal False san ??? rook_move 0000 is_ep False is_castle False");
    //     }
    //     Ok(())
    // }

    fn ext_uci_version(&mut self, _arg: &Args) -> Result<()> {
        Self::print(&format!("result:{}", Version::VERSION_NUMBER));
        Ok(())
    }

    fn ext_uci_make_moves(&mut self, arg: &Args) -> Result<()> {
        let mut b = Board::new_empty();
        Self::parse_fen(arg, &mut b)?;
        let var = Self::parse_moves(arg, &b)?;
        Self::print(&format!("result:{}", b.make_moves_old(&var).to_fen()));
        Ok(())
    }

    fn ext_uci_legal_moves(&mut self, arg: &Args) -> Result<()> {
        let mut b = Board::new_empty();
        Self::parse_fen(arg, &mut b)?;
        let moves = b.legal_moves();
        Self::print(&format!("result:{}", moves.to_uci()));
        Ok(())
    }

    fn uci_metrics(&mut self, arg: &Args) -> Result<()> {
        let mut eng = self.engine.lock().unwrap();
        eng.wait(); // wait for any search to complete (so metrics are captured)
        let filter = arg.words.get(1).map(|s| s.as_str()).unwrap_or("");
        let stats = eng.metrics(filter)?;
        Self::print(&Stats::to_pgn([("White", &stats), ("Black", &stats)]));
        Ok(())
    }

    fn uci_position(&mut self, arg: &Args) -> Result<()> {
        self.engine.lock().unwrap().search_stop();
        let mut origin = Board::default();
        Self::parse_fen(arg, &mut origin)?;
        let variation = Self::parse_moves(arg, &origin)?;
        let epd = Epd::from_var(origin, variation);
        self.board = epd.board();
        self.engine.lock().unwrap().set_position(epd);
        Ok(())
    }

    fn parse_fen(arg: &Args, b: &mut Board) -> Result<()> {
        let fen = arg.words.get(1);
        if let Some(fen) = fen {
            if fen == "startpos" {
                *b = Catalog::starting_board();
            } else if fen == "fen" {
                // expect pos, b/w, castling, ep and 2 x counts
                let fen = arg.words.get(2..8);
                if let Some(fen) = fen {
                    *b = Board::parse_fen(&fen.join(" "))?;
                } else {
                    bail!("Fen or parts of fen are missing");
                }
            } else {
                bail!("must specify fen or startpos after position command");
            }
        } else {
            bail!("must specify a fen position or startpos")
        }
        Ok(())
    }

    fn parse_moves(args: &Args, board: &Board) -> Result<Variation> {
        let mut variation = Variation::new();
        let index = args.index_of("moves");
        let mut b = board.clone();
        if let Some(index) = index {
            for mv in args.words[(index + 1)..].iter() {
                let mv = b.parse_uci_move(mv)?;
                b = b.make_move(mv);
                variation.push(mv)
            }
        }
        Ok(variation)
    }

    fn parse_movelist(args: &Args, board: &Board) -> Result<MoveList> {
        let mut movelist = MoveList::new();
        let index = args.index_of("searchmoves");
        if let Some(index) = index {
            for mv in args.words[(index + 1)..].iter() {
                let mv = Move::parse_uci(mv, board)?;
                movelist.push(mv)
            }
        }
        Ok(movelist)
    }

    fn uci_go(&mut self, input: &str) -> Result<()> {
        let args = Args::parse(input);

        self.engine.lock().unwrap().search_stop();
        let ponder = args.contain("ponder");
        info!("uci go args: {input}");

        let tc = &input[0..input.find("searchmoves").unwrap_or(input.len())];
        let mut tc = TimeControl::parse_uci(tc)?;
        if let TimeControl::UciFischer(ref mut rt) = tc {
            rt.our_color = self.board.color_us();
        }

        self.engine.lock().unwrap().search.set_timing_method(tc.clone());
        self.engine.lock().unwrap().search.mte.set_shared_ponder(ponder);
        // restrict search to this moves only
        // Example: After "position startpos" and "go infinite searchmoves e2e4 d2d4"
        // the engine should only search the two moves e2e4 and d2d4 in the initial position
        let search_moves = Self::parse_movelist(&args, &self.board).context("parsing searchmoves")?;
        self.engine.lock().unwrap().search.restrictions.search_moves = search_moves;
        // self.log_debug_message("starting search with configuration ...");
        // self.log_debug_message(&format!("{}", self.engine.lock().unwrap().algo));
        // self.log_debug_message(&format!("{}", self.board));
        info!("odonata: searching {} on tc {tc}", self.board.to_fen());
        self.engine.lock().unwrap().search_start();
        // if !self.prelude.is_empty() {
        //     self.engine.lock().unwrap().wait()
        // }
        Ok(())
    }

    pub fn uci_options(eng: &ThreadedSearch) -> Vec<String> {
        let mut ops: Vec<String> = Vec::new();

        // ops.push(format!("option name UCI_EngineAbout type string default {} {}", Version::NAME, Version::HOMEPAGE));
        // ops.push("option name Debug type check default false".to_string());
        for (k, v) in eng.options() {
            ops.push(format!("option name {k} type {v}",));
        }
        ops
    }

    fn uci_option_name_value(&mut self, name: &str, value: &str) -> Result<()> {
        // if name == "Debug" {
        //     self.debug = value == "true";
        // }
        let value = if value == "\"\"" { "" } else { value };
        // if name == "Debug" {
        //     engine.configment("debug", value)?;

        // } else
        let mut eng = self.engine.lock().unwrap();

        if eng.options().contains_key(name) {
            eng.set_option(name, value)?;
            return Ok(());
        }
        warn!("Unknown action '{}'", name);
        bail!(
            "Unknown option name '{}' value '{}' (opts:{})",
            name,
            value,
            eng.options().keys().join(",")
        );
    }

    fn uci_setoption(&mut self, input: &str) -> Result<()> {
        self.engine.lock().unwrap().search_stop();
        let s1 = input
            .trim_start()
            .strip_prefix("setoption")
            .ok_or(anyhow!("missing setoption"))?
            .trim();
        let s = s1.strip_prefix("name");
        let s = match s {
            Some(remaining) => remaining.trim(),
            // special case: Arena "forgets" to send "name" for set option name UCI_Opponent...
            None if s1.contains("UCI_Opponent") => s1.trim(),
            None => bail!("missing 'name' from setoption"),
        };

        let name_value = s.rsplit_once("value");
        if let Some((name, value)) = name_value {
            let (name, value) = (name.trim(), value.trim());
            info!("Configuring (setoption) {} with:<{}>", name, value);
            if name == "Config" {
                panic!("setoption name Config unexpected");
            } else {
                self.uci_option_name_value(name, value)?;
            }
            self.engine
                .lock()
                .unwrap()
                .set_position(Epd::from_board(self.board.clone()));
        } else {
            self.uci_option_name_value(s.trim(), "")?;
        };

        Ok(())
    }

    fn uci_show_options(&self) -> Result<()> {
        let mut engine = self.engine.lock().unwrap();
        engine.search_stop();
        for op in &Self::uci_options(&engine) {
            Self::print(op);
        }
        Ok(())
    }

    fn ext_uci_show_config(&mut self) -> Result<()> {
        Self::print(&format!("# show configuration:\n"));
        self.engine.lock().unwrap().search_stop();
        let engine = self.engine.lock().unwrap();
        let cfg = &engine.show_config()?;
        Self::print(&format!("# start config\n{cfg}"));
        Self::print(&format!("# end config\n"));
        Ok(())
    }

    fn ext_uci_bench(&mut self) -> Result<()> {
        Self::print(&format!("# benchmark:\n"));
        self.engine.lock().unwrap().search_stop();
        let engine = self.engine.lock().unwrap();
        Self::print(&format!("NODES {}", engine.search.clock.cumul_nodes_this_thread()));
        Self::print(&format!("NPS {}", engine.search.clock.cumul_knps_all_threads() * 1000));
        info!("{}", engine);
        Ok(())
    }

    fn ext_uci_explain_eval(&mut self) -> Result<()> {
        let mut lock = self.engine.lock().unwrap();
        lock.search_stop();
        let res_quiet = &lock.search.qsearch(Epd::from_board(self.board.clone()))?;
        let quiet_pv = res_quiet.pv();
        let eval = &lock.search.eval;
        let static_eval = eval.static_eval_explain(&lock.search.position);
        let mut dynamic_pos = lock.search.position.clone();
        dynamic_pos.push_moves(quiet_pv.clone());
        let dynamic_eval = eval.static_eval_explain(&dynamic_pos);
        Self::print(&format!(
            "Board:\n{diag}\n{fen}",
            diag = self.board.to_diagram(),
            fen = self.board.to_fen()
        ));
        Self::print(&format!("Quiet: {qspv}", qspv = quiet_pv.to_san(&self.board),));
        Self::print(&format!("Material advantage: {}", &self.board.material().balance()));
        Self::print(&format!("Static eval:\n{:#}", static_eval));

        if !quiet_pv.is_empty() {
            Self::print(&format!(
                "Board:\n{diag}\n{fen}",
                diag = dynamic_pos.board().to_diagram(),
                fen = dynamic_pos.board().to_fen()
            ));
            Self::print(&format!("Dynamic eval:\n{:#}", dynamic_eval));
        }
        drop(lock);
        Ok(())
    }

    fn uci_explain_last_search(&mut self) -> Result<()> {
        let mut eng = self.engine.lock().unwrap();
        eng.search_stop();
        Self::print("search");
        Self::print(&format!("{}", self.board));
        Self::print(&format!("{:#}", eng.search));
        Self::print(&format!("{}", eng.display_metrics()));
        let mut bytes = Vec::new();
        eng.search
            .response
            .write_explanation(&mut bytes, &eng.search.eval, eng.search.position.clone())?;

        let s = String::from_utf8(bytes)?;
        Self::print(&format!("{s}",));
        Ok(())
    }

    fn uci_board(&mut self) -> Result<()> {
        self.engine.lock().unwrap().search_stop();
        Self::print("board");
        Self::print(&format!("{}", self.board.to_diagram()));
        Self::print(&format!("fen:         {}", self.board.to_fen()));
        Self::print(&format!("hash:        {:016x}", self.board.hash()));
        Self::print(&format!(
            "legal moves: {}",
            self.board.legal_moves().to_san(&self.board)
        ));
        Ok(())
    }

    fn uci_stop(&mut self) -> Result<()> {
        self.engine.lock().unwrap().search_stop();
        // Self::print_bm_and_ponder(&self.algo.bm(), &self.algo.pv() );
        Ok(())
    }

    fn uci_ponder_hit(&mut self) -> Result<()> {
        self.engine.lock().unwrap().ponder_hit();
        Ok(())
    }

    pub fn uci_info(info: &Info) {
        if info.kind == InfoKind::BestMove {
            Self::print_bm_and_ponder(info.pv.as_ref());
        } else {
            Self::print(&format!("info {}", info.to_uci()));
        }
    }

    fn print_bm_and_ponder(var: Option<&Variation>) {
        let bm = if let Some(mv) = var.and_then(|v| v.first()) {
            mv
        } else {
            info!("---> Null  best move");
            Move::new_null()
        };
        let mut output = format!("bestmove {}", bm.to_uci());
        if var.is_some() && var.unwrap().len() > 1 {
            output = format!("{} ponder {}", output, var.unwrap().second().unwrap().to_uci());
        }
        Self::print(&output);
    }
}

struct Args {
    pub words: Vec<String>,
}

impl Args {
    pub fn parse(s: &str) -> Args {
        Args {
            // line: String::from(s),
            words: s.split_whitespace().map(|s| s.to_string()).collect(),
        }
    }

    pub fn contain(&self, s: &str) -> bool {
        self.words.contains(&s.into())
    }

    pub fn index_of(&self, s: &str) -> Option<usize> {
        self.words.iter().position(|x| x == s)
    }
}

#[cfg(test)]
mod tests {
    use test_log::test;

    use super::*;

    #[test]
    fn test_uci_basics() {
        UciServer::new()
            .unwrap()
            .add_prelude("isready")
            .add_prelude("debug off")
            .add_prelude("debug junk")
            .add_prelude("setoption name Threads value 5")
            .add_prelude("quit")
            .run();
    }

    #[test]
    fn test_uci_perft() {
        UciServer::new()
            .unwrap()
            .add_prelude("perft 1")
            .add_prelude("quit")
            .run();
    }

    #[test]
    fn test_uci_helpers() {
        UciServer::new()
            .unwrap()
            .add_prelude("b")
            .add_prelude("?")
            .add_prelude(".")
            .add_prelude("quit")
            .run();
    }

    // #[test]
    // fn test_uci_config_file() {
    //     let mut uci = UciServer::new().unwrap();
    //     // uci.uci_option_name_value("Config_File", "../odonata/resources/Xconfig.toml").unwrap();
    //     assert_eq!(uci.engine.lock().unwrap().algo.eval.quantum, 1);
    //     add_prelude("setoption name Config_File value ../odonata/resources/config.toml").
    //     add_prelude("setoption name Show_Config").
    //     add_prelude("quit").
    //     uci.run();
    //     assert_eq!(uci.engine.lock().unwrap().algo.eval.quantum, 1);
    // }

    #[test]
    fn test_uci_setoption() {
        let uci = UciServer::new().unwrap();
        assert_eq!(uci.engine.lock().unwrap().search.opening_book.own_book, false);
        let mut uci = uci
            .add_prelude("setoption name OwnBook value true")
            .add_prelude("setoption name Init value tt.enabled=false")
            .add_prelude("setoption name MultiPV value 6")
            .add_prelude("setoption name Init value tt.mb=2")
            .add_prelude("setoption name Best Book Move value false")
            .add_prelude("quit");
        uci.run();
        assert_eq!(uci.engine.lock().unwrap().search.opening_book.own_book, true);
        assert_eq!(uci.engine.lock().unwrap().search.controller.multi_pv, 6);
        assert_eq!(uci.engine.lock().unwrap().search.tt.enabled, false, "tt enabled");
        assert_eq!(uci.engine.lock().unwrap().search.tt.mb, 2);
        assert_eq!(uci.engine.lock().unwrap().search.opening_book.best_book_line, false);
    }

    #[test]
    fn test_uci_position() {
        let mut uci = UciServer::new().unwrap().
        // uci.preamble.push("debug on".into());
        add_prelude("position startpos")
            .add_prelude("board")
            .add_prelude("quit");
        uci.run();
        assert_eq!(uci.board.to_fen(), Catalog::starting_board().to_fen(),);

        let mut uci = UciServer::new()
            .unwrap()
            .add_prelude("position fen k7/8/8/8/8/8/8/7k w - - 0 2")
            .add_prelude("quit");
        uci.run();
        assert_eq!(uci.board, "k7/8/8/8/8/8/8/7k w - - 0 2".parse().unwrap());

        let mut uci = UciServer::new()
            .unwrap()
            .add_prelude("position startpos moves a2a3 a7a6")
            .add_prelude("quit");
        uci.run();
        assert_eq!(
            uci.board.to_fen(),
            Board::parse_fen("rnbqkbnr/1ppppppp/p7/8/8/P7/1PPPPPPP/RNBQKBNR w KQkq - 0 2")
                .unwrap()
                .to_fen()
        );

        let mut uci = UciServer::new()
            .unwrap()
            .add_prelude("position fen rnbqkbnr/1ppppppp/p7/8/8/P7/1PPPPPPP/RNBQKBNR w KQkq - 0 1 moves h2h3 h7h6")
            .add_prelude("quit");
        uci.run();
        assert_eq!(
            uci.board.to_fen(),
            Board::parse_fen("rnbqkbnr/1pppppp1/p6p/8/8/P6P/1PPPPPP1/RNBQKBNR w KQkq - 0 2")
                .unwrap()
                .to_fen()
        );
    }

    #[test]
    fn test_uci_go1() {
        UciServer::new()
            .unwrap()
            .add_prelude("position startpos moves d2d4")
            .add_prelude("board")
            .add_prelude("go depth 1")
            .add_prelude("isready")
            .add_prelude("quit")
            .run();
    }

    #[test]
    fn test_uci_go2() {
        UciServer::new()
            .unwrap()
            .add_prelude("debug on")
            .add_prelude("position startpos moves d2d4")
            .add_prelude("go wtime 1000 btime 1000")
            .add_prelude("isready")
            .add_prelude("newgame")
            .add_prelude("position startpos moves d2d4")
            .add_prelude("go wtime 10160 btime 10160 winc 160 binc 160 nodes 3000")
            .add_prelude("isready")
            .add_prelude("quit")
            .run()
        // println!("pvtable:\n{}", uci.algo.pv);
        // assert_eq!(uci.board, Catalog::starting_position());
    }

    #[test]
    fn test_ponder() {
        let mut uci = UciServer::new()
            .unwrap()
            .add_prelude("position startpos")
            .add_prelude("go ponder movetime 400")
            .add_prelude("sleep 100")
            .add_prelude("ponderhit")
            .add_prelude("isready")
            .add_prelude("quit");
        uci.run();
        println!("\n{}", uci.engine.lock().unwrap().search);
    }
}
