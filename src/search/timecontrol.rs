use anyhow::Context;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::infra::utils::{KeywordIter, Uci};
use crate::piece::{Color, Ply};
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
#[derive(Copy, Clone, Debug, PartialEq, SerializeDisplay, DeserializeFromStr)]
pub enum TimeControl {
    DefaultTime,          // depth "recommended" by EPD position or otherwise
    Depth(Ply),           // uci "depth"
    SearchTime(Duration), // uci "movetime"
    NodeCount(u64),       // uci "nodes"
    Cycles(u64),
    Instructions(u64),
    Infinite,    // uci "infinite"
    MateIn(u32), // uci "mate"
    UciFischer(RemainingTime),
    FischerMulti { moves: i32, secs: f32, inc: f32 },
}

impl fmt::Display for TimeControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.fmt_option(f)
        // match self {
        //     TimeControl::DefaultTime => write!(f, "Default")?,
        //     TimeControl::Depth(max_ply) => write!(f, "Depth({})", max_ply)?,
        //     TimeControl::SearchTime(duration) => {
        //         write!(f, "SearchTime({})", Formatting::duration(*duration))?
        //     }
        //     TimeControl::NodeCount(max_nodes) => {
        //         write!(f, "NodeCount({})", Formatting::u128(*max_nodes as u128))?
        //     }
        //     TimeControl::Infinite => write!(f, "Infinite")?,
        //     TimeControl::MateIn(depth) => write!(f, "MateIn({})", depth)?,
        //     TimeControl::UciFischer(rt) => {
        //         let duration = rt.our_time_and_inc().0;
        //         write!(f, "RemainingTime({})", Formatting::duration(duration))?;
        //     }
        //     TimeControl::FischerMulti { moves, secs, inc } => {
        //         write!(f, "Fischer({moves}",)?;
        //         if secs > &0. {
        //             write!(f, "/{s}", s = secs)?;
        //         }
        //         if inc > &0. {
        //             write!(f, "+{i}", i = inc)?;
        //         }
        //     }
        // }
        // Ok(())
    }
}

impl FromStr for TimeControl {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        TimeControl::parse_option(s)
    }
}

impl Default for TimeControl {
    fn default() -> Self {
        TimeControl::SearchTime(Duration::from_secs(5))
    }
}

impl Uci for TimeControl {
    fn fmt_uci(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use TimeControl::*;
        match self {
            DefaultTime => write!(f, "Default")?,
            Depth(ply) => write!(f, "depth {ply}")?,
            SearchTime(dur) => write!(f, "movetime {ms}", ms = dur.as_millis())?,
            NodeCount(nodes) => write!(f, "nodes {nodes}")?,
            Cycles(n) => write!(f, "cycles {n}")?,
            Instructions(n) => write!(f, "ins {n}")?,
            Infinite => write!(f, "infinite")?,
            MateIn(depth) => write!(f, "mate {depth}")?,
            UciFischer(RemainingTime {
                wtime,
                btime,
                winc,
                binc,
                moves_to_go,
                our_color: _,
            }) => {
                write!(
                    f,
                    "wtime {wt} btime {bt} ",
                    wt = wtime.as_millis(),
                    bt = btime.as_millis()
                )?;
                if !winc.is_zero() {
                    write!(f, "winc {wi} ", wi = winc.as_millis())?;
                }
                if !binc.is_zero() {
                    write!(f, "binc {bi} ", bi = binc.as_millis())?;
                }
                if *moves_to_go > 0 {
                    write!(f, "movestogo {moves_to_go} ")?;
                }
            }
            FischerMulti { .. } => panic!("Unable to format FischerMulti as a uci time control"),
        }
        Ok(())
    }

    fn parse_uci(s: &str) -> anyhow::Result<Self> {
        let words = [
            "depth",
            "wtime",
            "btime",
            "winc",
            "binc",
            "movestogo",
            "nodes",
            "cycles",
            "ins",
            "mate",
            "movetime",
            "infinite",
        ];

        let mut rt = RemainingTime::default();
        let mut fischer = false;
        let iter = KeywordIter::new(&words, None, &s);
        for (key, value) in iter {
            if ["wtime", "btime"].contains(&key.as_str()) {
                fischer = true;
            }
            let v = value.trim().to_string();
            match (key.as_str(), v) {
                ("depth", t) => return Ok(TimeControl::Depth(t.parse().context(t)?)),
                ("nodes", t) => return Ok(TimeControl::NodeCount(t.parse().context(t)?)),
                ("ins", t) => return Ok(TimeControl::Instructions(t.parse().context(t)?)),
                ("cycles", t) => return Ok(TimeControl::Cycles(t.parse().context(t)?)),
                ("mate", t) => return Ok(TimeControl::MateIn(t.parse().context(t)?)),
                ("movetime", t) => {
                    return Ok(TimeControl::from_move_time_millis(t.parse().context(t)?))
                }
                ("infinite", _) => return Ok(TimeControl::Infinite),
                ("wtime", t) => rt.wtime = Duration::from_millis(t.parse().context(t)?),
                ("btime", t) => rt.btime = Duration::from_millis(t.parse().context(t)?),
                ("winc", t) => rt.winc = Duration::from_millis(t.parse().context(t)?),
                ("binc", t) => rt.binc = Duration::from_millis(t.parse().context(t)?),
                ("movestogo", t) => rt.moves_to_go = t.parse().context(t)?,
                (k, _) => anyhow::bail!("Unknown item {k} in time control '{s}'"),
            }
        }
        if fischer {
            return Ok(TimeControl::UciFischer(rt));
        }
        Ok(TimeControl::default())
    }
}

impl TimeControl {
    pub fn fmt_option(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TimeControl::Infinite => write!(f, "inf")?,
            TimeControl::DefaultTime => write!(f, "def")?,
            TimeControl::SearchTime(dur) => write!(f, "st={d}", d = dur.as_secs_f64())?,
            TimeControl::MateIn(depth) => write!(f, "mate={depth}")?,
            TimeControl::Depth(d) => write!(f, "depth={d}")?,
            TimeControl::NodeCount(nodes) => write!(f, "nodes={nodes}")?,
            TimeControl::Cycles(n) => write!(f, "cycles={n}")?,
            TimeControl::Instructions(n) => write!(f, "ins={n}")?,
            TimeControl::UciFischer(rt) => {
                // let duration = rt.our_time_and_inc().0;
                write!(f, "tc=(rt={rt:?})")?;
            }
            TimeControl::FischerMulti { moves, secs, inc } => {
                write!(f, "{moves}",)?;
                if secs > &0. {
                    write!(f, "/{s}", s = secs)?;
                }
                if inc > &0. {
                    write!(f, "+{i}", i = inc)?;
                }
            }
        };
        Ok(())
    }

    pub fn parse_pgn(s: &str) -> anyhow::Result<TimeControl> {
        let tc = s.split(":").take(1).collect_vec().join("");
        let moves;
        let mut inc = 0.;
        let secs;
        if tc.contains('/') && !tc.contains('+') {
            match tc.split_once('/') {
                Some((s, _)) if tc.ends_with("/move") => {
                    secs = s
                        .parse::<f32>()
                        .context(format!("parsing secs in {s} from tc '{tc}'"))?;
                    return Ok(TimeControl::SearchTime(Duration::from_secs_f32(secs)));
                }
                Some((m, s)) => {
                    moves = m.parse::<i32>().context(format!("{m} in tc '{tc}'"))?;
                    secs = s.parse::<f32>().context(format!("{s} in tc '{tc}'"))?;
                    return Ok(TimeControl::FischerMulti { moves, secs, inc });
                }
                _ => anyhow::bail!("failed to parse time control as moves/secs"),
            }
        }
        if !tc.contains('/') && tc.contains('+') {
            match tc.split_once('+') {
                Some((s, i)) => {
                    secs = s.parse::<f32>().context(s.to_string())?;
                    inc = i.parse::<f32>().context(i.to_string())?;
                    return Ok(TimeControl::FischerMulti { moves: 0, secs, inc });
                }
                _ => anyhow::bail!("failed to parse time control '{s}' as moves+inc"),
            }
        }
        anyhow::bail!("expected time control format moves/secs or moves+inc but found '{s}'");
    }

    pub fn max_move_time(&self) -> Option<Duration> {
        match self {
            Self::SearchTime(d) => Some(*d),
            Self::UciFischer(rt) => Some(rt.our_time_and_inc().0),
            _ => None,
        }
    }

    fn parse_without_context(tc: &str) -> anyhow::Result<TimeControl> {
        let tc = tc.replace("_", "");
        if tc == "inf" {
            Ok(TimeControl::Infinite)
        } else if tc.ends_with("def") {
            Ok(TimeControl::DefaultTime)
        } else if let Some(tc) = tc.strip_prefix("st=") {
            if let Some(tc) = tc.strip_suffix("ms") {
                let ms = tc.parse::<u64>()?;
                Ok(TimeControl::SearchTime(Duration::from_millis(ms)))
            } else {
                let secs = tc.trim_end_matches("s").parse::<f64>()?;
                Ok(TimeControl::SearchTime(Duration::from_secs_f64(secs)))
            }
        } else if let Some(tc) = tc.strip_prefix("mate=") {
            let depth = tc.parse::<u32>()?;
            Ok(TimeControl::MateIn(depth))
        } else if let Some(tc) = tc.strip_prefix("tc=") {
            Self::parse_pgn(tc)
        } else if let Some(tc) = tc.strip_prefix("depth=") {
            let depth = tc.parse::<i32>()?;
            Ok(TimeControl::Depth(depth))
        } else if let Some(tc) = tc.strip_prefix("nodes=") {
            let nodes = tc.parse::<u64>()?;
            Ok(TimeControl::NodeCount(nodes))
        } else if let Some(tc) = tc.strip_prefix("cycles=") {
            let n = tc.parse::<u64>()?;
            Ok(TimeControl::Cycles(n))
        } else if let Some(tc) = tc.strip_prefix("ins=") {
            let n = tc.parse::<u64>()?;
            Ok(TimeControl::Instructions(n))
        } else {
            anyhow::bail!("Unable to parse time control {}", tc);
        }
    }

    pub fn parse_option(tc: &str) -> anyhow::Result<Self> {
        Self::parse_without_context(tc).with_context(|| format!("parsing time control '{tc}'"))
    }

    pub fn from_remaining_time(d: Duration) -> Self {
        TimeControl::UciFischer(RemainingTime {
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
    fn test_time_control() -> anyhow::Result<()> {
        type T = TimeControl;

        assert_eq!(T::parse_option("depth=3")?, T::Depth(3));

        // let tc = TC::parse("nodes=1000")?;
        // assert!(toml::to_string_pretty(&tc).unwrap().len() > 0);

        assert_eq!(T::parse_option("def")?, T::DefaultTime);
        assert_eq!(T::parse_option("inf")?, T::Infinite);
        assert_eq!(T::parse_option("nodes=1000")?, T::NodeCount(1000));
        assert_eq!(T::parse_option("cycles=10")?, T::Cycles(10));
        assert_eq!(T::parse_option("ins=100")?, T::Instructions(100));
        assert_eq!(T::parse_option("nodes=10_000")?, T::NodeCount(10_000));
        assert_eq!(
            T::parse_option("st=10s")?,
            T::SearchTime(Duration::from_secs(10))
        );
        assert_eq!(
            T::parse_option("st=11ms")?,
            T::SearchTime(Duration::from_millis(11))
        );

        assert_eq!(
            T::parse_option("st=10.980")?,
            T::SearchTime(Duration::from_millis(10980))
        );

        assert_eq!(T::parse_option("mate=3")?, T::MateIn(3));
        println!("{}", T::parse_option("mate=3")?);

        assert_eq!(
            T::parse_option("tc=5/60")?,
            T::FischerMulti {
                moves: 5,
                secs: 60.,
                inc: 0.
            }
        );

        assert_eq!(
            T::parse_option("tc=5+.1")?,
            T::FischerMulti {
                moves: 0,
                secs: 5.,
                inc: 0.1
            }
        );

        assert_eq!(
            T::parse_option("tc=40/960:40/960:40/960")?,
            T::FischerMulti {
                moves: 40,
                secs: 960.,
                inc: 0.
            }
        );

        Ok(())
    }
}
