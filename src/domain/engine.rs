use anyhow::Result;
use indexmap::IndexMap;
use std::fmt::{Debug, Display};

use crate::{search::timecontrol::TimeControl, Position};

use super::SearchResults;

pub trait Engine: Display + Debug {
    // const OPTION_MULTIPV: &'static str = "MultiPV";
    // const OPTION_HASH: &'static str = "Hash";

    fn name(&self) -> String;

    fn set_name(&mut self, name: String);

    fn start_game(&mut self) -> Result<()>;

    fn search(&mut self, pos: Position, tc: TimeControl) -> Result<SearchResults>;

    fn options(&self) -> IndexMap<String, String>;

    fn set_option(&mut self, name: &str, value: &str) -> Result<()>;

    fn set_multi_pv(&mut self, num: i32) -> Result<()> {
        self.set_option("MultiPV", &num.to_string())
    }

    fn set_hash(&mut self, mb: i32) -> Result<()> {
        self.set_option("Hash", &mb.to_string())
    }

    // fn last search_tree() -> Results<Tree>
    // fn metrics() -> Results<Metrics>
    // fn evaluation(pos) -> Results<Evaluation>
}
