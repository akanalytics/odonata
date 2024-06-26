use std::fmt;

use super::wdl::ScoreWdl;
use crate::infra::utils::{KeywordIter, Uci};
use crate::prelude::*;

// #[derive(Clone, Default, Debug, PartialEq, Eq, Hash)]
// pub struct BareMoveVariation(pub Vec<BareMove>);

// impl Uci for BareMoveVariation {
//     fn fmt_uci(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "{}", self.0.iter().map(BareMove::to_uci).join(" "))?;
//         Ok(())
//     }

//     fn parse_uci(s: &str) -> anyhow::Result<Self> {
//         let mut variation = BareMoveVariation::default();
//         for word in s.split_whitespace() {
//             variation.0.push(BareMove::parse_uci(word)?)
//         }
//         Ok(variation)
//     }
// }

// impl fmt::Display for BareMoveVariation {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         self.fmt_uci(f)
//     }
// }

// impl BareMoveVariation {
//     pub fn new() -> Self {
//         Self::default()
//     }

//     pub fn first(&self) -> Option<BareMove> {
//         self.0.first().map(BareMove::to_owned)
//     }

//     pub fn second(&self) -> Option<BareMove> {
//         self.0.iter().skip(1).next().map(BareMove::to_owned)
//     }

//     pub fn moves(&self) -> impl Iterator<Item = &BareMove> {
//         self.0.iter()
//     }

//     pub fn push(&mut self, mv: BareMove) {
//         self.0.push(mv);
//     }

//     pub fn to_san(&self, b: &Board) -> String {
//         Variation::from_inner(self, b).to_san(b)
//     }

//     // truncate the variation to length ply
//     // so the result does not include the ply-th move in the variation
//     // if len < ply just return all of the variation
//     pub fn take(&self, ply: usize) -> Self {
//         BareMoveVariation(self.0.iter().take(ply).cloned().collect_vec())
//     }

//     pub fn len(&self) -> usize {
//         self.0.len()
//     }
// }

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub enum InfoKind {
    #[default]
    Pv,
    BestMove,
    Refutation,
    NodeCounts,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Info {
    pub kind:                  InfoKind,
    pub depth:                 Option<Ply>,
    pub seldepth:              Option<Ply>,
    pub time_millis:           Option<u64>,
    pub multi_pv:              Option<usize>, // 1 offset, uci uses 1 offset
    pub pv:                    Option<Variation>,
    pub nodes:                 Option<u64>,
    pub nodes_thread:          Option<u64>,
    pub score:                 Option<Score>,
    pub wdl:                   Option<ScoreWdl>,
    pub currmove:              Option<Move>,
    pub currmovenumber_from_1: Option<u32>,
    pub hashfull_per_mille:    Option<u32>,
    pub nps:                   Option<u64>,
    pub tbhits:                Option<u64>,
    pub cpuload_per_mille:     Option<u32>,
    pub string_text:           Option<String>,
    pub refutation:            Option<(Move, Variation)>,
    pub currline:              Option<MoveList>,
}

impl Info {
    pub fn new() -> Self {
        Self::default()
    }

    fn set(&mut self, name: &str, value: &str, b: &Board) -> anyhow::Result<()> {
        trace!("setting info.{name} = _{value}_");
        match name {
            "depth" => self.depth = Some(value.parse()?),
            "seldepth" => self.seldepth = Some(value.parse()?),
            "multipv" => self.multi_pv = Some(value.parse::<usize>()?),
            "currmove" => self.currmove = Some(Move::parse_uci(value, b)?),
            "currmovenumber" => self.currmovenumber_from_1 = Some(value.parse()?),
            "score" => self.score = Some(Score::parse_uci(value)?),
            "wdl" => self.wdl = Some(value.parse()?),
            "nodes" => self.nodes = Some(value.parse()?),
            "nps" => self.nps = Some(value.parse()?),
            "hashfull" => self.hashfull_per_mille = Some(value.parse()?),
            "tbhits" => self.tbhits = Some(value.parse::<u64>()?),
            "cpuload" => self.cpuload_per_mille = Some(value.parse::<u32>()?),
            "time" => self.time_millis = Some(value.parse::<u64>()?),
            "pv" => self.pv = Some(Variation::parse_uci(value, b)?),
            "refutation" => todo!(),
            "string" => self.string_text = Some(value.to_string()),
            _ => panic!("unable to set info field {name} to value {value}"),
        };
        Ok(())
    }
}

impl fmt::Display for Info {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "bm={bm} depth={d} seldepth={sd} ms={ms} nodes={nodes} pv={pv}",
            d = self.depth.unwrap_or_default(),
            sd = self.seldepth.unwrap_or_default(),
            ms = self.time_millis.unwrap_or_default(),
            nodes = self.nodes.unwrap_or_default(),
            bm = self
                .pv
                .as_ref()
                .unwrap_or(&Variation::default())
                .first()
                .unwrap_or(Move::new_null()),
            pv = self.pv.as_ref().unwrap_or(&Variation::default()),
        )?;
        Ok(())
    }
}

// impl fmt::Display for Info {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         writeln!(f, "{:#?}", self)
//     }
// }

impl Info {
    pub fn to_uci(&self) -> String {
        Displayable(|fmt| self.fmt_uci(fmt)).to_string()
    }

    fn fmt_uci(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut v = vec![];
        #[rustfmt::skip] {
        self.depth.iter().for_each(|x| v.push(format!("depth {x}")));
        self.seldepth.iter().for_each(|x| v.push(format!("seldepth {x}")));
        self.multi_pv.iter().for_each(|x| v.push(format!("multipv {}", x)));
        self.currmove.iter().for_each(|x| v.push(format!("currmove {}", x.to_uci())));
        self.currmovenumber_from_1.iter().for_each(|x| v.push(format!("currmovenumber {x}")));
        self.score.iter().for_each(|x| v.push(format!("score {}", x.to_uci())));
        self.wdl.iter().for_each(|x| v.push(format!("wdl {} {} {}", x.w, x.d, x.l)));
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

    pub fn parse_uci(s: &str, b: &Board) -> anyhow::Result<Self> {
        let s = s
            .strip_prefix("info")
            .ok_or_else(|| anyhow::format_err!("no leading 'info' in '{s}'"))?
            .trim_start()
            .to_string();

        let words = [
            "depth",
            "seldepth",
            "multipv",
            "currmove",
            "currmovenumber",
            "score",
            "wdl",
            "nodes",
            "nps",
            "hashfull",
            "tbhits",
            "cpuload",
            "time",
            "pv",
            "refutation",
        ];

        let mut info = Info::new();
        let iter = KeywordIter::new(&words, Some("string"), &s);
        for (key, value) in iter {
            info.set(&key, value.trim(), b)
                .with_context(|| format!("setting info '{key}' to '{value}'"))?;
        }
        info.nodes_thread = info.nodes;
        Ok(info)
    }
}

#[cfg(test)]
mod tests {
    use test_log::test;

    use super::*;
    use crate::domain::score::ToScore;

    #[test]
    fn test_basic_variation() {
        let b = &Board::starting_pos();
        let var = "a2a3 h7h6 a3a4".var(b);
        assert_eq!(var.len(), 3);
        assert_eq!(var.first().unwrap().to_uci(), "a2a3");
        assert_eq!(var.second().unwrap().to_uci(), "h7h6");
        assert_eq!(var.take(1).to_uci(), "a2a3");
        assert_eq!(var.take(1).second(), None);
    }

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
        let b = Board::starting_pos();
        let info = Info::parse_uci(
            concat!(
                "info depth 10 seldepth 12 multipv 2 ",
                "score cp 13 nodes 27473 nps 1248772 tbhits 0 ",
                "time 22 ",
                "pv e2e4 c7c5 g1f3 d7d6 ",
                "string Hello World"
            ),
            &b,
        )
        .unwrap();
        assert_eq!(info.depth, Some(10));
        assert_eq!(info.seldepth, Some(12));
        assert_eq!(info.score, Some(13.cp()));
        assert_eq!(info.time_millis, Some(22));
        assert_eq!(info.multi_pv, Some(2));
        assert_eq!(info.string_text, Some("Hello World".into()));
        assert_eq!(info.pv.unwrap().to_uci(), "e2e4 c7c5 g1f3 d7d6");
        assert_eq!(info.cpuload_per_mille, None);

        assert_eq!(
            Info::parse_uci("info depth 5 seldepth 6", &b).unwrap().to_uci(),
            "depth 5 seldepth 6",
        );
    }
}
