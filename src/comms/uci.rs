use crate::version::Version;
use crate::perft::Perft;
use crate::board::Board;
use crate::board::boardbuf::BoardBuf;
use crate::board::makemove::MoveMaker;
use crate::movelist::MoveValidator;
use crate::catalog::Catalog;
use std::io::{self, Write, Stdout};

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
}

impl Uci {

    pub fn new() -> Uci {
        Uci {
            preamble: vec![String::from("version")],
            running: false,
            board: Board::default(),
        }
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
            "uci" => self.uci_unknown(&words),
            "isready" => self.uci_isready(),
            "debug" => self.uci_debug(&words[1..]),
            "setoption" => self.uci_unknown(&words),
            "register" => self.uci_unknown(&words),
            "ucinewgame" => self.uci_unknown(&words),
            "position" => self.uci_position(&words[1..]),
            "go" => self.uci_unknown(&words),
            "stop" => self.uci_unknown(&words),
            "ponderhit" => self.uci_unknown(&words),
            "quit" => self.uci_quit(),

            // extensions
            "version" => self.uci_version(),
            "perft" => self.uci_perft(&words[1..]),
            "tune" => self.uci_unknown(&words),
            "display" => self.uci_unknown(&words),
            "eval" => self.uci_unknown(&words),
            "bench" => self.uci_unknown(&words),
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

    fn uci_debug(&mut self, words: &[&str]) -> Result<(), String>  {
        println!("command {:?}", words);
        match words.first().copied() {
            Some("on") => println!("--on"),
            Some("off") => println!("--off"),
            _ => return Err("unknown debug option".into()),
        }
        Ok(())
    }

    fn uci_isready(&mut self) -> Result<(), String>  {
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
        println!("uciok");
        Ok(())
    }

    fn uci_perft(&self, words: &[&str]) -> Result<(), String> {
        let depth = words.first().ok_or("Must specify a depth")?;
        let depth = depth.parse::<u32>().or(
            Err(format!("Depth {} must be numeric", depth))
        )?;
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
            },
            _ => {
                let fen = words[0..].join(" ");
                self.board = Board::parse_fen(&*fen)?;
                Ok(())
            }
        }
    }
 
}




#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(uci.board, Board::parse_fen("rnbqkbnr/1ppppppp/p7/8/8/P7/1PPPPPPP/RNBQKBNR w KQkq - 0 2").unwrap());
    }
}
