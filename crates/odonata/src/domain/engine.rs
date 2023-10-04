use anyhow::Result;
use indexmap::map::IndexMap;
use std::fmt::{Debug, Display};

use crate::{eval::score::Score, search::timecontrol::TimeControl, Position};

use super::{SearchOptions, SearchResults};

pub trait Engine: Display + Debug {
    // const OPTION_MULTIPV: &'static str = "MultiPV";
    // const OPTION_HASH: &'static str = "Hash";

    fn name(&self) -> String;

    fn set_name(&mut self, name: String);

    fn start_game(&mut self) -> Result<()>;

    fn search(&mut self, pos: Position, tc: TimeControl) -> Result<SearchResults> {
        self.search_with_options(pos, tc, SearchOptions::none())
    }

    fn search_with_options(
        &mut self,
        pos: Position,
        tc: TimeControl,
        options: SearchOptions,
    ) -> Result<SearchResults>;

    fn eval(&mut self, pos: Position) -> Result<Score>;

    fn options(&self) -> IndexMap<String, String>;

    fn set_option(&mut self, name: &str, value: &str) -> Result<()>;

    fn set_multi_pv(&mut self, num: i32) -> Result<()> {
        self.set_option("MultiPV", &num.to_string())
    }

    fn set_hash(&mut self, mb: i32) -> Result<()> {
        self.set_option("Hash", &mb.to_string())
    }

    fn qsearch(&mut self, pos: Position) -> anyhow::Result<SearchResults> {
        if self.name().starts_with("Odonata") {
            return self.search_with_options(pos, TimeControl::Depth(0), SearchOptions::none());
        }

        // do a depth=1 search on a 1-shorter pv, restricted by pv's final move
        if let Some(sv) = &pos.tags().sv {
            if let Some(last) = sv.last() {
                let sv = sv.take(sv.len() - 1);
                let tc = TimeControl::Depth(1);
                let mut pos = pos.clone();
                pos.tags_mut().sv = Some(sv);
                let opts = SearchOptions {
                    root_moves: [last].into(),
                };
                let mut sr = self.search_with_options(pos, tc, opts)?;
                sr.multi_pv.iter_mut().for_each(|sv| {
                    sv.var.pop_front();
                });
                return Ok(sr);
            }
        }
        anyhow::bail!("unable to qsearch a position without sv {pos}")
    }

    // fn last search_tree() -> Results<Tree>
    // fn metrics() -> Results<Metrics>
    // fn evaluation(pos) -> Results<Evaluation>
}
