use clap::{App, Arg};
use odonata::comms::console::Console;
use odonata::comms::uci::Uci;
use odonata::comms::bench::Bench;
use odonata::version::Version;

// pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
// pub const AUTHORS: &'static str = env!("CARGO_PKG_AUTHORS");
// pub const NAME: &'static str = env!("CARGO_PKG_NAME");
// pub const HOMEPAGE: &'static str = env!("CARGO_PKG_HOMEPAGE");
// pub const IMAGE: &'static str = r##"


fn main() -> Result<(), std::num::ParseIntError> {
    let matches = App::new(Version::NAME)
        .version(Version::VERSION)
        .author(Version::AUTHORS)
        .about(Version::HOMEPAGE)
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
            .takes_value(true))
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

    // env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    if matches.is_present("uci") {
        Uci::new().run();
    } if let Some(depth) = matches.value_of("perft") {
        let depth = depth.parse::<u32>()?;
        Bench::perft(depth);
    } else {
        Console::run();
    }
    Ok(())
}
