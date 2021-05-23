use odonata::catalog::*;
use odonata::search::algo::Algo;
use odonata::perft::Perft;
use odonata::search::timecontrol::TimeControl;
use env_logger;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    println!("{}", Perft::perft(&mut Catalog::starting_position().clone(), 6));
    let pos = &Catalog::test_position();
    let mut search =
        Algo::new().set_timing_method(TimeControl::move_time_secs(1)).build();
    for _i in 0..6 {
        search.search(pos.board());
        // println!("{}", search.results());
        search.new_game();
    }
    println!("\n\n{}\n", search);
    println!("Bye world!");
}

    