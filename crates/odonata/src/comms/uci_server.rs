use crate::{
    boards::Board,
    catalog::Catalog,
    comms::json_rpc::JsonRpc,
    domain::{
        engine::Engine,
        info::{Info, InfoKind},
    },
    eval::eval::Eval,
    infra::{
        component::{Component, State},
        metric::METRICS_TOTAL,
        utils::{Formatting, Uci},
        version::Version,
    },
    movelist::MoveList,
    mv::Move,
    other::Perft,
    position::Position,
    search::{engine::ThreadedSearch, node::Node, timecontrol::TimeControl},
    variation::Variation,
};
use std::{
    collections::HashMap,
    io::{self, Write},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
// use crate::logger::LogInit;
use anyhow::{anyhow, bail, Context, Result};
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
#[derive(Debug)]
pub struct UciServer {
    pub prelude:               Vec<String>,
    pub strict_error_handling: bool,
    running:                   bool,
    board:                     Board,
    engine:                    Arc<Mutex<ThreadedSearch>>,
    debug:                     bool,
    json_rpc:                  JsonRpc,
}

impl Component for UciServer {
    fn new_game(&mut self) {
        self.engine.lock().unwrap().set_state(State::NewGame);
    }

    fn new_position(&mut self) {}
}

impl Default for UciServer {
    fn default() -> Self {
        UciServer::new(HashMap::new())
    }
}

#[allow(clippy::useless_format)]
impl UciServer {
    pub fn new(settings: HashMap<String, String>) -> UciServer {
        let engine = Arc::new(Mutex::new(ThreadedSearch::configure(settings)));
        let uci = UciServer {
            board:                 Catalog::starting_board(),
            engine:                Arc::clone(&engine),
            json_rpc:              JsonRpc::new(Arc::clone(&engine)),
            debug:                 false,
            running:               false,
            prelude:               Vec::default(),
            strict_error_handling: false,
        };
        uci.engine
            .lock()
            .unwrap()
            .set_position(Position::from_board(uci.board.clone()));
        uci.engine.lock().unwrap().algo.set_callback(Self::uci_info);
        uci
    }

    pub fn add_prelude(&mut self, s: &str) {
        self.prelude.extend(s.split(';').map(String::from));
    }

    fn recv(receive: &str) {
        info!(">> {receive}");
        debug!(target: "uci", ">> {receive}");
    }

    fn print_info_string(msg: &str) {
        let send = format!("info string {}", msg.replace('\n', "\ninfo string "));
        info!("<< {send}");
        println!("{send}");
        debug!(target: "uci", "<< {send}");
    }

    fn print(send: &str) {
        println!("{}", send);
        info!("<< {}", send);
        debug!(target: "uci", "<< {send}");
    }

    pub fn uci_compiler(&self) -> Result<()> {
        println!("{} {}\n", Version::NAME, Version::VERSION);
        println!("{}", Version::small_splash());
        println!();
        println!("Please see {} for updates,", Version::HOMEPAGE);
        println!("releases and licence details.");
        println!("Commands...");
        println!("{:<10} enter uci protocol mode", "uci");
        println!("{:<10} quit the program", "quit");
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
            "ext:static_eval" => self.ext_uci_static_eval(&Args::parse(&input)),
            "ext:move_attributes" => self.ext_uci_move_attributes(&Args::parse(&input)),
            "sleep" => self.uci_sleep(&words[1..]),
            "perft" => self.uci_perft(&words[1..]),
            "perft_cat" => self.uci_perft_cat(&words[1..]),
            "board" | "d" => self.uci_board(),
            "bench" => self.ext_uci_bench(),
            "compiler" => self.uci_compiler(),
            "show_options" => self.uci_show_options(),
            "show_config" => self.ext_uci_show_config(),
            "eval" | "." => self.ext_uci_explain_eval(),
            "explain_last_search" | "?" => self.uci_explain_last_search(),

            _ if self.is_json_request(&input) => self.json_method(&input),

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
        let mut eng = self.engine.lock().unwrap();
        if is_debug {
            Self::print("setting debug mode on");
        } else {
            Self::print("setting debug mode off");
        }
        // eng.configment("debug", if self.debug { "true" } else { "false" })?;
        eng.algo.set_callback(Self::uci_info);

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
        if eng.algo.explainer.show_metrics_on_exit {
            warn!("{}", *METRICS_TOTAL.read());
        }
        Ok(())
    }

    fn uci_uci(&mut self) -> Result<()> {
        Self::print(&format!("id name {} {}", Version::NAME, Version::VERSION));
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
            let p = Perft::perft(&mut self.board, d);
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
            let mut p = Perft::default();
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

    // ['from', 'to', 'capture', 'ep', 'legal', 'pseudo_legal', 'san', 'rook_move', 'is_ep', 'is_castle']:
    fn ext_uci_move_attributes(&mut self, arg: &Args) -> Result<()> {
        let mut b = Board::new_empty();
        Self::parse_fen(arg, &mut b)?;
        let var = Self::parse_variation(arg, &b);
        if let Ok(var) = var {
            if let Some(mv) = var.first() {
                let from = mv.from().uci();
                let to = mv.to().uci();
                let capture = mv.capture_square(&b).uci();
                let ep = mv.double_push_en_passant_square().uci();
                let legal = mv.is_valid(&b);
                let san = if legal {
                    b.to_san(mv)
                } else {
                    "???".to_string()
                };
                let rook_move = mv.rook_move(&b).to_uci();
                let is_ep = mv.is_ep_capture(&b);
                let is_castle = mv.is_castle(&b);
                Self::print(&format!("result:from {from} to {to} capture {capture} ep {ep} legal {legal} san {san} rook_move {rook_move} is_ep {is_ep} is_castle {is_castle}", 
                    from = from,
                    to = to,
                    capture = capture,
                    ep = ep,
                    // pseudo_legal,
                    legal = legal,
                    san = san,
                    rook_move = rook_move,
                    is_ep = is_ep,
                    is_castle = is_castle));
            } else {
                bail!("Empty variation. Move not specificed");
            }
        } else {
            Self::print_info_string("result:from 00 to 00 capture 00 ep - legal False san ??? rook_move 0000 is_ep False is_castle False");
        }
        Ok(())
    }

    fn ext_uci_static_eval(&mut self, arg: &Args) -> Result<()> {
        let mut b = Board::new_empty();
        Self::parse_fen(arg, &mut b)?;
        let eval = Eval::new();
        let score = b.eval_with_outcome(&eval, &Node::root(0));
        Self::print(&format!("result:{}", score));
        Ok(())
    }

    fn json_method(&mut self, request: &str) -> Result<()> {
        let response = self
            .json_rpc
            .invoke(request)
            .ok_or(anyhow!("json rpc error"))?;
        Self::print(&format!("{}", response));
        Ok(())
    }

    fn ext_uci_version(&mut self, _arg: &Args) -> Result<()> {
        Self::print(&format!("result:{}", Version::VERSION));
        Ok(())
    }

    fn ext_uci_make_moves(&mut self, arg: &Args) -> Result<()> {
        let mut b = Board::new_empty();
        Self::parse_fen(arg, &mut b)?;
        let var = Self::parse_variation(arg, &b)?;
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

    fn uci_position(&mut self, arg: &Args) -> Result<()> {
        self.engine.lock().unwrap().search_stop();
        Self::parse_fen(arg, &mut self.board)?;
        let variation = Self::parse_variation(arg, &self.board)?;
        let mut pos = Position::from_board(self.board.clone());
        pos.tags_mut().sv = Some(variation);
        self.board = pos.board().make_moves_old(pos.supplied_variation());
        self.engine.lock().unwrap().set_position(pos);
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

    fn parse_variation(args: &Args, board: &Board) -> Result<Variation> {
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

        if self.debug {
            // debug mode we clear hash / history etc every search
            Self::print_info_string("Debug enabled, clearing history and transposition tables...");
            self.uci_newgame()?;
        }

        let tc = &input[0..input.find("searchmoves").unwrap_or(input.len())];
        let mut tc = TimeControl::parse_uci(tc)?;
        if let TimeControl::UciFischer(ref mut rt) = tc {
            rt.our_color = self.board.color_us();
        }

        self.engine
            .lock()
            .unwrap()
            .algo
            .set_timing_method(tc.clone());
        self.engine
            .lock()
            .unwrap()
            .algo
            .mte
            .set_shared_ponder(ponder);
        // restrict search to this moves only
        // Example: After "position startpos" and "go infinite searchmoves e2e4 d2d4"
        // the engine should only search the two moves e2e4 and d2d4 in the initial position
        let search_moves =
            Self::parse_movelist(&args, &self.board).context("parsing searchmoves")?;
        self.engine.lock().unwrap().algo.restrictions.search_moves = search_moves;
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

    pub fn uci_options(engine: &ThreadedSearch) -> Vec<String> {
        let mut ops: Vec<String> = Vec::new();

        // ops.push(format!("option name UCI_EngineAbout type string default {} {}", Version::NAME, Version::HOMEPAGE));
        // ops.push("option name Debug type check default false".to_string());
        ops.push(format!(
            "option name Threads type spin default {} min 1 max 16",
            engine.thread_count
        ));
        // ops.push(format!("option name nodestime type spin default {} min 0 max 10000", engine.algo.mte.nodestime));
        ops.push(format!(
            "option name MultiPV type spin default {} min 1 max 64",
            engine.algo.controller.multi_pv
        ));
        ops.push(format!(
            "option name Hash type spin default {} min 0 max 4000",
            engine.algo.tt.mb
        ));
        ops.push("option name Debug_Log_File type string default \"\"".to_string());

        ops.push("option name UCI_AnalyseMode type check default false".to_string());
        ops.push("option name Analyse_Mode type check default false".to_string());
        ops.push("option name UCI_Opponent type string default \"\"".to_string());
        ops.push("option name Ponder type check default false".to_string());
        ops.push("option name OwnBook type check default false".to_string());
        ops.push("option name Book File type string default \"\"".to_string());
        ops.push("option name Best Book Move type check default true".to_string());
        ops.push("option name Clear Hash type button".to_string());
        // ops.push(format!(
        //     "option name Config_File type string default {}",
        //     engine.config_filename
        // ));
        // ops.push("option name Config type string default \"\"".to_string());
        ops
    }

    // fn uci_option_button(&mut self, name: &str) -> Result<()> {
    //     // let name = name.trim();
    //     // info!("Actioning (setoption) {}", name);
    //     if name == "Explain_Eval" {
    //         self.ext_uci_explain_eval()?;
    //     } else if name == "Explain_Last_Search" {
    //         self.uci_explain_last_search()?;
    //     } else if name == "Explain_Quiesce" {
    //         self.ext_uci_explain_eval()?;
    //     } else if name == "Show_Config" {
    //         self.ext_uci_show_config()?;
    //     } else if name == "Clear Hash" {
    //         Self::print("Clearing hash");
    //         self.uci_newgame()?;
    //     } else {
    //         warn!("Unknown action '{}'", name);
    //     }
    //     Ok(())
    // }

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
            return eng.set_option(name, value);
        }
        drop(eng);

        // if name == "Threads" {
        //     let mut eng = self.engine.lock().unwrap();
        //     eng.thread_count = value.parse()?;
        // // } else if name == "MultiPV" {
        // //     eng.configment("restrictions.multi_pv_count", value)?;
        // // } else if name == "nodestime" {
        // //     eng.configment("mte.nodestime", value)?;
        // // } else if name == "Hash" {
        // //     eng.configment("tt.mb", value)?;
        // //     eng.algo.tt.set_state(State::NewGame);
        // // } else if name == "UCI_AnalyseMode" {
        // //     eng.configment("analyse_mode", value)?;
        // } else if name == "UCI_Opponent" {
        //     let player = Player::parse_uci(value)?;
        //     info!("UCI_Opponent {value} {player:?}");
        // } else if name == "UCI_ShowRefutations" {
        //     let mut eng = self.engine.lock().unwrap();
        //     eng.algo.controller.show_refutations =  value.parse()?;
        // } else if name == "Ponder" {
        //     // pondering determined by "go ponder", so no variable to track

        // if name == "Config_File" {
        //     let mut eng = self.engine.lock().unwrap();
        //     if !value.is_empty() && value != "config.toml" {
        //         eng.config_filename = value.to_string();
        //         use figment::{
        //             providers::{Format, Toml},
        //             Figment,
        //         };
        //         use std::path::Path;
        //         // use toml;
        //         let path = Path::new(&eng.config_filename);
        //         if !path.is_file() {
        //             bail!("Config_File '{}' not found", eng.config_filename);
        //         }
        //         let fig = Figment::new()
        //             .merge(&*eng)
        //             .merge(Toml::file(&eng.config_filename));

        //         let new: ThreadedSearch = fig
        //             .extract()
        //             .with_context(|| format!("error in config file {}", &eng.config_filename))?;
        //         *eng = new;
        //     }

        // } else if name == "Clear Hash" {
        //     Self::print("info clearing hash...");
        //     self.uci_newgame()?;
        warn!("Unknown action '{}'", name);
        bail!("Unknown option name '{}' value '{}'", name, value);

        // self.set_debug(self.debug); // set the info calback again
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
                // let mut kvs = HashMap::new();
                // let statements = value.split(';').collect_vec();
                // for s in statements {
                //     let s = s.trim();
                //     if !s.is_empty() {
                //         if let Some((name, value)) = s.split_once('=') {
                //             kvs.insert(name.trim().to_string(), value.trim().to_string());
                //         } else {
                //             // bail!("Expected key=value but '{}' found instead", s)
                //         }
                //     }
                // }
                // let mut eng = self.engine.lock().unwrap();
                // eng.configment_many(kvs)?;
                // eng.algo.set_callback(Self::uci_info);
            } else {
                self.uci_option_name_value(name, value)?;
            }
            // self.engine = Arc::new(Mutex::new(new_engine));
            // let c = ParsedConfig::new().set(&name, &value);
            // self.configure(&c);
            self.engine
                .lock()
                .unwrap()
                .set_position(Position::from_board(self.board.clone()));
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
        let text = toml::to_string(&*engine).context("toml::to_string")?;
        Self::print(&format!("# start configuration:\n{}", text));
        Self::print(&format!("# end of configuration:\n"));
        Ok(())
    }

    fn ext_uci_bench(&mut self) -> Result<()> {
        Self::print(&format!("# benchmark:\n"));
        self.engine.lock().unwrap().search_stop();
        let engine = self.engine.lock().unwrap();
        Self::print(&format!(
            "NODES {}",
            engine.algo.clock.cumul_nodes_this_thread()
        ));
        Self::print(&format!(
            "NPS {}",
            engine.algo.clock.cumul_knps_all_threads() * 1000
        ));
        info!("{}", engine);
        Ok(())
    }

    fn ext_uci_explain_eval(&mut self) -> Result<()> {
        let mut lock = self.engine.lock().unwrap();
        lock.search_stop();
        let res_quiet = &lock
            .algo
            .qsearch(Position::from_board(self.board.clone()))?;
        let quiet_pv = res_quiet.pv();
        let eval = &lock.algo.eval;
        let static_eval = eval.w_eval_explain(&self.board);
        let dynamic_board = &self.board.make_moves(&quiet_pv);
        let dynamic_eval = eval.w_eval_explain(dynamic_board);
        Self::print(&format!(
            "Board:\n{diag}\n{fen}",
            diag = self.board.to_diagram(),
            fen = self.board.to_fen()
        ));
        Self::print(&format!(
            "Quiet: {qspv}",
            qspv = quiet_pv.to_san(&self.board),
        ));
        Self::print(&format!(
            "Material advantage: {}",
            &self.board.material().advantage()
        ));
        Self::print(&format!("Static eval:\n{:#}", static_eval));

        if !quiet_pv.is_empty() {
            Self::print(&format!(
                "Board:\n{diag}\n{fen}",
                diag = dynamic_board.to_diagram(),
                fen = dynamic_board.to_fen()
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
        Self::print(&format!("{:#}", eng.algo));
        Self::print(&format!("{}", eng.display_metrics()));
        Self::print(&format!(
            "{}",
            eng.algo.results.explain(&eng.algo.eval, &self.board)
        ));
        Ok(())
    }

    fn uci_board(&mut self) -> Result<()> {
        self.engine.lock().unwrap().search_stop();
        Self::print("board");
        Self::print(&format!("{}", self.board.to_diagram()));
        Self::print(&format!("fen:         {}", self.board.to_fen()));
        Self::print(&format!("hash:        {:016x}", self.board.hash()));
        Self::print(&format!("outcome:     {}", self.board.outcome()));
        Self::print(&format!("legal moves: {}",
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
        let bm = if var.is_some() && !var.unwrap().is_empty() {
            var.unwrap().first().unwrap()
        } else {
            info!("---> Null  best move");
            Move::new_null()
        };
        let mut output = format!("bestmove {}", bm.to_uci());
        if var.is_some() && var.unwrap().len() > 1 {
            output = format!(
                "{} ponder {}",
                output,
                var.unwrap().second().unwrap().to_uci()
            );
        }
        Self::print(&output);
    }
}

// struct UciInfo2<'a>(&'a Info);

// impl<'a> fmt::Display for UciInfo2<'a> {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         if let SearchProgressMode::Refutation = self.0.mode {
//             let strings: Vec<String> = self.0.best_pv().iter().map(Move::to_string).collect();
//             write!(f, "refutation {}", strings.join(" "))?;
//         }
//         if let SearchProgressMode::PvChange = self.0.mode {
//             write!(f, "depth {} ", self.0.depth)?;
//             write!(f, "seldepth {} ", self.0.seldepth)?;

//             if let Some(nodes) = self.0.nodes {
//                 write!(f, "nodes {} ", nodes)?;
//             }
//             if let Some(nps) = self.0.nps {
//                 write!(f, "nps {} ", nps)?;
//             }
//             write!(f, "score {} ", self.0.score.to_uci())?;
//             if let Some(currmovenumber) = self.0.currmovenumber_from_1 {
//                 write!(f, "currmovenumber {} ", currmovenumber)?;
//             }
//             if let Some(currmove) = self.0.currmove {
//                 write!(f, "currmove {} ", currmove)?;
//             }
//             if let Some(hashfull) = self.0.hashfull_per_mille {
//                 write!(f, "hashfull {} ", hashfull)?;
//             }
//             if let Some(tbhits) = self.0.tbhits {
//                 write!(f, "tbhits {} ", tbhits)?;
//             }
//             if let Some(cpuload) = self.0.cpuload_per_mille {
//                 write!(f, "cpuload {} ", cpuload)?;
//             }
//             if let Some(time_millis) = self.0.time_millis {
//                 write!(f, "time {} ", time_millis)?;
//                 if self.0.multi_pv_index_of > 1 {
//                     write!(f, "multipv {} ", self.0.multi_pv_index + 1)?;
//                 }
//                 let strings: Vec<String> = self.0.pv.iter().map(Move::to_string).collect();
//                 write!(f, "pv {}", strings.join(" "))?;
//             }
//         }
//         if let SearchProgressMode::NodeCounts = self.0.mode {
//             // write!(f, "depth {} ", self.0.depth)?;
//             // write!(f, "seldepth {} ", self.0.seldepth)?;

//             if let Some(nodes) = self.0.nodes {
//                 write!(f, "nodes {} ", nodes)?;
//             }
//             if let Some(nps) = self.0.nps {
//                 write!(f, "nps {} ", nps)?;
//             }
//             if let Some(hashfull) = self.0.hashfull_per_mille {
//                 write!(f, "hashfull {} ", hashfull)?;
//             }
//             if let Some(tbhits) = self.0.tbhits {
//                 write!(f, "tbhits {} ", tbhits)?;
//             }
//             if let Some(cpuload) = self.0.cpuload_per_mille {
//                 write!(f, "cpuload {} ", cpuload)?;
//             }
//             if let Some(time_millis) = self.0.time_millis {
//                 write!(f, "time {} ", time_millis)?;
//             }
//         }
//         Ok(())
//     }
// }

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
    use super::*;
    use crate::piece::Piece;
    use test_log::test;

    #[test]
    fn test_uci_basics() {
        let mut uci = UciServer::default();
        uci.add_prelude("isready");
        // uci.preamble.push("debug on");
        uci.add_prelude("debug off");
        uci.add_prelude("debug junk");
        uci.add_prelude("setoption name Threads value 5");
        uci.add_prelude("quit");
        uci.run();
    }

    #[test]
    fn test_uci_perft() {
        let mut uci = UciServer::default();
        uci.add_prelude("perft 1");
        uci.add_prelude("quit");
        uci.run();
    }

    #[test]
    fn test_uci_helpers() {
        let mut uci = UciServer::default();
        uci.add_prelude("b");
        uci.add_prelude("?");
        uci.add_prelude(".");
        uci.add_prelude("quit");
        uci.run();
    }

    // #[test]
    // fn test_uci_config_file() {
    //     let mut uci = UciServer::default();
    //     // uci.uci_option_name_value("Config_File", "../odonata/resources/Xconfig.toml").unwrap();
    //     assert_eq!(uci.engine.lock().unwrap().algo.eval.quantum, 1);
    //     uci.add_prelude("setoption name Config_File value ../odonata/resources/config.toml");
    //     uci.add_prelude("setoption name Show_Config");
    //     uci.add_prelude("quit");
    //     uci.run();
    //     assert_eq!(uci.engine.lock().unwrap().algo.eval.quantum, 1);
    // }


    #[ignore]
    #[test]
    fn test_uci_setoption() {
        let mut uci = UciServer::default();
        let _bishop = uci.engine.lock().unwrap().algo.eval.mb.piece_weights[Piece::Bishop];
        uci.add_prelude("setoption name Config value eval.mb.b.s=700");
        uci.add_prelude("setoption name Config value eval.mb.n = { s=400, e = 429 }");
        uci.add_prelude("setoption name Config value eval.quantum=2");
        uci.add_prelude("setoption name Explain_Eval");
        uci.add_prelude("setoption name Config value eval.pst.p.a2.s = 10");
        uci.add_prelude("setoption name Show_Config");
        uci.add_prelude("quit");
        uci.run();
        let eval = &uci.engine.lock().unwrap().algo.eval;
        assert_eq!(eval.quantum, 2);
        assert_eq!(eval.mb.piece_weights[Piece::Knight].s() as i32, 400);
        assert_eq!(eval.mb.piece_weights[Piece::Knight].e() as i32, 429);
        assert_eq!(eval.mb.piece_weights[Piece::Bishop].s() as i32, 700);
    }

    #[test]
    fn test_uci_position() {
        let mut uci = UciServer::default();
        // uci.preamble.push("debug on".into());
        uci.add_prelude("position startpos");
        uci.add_prelude("display");
        uci.add_prelude("quit");
        uci.run();
        assert_eq!(uci.board, Catalog::starting_board());

        let mut uci = UciServer::default();
        uci.add_prelude("position fen k7/8/8/8/8/8/8/7k w - - 0 2");
        uci.add_prelude("quit");
        uci.run();
        assert_eq!(uci.board, "k7/8/8/8/8/8/8/7k w - - 0 2".parse().unwrap());

        let mut uci = UciServer::default();
        uci.add_prelude("position startpos moves a2a3 a7a6");
        uci.add_prelude("quit");
        uci.run();
        assert_eq!(
            uci.board.to_fen(),
            Board::parse_fen("rnbqkbnr/1ppppppp/p7/8/8/P7/1PPPPPPP/RNBQKBNR w KQkq - 0 2")
                .unwrap()
                .to_fen()
        );

        let mut uci = UciServer::default();
        uci.add_prelude("position fen rnbqkbnr/1ppppppp/p7/8/8/P7/1PPPPPPP/RNBQKBNR w KQkq - 0 1 moves h2h3 h7h6");
        uci.add_prelude("quit");
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
        let mut uci = UciServer::default();
        // uci.preamble.push("debug on".into());
        uci.add_prelude("position startpos moves d2d4");
        uci.add_prelude("go depth 1");
        uci.add_prelude("isready");
        uci.add_prelude("quit");
        uci.run();
    }

    #[test]
    fn test_uci_go2() {
        let mut uci = UciServer::default();
        uci.add_prelude("debug on");
        uci.add_prelude("position startpos moves d2d4");
        uci.add_prelude("go wtime 1000 btime 1000");
        uci.add_prelude("isready");
        uci.add_prelude("ucinewgame");
        uci.add_prelude("position startpos moves d2d4");
        uci.add_prelude("go wtime 10160 btime 10160 winc 160 binc 160 nodes 3000");
        uci.add_prelude("isready");
        uci.add_prelude("quit");
        uci.run();
        // println!("pvtable:\n{}", uci.algo.pv);
        // assert_eq!(uci.board, Catalog::starting_position());
    }

    #[test]
    fn test_ponder() {
        let mut uci = UciServer::default();
        // uci.add_prelude("debug on".to_string());
        uci.add_prelude("position startpos");
        uci.add_prelude("go ponder movetime 400");
        uci.add_prelude("sleep 100");
        uci.add_prelude("ponderhit");
        uci.add_prelude("isready");
        uci.add_prelude("quit");
        uci.run();
        println!("\n{}", uci.engine.lock().unwrap().algo);
    }
}
