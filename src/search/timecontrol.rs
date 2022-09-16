use serde::{Serialize, Deserialize};

use crate::piece::{Color, Ply};
use crate::utils::Formatting;
use std::fmt;
use std::str::FromStr;
use std::time::Duration;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct RemainingTime {
    pub our_color: Color,
    pub wtime: Duration,
    pub btime: Duration,
    pub winc: Duration,
    pub binc: Duration,
    pub moves_to_go: u16,
}

impl RemainingTime {
    pub fn our_time_and_inc(&self) -> (Duration, Duration) {
        self.our_color
            .chooser_wb((self.wtime, self.winc), (self.btime, self.binc))
    }

    pub fn their_time_and_inc(&self) -> (Duration, Duration) {
        self.our_color
            .opposite()
            .chooser_wb((self.wtime, self.winc), (self.btime, self.binc))
    }
}

/// https://en.wikipedia.org/wiki/Time_control
///
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize)]
pub enum TimeControl {
    DefaultTime,          // depth "recommended" by EPD position or otherwise
    Depth(Ply),           // uci "depth"
    SearchTime(Duration), // uci "movetime"
    NodeCount(u64),       // uci "nodes"
    Infinite,             // uci "infinite"
    MateIn(u32),          // uci "mate"
    Fischer(RemainingTime),
}

impl fmt::Display for TimeControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TimeControl::DefaultTime => write!(f, "Default")?,
            TimeControl::Depth(max_ply) => write!(f, "Depth({})", max_ply)?,
            TimeControl::SearchTime(duration) => {
                write!(f, "SearchTime({})", Formatting::duration(*duration))?
            }
            TimeControl::NodeCount(max_nodes) => {
                write!(f, "NodeCount({})", Formatting::u128(*max_nodes as u128))?
            }
            TimeControl::Infinite => write!(f, "Infinite")?,
            TimeControl::MateIn(depth) => write!(f, "MateIn({})", depth)?,
            TimeControl::Fischer(rt) => {
                let duration = rt.our_time_and_inc().0;
                write!(f, "RemainingTime({})", Formatting::duration(duration))?;
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
        TimeControl::SearchTime(Duration::from_secs(5))
    }
}

impl TimeControl {
    pub fn parse(tc: &str) -> Result<Self, String> {
        if tc == "inf" {
            Ok(TimeControl::Infinite)
        } else if tc.ends_with("def") {
            Ok(TimeControl::DefaultTime)
        } else if let Some(tc) = tc.strip_prefix("st=") {
            let secs = tc.parse::<f64>().map_err(|e| e.to_string())?;
            Ok(TimeControl::SearchTime(Duration::from_secs_f64(secs)))
        } else if let Some(tc) = tc.strip_prefix("mate=") {
            let depth = tc.parse::<u32>().map_err(|e| e.to_string())?;
            Ok(TimeControl::MateIn(depth))
        } else if let Some(tc) = tc.strip_prefix("tc=") {
            let time = tc.parse::<u64>().map_err(|e| e.to_string())?;
            let rt = RemainingTime {
                wtime: Duration::from_secs(time),
                btime: Duration::from_secs(time),
                ..RemainingTime::default()
            };
            Ok(TimeControl::Fischer(rt))
        } else if let Some(tc) = tc.strip_prefix("depth=") {
            let depth = tc.parse::<i32>().map_err(|e| e.to_string())?;
            Ok(TimeControl::Depth(depth))
        } else if let Some(tc) = tc.strip_prefix("nodes=") {
            let nodes = tc.parse::<u64>().map_err(|e| e.to_string())?;
            Ok(TimeControl::NodeCount(nodes))
        } else {
            Err(format!("Unable to parse time control {}", tc))
        }
    }

    pub fn from_remaining_time(d: Duration) -> Self {
        TimeControl::Fischer(RemainingTime {
            wtime: d,
            btime: d,
            ..Default::default()
        })
    }

    pub fn from_move_time_millis(ms: u64) -> Self {
        let d = Duration::from_millis(ms);
        TimeControl::SearchTime(d)
    }

    pub fn from_game_time_secs(s: u64) -> Self {
        Self::from_remaining_time(Duration::from_secs(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_control() -> Result<(), String> {
        assert_eq!(TimeControl::parse("depth=3".into())?, TimeControl::Depth(3));
        println!("{}", TimeControl::parse("depth=3".into())?);

        // let tc = TimeControl::parse("nodes=1000".into())?;
        // assert!(toml::to_string_pretty(&tc).unwrap().len() > 0);

        assert_eq!(TimeControl::parse("def".into())?, TimeControl::DefaultTime);
        println!("{}", TimeControl::parse("def".into())?);

        assert_eq!(TimeControl::parse("inf".into())?, TimeControl::Infinite);
        println!("{}", TimeControl::parse("inf".into())?);

        assert_eq!(
            TimeControl::parse("nodes=1000".into())?,
            TimeControl::NodeCount(1000)
        );
        println!("{}", TimeControl::parse("nodes=1000".into())?);

        assert_eq!(
            TimeControl::parse("st=10.980".into())?,
            TimeControl::SearchTime(Duration::from_millis(10980))
        );
        println!("{}", TimeControl::parse("st=10.980".into())?);

        assert_eq!(TimeControl::parse("mate=3".into())?, TimeControl::MateIn(3));
        println!("{}", TimeControl::parse("mate=3".into())?);
        Ok(())
    }
}
