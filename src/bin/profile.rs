use odonata::catalog::*;
use odonata::search::algo::Algo;
use odonata::search::timecontrol::TimeControl;
use env_logger;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let board = Catalog::starting_position();
    let mut search =
        Algo::new().set_timing_method(TimeControl::move_time_secs(1)).build();
    for _i in 0..3 {
        search.search(&board);
        println!("{}", search.results());
        search.reset();
    }
    println!("\n\n{}\n", search);
    println!("Bye world!");
}

    