use std::cell::Cell;
use std::path::PathBuf;

use odonata_base::infra::component::{Component, State};
use odonata_base::prelude::*;
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};

use super::polyglot::Polyglot;
use crate::search::restrictions::Restrictions;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpeningBook {
    pub own_book:       bool,
    pub book_file:      PathBuf,
    pub best_book_line: bool,

    #[serde(skip)]
    book_exhausted: Cell<bool>,

    #[serde(skip)]
    polyglot: Polyglot,
}

impl Default for OpeningBook {
    fn default() -> Self {
        Self {
            own_book:       false,
            book_file:      PathBuf::new(),
            best_book_line: true,
            book_exhausted: Cell::new(false),
            polyglot:       Polyglot::new(),
        }
    }
}

impl Configurable for OpeningBook {
    fn set(&mut self, p: Param) -> Result<bool> {
        self.own_book.set(p.get("own_book"))?;
        self.book_file.set(p.get("book_file"))?;
        self.best_book_line.set(p.get("best_book_line"))?;
        Ok(p.is_modified())
    }
}

impl Component for OpeningBook {
    fn set_state(&mut self, s: State) {
        use State::*;
        match s {
            NewGame => {
                self.book_exhausted.set(false);
            }
            SetPosition => {}
            StartSearch => {}
            EndSearch => {}
            StartDepthIteration(_) => {}
            Shutdown => {}
        }
    }

    fn new_game(&mut self) {}

    fn new_position(&mut self) {}
}

impl OpeningBook {
    pub fn reload(&mut self) -> anyhow::Result<()> {
        if self.own_book && !self.book_file.as_os_str().is_empty() {
            self.polyglot.load(&self.book_file)?;
        }
        Ok(())
    }

    pub fn lookup(&self, board: &Board, res: &Restrictions) -> Option<Move> {
        if !self.own_book || self.book_file.as_os_str().is_empty() || self.book_exhausted.get() {
            return None;
        }
        let entries = self.polyglot.find_best_matching(board, res).collect_vec();

        if entries.is_empty() {
            if res.is_none() {
                // as long as multi-pv not being applied,
                // we can flag book as exhaused
                self.book_exhausted.set(true);
            }
            return None;
        }

        if self.best_book_line {
            // take the first
            Some(entries.first().unwrap().calc_move(board))
        } else {
            // randomly select one of the best
            let mut rng = thread_rng();
            Some(entries.choose(&mut rng).unwrap().calc_move(board))
        }
    }
}
