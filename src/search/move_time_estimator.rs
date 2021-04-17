use crate::board::Board;
use crate::search::searchstats::SearchStats;
use std::fmt;
use std::time::Duration;
use crate::clock::Clock;
use crate::search::timecontrol::TimeControl;
use crate::config::{Config, Configurable};
use crate::log_debug;


#[derive(Clone, Debug)]
pub struct MoveTimeEstimator {
    pub time_control: TimeControl,
    board: Board,
    branching_factor: u16,
    moves_rem: u16,
    pub time_estimate: Duration,
    pub elapsed_used: Duration,
    pub deterministic: bool,
}


impl Configurable for MoveTimeEstimator {
    fn settings(&self, c: &mut Config) {
        c.set("mte.branching_factor", "type spin default 12 min 1 max 100");
        c.set("mte.moves_rem", "type spin default 20 min 1 max 100");
        c.set("mte.deterministic", "type check default false");
    }
    
    fn configure(&mut self, c: &Config) {
        log_debug!("mte.configure with {}", c);
        self.branching_factor = c.int("mte.branching_factor").unwrap_or(self.branching_factor as i64) as u16;
        self.moves_rem = c.int("mte.moves_rem").unwrap_or(self.moves_rem as i64) as u16;
        self.deterministic = c.bool("mte.deterministic").unwrap_or(self.deterministic);

    }
}

impl Default for MoveTimeEstimator {
    fn default() -> Self {
        MoveTimeEstimator {
            branching_factor: 12,
            moves_rem: 20,
            board: Board::default(),
            time_control: TimeControl::default(),
            time_estimate: Duration::default(),
            elapsed_used: Duration::default(),
            deterministic: false,
        }
    }
}

impl fmt::Display for MoveTimeEstimator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "time_control     : {}", self.time_control)?;
        writeln!(f, "board            : {}", self.board.to_fen())?;
        writeln!(f, "branching factor : {}", self.branching_factor)?;
        writeln!(f, "const moves rem. : {}", self.moves_rem)?;
        writeln!(f, "allotted for mv  : {}", Clock::format_duration(self.alloted_time_for_move()))?;
        writeln!(f, "time estimate    : {}", Clock::format_duration(self.time_estimate))?;
        writeln!(f, "deterministic    : {}", self.deterministic)?;
        writeln!(f, "elapsed used     : {}", Clock::format_duration(self.elapsed_used))?;
        Ok(())
    }
}

impl MoveTimeEstimator {

    pub fn is_time_up(&self, _ply: u32, search_stats: &SearchStats) -> bool {
        let elapsed = search_stats.elapsed(self.deterministic);

        let time_up = match self.time_control {
            TimeControl::Depth(_max_ply) => false, // ply > max_ply,  // dont cause an abort on last iteration
            TimeControl::MoveTime(duration) => elapsed > duration,
            TimeControl::NodeCount(max_nodes) => search_stats.total().nodes() > max_nodes,
            TimeControl::Infinite => false,
            TimeControl::MateIn(_) => false,
            TimeControl::RemainingTime { our_color, wtime, btime, winc, binc, movestogo: _ } => {
                let (time, _inc) = our_color.chooser_wb((wtime, winc), (btime, binc));
                elapsed > time / self.moves_rem as u32
            }
        };
        time_up
    }

    pub fn calc_estimates_for_ply(&mut self, _ply: u32, search_stats: &SearchStats) {
        // debug_assert!(search_stats.depth() >= ply-1, "ensure we have enough stats");
        let _forecast_depth = search_stats.depth();
        self.elapsed_used = search_stats.elapsed(self.deterministic);
        self.time_estimate = self.elapsed_used * self.branching_factor as u32;
        
    }

    pub fn probable_timeout(&self, _search_stats: &SearchStats) -> bool {
        match self.time_control {
            TimeControl::RemainingTime { our_color, wtime, btime, winc, binc, movestogo: _ } => {
                let (_time, _inc) = our_color.chooser_wb((wtime, winc), (btime, binc));
                self.time_estimate > self.alloted_time_for_move()
            },
            _ => false,
        }
    }

    fn alloted_time_for_move(&self) -> Duration {
        let zero = Duration::from_secs(0);
        match self.time_control {
            TimeControl::Depth(_) => zero,
            TimeControl::MoveTime(duration) => duration,
            TimeControl::NodeCount(_) => zero,
            TimeControl::Infinite => zero,
            TimeControl::MateIn(_) => zero,
            TimeControl::RemainingTime { our_color, wtime, btime, winc, binc, movestogo: _ } => {
                let (time, _inc) = our_color.chooser_wb((wtime, winc), (btime, binc));
                time / self.moves_rem as u32
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::*;
    use crate::search::algo::*;
    use crate::comms::uci::*;
    use crate::eval::eval::*;
    use crate::eval::score::*;

    #[test]
    fn test_display_mte() {
        let mte = MoveTimeEstimator::default();
        println!("{}", mte);
        println!("{:?}", mte);
        println!("{:#?}", mte);
    }

    #[test]
    fn test_mate_with_tc() {
        let position = Catalog::mate_in_2()[0].clone();
        let eval = SimpleScorer::new().set_position(false);
        let mut search = Algo::new()
            .set_timing_method(TimeControl::from_remaining_time(Duration::from_secs(3)))
            .set_minmax(false)
            .set_eval(eval)
            .set_iterative_deepening(true)
            .set_callback(Uci::uci_info).clone();
        search.mte.deterministic = true;
        search.search(position.board().clone());
        println!("{}", search);
        assert_eq!(search.search_stats().total().nodes(), 2200);  // with qsearch
        //assert_eq!(search.search_stats().total().nodes(), 2108);  // with ordering pv + mvvlva
        // assert_eq!(search.search_stats().total().nodes(), 3560); 
        // assert_eq!(search.search_stats().total().nodes(), 6553);  // with ordering pv
        // assert_eq!(search.search_stats().total().nodes(), 6740);
        assert_eq!(search.pv(), &position.pv().unwrap());
        assert_eq!(search.score(), Score::WhiteWin { minus_ply: -3 });
    }
}

