use anyhow::Result;
use indexmap::map::IndexMap;
use std::fmt::{Debug, Display};

use crate::search::search_results::SearchResults;
use odonata_base::{
    domain::{score::Score, timecontrol::TimeControl},
    infra::value::Stats,
    prelude::SearchOptions,
    Epd,
};

pub trait Engine: Display + Debug {
    // const OPTION_MULTIPV: &'static str = "MultiPV";
    // const OPTION_HASH: &'static str = "Hash";

    fn name(&self) -> String;

    fn set_name(&mut self, name: String);

    fn start_game(&mut self) -> Result<()>;

    fn search(&mut self, pos: Epd, tc: TimeControl) -> Result<SearchResults> {
        self.search_with_options(pos, tc, SearchOptions::none())
    }

    fn has_feature(&self, feature: &str) -> bool {
        if let Some(features) = self.options().get("Features") {
            let features = features
                .trim_start()
                .trim_start_matches("string")
                .trim_start()
                .trim_start_matches("default")
                .trim();
            features
                .replace(['[', ']'], ",")
                .split(',')
                .any(|w| w == feature)
        } else {
            false
        }
    }

    fn search_with_options(
        &mut self,
        pos: Epd,
        tc: TimeControl,
        options: SearchOptions,
    ) -> Result<SearchResults>;

    fn static_eval(&mut self, pos: Epd) -> Result<Score>;

    // fn general_command(&mut self, str: &str, wait_for: &str) -> Result<String>;

    fn options(&self) -> IndexMap<String, String>;

    fn set_option(&mut self, name: &str, value: &str) -> Result<()>;

    fn set_multi_pv(&mut self, num: i32) -> Result<()> {
        self.set_option("MultiPV", &num.to_string())
    }

    fn set_hash(&mut self, mb: i32) -> Result<()> {
        self.set_option("Hash", &mb.to_string())
    }

    fn qsearch(&mut self, pos: Epd) -> anyhow::Result<SearchResults> {
        if self.name().starts_with("Odonata") {
            return self.search_with_options(pos, TimeControl::Depth(0), SearchOptions::none());
        }

        // do a depth=1 search on a 1-shorter pv, restricted by pv's final move
        if !pos.played().is_empty() {
            let var = pos.played().clone();
            if let Some(last) = var.last() {
                let var = var.take(var.len() - 1);
                let tc = TimeControl::Depth(1);
                let pos = Epd::from_var(pos.setup_board(), var); // a variation one short
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
    fn metrics(&mut self, filter: &str) -> Result<Stats>;
    // fn evaluation(pos) -> Results<Evaluation>
}
