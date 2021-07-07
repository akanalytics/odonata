use crate::board::Board;
use crate::comms::uci::Uci;
use crate::search::algo::Algo;
use crate::version::Version;
use std::io;

pub struct Console {
    _board: Board,
    _algo: Algo,
}


// see https://docs.rs/built/0.5.1/built/
pub mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
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
                    let mut uci = Uci::new();
                    uci.run();
                    break;
                }
                Some(&"quit") => {
                    break;
                }
                Some(&"splash") => {
                    Self::splash();
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
        println!("Exiting console...");
    }

    pub fn help() {
        println!("{} version {} by {}", Version::NAME, Version::VERSION, Version::AUTHORS);
        println!();
        Self::small_splash();
        println!();
        println!("Commands...");
        println!("{:<10} {}", "uci", "enter uci protocol mode");
        println!("{:<10} {}", "quit", "quit the program");
        println!("{:<10} {}", "splash", "splash screen");
        println!("{:<10} {}", "help", "display (this) help");
        println!();
    }

    pub fn splash() {
        println!();
        println!("{} version {} by {}", Version::NAME, Version::VERSION, Version::AUTHORS);
        println!("{}", Version::IMAGE);
        println!("please see {} for updates,\nreleases and license details.", Version::HOMEPAGE);
        println!();
    }

    pub fn small_splash() {
        println!("target       : {}", built_info::TARGET);
        println!("profile      : {}", built_info::PROFILE);
        println!("optimization : {}",   built_info::OPT_LEVEL);
        println!("rustc        : {}", built_info::RUSTC_VERSION);
        println!("features     : {}",  built_info::FEATURES_STR);
        println!("compiled at  : {}",  built_info::BUILT_TIME_UTC);
    }
}
