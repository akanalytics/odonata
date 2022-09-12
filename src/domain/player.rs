use anyhow::{bail, Context};
use std::str::FromStr;

use itertools::Itertools;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum PlayerType {
    #[default]
    Human,
    Computer,
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Player {
    name: String,
    elo: String,
    title: String,
    player_type: PlayerType,
}

impl FromStr for Player {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut words = s.split_whitespace().fuse();
        let title = words
            .next()
            .context("No title specified for opponent")?
            .to_string();
        let elo = words
            .next()
            .context("No elo specified for opponent")?
            .to_string();
        let player_type = match words
            .next()
            .context("No player type specified for opponent")?
        {
            "human" => PlayerType::Human,
            "computer" => PlayerType::Computer,
            text => bail!("Unexpected player type {text}"),
        };
        let name = words.join(" ");
        Ok(Player {
            name,
            elo,
            title,
            player_type,
        })
    }
}
