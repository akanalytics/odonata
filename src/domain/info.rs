use itertools::Itertools;
use std::fmt;

use crate::{
    eval::score::Score,
    infra::utils::Uci,
    mv::{BareMove, Move},
    piece::Ply,
    variation::Variation,
    MoveList,
};

#[derive(Clone, Default, Debug)]
struct BareMoveVariation(Vec<BareMove>);

impl Uci for BareMoveVariation {
    fn fmt_uci(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.0.iter().map(|mv| mv.to_uci()).collect_vec().join(" ")
        )?;
        Ok(())
    }

    fn parse_uci(s: &str) -> anyhow::Result<Self> {
        let mut variation = BareMoveVariation::default();
        for word in s.split_whitespace() {
            let mv = BareMove::parse_uci(word)?;
            variation.0.push(mv)
        }
        Ok(variation)
    }
}

#[derive(Clone, Default, Debug)]
pub struct Info {
    depth: Option<Ply>,
    seldepth: Option<Ply>,
    time_millis: Option<u64>,
    multi_pv: Option<usize>,
    pv: Option<BareMoveVariation>,
    nodes: Option<u64>,
    nodes_thread: Option<u64>,
    score: Option<Score>,
    currmove: Option<BareMove>,
    currmovenumber_from_1: Option<u32>,
    hashfull_per_mille: Option<u32>,
    nps: Option<u64>,
    tbhits: Option<u64>,
    cpuload_per_mille: Option<u32>,
    string_text: Option<String>,
    refutation: Option<(Move, Variation)>,
    currline: Option<MoveList>,
}

impl Info {
    pub fn new() -> Self {
        Self::default()
    }
    fn set(&mut self, name: &str, value: &str) -> anyhow::Result<()> {
        debug!("setting info.{name} = _{value}_");
        match name {
            "depth" => self.depth = Some(value.parse()?),
            "seldepth" => self.seldepth = Some(value.parse()?),
            "multipv" => self.multi_pv = Some(value.parse::<usize>()? - 1),
            "currmove" => self.currmove = Some(BareMove::parse_uci(value)?),
            "currmovenumber_from_1" => self.currmovenumber_from_1 = Some(value.parse()?),
            "score" => self.score = Some(Score::parse_uci(value)?),
            "nodes" => self.nodes = Some(value.parse()?),
            "nps" => self.nps = Some(value.parse()?),
            "hashfull_per_mille" => self.hashfull_per_mille = Some(value.parse()?),
            "tbhits" => self.tbhits = Some(value.parse::<u64>()?),
            "cpuload_per_mille" => self.cpuload_per_mille = Some(value.parse::<u32>()?),
            "time" => self.time_millis = Some(value.parse::<u64>()?),
            "pv" => self.pv = Some(BareMoveVariation::default()),
            "refutation" => todo!(),
            "text" => self.string_text = Some(value.to_string()),
            _ => panic!("unable to set info field {name} to value {value}"),
        };
        Ok(())
    }
}

impl fmt::Display for Info {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:#?}", self)
    }
}

impl Uci for Info {
    fn fmt_uci(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut v = vec!["info".to_string()];
        #[rustfmt::skip] {
        self.depth.iter().for_each(|x| v.push(format!("depth {x}")));
        self.seldepth.iter().for_each(|x| v.push(format!("seldepth {x}")));
        self.multi_pv.iter().for_each(|x| v.push(format!("multipv {}", x)));
        self.currmove.iter().for_each(|x| v.push(format!("currmove {}", x.to_uci())));
        self.currmovenumber_from_1.iter().for_each(|x| v.push(format!("currmovenumber {x}")));
        self.score.iter().for_each(|x| v.push(format!("score {}", x.to_uci())));
        self.nodes.iter().for_each(|x| v.push(format!("nodes {x}")));
        self.nps.iter().for_each(|x| v.push(format!("nps {x}")));
        self.hashfull_per_mille.iter().for_each(|x| v.push(format!("hashfull {x}")));
        self.tbhits.iter().for_each(|x| v.push(format!("tbhits {x}")));
        self.cpuload_per_mille.iter().for_each(|x| v.push(format!("cpuload {x}")));
        self.time_millis.iter().for_each(|x| v.push(format!("time {x}")));
        self.pv.iter().for_each(|x| v.push(format!("pv {}", x.to_uci())));
        self.refutation.iter().for_each(|x| v.push(format!("refutation {} {}", x.0.to_uci(), x.1.to_uci())));
        self.string_text.iter().for_each(|x| v.push(format!("string {x}")));
        };
        write!(f, "{}", v.join(" "))?;
        Ok(())
    }

    fn parse_uci(s: &str) -> anyhow::Result<Self> {
        let mut info = Info::new();
        let mut s = s
            .strip_prefix("info")
            .ok_or(anyhow::format_err!("no leading 'info' in '{s}'"))?
            .trim_start()
            .to_string();

        if let Some((a, b)) = s.split_once("string") {
            info.string_text = Some(b.trim().to_string());
            s = a.to_string();
        }
        let words = [
            "depth",
            "seldepth",
            "multipv",
            "currmove",
            "currmovenumber",
            "score",
            "nodes",
            "nps",
            "hashfull",
            "tbhits",
            "cpuload",
            "time",
            "pv",
            "refutation",
            "string",
        ];

        'outer: loop {
            for &word in &words {
                match s.strip_prefix(word) {
                    None => continue,
                    Some(following) if word == "pv" => {
                        info.pv = Some(BareMoveVariation::parse_uci(following)?);
                        continue 'outer;
                    }
                    Some(following) => {
                        match following.trim_start().split_once(" ") {
                            Some((first, rest)) => {
                                info.set(word, first)?;
                                s = rest.to_string();
                                continue 'outer;
                            }
                            None => {
                                info.set(word, following.trim_start())?;
                                // s = "".to_string();
                                break 'outer;
                            }
                        }
                    }
                }
            }
            break 'outer;
        }
        Ok(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn test_info_to_uci() -> anyhow::Result<()> {
        let mut info = Info::new();
        info.depth = Some(5);
        info.seldepth = Some(7);
        println!("{info}");
        assert_eq!(&info.to_uci(), "depth 5 seldepth 7");
        Ok(())
    }
    #[test]
    fn test_info_parse_uci() {
        let info = Info::parse_uci(concat!(
            "info depth 10 seldepth 12 multipv 2 ",
            "score cp 12 nodes 27473 nps 1248772 tbhits 0 ",
            "time 22 ",
            "pv e2e4 c7c5 g1f3 d7d6 ",
            "string Hello World"
        ))
        .unwrap();
        assert_eq!(info.depth, Some(10));
        assert_eq!(info.seldepth, Some(12));
        assert_eq!(info.time_millis, Some(22));
        assert_eq!(info.multi_pv, Some(2));
        assert_eq!(info.string_text, Some("Hello World".into()));
        assert_eq!(info.pv.unwrap().to_uci(), "e2e4 c7c5 g1f3 d7d6");
        assert_eq!(info.cpuload_per_mille, None);

        assert_eq!(
            Info::parse_uci("info depth 5 seldepth 6").unwrap().to_uci(),
            "info depth 5 seldepth 6"
        );
    }
}
