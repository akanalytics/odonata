use std::fmt::{Display, Debug};

use crate::{Position, search::timecontrol::TimeControl};

use super::SearchResults;

pub trait Engine: Display + Debug {
    fn search(&mut self, pos: Position, tc: TimeControl) -> anyhow::Result<SearchResults>;
    fn set_option(&mut self, name: &str, value: &str) -> anyhow::Result<()>;

    fn set_multi_pv(&mut self, num: i32) ->anyhow::Result<()> {
        self.set_option("MultiPV", &num.to_string())
    }

    fn set_hash(&mut self, mb: i32) ->anyhow::Result<()> {
        self.set_option("Hash", &mb.to_string())
    }
}


