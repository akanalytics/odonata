use odonata::catalog::*;
use odonata::search::algo::Algo;
use odonata::search::timecontrol::TimeControl;


fn main() {
    println!("Hello world!");

    let board = Catalog::starting_position();
    let mut search =
        Algo::new().set_timing_method(TimeControl::move_time_secs(1)).build();
    for _i in 0..10 {
        search.search(&board);
        println!("{}", search.results());
    }
    println!("\n\n{}\n", search);
    println!("Bye world!");
}

    