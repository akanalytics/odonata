use std::fmt;

use odonata_base::movelist::MoveList;
use odonata_base::mv::Move;
use odonata_base::piece::Ply;

#[derive(Clone, Debug)]
pub struct Restrictions {
    pub search_moves:   MoveList, // restrict moves if set
    pub excluded_moves: MoveList,
}

impl Default for Restrictions {
    fn default() -> Self {
        Self {
            search_moves:   MoveList::new(),
            excluded_moves: MoveList::new(),
        }
    }
}

impl Restrictions {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn is_none(&self) -> bool {
        self.excluded_moves.is_empty() && self.search_moves.is_empty()
    }

    pub fn skip_move(&self, ply: Ply, mv: Move) -> bool {
        ply == 0 && (!self.search_moves.is_empty() && !self.search_moves.contains(&mv))
            || self.excluded_moves.contains(&mv)
    }
}

impl fmt::Display for Restrictions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:#?}", self)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use odonata_base::prelude::testing::Testing;
    use odonata_base::prelude::Board;
    use test_log::test;

    use super::Restrictions;

    #[test]
    fn test_restrictions() {
        let b = Board::starting_pos();

        let none = Restrictions::default();
        assert_eq!(none.is_none(), true, "restrictions are none");

        let restrict = Restrictions {
            search_moves:   "d4,e4".moves(&b),
            excluded_moves: "a3,a4,e4".moves(&b),
        };

        assert_eq!(restrict.skip_move(0, "a4".mv(&b)), true, "a4 not in search");
        assert_eq!(restrict.skip_move(0, "e4".mv(&b)), true, "e4 in exclude");
    }
}
