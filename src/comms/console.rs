use crate::boards::Board;
use crate::comms::uci_server::UciServer;
use crate::infra::version::Version;
use crate::search::algo::Algo;
use std::io;

pub struct Console {
    _board: Board,
    _algo: Algo,
}

impl Console {
    pub fn run() {
        Self::help();
        loop {
            let mut input = String::new();
            println!("enter command...");

            io::stdin().read_line(&mut input).unwrap();
            let words: Vec<&str> = input.split_whitespace().collect();
            match words.first() {
                Some(&"uci") => {
                    let mut uci = UciServer::new();
                    uci.run();
                    break;
                }
                Some(&"quit") => {
                    break;
                }
                Some(&"help") => {
                    Self::help();
                }
                Some(word) => {
                    println!("Unknown command '{}'", word);
                }
                _ => {
                    println!();
                }
            };
        }
        // dont print anything after exit as pipe will not exist
        // println!("Exiting console...");
    }

    pub fn help() {
        println!();
        println!("{} {}\n", Version::NAME, Version::VERSION);
        println!("{}", Version::small_splash());
        println!(
            "Please see {} for updates,\nreleases and licence details. ",
            Version::HOMEPAGE
        );
        println!();
        println!("Commands...");
        println!("{:<10} enter uci protocol mode", "uci");
        println!("{:<10} quit the program", "quit");
        println!("{:<10} display (this) help", "help");
        println!();
    }
}
