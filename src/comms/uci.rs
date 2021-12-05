use crate::board::boardbuf::BoardBuf;
use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::catalog::Catalog;
use crate::comms::json_rpc::JsonRpc;
use crate::eval::eval::SimpleScorer;
use crate::infra::component::{Component, State};
use crate::infra::version::Version;
use crate::movelist::MoveList;
use crate::mv::Move;
use crate::perft::Perft;
use crate::position::Position;
use crate::search::engine::Engine;
use crate::search::node::Node;
use crate::search::search_results::{SearchResults, SearchResultsMode};
use crate::search::timecontrol::TimeControl;
use crate::tags::Tag;
use crate::types::Ply;
use crate::utils::Formatting;
use crate::variation::Variation;
use std::collections::HashMap;
use std::fmt;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
// use crate::logger::LogInit;
use anyhow::{anyhow, bail, Result};
use itertools::Itertools;
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
pub struct Uci {
    pub prelude: Vec<String>,
    pub strict_error_handling: bool,
    running: bool,
    board: Board,
    engine: Arc<Mutex<Engine>>,
    debug: bool,
    json_rpc: JsonRpc,
}

impl Component for Uci {
    fn new_game(&mut self) {
        self.engine.lock().unwrap().set_state(State::NewGame);
        self.engine.lock().unwrap().algo.set_callback(Self::uci_info);
    }

    fn new_position(&mut self) {
        // self.engine.lock().unwrap().new_position();
        // self.engine.lock().unwrap().algo.set_callback(|sp| Self::uci_info(sp));
    }
}

impl Default for Uci {
    fn default() -> Self {
        Uci::new()
    }
}

#[allow(clippy::useless_format)]
impl Uci {
    pub fn new() -> Uci {
        let engine = Arc::new(Mutex::new(Engine::new()));
        let uci = Uci {
            board: Catalog::starting_board(),
            engine: Arc::clone(&engine),
            json_rpc: JsonRpc::new(Arc::clone(&engine)),
            debug: false,
            running: false,
            prelude: Vec::default(),
            strict_error_handling: false,
        };
        uci.engine.lock().unwrap().set_position(Position::from_board(uci.board.clone()));
        uci.engine.lock().unwrap().algo.set_callback(Self::uci_info);
        uci
    }

    fn recv(receive: &str) {
        info!("<< {}", receive);
    }

    fn print(send: &str) {
        info!(">> {}", send);
        println!("{}", send);
    }

    pub fn banner() {
        println!("{} {}\n", Version::NAME, Version::VERSION);
        println!("{}", Version::small_splash());
        println!();
        println!("Please see {} for updates,", Version::HOMEPAGE);
        println!("releases and licence details.");
        println!("Commands...");
        println!("{:<10} enter uci protocol mode", "uci");
        println!("{:<10} quit the program", "quit");
    }

    pub fn run(&mut self) {
        self.running = true;
        Self::banner();
        while self.running {
            self.readline_and_execute();
        }
        // Self::print("info string exiting...");
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
            "sleep" => self.uci_sleep(&words[1..]),
            "perft" => self.uci_perft(&words[1..]),
            "search" | "?" => self.uci_explain_last_search(),
            "board" | "b" => self.uci_board(),
            "eval" | "." => self.ext_uci_explain_eval(),
            "settings" => self.ext_uci_show_config(),

            _ if self.is_json_request(&input) => self.json_method(&input),

            _ => self.uci_unknown(&words),
        };
        if let Err(s) = res {
            warn!("uci error '{:#}'", s);
            Self::print(&format!("info string error '{:#}'", s));
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
        self.debug = match words.first().copied() {
            Some("on") => {
                Self::print("info string debug on");
                true
            }
            Some("off") => {
                Self::print("info string debug off");
                false
            }
            _ => bail!("unknown debug option"),
        };
        Ok(())
    }

    fn uci_isready(&mut self) -> Result<()> {
        Self::print("readyok");
        Ok(())
    }

    fn uci_newgame(&mut self) -> Result<()> {
        // clear the transposition tables/eval caches and repetition counts before the new game
        self.set_state(State::NewGame);
        Ok(())
    }

    fn uci_quit(&mut self) -> Result<()> {
        Self::print("info string quitting...");
        self.engine.lock().unwrap().search_stop();
        Self::print("info string stopped...");
        self.running = false;
        // info!("{}", self.algo);
        Ok(())
    }

    fn uci_uci(&mut self) -> Result<()> {
        Self::print(&format!("id name {} v{}", Version::NAME, Version::VERSION));
        Self::print(&format!("id author {}", Version::AUTHORS));
        self.uci_show_options();
        Self::print("uciok");
        Ok(())
    }

    fn uci_sleep(&mut self, words: &[&str]) -> Result<()> {
        let time = words.first().ok_or(anyhow!("Must specify a sleep time"))?;
        let time = time.parse::<u64>().or_else(|_|Err(anyhow!("Sleep time {} must be numeric", time)))?;
        let millis = Duration::from_millis(time);
        thread::sleep(millis);
        Ok(())
    }

    fn uci_perft(&mut self, words: &[&str]) -> Result<()> {
        self.engine.lock().unwrap().search_stop();
        let depth = words.first().ok_or(anyhow!("Must specify a depth"))?;
        let depth = depth.parse::<u32>().or_else(|_| Err(anyhow!("Depth {} must be numeric", depth)))?;
        let mut board = Catalog::starting_board();
        for d in 1..=depth {
            let t = Instant::now();
            let p = Perft::perft(&mut board, d);
            Self::print(&format!(
                "info string perft({}) = {:<12} in {}",
                d,
                p,
                Formatting::duration(t.elapsed())
            ));
        }
        Ok(())
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
                let capture = mv.capture_square().uci();
                let ep = mv.ep().uci();
                // pseudo_legal=b.is_pseudo_legal_move(&mv);
                let legal = b.is_legal_move(mv);
                let san = if legal { b.to_san(mv) } else { "???".to_string() };
                let rook_move = mv.rook_move().uci();
                let is_ep = mv.is_ep_capture();
                let is_castle = mv.is_castle();
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
            Self::print("result:from 00 to 00 capture 00 ep - legal False san ??? rook_move 0000 is_ep False is_castle False");
        }
        Ok(())
    }

    fn ext_uci_static_eval(&mut self, arg: &Args) -> Result<()> {
        let mut b = Board::new_empty();
        Self::parse_fen(arg, &mut b)?;
        let eval = SimpleScorer::new();
        let score = b.eval(&eval, &Node::root(0));
        Self::print(&format!("result:{}", score));
        Ok(())
    }

    fn json_method(&mut self, request: &str) -> Result<()> {
        let response = self.json_rpc.invoke(request).ok_or(anyhow!("json rpc error"))?;
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
        Self::print(&format!("result:{}", b.make_moves(&var).to_fen()));
        Ok(())
    }

    fn ext_uci_legal_moves(&mut self, arg: &Args) -> Result<()> {
        let mut b = Board::new_empty();
        Self::parse_fen(arg, &mut b)?;
        let moves = b.legal_moves();
        Self::print(&format!("result:{}", moves.uci()));
        Ok(())
    }

    fn uci_position(&mut self, arg: &Args) -> Result<()> {
        self.engine.lock().unwrap().search_stop();
        Self::parse_fen(arg, &mut self.board)?;
        let variation = Self::parse_variation(arg, &self.board)?;
        let mut pos = Position::from_board(self.board.clone());
        pos.set(Tag::SuppliedVariation(variation));
        self.board = pos.supplied_variation().apply_to(pos.board());
        self.engine.lock().unwrap().set_position(pos);
        self.engine.lock().unwrap().algo.set_callback(Self::uci_info);
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
                bail!("Must specify fen or startpos after position command");
            }
        } else {
            bail!("Must specify a fen position or startpos")
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
                b = b.make_move(&mv);
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
                let mv = board.parse_uci_move(mv)?;
                movelist.push(mv)
            }
        }
        Ok(movelist)
    }

    fn uci_go(&mut self, args: &Args) -> Result<()> {
        self.engine.lock().unwrap().search_stop();
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

        let tc = if infinite {
            TimeControl::Infinite
        } else if let Some(depth) = depth {
            TimeControl::Depth(depth as Ply)
        } else if let Some(nodes) = nodes {
            TimeControl::NodeCount(nodes as u64)
        } else if let Some(movetime) = movetime {
            TimeControl::SearchTime(Duration::from_millis(movetime as u64))
        } else if let Some(mate) = mate {
            TimeControl::MateIn(mate as u32)
        } else if let Some(wtime) = wtime {
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
        } else {
            TimeControl::default()
        };

        self.engine.lock().unwrap().algo.set_timing_method(tc);
        self.engine.lock().unwrap().algo.mte.set_shared_ponder(ponder);
        // restrict search to this moves only
        // Example: After "position startpos" and "go infinite searchmoves e2e4 d2d4"
        // the engine should only search the two moves e2e4 and d2d4 in the initial position
        let search_moves = Self::parse_movelist(args, &self.board)?;
        self.engine.lock().unwrap().algo.restrictions.include_moves = search_moves;
        // self.log_debug_message("starting search with configuration ...");
        // self.log_debug_message(&format!("{}", self.engine.lock().unwrap().algo));
        // self.log_debug_message(&format!("{}", self.board));
        info!("odonata: searching {} on tc {}", self.board.to_fen(), tc);
        self.engine.lock().unwrap().search_start();
        Ok(())
    }

    pub fn uci_options(engine: &Engine) -> Vec<String> {
        let mut ops: Vec<String> = Vec::new();

        // ops.push(format!("option name UCI_EngineAbout type string default {} {}", Version::NAME, Version::HOMEPAGE));
        ops.push("option name debug type check default false".to_string());
        ops.push(format!(
            "option name Threads type spin default {} min 1 max 16",
            engine.thread_count
        ));
        // ops.push(format!("option name nodestime type spin default {} min 0 max 10000", engine.algo.mte.nodestime));
        ops.push(format!(
            "option name MultiPV type spin default {} min 1 max 64",
            engine.algo.restrictions.multi_pv_count
        ));
        ops.push(format!("option name Hash type spin default {} min 0 max 4000", engine.algo.tt.mb));
        ops.push("option name UCI_AnalyseMode type check default false".to_string());
        ops.push("option name Ponder type check default false".to_string());
        ops.push("option name Clear Hash type button".to_string());
        ops.push("option name Explain_Eval type button".to_string());
        ops.push("option name Explain_Last_Search type button".to_string());
        ops.push("option name Explain_Quiesce type button".to_string());
        ops.push("option name Show_Config type button".to_string());
        ops.push(format!("option name Config_File type string default {}", engine.config_filename));
        ops.push("option name Config type string default \"\"".to_string());
        ops
    }

    fn uci_option_button(&mut self, name: &str) -> Result<()> {
        let name = name.trim();
        info!("Actioning (setoption) {}", name);
        if name == "Explain_Eval" {
            self.ext_uci_explain_eval()?;
        } else if name == "Explain_Last_Search" {
            self.uci_explain_last_search()?;
        } else if name == "Explain_Quiesce" {
            self.ext_uci_explain_eval()?;
        } else if name == "Show_Config" {
            self.ext_uci_show_config()?;
        } else if name == "Clear Hash" {
            Self::print("Clearing hash");
            self.uci_newgame()?;
        } else {
            warn!("Unknown action '{}'", name);
        }
        Ok(())
    }

    fn uci_option_name_value(&mut self, name: &str, value: &str) -> Result<()> {
        let mut engine = self.engine.lock().unwrap();
        let value = if value == "\"\"" { "" } else { value };

        // handle specific name/value uci options
        // FIXME! as hardcoding names of variables
        if name == "debug" {
            self.debug = value == "true";
        } else if name == "Threads" {
            engine.configment("thread_count", value)?;
        } else if name == "MultiPV" {
            engine.configment("restrictions.multi_pv_count", value)?;
        } else if name == "nodestime" {
            engine.configment("mte.nodestime", value)?;
        } else if name == "Hash" {
            engine.configment("tt.mb", value)?;
            engine.algo.tt.set_state(State::NewGame);
        } else if name == "UCI_AnalyseMode" {
            engine.configment("analyse_mode", value)?;
        } else if name == "UCI_ShowRefutations" {
            engine.configment("show_refutations", value)?;
        } else if name == "Ponder" {
            // pondering determined by "go ponder", so no variable to track
        } else if name == "Config_File" {
            if !value.is_empty() && value != "config.toml" {
                engine.config_filename = value.to_string();
                use anyhow::Context;
                use figment::providers::{Format, Toml};
                use figment::Figment;
                use std::path::Path;
                // use toml;
                let path = Path::new(&engine.config_filename);
                if !path.is_file() {
                    bail!("Config_File '{}' not found", engine.config_filename);
                }
                let fig = Figment::new().merge(&*engine).merge(Toml::file(&engine.config_filename));

                let new: Engine = fig.extract().context(format!("error in config file {}", &engine.config_filename))?;
                *engine = new;
            }
        } else {
            bail!("Unknown option name '{}' value '{}'", name, value);
        }
        Ok(())
    }

    fn uci_setoption(&mut self, input: &str) -> Result<()> {
        self.engine.lock().unwrap().search_stop();
        let s = input.strip_prefix("setoption").ok_or(anyhow!("missing setoption"))?.trim();
        let s = s.strip_prefix("name").ok_or(anyhow!("missing 'name' from setoption"))?.trim();
        let name_value = s.rsplit_once("value");
        if let Some((name, value)) = name_value {
            let (name, value) = (name.trim(), value.trim());
            info!("Configuring (setoption) {} with:<{}>", name, value);
            if name == "Config" {
                let mut kvs = HashMap::new();
                let statements = value.split(';').collect_vec();
                for s in statements {
                    let s = s.trim();
                    if !s.is_empty() {
                        if let Some((name, value)) = s.split_once("=") {
                            kvs.insert(name.trim().to_string(), value.trim().to_string());
                        } else {
                            // bail!("Expected key=value but '{}' found instead", s)
                        }
                    }
                }
                let mut engine = self.engine.lock().unwrap();
                engine.configment_many(kvs)?
            } else {
                self.uci_option_name_value(name, value)?
            }
            // self.engine = Arc::new(Mutex::new(new_engine));
            // let c = ParsedConfig::new().set(&name, &value);
            // self.configure(&c);
            self.engine.lock().unwrap().set_position(Position::from_board(self.board.clone()));
            self.engine.lock().unwrap().algo.set_callback(Self::uci_info);
        } else {
            self.uci_option_button(s)?;
        };
        Ok(())
    }

    fn uci_show_options(&self) {
        let mut engine = self.engine.lock().unwrap();
        engine.search_stop();
        for op in &Self::uci_options(&engine) {
            Self::print(op);
        }
    }

    fn ext_uci_show_config(&mut self) -> Result<()> {
        Self::print(&format!("# show configuration:\n"));
        self.engine.lock().unwrap().search_stop();
        let engine = self.engine.lock().unwrap();
        let text = toml::to_string(&*engine)?;
        Self::print(&format!("# start configuration:\n{}", text));
        Self::print(&format!("# end configuration:"));
        Ok(())
    }

    fn ext_uci_explain_eval(&mut self) -> Result<()> {
        self.engine.lock().unwrap().search_stop();
        let eval = &self.engine.lock().unwrap().algo.eval;
        let s = eval.w_eval_explain(&self.board);
        Self::print(&format!("Board:\n{}", &self.board));
        Self::print(&format!("Material advantage: {}", &self.board.material().advantage()));
        Self::print(&format!("Eval:\n{}", s));
        Ok(())
    }

    fn uci_explain_last_search(&mut self) -> Result<()> {
        self.engine.lock().unwrap().search_stop();
        Self::print("search");
        Self::print(&format!("{}", self.board));
        Self::print(&format!("{}", self.engine.lock().unwrap()));
        Ok(())
    }

    fn uci_board(&mut self) -> Result<()> {
        self.engine.lock().unwrap().search_stop();
        Self::print("board");
        Self::print(&format!("{}", self.board));
        Self::print(&format!("outcome {}", self.board.outcome()));
        Self::print(&format!("legal moves:{}", self.board.legal_moves().uci()));
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

    pub fn uci_info(sr: &SearchResults) {
        if let SearchResultsMode::BestMove = sr.mode {
            Self::print_bm_and_ponder(&sr.best_pv);
        } else {
            Self::print(&format!("info {}", UciInfo(sr)));
        }
    }

    fn print_bm_and_ponder(var: &Variation) {
        let bm = if var.len() > 0 {
            var[0]
        } else {
            info!("---> Null  best move");
            Move::NULL_MOVE
        };
        let mut output = format!("bestmove {}", bm.uci());
        if var.len() > 1 {
            output = format!("{} ponder {}", output, var[1].uci());
        }
        Self::print(&output)
    }
}

struct UciInfo<'a>(&'a SearchResults);

impl<'a> fmt::Display for UciInfo<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let SearchResultsMode::Refutation = self.0.mode {
            let strings: Vec<String> = self.0.best_pv.iter().map(Move::to_string).collect();
            write!(f, "refutation {}", strings.join(" "))?;
        }
        if let SearchResultsMode::PvChange = self.0.mode {
            write!(f, "depth {} ", self.0.depth)?;
            write!(f, "seldepth {} ", self.0.seldepth)?;

            if let Some(nodes) = self.0.nodes {
                write!(f, "nodes {} ", nodes)?;
            }
            if let Some(nps) = self.0.nps {
                write!(f, "nps {} ", nps)?;
            }
            write!(f, "score {} ", self.0.score.uci())?;
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
                if self.0.multi_pv_index_of > 1 {
                    write!(f, "multipv {} ", self.0.multi_pv_index + 1)?;
                }
                let strings: Vec<String> = self.0.pv.iter().map(Move::to_string).collect();
                write!(f, "pv {}", strings.join(" "))?;
            }
        }
        if let SearchResultsMode::NodeCounts = self.0.mode {
            // write!(f, "depth {} ", self.0.depth)?;
            // write!(f, "seldepth {} ", self.0.seldepth)?;

            if let Some(nodes) = self.0.nodes {
                write!(f, "nodes {} ", nodes)?;
            }
            if let Some(nps) = self.0.nps {
                write!(f, "nps {} ", nps)?;
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
            }
        }
        Ok(())
    }
}

struct Args {
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
    use test_log::test;

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
        uci.prelude.push("isready".into());
        // uci.preamble.push("debug on".into());
        uci.prelude.push("debug off".into());
        uci.prelude.push("debug junk".into());
        uci.prelude.push("quit".into());
        uci.run();
    }

    #[test]
    fn test_uci_perft() {
        let mut uci = Uci::new();
        uci.prelude.push("perft 1".into());
        uci.prelude.push("quit".into());
        uci.run();
    }

    #[test]
    fn test_uci_helpers() {
        let mut uci = Uci::new();
        uci.prelude.push("b".into());
        uci.prelude.push("?".into());
        uci.prelude.push(".".into());
        uci.prelude.push("quit".into());
        uci.run();
    }

    #[test]
    fn test_uci_config_file() {
        let mut uci = Uci::new();
        // uci.uci_option_name_value("Config_File", "../odonata/resources/Xconfig.toml").unwrap();
        assert_eq!(uci.engine.lock().unwrap().algo.eval.position, true);
        uci.prelude
            .push("setoption name Config_File value ../odonata/resources/config.toml".into());
        uci.prelude.push("setoption name Show_Config".into());
        uci.prelude.push("quit".into());
        uci.run();
        assert_eq!(uci.engine.lock().unwrap().algo.eval.position, true);
    }

    #[test]
    fn test_uci_setoption() {
        let mut uci = Uci::new();
        let _bishop = uci.engine.lock().unwrap().algo.eval.mb.piece_weights[Piece::Bishop];
        assert_eq!(uci.engine.lock().unwrap().algo.eval.position, true);
        uci.prelude.push("setoption name Config value eval.mb.b.s=700".into());
        uci.prelude
            .push("setoption name Config value eval.mb.n = { s=400, e = 429 }".into());
        uci.prelude.push("setoption name Config value eval.position=false".into());
        uci.prelude.push("setoption name Explain_Eval".into());
        uci.prelude.push("setoption name Config value eval.pst.p.a2.s = 10".into());
        uci.prelude.push("setoption name Show_Config".into());
        uci.prelude.push("quit".into());
        uci.run();
        let eval = &uci.engine.lock().unwrap().algo.eval;
        assert_eq!(eval.position, false);
        assert_eq!(eval.mb.piece_weights[Piece::Knight].s() as i32, 400);
        assert_eq!(eval.mb.piece_weights[Piece::Knight].e() as i32, 429);
        assert_eq!(eval.mb.piece_weights[Piece::Bishop].s() as i32, 700);
    }

    #[test]
    fn test_uci_position() {
        let mut uci = Uci::new();
        //uci.preamble.push("debug on".into());
        uci.prelude.push("position startpos".into());
        uci.prelude.push("display".into());
        uci.prelude.push("quit".into());
        uci.run();
        assert_eq!(uci.board, Catalog::starting_board());

        let mut uci = Uci::new();
        uci.prelude.push("position fen k7/8/8/8/8/8/8/7k w - - 0 2".into());
        uci.prelude.push("quit".into());
        uci.run();
        assert_eq!(uci.board, Board::parse_fen("k7/8/8/8/8/8/8/7k w - - 0 2").unwrap());

        let mut uci = Uci::new();
        uci.prelude.push("position startpos moves a2a3 a7a6".into());
        uci.prelude.push("quit".into());
        uci.run();
        assert_eq!(
            uci.board.to_fen(),
            Board::parse_fen("rnbqkbnr/1ppppppp/p7/8/8/P7/1PPPPPPP/RNBQKBNR w KQkq - 0 2")
                .unwrap()
                .to_fen()
        );

        let mut uci = Uci::new();
        uci.prelude
            .push("position fen rnbqkbnr/1ppppppp/p7/8/8/P7/1PPPPPPP/RNBQKBNR w KQkq - 0 1 moves h2h3 h7h6".into());
        uci.prelude.push("quit".into());
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
        let mut uci = Uci::new();
        // uci.preamble.push("debug on".into());
        uci.prelude.push("position startpos moves d2d4".into());
        uci.prelude.push("go depth 1".into());
        uci.prelude.push("quit".into());
        uci.run();
        thread::sleep(Duration::from_millis(600));
    }

    #[test]
    fn test_uci_go2() {
        let mut uci = Uci::new();
        uci.prelude.push("debug on".to_string());
        uci.prelude.push("position startpos moves d2d4".to_string());
        uci.prelude.push("go wtime 1000 btime 1000".to_string());
        uci.prelude.push("sleep 1100".to_string());
        uci.prelude.push("ucinewgame".to_string());
        uci.prelude.push("position startpos moves d2d4".to_string());
        uci.prelude
            .push("go wtime 20160 btime 20160 winc 160 binc 160 nodes 3000".to_string());
        uci.prelude.push("sleep 500".to_string());
        uci.prelude.push("quit".to_string());
        uci.run();
        // println!("pvtable:\n{}", uci.algo.pv);
        // assert_eq!(uci.board, Catalog::starting_position());
    }

    #[test]
    fn test_ponder() {
        let mut uci = Uci::new();
        uci.prelude.push("debug on".to_string());
        uci.prelude.push("position startpos".to_string());
        uci.prelude.push("go ponder movetime 1000".to_string());
        uci.prelude.push("sleep 300".to_string());
        uci.prelude.push("ponderhit".to_string());
        uci.prelude.push("sleep 1100".to_string());
        uci.prelude.push("quit".to_string());
        uci.run();
        println!("\n{}", uci.engine.lock().unwrap().algo);
    }
}
