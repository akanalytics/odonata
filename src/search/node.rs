use crate::eval::score::Score;
// use crate::board::Board;
use crate::types::{Ply};



#[derive(Copy, Clone, Debug)]
pub struct Node {
    pub ply: Ply,
    pub alpha: Score,
    pub beta: Score,
}

impl Node {
    #[inline]
    pub fn root() -> Node {
        Node {
            ply: 0,
            alpha: Score::MinusInf,
            beta: Score::PlusInf,
        }
    }

    #[inline]
    pub fn new_child(&self) -> Node {
        Node {
            alpha: -self.beta,
            beta: -self.alpha,
            ply: self.ply + 1,
        }
    }


    #[inline]
    pub fn is_root(&self) -> bool {
        self.ply == 0
    }
}
