use std::fmt;

use anyhow::{bail, Context};

use itertools::Itertools;

use crate::infra::utils::Uci;



#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum PlayerType {
    #[default]
    Human,
    Computer,
}

impl fmt::Display for PlayerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            PlayerType::Human => "human",
            PlayerType::Computer => "computer",
        })
    }
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Player {
    pub name: String,
    pub elo: Option<i32>,
    pub title: String,
    pub player_type: PlayerType,
}

impl Uci for Player {
    fn fmt_uci(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{title} {elo} {type} {name}",
            title = self.title,
            elo = self
                .elo
                .map(|e| e.to_string())
                .unwrap_or("none".to_string()),
                type = self.player_type,
            name = self.name
        )
    }

    /// The format of the string has to be [GM|IM|FM|WGM|WIM|none] [|none] [computer|human] name
    fn parse_uci(s: &str) -> anyhow::Result<Self> {
        let mut words = s.split_whitespace().fuse();
        let title = words
            .next()
            .with_context(|| "no title specified for opponent")?
            .to_string();
        let elo = words
            .next()
            .with_context(|| "no elo specified for opponent")?
            .to_string();
        let elo = match elo.as_str() {
            "none" => None,
            _ => Some(elo.parse().with_context(|| format!("Parsing elo {elo}"))?),
        };
        let player_type = match words
            .next()
            .with_context(|| "no player type specified for opponent")?
        {
            "human" => PlayerType::Human,
            "computer" => PlayerType::Computer,
            text => bail!("unexpected player type {text}"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player() {
        let p = Player::parse_uci("GM 2800 computer  Odonata").unwrap();
        assert_eq!(p.to_uci(), "GM 2800 computer Odonata");

        let p = Player::parse_uci("IM none human Odonata").unwrap();
        assert_eq!(p.to_uci(), "IM none human Odonata");
    }
}
