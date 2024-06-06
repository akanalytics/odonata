// use rose_tree::{NodeIndex, };
use std::fmt;
use std::fmt::{Debug, Display};

use itertools::Itertools;
use petgraph::graph::NodeIndex;

use crate::domain::node::{Event, Node};
use crate::domain::score::Score;
use crate::domain::BoundType;
use crate::mv::Move;
use crate::prelude::Board;
use crate::variation::Variation;

// copied/inspired by https://crates.io/crates/treeline (License MIT)
// and
// https://github.com/mitchmindtree/rose_tree-rs
// (licence Apache https://github.com/mitchmindtree/rose_tree-rs/blob/master/LICENSE-APACHE)

pub type PetGraph<N> = petgraph::Graph<N, (), petgraph::Directed, u32>;

#[derive(Debug, Clone)]
struct Tree<N> {
    graph: PetGraph<N>,
    root:  NodeIndex,
}

impl Tree<TreeNode> {
    fn display_leaves(
        &self,
        f: &mut fmt::Formatter,
        b: &Board,
        leaves: &[NodeIndex],
        spaces: &Vec<bool>,
        var: &mut Variation,
    ) -> fmt::Result {
        for (i, &leaf) in leaves.iter().rev().enumerate() {
            let last = i >= leaves.len() - 1;
            let mut clone = spaces.clone();
            // print single line
            for s in spaces.iter() {
                if *s {
                    write!(f, "    ")?;
                } else {
                    write!(f, "|   ")?;
                }
            }
            if last {
                write!(f, "└── ")?;
            } else {
                write!(f, "├── ")?;
            }

            let node = &self[leaf];
            write!(f, "{}{}", node.mv, if node.is_best_move { "*" } else { " " })?;

            for _ in spaces.len()..5 {
                write!(f, "    ")?;
            }

            var.push(node.mv);
            writeln!(f, "{}  --> {}", node, var.to_san(b))?;

            if last && self.children(leaf).count() == 0 {
                for s in spaces.iter() {
                    if *s {
                        write!(f, "    ")?;
                    } else {
                        write!(f, "|   ")?;
                    }
                }
                // writeln!(f, "{} {} {}", "----", i, leaves.len())?;
                writeln!(f)?;
            }

            // recurse
            if self.children(leaf).count() > 0 {
                clone.push(last);
                self.display_leaves(f, b, &self.children(leaf).collect_vec(), &clone, var)?;
            }
            var.pop();
        }
        write!(f, "")
    }

    // fn variation(&self, leaf: NodeIndex) -> Variation {
    //     let mut vec = Vec::new();
    //     while let Some(parent) = self.parent(leaf) {
    //         let TreeNode { mv, ..}  = self[parent];
    //         vec.push(mv);
    //     }
    //     vec.reverse();
    //     let mut var = Variation::new();
    //     var.extend_from_slice(&vec);
    //     var
    // }

    fn write(&self, f: &mut fmt::Formatter, b: &Board) -> fmt::Result {
        let root = self.root();
        writeln!(f, "{}", self.graph.node_weight(root).unwrap())?;
        let leaves = self.children(root).collect_vec();
        let mut var = Variation::new();
        self.display_leaves(f, b, &leaves, &Vec::new(), &mut var)
    }
}

impl<N> Default for Tree<N>
where
    N: Default,
{
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<N> Tree<N> {
    // fn new(root: N) -> Self {
    //     let (rose_tree, root) = RoseTree::new(root);
    //     Self { rose_tree, root }
    // }

    pub fn new(root: N) -> Self {
        let mut graph = PetGraph::with_capacity(1, 1);
        let root = graph.add_node(root);
        Self { graph, root }
    }

    fn root(&self) -> NodeIndex {
        self.root
    }

    /// Add a child node to the node at the given NodeIndex.
    /// Returns an index into the child's position within the tree.
    fn add_child(&mut self, parent: NodeIndex, kid: N) -> NodeIndex {
        let kid = self.graph.add_node(kid);
        self.graph.add_edge(parent, kid, ());
        kid
    }

    fn _parent(&self, child: NodeIndex) -> Option<NodeIndex> {
        self.graph.neighbors_directed(child, petgraph::Incoming).last()
    }

    fn children(&self, parent: NodeIndex) -> impl Iterator<Item = NodeIndex> + '_ {
        self.graph.neighbors_directed(parent, petgraph::Outgoing)
    }
}

impl<N> std::ops::Index<NodeIndex> for Tree<N> {
    type Output = N;
    #[inline]
    fn index(&self, i: NodeIndex) -> &Self::Output {
        self.graph.node_weight(i).unwrap()
    }
}

impl<N> std::ops::IndexMut<NodeIndex> for Tree<N> {
    #[inline]
    fn index_mut(&mut self, i: NodeIndex) -> &mut Self::Output {
        self.graph.node_weight_mut(i).unwrap()
    }
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct TreeNode {
    mv:               Move,
    pub count:        u32,
    pub ext:          i32,
    pub red:          i32,
    pub node:         Node,
    pub score:        Score,
    pub eval:         Score,
    pub event:        Event,
    pub cause:        Event,
    pub nt:           BoundType,
    pub is_best_move: bool,
}

impl Display for TreeNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "#{:<3} S:{:>5} E:{:>5} D:{:>2}{:<5} [{:>4},{:>4}] {:<3} {:<20} {:<20}",
            self.count.to_string(),
            self.score.to_string(),
            self.eval.to_string(),
            self.node.depth,
            format!("[{}/{}]", self.ext, self.red),
            self.node.alpha.to_string(),
            self.node.beta.to_string(),
            self.nt.to_string(),
            self.event.to_string(),
            self.cause.to_string(),
        )

        // write!(
        //     f,
        //     "{}{} {} {} {} {}",
        //     self.mv,
        //     if self.is_best_move { "*" } else { " " },
        //     self.score,
        //     self.node,
        //     self.event,
        //     self.nt
        // )
    }
}

#[derive(Clone, Debug, Default)]
struct SearchTree {
    pub initial_position: Board,
    pub tree:             Tree<TreeNode>,
}

impl Display for SearchTree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.tree.write(f, &self.initial_position)
    }
}

impl SearchTree {
    pub fn new(b: Board) -> Self {
        SearchTree {
            initial_position: b,
            tree:             Tree::default(),
        }
    }

    /// empty variation finds root, not found is None
    fn find(&self, var: &Variation) -> Option<NodeIndex> {
        let mut node = self.tree.root();
        'outer: for mv in var.moves() {
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

    pub fn get_or_insert(&mut self, var: &Variation) -> &mut TreeNode {
        if let Some(n) = self.find(var) {
            return &mut self.tree[n];
        }
        if let Some(stem) = var.stem() {
            self.get_or_insert(&stem);
            if let Some(n) = self.find(&stem) {
                let w = TreeNode {
                    mv: var.last().unwrap_or(Move::new_null()),
                    ..TreeNode::default()
                };
                let new = self.tree.add_child(n, w);
                return &mut self.tree[new];
            }
        }
        let root = self.tree.root();
        &mut self.tree[root]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bits::square::Square;
    use crate::catalog::Catalog;
    use crate::test_log::test;
    use crate::Piece;

    #[test]
    fn test_generic_tree() {
        let mut tree = Tree::new(String::from("Root"));
        let root = tree.root();
        let cat = tree.add_child(root, "Cat".into());
        assert_eq!(tree.children(cat).count(), 0);
        let _mouse = tree.add_child(root, "Mouse".into());
        let dog = tree.add_child(root, "Dog".into());
        assert_eq!(tree[dog], "Dog".to_owned());
        tree.add_child(dog, "Bark".to_owned());
        let _woof = tree.add_child(dog, "Woof".to_owned());
        assert_eq!(tree.children(dog).count(), 2);
        assert_eq!(tree.children(root).count(), 3);
        // assert_eq!(
        //     tree.rose_tree
        //         .parent_recursion(woof)
        //         .map(|i| tree.rose_tree[i].clone())
        //         .collect_vec(),
        //     vec!["Dog".to_owned(), "Root".to_owned()]
        // );
        // println!("Tree:\n{}", tree);
    }

    #[test]
    fn test_search_tree() {
        let bd = Catalog::starting_board();
        let mut st = SearchTree::new(bd.clone());
        let mut var = Variation::new();

        var.push(Move::new_quiet(Piece::Pawn, Square::H2, Square::H3, &bd));
        st.get_or_insert(&var).node = Node {
            alpha: Score::from_cp(4),
            ..Node::default()
        };
        println!("Tree1:\n{st}");

        var.push(Move::new_quiet(Piece::Pawn, Square::H7, Square::H6, &bd));
        st.get_or_insert(&var).node = Node {
            alpha: Score::from_cp(5),
            ..Node::default()
        };
        println!("Tree:2\n{st}");
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
