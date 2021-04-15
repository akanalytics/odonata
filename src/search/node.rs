use crate::eval::Score;
use crate::board::Board;
use crate::types::Color;
use crate::movelist::Move;



#[derive(Debug)]
pub struct Node<'b> {
    pub board: &'b mut Board,
    pub ply: u32,
    pub alpha: Score,
    pub beta: Score,
    pub score: Score,
    pub last_move: &'b Move,
}

impl Node<'_> {
    #[inline]
    pub fn new_root(board: &mut Board) -> Node {
        let score = if Self::is_maximizing(board) { Score::MinusInfinity } else { Score::PlusInfinity };
        Node {
            board,
            ply: 0,
            alpha: Score::MinusInfinity,
            beta: Score::PlusInfinity,
            score,
            last_move: &Move::NULL_MOVE,
        }
    }

    #[inline]
    pub fn new_child<'c>(&self, mv: &'c Move, board: &'c mut Board) -> Node<'c> {
        let score = if Self::is_maximizing(board) { Score::MinusInfinity } else { Score::PlusInfinity };
        Node {
            board,
            alpha: self.alpha,
            beta: self.beta,
            ply: self.ply + 1,
            score,
            last_move: mv,
        }
    }

    #[inline]
    pub fn is_maximizing(board: &Board) -> bool {
        // node.ply % 2 == 0 // 0 ply looks at our moves - maximising if white
        board.color_us() == Color::White
    }

    #[inline]
    pub fn is_root(&self) -> bool {
        self.ply == 0
    }
}
