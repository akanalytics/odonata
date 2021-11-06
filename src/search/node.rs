use crate::eval::score::Score;
// use crate::board::Board;
use crate::types::{Ply};



#[derive(Copy, Clone, Debug, Default)]
pub struct Node {
    pub ply: Ply,
    pub depth: Ply,
    pub alpha: Score,
    pub beta: Score,
}

impl Node {
    #[inline]
    pub fn root(depth: Ply) -> Node {
        Node {
            ply: 0,
            depth,
            alpha: -Score::INFINITY,
            beta: Score::INFINITY,
        }
    }

    #[inline]
    pub fn new_child(&self) -> Node {
        Node {
            alpha: -self.beta,
            depth: self.depth - 1,
            beta: -self.alpha,
            ply: self.ply + 1,
        }
    }


    #[inline]
    pub fn is_root(&self) -> bool {
        self.ply == 0
    }

    #[inline]
    pub fn is_zw(&self) -> bool {
        self.alpha.is_numeric() && self.beta.is_numeric() && self.alpha.as_i16() + 1 == self.beta.as_i16()
    }

    #[inline]
    pub fn is_pv(&self) -> bool {
        !self.is_zw()
    }

    #[inline]
    pub fn is_qs(&self) -> bool {
        self.depth < 0
    }
}
