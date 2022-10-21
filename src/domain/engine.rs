use std::fmt::{Debug, Display};

use crate::{search::timecontrol::TimeControl, Position};

use super::SearchResults;

pub trait Engine: Display + Debug {
    // const OPTION_MULTIPV: &'static str = "MultiPV";
    // const OPTION_HASH: &'static str = "Hash";

    fn start_game(&mut self)-> anyhow::Result<()>;
    fn search(&mut self, pos: Position, tc: TimeControl) -> anyhow::Result<SearchResults>;
    fn set_option(&mut self, name: &str, value: &str) -> anyhow::Result<()>;

    fn set_multi_pv(&mut self, num: i32) -> anyhow::Result<()> {
        self.set_option("MultiPV", &num.to_string())
    }

    fn set_hash(&mut self, mb: i32) -> anyhow::Result<()> {
        self.set_option("Hash", &mb.to_string())
    }
}


pub struct EnginePool {
    engines: Vec<Box<dyn Engine>>,
    
}

