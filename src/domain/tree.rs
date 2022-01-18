use itertools::Itertools;
use rose_tree::{NodeIndex, RoseTree};
use std::fmt;
use std::fmt::{Debug, Display};

use crate::board::Board;
use crate::bound::NodeType;
use crate::eval::score::Score;
use crate::mv::Move;
use crate::search::node::Event;
use crate::search::node::Node;
use crate::variation::Variation;

// inspired by https://crates.io/crates/treeline (License MIT)

#[derive(Debug, Clone)]
pub struct Tree<N> {
    rose_tree: RoseTree<N>,
    root: NodeIndex,
}

impl<N> Tree<N>
where
    N: Display,
{
    fn display_leaves(&self, f: &mut fmt::Formatter, leaves: &Vec<NodeIndex>, spaces: Vec<bool>) -> fmt::Result {
        for (i, &leaf) in leaves.iter().rev().enumerate() {
            let last = i >= leaves.len() - 1;
            let mut clone = spaces.clone();
            // print single line
            for s in &spaces {
                if *s {
                    write!(f, "    ")?;
                } else {
                    write!(f, "|   ")?;
                }
            }
            if last {
                writeln!(f, "└── {}", self[leaf])?;
            } else {
                writeln!(f, "├── {}", self[leaf])?;
            }

            // recurse
            if self.rose_tree.children(leaf).count() > 0 {
                clone.push(last);
                self.display_leaves(f, &self.children(leaf).collect_vec(), clone)?;
            }
        }
        write!(f, "")
    }
}

impl<N> Display for Tree<N>
where
    N: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let root = self.root();
        writeln!(f, "{}", self.rose_tree.node_weight(root).unwrap())?;
        let leaves = self.rose_tree.children(root).collect();
        self.display_leaves(f, &leaves, Vec::new())
    }
}

impl<N> Default for Tree<N>
where
    N: Default,
{
    fn default() -> Self {
        let (rose_tree, root) = RoseTree::new(Default::default());
        Self { rose_tree, root }
    }
}

impl<N> Tree<N> {
    fn new(root: N) -> Self {
        let (rose_tree, root) = RoseTree::new(root);
        Self { rose_tree, root }
    }

    fn root(&self) -> NodeIndex {
        self.root
    }

    fn add_child(&mut self, parent: NodeIndex, kid: N) -> NodeIndex {
        self.rose_tree.add_child(parent, kid)
    }

    fn children(&self, parent: NodeIndex) -> impl Iterator<Item = NodeIndex> + '_ {
        self.rose_tree.children(parent)
    }
}

impl<N> std::ops::Index<NodeIndex> for Tree<N> {
    type Output = N;
    #[inline]
    fn index(&self, i: NodeIndex) -> &Self::Output {
        self.rose_tree.node_weight(i).unwrap()
    }
}

impl<N> std::ops::IndexMut<NodeIndex> for Tree<N> {
    #[inline]
    fn index_mut(&mut self, i: NodeIndex) -> &mut Self::Output {
        self.rose_tree.node_weight_mut(i).unwrap()
    }
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct SearchTreeWeight {
    mv: Move,
    pub node: Node,
    pub score: Score,
    pub event: Event,
    pub nt: NodeType,
    pub is_best_move: bool,
}

impl Display for SearchTreeWeight {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}{} {} {} {} {}",
            self.mv,
            if self.is_best_move { "*" } else { " " },
            self.score,
            self.node,
            self.event,
            self.nt
        )
    }
}

#[derive(Clone, Debug, Default)]
pub struct SearchTree {
    pub initial_position: Board,
    pub tree: Tree<SearchTreeWeight>,
}

impl SearchTree {
    pub fn new(b: Board) -> Self {
        SearchTree {
            initial_position: b,
            tree: Tree::default(),
        }
    }

    /// empty variation finds root, not found is None
    fn find(&self, var: &Variation) -> Option<NodeIndex> {
        let mut node = self.tree.root();
        'outer: for &mv in var.moves() {
            for child in self.tree.children(node) {
                if self.tree[child].mv == mv {
                    node = child;
                    continue 'outer;
                }
            }
            return None;
        }
        Some(node)
    }

    pub fn get_or_insert(&mut self, var: &Variation) -> &mut SearchTreeWeight {
        if let Some(n) = self.find(&var) {
            return &mut self.tree[n];
        }
        if let Some(stem) = var.stem() {
            self.get_or_insert(&stem);
            if let Some(n) = self.find(&stem) {
                let w = SearchTreeWeight {
                    mv: var.last().unwrap_or(&Move::NULL_MOVE).to_owned(),
                    ..SearchTreeWeight::default()
                };
                let new = self.tree.add_child(n, w);
                return &mut self.tree[new];
            }
        }
        let root = self.tree.root();
        return &mut self.tree[root];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bitboard::square::Square, catalog::Catalog, test_log::test};

    #[test]
    fn test_generic_tree() {
        let mut tree = Tree::new(String::from("Root"));
        let root = tree.root();
        let cat = tree.add_child(root, "Cat".into());
        assert_eq!(tree.rose_tree.children(cat).count(), 0);
        let _mouse = tree.add_child(root, "Mouse".into());
        let dog = tree.add_child(root, "Dog".into());
        assert_eq!(tree[dog], "Dog".to_owned());
        tree.add_child(dog, "Bark".to_owned());
        let woof = tree.add_child(dog, "Woof".to_owned());
        assert_eq!(tree.children(dog).count(), 2);
        assert_eq!(tree.children(root).count(), 3);
        assert_eq!(
            tree.rose_tree
                .parent_recursion(woof)
                .map(|i| tree.rose_tree[i].clone())
                .collect_vec(),
            vec!["Dog".to_owned(), "Root".to_owned()]
        );
        println!("Tree:\n{}", tree);
    }

    #[test]
    fn test_search_tree() {
        use crate::types::Piece::*;
        let mut st = SearchTree::new(Catalog::starting_board());
        let mut var = Variation::new();

        var.push(Move::new_quiet(Pawn, Square::H2, Square::H3));
        st.get_or_insert(&var).node = Node {
            alpha: Score::from_cp(4),
            ..Node::default()
        };
        println!("Tree1:\n{}", st.tree);

        var.push(Move::new_quiet(Pawn, Square::H7, Square::H6));
        st.get_or_insert(&var).node = Node {
            alpha: Score::from_cp(5),
            ..Node::default()
        };
        println!("Tree:2\n{}", st.tree);
    }
}

// fn display_leaves(self, f: &mut fmt::Formatter, parent: NodeIndex, spaces: Vec<bool>) -> fmt::Result {
//     for (i, leaf) in self.rose_tree.children(parent).enumerate() {
//         let last = i >= self.rose_tree.children(parent).count() - 1;
//         // print single line
//         for s in &spaces {
//             if *s {
//                 write!(f, "    ")?;
//             } else {
//                 write!(f, "|   ")?;
//             }
//         }
//         if last {
//             writeln!(f, "└── {}", self.rose_tree.node_weight(leaf).unwrap())?;
//         } else {
//             writeln!(f, "├── {}", self.rose_tree.node_weight(leaf).unwrap())?;
//         }

//         // recurse
//         if !leaf.leaves.is_empty() {
//             let mut clone = spaces.clone();
//             clone.push(last);
//             Self::display_leaves(f, &leaf.leaves, clone)?;
//         }
//     }
//     write!(f, "")
// }
