use crate::board::Board;
use crate::comms::uci::Uci;
use crate::search::algo::Algo;
use crate::version::Version;
use std::io::{self, Write};

pub struct Console {
    board: Board,
    algo: Algo,
}

impl Console {
    pub fn run() {
        Self::banner();
        Self::help();
        loop {
            let mut input = String::new();
            println!("enter command...");

            io::stdin().read_line(&mut input).unwrap();
            let words: Vec<&str> = input.split_whitespace().collect();
            match words.first() {
                Some(&"uci") => {
                    let mut uci = Uci::new();
                    uci.run();
                    break;
                }
                Some(&"exit") => {
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
    }

    pub fn help() {
        println!("{:<10} {}", "uci", "enter uci protocol mode");
        println!("{:<10} {}", "exit", "exit the program");
        println!("{:<10} {}", "help", "display (this) help");
        println!();
    }

    pub fn banner() {
        println!();
        println!("{} version {} by {}", Version::NAME, Version::VERSION, Version::AUTHORS);
        println!("{}", Version::IMAGE);
        println!("please see {} for updates,\nreleases and license details.", Version::HOMEPAGE);
        println!();
    }
}
