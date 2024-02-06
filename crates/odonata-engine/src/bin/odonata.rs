use clap::{Parser, Subcommand};
use odonata_base::{
    domain::timecontrol::TimeControl, infra::utils::ToStringOr, trace::logger::LoggingSystem,
};
use odonata_engine::comms::{bench::Bench, uci_server::UciServer};
use std::{backtrace::Backtrace, collections::HashMap, panic};
use tracing::{debug, error};

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct OdonataCli {
    /// Specifies a custom config file
    #[arg(short, long, value_name = "FILE", default_value_t = String::from("default"))]
    config: String,

    /// Defines a startup parameter
    #[arg(short = 'D', long, value_name = "VAR=VALUE")]
    define: Vec<String>,

    /// Specifies the number of threads to use
    #[arg(short, long, value_name = "THREADS")]
    threads: Option<u32>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    /// Illegal uci commands and settings are fatal - useful for cutechess-cli
    #[arg(short, long)]
    strict: bool,

    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand, Debug, Clone)]
enum Cmd {
    /// Runs the chess engine (this is the default if no command is specified)
    Engine,

    /// Executes benchmarks for OpenBench
    Bench,

    /// Show uci settings and other configuration
    ShowConfig,

    /// Execute a series of uci commands
    Uci { command: String },

    /// Runs code to assist with profiling
    Profile,

    /// Runs perft from standard chess opening position
    Perft { depth: u32 },

    /// Runs perft with category breakdown from standard chess opening position
    PerftCat { depth: u32 },

    /// Analyse a series of test positions with using a given depth, nodes, st or tc. Use odonata_var_eval__eval_weights_kind=SoftcodedF64 etc
    Search {
        #[arg(short = 't', long, value_name = "TIME_CONTROL")]
        time_control: TimeControl,
    },
}

pub fn main() -> anyhow::Result<()> {
    LoggingSystem::init()?;
    let cli = OdonataCli::parse();
    debug!("Command line {cli:#?}");

    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let backtrace = Backtrace::force_capture();
        error!(
            "Thread {name} panic {panic_info:?}\nbacktrace\n{backtrace}",
            name = std::thread::current().name().to_string_or("unknown")
        );
        orig_hook(panic_info);
        std::process::exit(1);
    }));

    // logger::init_logging();

    // if let Some(filename) = matches.value_of("config") {
    //     let config = ParsedConfig::read_from_file(filename)?;
    //     ParsedConfig::set_global(config);
    // }
    let mut settings = HashMap::new();
    for kv in &cli.define {
        let i = kv
            .find('=')
            .ok_or(anyhow::anyhow!("unable to find '=' in -D/--define '{kv}'"))?;
        let (k, v) = kv.split_at(i);
        settings.insert(k.to_string(), v[1..].to_string());
    }
    let mut uci = UciServer::configure(settings)?;
    uci.strict_error_handling = cli.strict;

    match cli.command.unwrap_or(Cmd::Engine) {
        Cmd::ShowConfig => uci.add_prelude("uci; show_config; quit").run(),
        Cmd::Bench => uci
            .add_prelude("position startpos; go depth 11; isready; bench; quit")
            .run(),
        Cmd::Profile => Bench::profile_me(),
        Cmd::Perft { depth } => uci.add_prelude(&format!("perft {depth}; quit")).run(),
        Cmd::PerftCat { depth } => uci.add_prelude(&format!("perft_cat {depth}; quit")).run(),
        Cmd::Search { time_control } => drop(Bench::search(time_control, cli.threads)),
        Cmd::Uci { command } => uci.add_prelude(&(command + "; isready; quit")).run(),
        Cmd::Engine => uci.add_prelude("compiler").run(),
    };
    Ok(())
}
