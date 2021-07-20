use crate::clock::DurationNewType;
use crate::types::{Color, Ply};
use std::fmt;
use std::time::Duration;
use std::str::FromStr;

/// https://en.wikipedia.org/wiki/Time_control
///
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TimeControl {
    EpdDepth,         // depth "recommended" by EPD position
    Depth(Ply),         // uci "depth"
    MoveTime(Duration), // uci "movetime"
    NodeCount(u64),     // uci "nodes"
    Infinite,           // uci "infinite"
    MateIn(u32),        // uci "mate"
    RemainingTime {
        our_color: Color,
        wtime: Duration,
        btime: Duration,
        winc: Duration,
        binc: Duration,
        movestogo: u16,
    },
}

impl fmt::Display for TimeControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TimeControl::EpdDepth => write!(f, "EpdDepth()")?,
            TimeControl::Depth(max_ply) => write!(f, "Depth({})", max_ply)?,
            TimeControl::MoveTime(duration) => write!(f, "MoveTime({})", DurationNewType(*duration))?,
            TimeControl::NodeCount(max_nodes) => write!(f, "NodeCount({})", max_nodes)?,
            TimeControl::Infinite => write!(f, "Infinite")?,
            TimeControl::MateIn(depth) => write!(f, "MateIn({})", depth)?,
            TimeControl::RemainingTime { our_color, wtime, btime, winc: _, binc: _, movestogo: _ } => {
                let duration = our_color.chooser_wb(wtime, btime);
                write!(f, "RemainingTime({})", DurationNewType(*duration))?;
            }
        }
        Ok(())
    }
}



impl FromStr for TimeControl {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        TimeControl::parse(s)
    }
}



impl Default for TimeControl {
    fn default() -> Self {
        TimeControl::MoveTime(Duration::from_secs(5))
    }
}

impl TimeControl {

    pub fn parse(tc: &str) -> Result<Self, String> {
        if tc == "inf" {
            Ok(TimeControl::Infinite)
        } else if tc.ends_with("epd") {
            Ok(TimeControl::EpdDepth)
        } else if tc.ends_with("ms") {
            let ms = tc.parse::<u64>().map_err(|e| e.to_string())?;
            Ok(TimeControl::MoveTime(Duration::from_millis(ms)))
        } else if tc.ends_with("s") {
            let secs = tc.parse::<u64>().map_err(|e| e.to_string())?;
            Ok(TimeControl::MoveTime(Duration::from_secs(secs)))
        } else if tc.ends_with("mate") {
            let depth = tc.parse::<u32>().map_err(|e| e.to_string())?;
            Ok(TimeControl::MateIn(depth))
        } else if tc.contains("+") {
            let wtime = Duration::from_secs(0);
            let btime = Duration::from_secs(0); 
            let winc = Duration::from_secs(0); 
            let binc = Duration::from_secs(0); 
            Ok(TimeControl::RemainingTime{ our_color:Color::White, wtime, btime, winc, binc, movestogo:0})
        } else if tc.ends_with("d") {
            let depth = tc.parse::<i32>().map_err(|e| e.to_string())?;
            Ok(TimeControl::Depth(depth))
        } else if tc.ends_with("n") {
            let nodes = tc.parse::<u64>().map_err(|e| e.to_string())?;
            Ok(TimeControl::NodeCount(nodes))
        } else {
            Err(format!("Unable to parse time control {}", tc))
        }
    }


    pub fn from_remaining_time(d: Duration) -> Self {
        let zero = Duration::default();
        TimeControl::RemainingTime {
            our_color: Color::White,
            wtime: d,
            btime: d,
            winc: zero,
            binc: zero,
            movestogo: 0,
        }
    }

    pub fn from_move_time_millis(ms: u64) -> Self {
        let d = Duration::from_millis(ms);
        TimeControl::MoveTime(d)
    }

    pub fn from_game_time_secs(s: u64) -> Self {
        let zero = Duration::default();
        let d = Duration::from_secs(s);
        TimeControl::RemainingTime {
            our_color: Color::White,
            wtime: d,
            btime: d,
            winc: zero,
            binc: zero,
            movestogo: 0,
        }
    }
}




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_control() -> Result<(), String> {
        assert_eq!(TimeControl::parse("3d".into())?, TimeControl::Depth(3));
        Ok(())
    }
}
