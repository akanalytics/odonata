use std::fmt;
use std::str::FromStr;
use std::time::Duration;

use anyhow::Context;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::infra::utils::{KeywordIter, Uci};
use crate::piece::{Color, FlipSide, Ply};

// from cutechess we see examples such as
// go wtime 75961 btime 75600 winc 600 binc 600 depth 8 (depth=8; tc = "75+0.6")
//
// tc=40/0:02
// go wtime 355 btime 353 movestogo 2
// go wtime 303 btime 294 movestogo 1
// go wtime 2240 btime 2218 movestogo 40
//
// 40/0:02+2
// go wtime 10878 btime 10185 winc 1000 binc 1000 movestogo 7
// go wtime 12878 btime 12185 winc 1000 binc 1000 movestogo 6
//
// SD/2:00
// go wtime 2000 btime 2000
// go wtime 1932 btime 1945

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct RemainingTime {
    pub our_color:   Color,
    pub wtime:       Duration,
    pub btime:       Duration,
    pub winc:        Duration,
    pub binc:        Duration,
    pub moves_to_go: u16,
}

impl RemainingTime {
    pub fn our_time_and_inc(&self) -> (Duration, Duration) {
        self.our_color
            .chooser_wb((self.wtime, self.winc), (self.btime, self.binc))
    }

    pub fn their_time_and_inc(&self) -> (Duration, Duration) {
        self.our_color
            .flip_side()
            .chooser_wb((self.wtime, self.winc), (self.btime, self.binc))
    }
}

/// https://en.wikipedia.org/wiki/Time_control
#[derive(Clone, Debug, PartialEq, SerializeDisplay, DeserializeFromStr)]
pub enum TimeControl {
    DefaultTime,          // depth "recommended" by EPD position or otherwise
    Depth(Ply),           // uci "depth"
    SearchTime(Duration), // uci "movetime"
    NodeCount(u64),       // uci "nodes"
    Cycles(u64),
    DepthNodeCount(Ply, u64),
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
        TimeControl::parse_cli_option(s)
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
            &DepthNodeCount(depth, nodes) => write!(f, "depth {depth} nodes {nodes}")?,
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
        fn _parse_uci(s: &str) -> anyhow::Result<TimeControl> {
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
            let mut depth = None;
            let mut nodes = None;

            let iter = KeywordIter::new(&words, None, s);
            for (key, value) in iter {
                if ["wtime", "btime"].contains(&key.as_str()) {
                    fischer = true;
                }
                let v = value.trim().to_string();
                match (key.as_str(), v) {
                    ("depth", t) => depth = Some(t.parse::<Ply>().context(t)?),
                    ("nodes", t) => nodes = Some(t.parse::<u64>().context(t)?),
                    ("ins", t) => return Ok(TimeControl::Instructions(t.parse().context(t)?)),
                    ("cycles", t) => return Ok(TimeControl::Cycles(t.parse().context(t)?)),
                    ("mate", t) => return Ok(TimeControl::MateIn(t.parse().context(t)?)),
                    ("movetime", t) => return Ok(TimeControl::from_move_time_millis(t.parse().context(t)?)),
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
            match (depth, nodes) {
                (Some(d), Some(n)) => Ok(TimeControl::DepthNodeCount(d, n)),
                (Some(d), None) => Ok(TimeControl::Depth(d)),
                (None, Some(n)) => Ok(TimeControl::NodeCount(n)),
                _ => Ok(TimeControl::default()),
            }
        }
        _parse_uci(s).with_context(|| format!("parsing time control from uci '{s}'"))
    }
}

impl TimeControl {
    pub fn fmt_option(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use TimeControl::*;
        match self {
            Infinite => write!(f, "inf")?,
            DefaultTime => write!(f, "def")?,
            SearchTime(dur) => write!(f, "st={d:.3}", d = dur.as_secs_f64())?,
            MateIn(depth) => write!(f, "mate={depth}")?,
            Depth(d) => write!(f, "depth={d}")?,
            DepthNodeCount(d, n) => write!(f, "depth/nodes={d}/{n}")?,
            NodeCount(nodes) => write!(f, "nodes={nodes}")?,
            Cycles(n) => write!(f, "cycles={n}")?,
            Instructions(n) => write!(f, "ins={n}")?,
            UciFischer(rt) => {
                // let duration = rt.our_time_and_inc().0;
                write!(f, "tc=(rt={rt:?})")?;
            }
            &FischerMulti { moves, secs, inc } => {
                if moves > 0 {
                    write!(f, "{moves}/",)?;
                }
                if secs > 0. {
                    if secs.fract() == 0.0 {
                        write!(f, "{secs:.0}")?;
                    } else {
                        write!(f, "{secs:.3}")?;
                    }
                }
                if inc > 0. {
                    if inc.fract() == 0.0 {
                        write!(f, "+{inc:.0}")?;
                    } else {
                        write!(f, "+{inc:.3}")?;
                    }
                }
            }
        };
        Ok(())
    }

    /// For some time controls we aren't worried about node counts or search times, so we
    /// can avoid optimizations elsewhere
    pub fn is_time_sensitive(&self) -> bool {
        use TimeControl::*;
        match self {
            DefaultTime => true,
            SearchTime(_duration) => true,
            UciFischer { .. } => true,
            Depth(_max_ply) => false,
            DepthNodeCount(..) => false,
            NodeCount(_max_nodes) => false,
            Instructions(_) => true,
            Cycles(_) => true,
            Infinite => false,
            MateIn(_) => false,
            FischerMulti { .. } => panic!("FischerMulti"),
        }
    }

    pub fn mul_f32(&self, scaling: f32) -> TimeControl {
        use TimeControl::*;

        match &self {
            DefaultTime => self.clone(),
            Depth(..) => self.clone(),
            // SearchTime(dur) => SearchTime(Duration::from_secs_f32(dur.as_secs_f32() * scaling)),
            SearchTime(dur) => SearchTime(dur.mul_f32(scaling)),
            NodeCount(..) => self.clone(),
            DepthNodeCount(..) => self.clone(),
            Cycles(n) => Cycles((*n as f32 * scaling) as u64),
            Instructions(n) => Instructions((*n as f32 * scaling) as u64),
            Infinite => Infinite,
            MateIn(..) => self.clone(),
            UciFischer(RemainingTime {
                wtime,
                btime,
                winc,
                binc,
                moves_to_go,
                our_color: color,
            }) => UciFischer(RemainingTime {
                wtime:       wtime.mul_f32(scaling),
                btime:       btime.mul_f32(scaling),
                winc:        winc.mul_f32(scaling),
                binc:        binc.mul_f32(scaling),
                moves_to_go: *moves_to_go,
                our_color:   *color,
            }),
            FischerMulti { moves, secs, inc } => FischerMulti {
                moves: *moves,
                secs:  secs * scaling,
                inc:   inc * scaling,
            },
        }
    }

    pub fn parse_pgn(s: &str) -> anyhow::Result<TimeControl> {
        let tc = s.split(':').take(1).collect_vec().join("");
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

    pub fn depth(&self) -> Option<Ply> {
        match self {
            Self::Depth(d) => Some(*d),
            Self::DepthNodeCount(d, _) => Some(*d),
            _ => None,
        }
    }

    fn parse_cli_option(tc: &str) -> anyhow::Result<TimeControl> {
        Self::parse_cli_option_without_context(tc).with_context(|| format!("parsing time control '{tc}'"))
    }

    fn parse_cli_option_without_context(tc: &str) -> anyhow::Result<TimeControl> {
        let tc = tc.replace('_', "");
        if tc == "inf" {
            return Ok(TimeControl::Infinite);
        }
        if tc.ends_with("def") {
            return Ok(TimeControl::DefaultTime);
        }
        if let Some(tc) = tc.strip_prefix("st=") {
            if let Some(tc) = tc.strip_suffix("ms") {
                let ms = tc.parse::<u64>()?;
                return Ok(TimeControl::SearchTime(Duration::from_millis(ms)));
            } else {
                let secs = tc.trim_end_matches('s').parse::<f64>()?;
                return Ok(TimeControl::SearchTime(Duration::from_secs_f64(secs)));
            }
        }
        if let Some(tc) = tc.strip_prefix("mate=") {
            let depth = tc.parse::<u32>()?;
            return Ok(TimeControl::MateIn(depth));
        }
        if let Some(tc) = tc.strip_prefix("tc=") {
            return Self::parse_pgn(tc);
        }
        if let Some(tc) = tc.strip_prefix("depth=") {
            let depth = tc.parse::<i32>()?;
            return Ok(TimeControl::Depth(depth));
        }
        if let Some(tc) = tc.strip_prefix("depth/nodes=") {
            if let Some((d, n)) = tc.split_once('/') {
                let (d, n) = (d.parse()?, n.parse()?);
                return Ok(TimeControl::DepthNodeCount(d, n));
            }
        }
        if let Some(tc) = tc.strip_prefix("nodes=") {
            let nodes = tc.parse::<u64>()?;
            return Ok(TimeControl::NodeCount(nodes));
        }
        if let Some(tc) = tc.strip_prefix("cycles=") {
            let n = tc.parse::<u64>()?;
            return Ok(TimeControl::Cycles(n));
        }
        if let Some(tc) = tc.strip_prefix("ins=") {
            let n = tc.parse::<u64>()?;
            return Ok(TimeControl::Instructions(n));
        }
        anyhow::bail!("Unable to parse time control '{}'", tc);
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

        assert_eq!(T::parse_cli_option("depth=3")?, T::Depth(3));

        // let tc = TC::parse("nodes=1000")?;
        // assert!(toml::to_string_pretty(&tc).unwrap().len() > 0);

        assert_eq!(T::parse_cli_option("def")?, T::DefaultTime);
        assert_eq!(T::parse_cli_option("inf")?, T::Infinite);
        assert_eq!(T::parse_cli_option("nodes=1000")?, T::NodeCount(1000));
        assert_eq!(T::parse_cli_option("depth=5")?, T::Depth(5));
        assert_eq!(T::parse_cli_option("cycles=10")?, T::Cycles(10));
        assert_eq!(T::parse_cli_option("ins=100")?, T::Instructions(100));
        assert_eq!(T::parse_cli_option("nodes=10_000")?, T::NodeCount(10_000));
        assert_eq!(
            T::parse_cli_option("depth/nodes=5/10_000")?,
            T::DepthNodeCount(5, 10_000)
        );
        assert_eq!(T::parse_cli_option("st=10s")?, T::SearchTime(Duration::from_secs(10)));
        assert_eq!(
            T::parse_cli_option("st=11ms")?,
            T::SearchTime(Duration::from_millis(11))
        );

        assert_eq!(
            T::parse_cli_option("st=10.980")?,
            T::SearchTime(Duration::from_millis(10980))
        );

        assert_eq!(T::parse_cli_option("mate=3")?, T::MateIn(3));
        println!("{}", T::parse_cli_option("mate=3")?);

        assert_eq!(T::parse_cli_option("tc=5/60")?, T::FischerMulti {
            moves: 5,
            secs:  60.,
            inc:   0.,
        });
        assert_eq!(T::parse_cli_option("tc=5/60")?.to_string(), "5/60");
        assert_eq!(T::parse_cli_option("tc=5/60.5")?.to_string(), "5/60.500");

        assert_eq!(T::parse_cli_option("tc=5+.1")?, T::FischerMulti {
            moves: 0,
            secs:  5.,
            inc:   0.1,
        });

        assert_eq!(T::parse_cli_option("tc=40/960:40/960:40/960")?, T::FischerMulti {
            moves: 40,
            secs:  960.,
            inc:   0.,
        });

        assert_eq!(T::parse_uci("nodes 10 depth 5")?, T::DepthNodeCount(5, 10));

        Ok(())
    }
}
