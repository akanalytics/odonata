use crate::version::Version;
use crate::perft::Perft;
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
}

impl Uci {

    pub fn new() -> Uci {
        Uci {
            preamble: vec![String::from("version")],
            running: false
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

        match words[0] {
            "uci" => self.uci_unknown(&words),
            "isready" => self.uci_isready(),
            "debug" => self.uci_debug(&words[1..]),
            "setoption" => self.uci_unknown(&words),
            "register" => self.uci_unknown(&words),
            "ucinewgame" => self.uci_unknown(&words),
            "go" => self.uci_unknown(&words),
            "stop" => self.uci_unknown(&words),
            "ponderhit" => self.uci_unknown(&words),
            "quit" => self.uci_quit(),

            // extensions
            "version" => self.uci_version(),
            "perft" => self.uci_unknown(&words),
            "tune" => self.uci_unknown(&words),
            "display" => self.uci_unknown(&words),
            "eval" => self.uci_unknown(&words),
            "bench" => self.uci_unknown(&words),
            _ => self.uci_unknown(&words),
        }
        io::stdout().flush().ok();
     }

    fn uci_unknown(&mut self, words: &[&str]) {
        println!("unknown command {:?}", words);
    }

    fn uci_debug(&mut self, words: &[&str]) {
        println!("command {:?}", words);
        match words.first().copied() {
            Some("on") => println!("--on"),
            Some("off") => println!("--off"),
            _ => println!("unknown"),
        }
    }

    fn uci_isready(&mut self) {
        println!("isready");
    }

    fn uci_quit(&mut self) {
        self.running = false;
    }

    fn uci_version(&self) {
        println!("id name {} v{}", Version::NAME, Version::VERSION);
        println!("id author {}", Version::AUTHORS);
        println!("uciok");
    }

    fn uci_perft(&self, words: &[&str]) -> Result<(), String> {
        // let depth = words.first().expect("") {
        //     Some(word) => {
        //         if let Ok(depth) = word.parse::<u32>() {
        //             for d in 1..=depth {
        //                 println!("perft({}) = {}", d, Perft::perft(d));
        //             }
        //         }
        //     }
        //     _ => println!("unknown"),
        // }
        Ok(())
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
}
