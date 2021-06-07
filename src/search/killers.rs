use crate::board::Board;
use crate::config::{Config, Component};
use crate::{debug, logger::LogInit};
use crate::mv::Move;
use crate::movelist::MoveList;
use crate::types::Ply;
use crate::types::MAX_PLY;
use crate::board::boardbuf::BoardBuf;
use std::fmt;

#[derive(Clone, Debug)]
pub struct Killers {
    enabled: bool,
    killers: Vec<[Move; 2]>,
}

impl Component for Killers {
    fn settings(&self, c: &mut Config) {
        c.set("killers.enabled", "type check default true");
    }
    fn configure(&mut self, c: &Config) {
        debug!("killers.configure with {}", c);
        self.enabled = c.bool("killers.enabled").unwrap_or(self.enabled);
    }
    fn new_game(&mut self) {
        self.new_search();
    }

    fn new_search(&mut self) {
        self.killers.fill([Move::new_null(); 2]);
    }

}

impl Default for Killers {
    fn default() -> Self {
        Killers {
            enabled: true,
            killers: vec![[Move::new_null(); 2]; MAX_PLY as usize],
        }
    }
}

impl fmt::Display for Killers {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        // writeln!(f, "{}", ArrayPlyStat(&[&self.dummy,]))?;
        Ok(())
    }
}

impl Killers {
    pub fn legal_moves_for(&self, y: Ply, b: &Board, moves: &mut MoveList) {
        if !self.enabled {
            return;
        }
        for m in self.killers[y as usize].iter() {
            if !m.is_null() && b.is_pseudo_legal_move(m) && b.is_legal_move(m) {
                debug_assert!(
                    b.validate().is_ok(),
                    "board:{} is not valid",
                    b
                );
                debug_assert!(
                    b.legal_moves().contains(m),
                    "board:{:#} mv: {} {:?} is not in board.legal_moves",
                    b,
                    m, m
                );
                moves.push(*m);
            }
        }
    }

    pub fn store(&mut self, y: Ply, m: &Move) {
        // killers are quiet
        if !self.enabled || m.is_castle() || m.is_capture() {
            return;
        }
        // self.killers[y as usize][3] = self.killers[y as usize][2];
        // self.killers[y as usize][2] = self.killers[y as usize][1];
        self.killers[y as usize][1] = self.killers[y as usize][0];
        self.killers[y as usize][0] = *m;
    }
}
