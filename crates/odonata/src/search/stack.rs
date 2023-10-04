use crate::piece::{Ply, MAX_PLY};
use std::{fmt, ops};

#[derive(Debug, Clone)]
pub struct Stack<T> {
    items: Vec<T>,
    size:  usize,
}

impl<T> Stack<T>
where
    T: Default,
    T: Clone,
{
    pub fn clear(&mut self) {
        self.items.fill(T::default());
        self.size = 0;
    }
}

impl<T> Default for Stack<T>
where
    T: Default + Clone,
{
    fn default() -> Self {
        Stack {
            items: vec![T::default(); MAX_PLY as usize],
            size:  0,
        }
    }
}

impl<T> fmt::Display for Stack<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for t in &self.items[0..self.size] {
            t.fmt(f)?;
        }
        Ok(())
    }
}

impl<T> ops::Index<Ply> for Stack<T> {
    type Output = T;

    fn index(&self, ply: Ply) -> &Self::Output {
        &self.items[ply as usize]
    }
}

impl<T> ops::IndexMut<Ply> for Stack<T> {
    fn index_mut(&mut self, ply: Ply) -> &mut Self::Output {
        let ply = ply as usize;
        if ply > self.size {
            self.size = ply;
        }
        &mut self.items[ply]
    }
}
