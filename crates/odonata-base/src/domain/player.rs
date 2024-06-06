use std::fmt;

use anyhow::{bail, Context};
use itertools::Itertools;

use crate::infra::utils::Uci;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum PlayerKind {
    #[default]
    Computer,
    Human,
}

impl fmt::Display for PlayerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            PlayerKind::Human => "human",
            PlayerKind::Computer => "computer",
        })
    }
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Player {
    pub name:  String,
    pub elo:   Option<i32>,
    pub title: String,
    pub kind:  PlayerKind,
}

impl Uci for Player {
    fn fmt_uci(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{title} {elo} {kind} {name}",
            title = self.title,
            elo = self.elo.map(|e| e.to_string()).unwrap_or("none".to_string()),
            kind = self.kind,
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
        let kind = match words.next().with_context(|| "no player type specified for opponent")? {
            "human" => PlayerKind::Human,
            "computer" => PlayerKind::Computer,
            text => bail!("unexpected player type {text}"),
        };
        let name = words.join(" ");
        Ok(Player { name, elo, title, kind })
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
