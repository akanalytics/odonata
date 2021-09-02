use clap::{App, Arg};
use odonata::comms::console::Console;
use odonata::comms::uci::Uci;
use odonata::comms::bench::Bench;
use odonata::version::Version;
use odonata::search::timecontrol::TimeControl;


// pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
// pub const AUTHORS: &'static str = env!("CARGO_PKG_AUTHORS");
// pub const NAME: &'static str = env!("CARGO_PKG_NAME");
// pub const HOMEPAGE: &'static str = env!("CARGO_PKG_HOMEPAGE");
// pub const IMAGE: &'static str = r##"


fn main() -> Result<(), String> {
    let matches = App::new(Version::NAME)
        .version(Version::VERSION)
        .about(&*Version::small_splash())
        // .arg(
        //     Arg::with_name("v")
        //         .short("v")
        //         .multiple(true)
        //         .help("sets the level of logging verbosity: eg -vv or -vvv"),
        // )
        .arg(Arg::with_name("uci")
            .help("enter uci mode without waiting for 'uci' on stdin")
            .short("u")
            .long("uci")
        )
        .arg(Arg::with_name("perft")
            .help("runs perft from standard chess opening position")
            .long("perft")
            .value_name("depth")
            .takes_value(true)
        )
        .arg(Arg::with_name("profile")
            .help("runs code to assist with profiling")
            .long("profile")
        )
        .arg(Arg::with_name("threads")
            .help("sets the number of threads to use")
            .long("threads")
            .value_name("n")
            .default_value("1")
            .takes_value(true)
        )
        .arg(Arg::with_name("perft_cat")
            .help("runs perft with category breakdown from standard chess opening position")
            .long("perft_cat")
            .value_name("depth")
            .takes_value(true)
        )
        .arg(Arg::with_name("search")
            .help("analyse a series of test positions with using a given depth, nodes, st or tc")
            .long("search")
            .value_name("time control")
            .default_value("nodes=1000000")   // "st=0.300"
            .takes_value(true)
        )
        .get_matches();

    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
    // let log_level = match matches.occurrences_of("v") {
    //     0 => "error",
    //     1 => "warn",
    //     2 => "info",
    //     3 => "debug",
    //     4 | _ => "trace",
    // };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    if matches.is_present("uci") {
        Uci::new().run();
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
        let tc = TimeControl::parse(tc).map_err(|e| e.to_string())?;
        let threads = matches.value_of("threads").unwrap();
        let threads = threads.parse::<u32>().map_err(|e| e.to_string())?;
        Bench::search(tc, threads);
        // } else {
        //     unreachable!("search always has a default millis")
        // }
    } else {
        Console::run();
    }
    Ok(())
}
