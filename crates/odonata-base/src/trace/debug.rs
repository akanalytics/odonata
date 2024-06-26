use std::fmt;

use crate::domain::node::Node;
use crate::domain::score::Score;
use crate::mv::Move;
use crate::piece::{Color, Ply};
use crate::prelude::Board;

#[derive(Clone, Debug)]
pub struct Debug {
    board: Board,
    items: Vec<String>,
}

impl Board {
    pub fn debug(&self) -> Debug {
        Debug {
            board: self.clone(),
            items: Vec::new(),
        }
    }
}

impl Debug {
    fn append_move(&mut self, mv: Move) -> &mut Debug {
        self.items.push(self.board.to_san(mv));
        self
    }

    fn append_turn(&mut self, c: Color) -> &mut Debug {
        self.items.push(c.to_string());
        self
    }

    fn append_score(&mut self, score: Score) -> &mut Debug {
        self.items.push(if self.board.color_us() == Color::White {
            score.to_string()
        } else {
            score.negate().to_string()
        });
        self
    }

    fn append_ply(&mut self, ply: Ply) -> &mut Debug {
        self.items.push(ply.to_string());
        self
    }

    fn append_text(&mut self, s: &str) -> &mut Debug {
        self.items.push(s.to_string());
        self
    }

    fn append_node(&mut self, node: &Node) -> &mut Debug {
        self.append_text(&format!("ply={} depth={}", node.ply, node.depth));
        if self.board.color_us() == Color::White {
            self.append_text(&format!("(a={},b={})", node.alpha, node.beta));
        } else {
            self.append_text(&format!("(a={},b={})", -node.beta, -node.alpha));
        }
        self
    }
}

impl fmt::Display for Debug {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.items.join(" "))
    }
}

impl std::ops::Add<Move> for Debug {
    type Output = Debug;

    fn add(mut self, other: Move) -> Debug {
        self.append_move(other).clone()
    }
}

impl std::ops::Add<Node> for Debug {
    type Output = Debug;

    fn add(mut self, o: Node) -> Debug {
        self.append_node(&o).clone()
    }
}

impl std::ops::Add<&str> for Debug {
    type Output = Debug;

    fn add(mut self, o: &str) -> Debug {
        self.append_text(o).clone()
    }
}

impl std::ops::Add<Score> for Debug {
    type Output = Debug;

    fn add(mut self, o: Score) -> Debug {
        self.append_score(o).clone()
    }
}

impl std::ops::Add<Color> for Debug {
    type Output = Debug;

    fn add(mut self, o: Color) -> Debug {
        self.append_turn(o).clone()
    }
}

impl std::ops::Add<Ply> for Debug {
    type Output = Debug;

    fn add(mut self, o: Ply) -> Debug {
        self.append_ply(o).clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::*;

    #[test]
    fn test_debug() {
        let b = Catalog::starting_board();
        let moves = b.legal_moves();
        println!(
            "{}",
            b.debug() + moves[0] + 3 + Color::White + Node::root(0) + Score::from_cp(100) + "test"
        );
    }
}
