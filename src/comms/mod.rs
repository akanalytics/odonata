pub mod bench;
pub mod console;
pub mod json_rpc;
pub mod uci_server;

use crate::comms::bench::Bench;
use crate::comms::uci_server::UciServer;
use crate::infra::version::Version;
use crate::search::timecontrol::TimeControl;
use clap::{Command, Arg};

pub fn main() -> Result<(), String> {
    let matches = Command::new(Version::NAME)
        .version(Version::VERSION)
        .about(&*Version::small_splash())
        // .arg(
        //     Arg::with_name("v")
        //         .short("v")
        //         .multiple(true)
        //         .help("sets the level of logging verbosity: eg -vv or -vvv"),
        // )
        .subcommand(Command::new("bench").about("execute benchmarks for OpenBench"))
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("specifies a custom config file")
                .takes_value(true),
        )
        .arg(
            Arg::new("uci")
                .help("show uci settings and exit")
                .short('u')
                .long("uci"),
        )
        .arg(
            Arg::new("strict")
                .help("illegal uci commands and settings are fatal - useful for cutechess-cli")
                .short('s')
                .long("strict"),
        )
        .arg(
            Arg::new("perft")
                .help("runs perft from standard chess opening position")
                .long("perft")
                .value_name("depth")
                .takes_value(true),
        )
        .arg(
            Arg::new("profile")
                .help("runs code to assist with profiling")
                .long("profile"),
        )
        .arg(
            Arg::new("threads")
                .help("sets the number of threads to use")
                .long("threads")
                .value_name("n")
                .takes_value(true),
        )
        .arg(
            Arg::new("perft_cat")
                .help("runs perft with category breakdown from standard chess opening position")
                .long("perft_cat")
                .value_name("depth")
                .takes_value(true),
        )
        .arg(
            Arg::new("search")
                .help(
                    "analyse a series of test positions with using a given depth, nodes, st or tc",
                )
                .long("search")
                .value_name("time control")
                .default_missing_value("nodes=100000") // "st=0.300"
                .takes_value(true),
        )
        .get_matches();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    // logger::init_logging();

    // if let Some(filename) = matches.value_of("config") {
    //     let config = ParsedConfig::read_from_file(filename)?;
    //     ParsedConfig::set_global(config);
    // }
    if matches.subcommand_name() == Some("bench") {
        let mut uci = UciServer::new();
        uci.prelude.push("position startpos".to_string());
        uci.prelude.push("go depth 16".to_string());
        uci.prelude.push("bench".to_string());
        uci.prelude.push("quit".to_string());
        uci.run();
    } else if matches.is_present("profile") {
        Bench::profile_me();
    } else if let Some(depth) = matches.value_of("perft") {
        let depth = depth.parse::<u32>().map_err(|e| e.to_string())?;
        Bench::perft(depth);
    } else if let Some(depth) = matches.value_of("perft_cat") {
        let depth = depth.parse::<u32>().map_err(|e| e.to_string())?;
        Bench::perft_cat(depth);
    } else if matches.occurrences_of("search") > 0 {
        let tc = matches.value_of("search").unwrap();
        let tc = TimeControl::parse(tc).map_err(|e| e)?;
        let str = matches.value_of("threads");
        let threads = if let Some(str) = str {
            Some(str.parse::<u32>().map_err(|e| e.to_string())?)
        } else {
            None
        };
        Bench::search(tc, threads);
    } else {
        let mut uci = UciServer::new();
        uci.strict_error_handling = matches.is_present("strict");
        if matches.is_present("uci") {
            uci.prelude.push("settings".to_string());
            uci.prelude.push("uci".to_string());
            uci.prelude.push("quit".to_string());
        }
        uci.run();
    }
    Ok(())
}
