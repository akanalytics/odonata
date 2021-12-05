use crate::board::Board;
use crate::infra::component::Component;
// use crate::{debug, logger::LogInit};
use crate::board::boardbuf::BoardBuf;
use crate::movelist::MoveList;
use crate::mv::Move;
use crate::types::Ply;
use crate::types::MAX_PLY;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Killers {
    enabled: bool,
    clear_every_move: bool,
    use_ply_below: bool,

    #[serde(skip)]
    killers: Vec<[Move; 2]>,
}

impl Default for Killers {
    fn default() -> Self {
        Killers {
            enabled: true,
            clear_every_move: false,
            use_ply_below: true,
            killers: vec![[Move::new_null(); 2]; MAX_PLY as usize],
        }
    }
}

impl Component for Killers {
    fn new_game(&mut self) {
        self.killers.fill([Move::new_null(); 2]);
    }

    fn new_position(&mut self) {
        if self.clear_every_move {
            self.killers.fill([Move::new_null(); 2]);
        } else {
            self.killers.remove(0);
            self.killers.push([Move::new_null(); 2]);
            self.killers.remove(0);
            self.killers.push([Move::new_null(); 2]);
        }
    }
}

impl fmt::Display for Killers {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "clear.every.move : {}", self.clear_every_move)?;
        writeln!(f, "use.ply.below    : {}", self.use_ply_below)?;
        // writeln!(f, "{}", ArrayPlyStat(&[&self.dummy,]))?;
        Ok(())
    }
}

impl Killers {
    pub fn legal_moves_for(&self, y: Ply, b: &Board, moves: &mut MoveList) {
        if !self.enabled {
            return;
        }
        self.legal_moves_for_single_ply(y, b, moves);
        if y > 2 && self.use_ply_below {
            self.legal_moves_for_single_ply(y - 2, b, moves);
        }
    }

    fn legal_moves_for_single_ply(&self, y: Ply, b: &Board, moves: &mut MoveList) {
        for m in self.killers[y as usize].iter() {
            if !m.is_null() && b.is_pseudo_legal_move(m) && b.is_legal_move(m) {
                debug_assert!(b.validate().is_ok(), "board:{} is not valid", b);
                debug_assert!(
                    b.legal_moves().iter().any(|mv| mv == m),
                    "board:{:#} mv: {} {:?} is not in board.legal_moves",
                    b,
                    m,
                    m
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
        // dont store duplicates
        if m == &self.killers[y as usize][0] || m == &self.killers[y as usize][1] {
            return;
        }
        // self.killers[y as usize][3] = self.killers[y as usize][2];
        // self.killers[y as usize][2] = self.killers[y as usize][1];
        self.killers[y as usize][1] = self.killers[y as usize][0];
        self.killers[y as usize][0] = *m;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use figment::providers::{Format, Toml};
    use figment::Figment;
    use test_log::test;
    use toml;

    #[test]
    fn serde_killers_test() -> Result<()> {
        info!("{}", toml::to_string(&Killers::default())?);

        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "config.toml",
                r#"
                enabled = true
                clear_every_move = false
                use_ply_below = true
            "#,
            )?;

            // jail.set_env("config_name", "env-test");

            // jail.create_file("Config.json", r#"
            //     {
            //         "name": "json-test",
            //         "debug": true
            //     }
            // "#)?;

            let _killers: Killers = Figment::new()
                .merge(Toml::file("config.toml"))
                // .merge(Env::prefixed("CONFIG_"))
                // .join(Json::file("Config.json"))
                .extract()?;

            // assert_eq!(killers, Killers {
            //     enabled: true,
            //     clear_every_move: false,
            //     use_ply_below: true,
            //     .. Default::default()
            //     });
            Ok(())
        });
        Ok(())
    }
}
