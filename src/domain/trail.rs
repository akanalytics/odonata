use std::{fmt, mem};

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    board::Board,
    bound::NodeType,
    eval::score::Score,
    infra::utils::Displayable,
    mv::Move,
    piece::{Ply, LEN_PLY},
    search::node::{Event, Node},
    variation::Variation,
};

#[derive(Clone, Default, PartialEq, Eq)]
pub struct TreeNode {
    pub index: usize,
    pub id: NodeId,
    pub parent: NodeId,
    pub mv: Move,
}

impl fmt::Debug for TreeNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TreeNode {{ ")?;
        write!(f, "index: {:<4} ", self.index)?;
        write!(f, "id: {:<4} ", self.id.0)?;
        write!(f, "parent: {:<4} ", self.parent.0)?;
        write!(f, "mv: {:<5} }}", self.mv.to_uci())?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NodeId(i32);

#[derive(Clone, Default, PartialEq)]
struct Tree {
    nodes: Vec<TreeNode>,
}

impl fmt::Debug for Tree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Tree {{")?;
        for tn in self.nodes.iter() {
            write!(f, "{tn:?}")?;
            if f.alternate() {
                write!(f, "  var: {}", self.variation_of(tn.id))?;
            }
            writeln!(f)?;
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}

const ROOT: TreeNode = TreeNode {
    index: 0,
    id: NodeId(0),
    parent: NodeId(-1),
    mv: Move::NULL_MOVE,
};

impl Tree {
    fn new() -> Self {
        Tree { nodes: vec![ROOT] }
    }

    fn add(&mut self, var: &Variation, index: usize) {
        let mv = var.last().expect("variation empty - cannot add root node");
        assert!(self.find_by_var(&var).is_none(), "duplicate: {var}");
        let parent = self
            .find_by_var(&var.stem().unwrap())
            .expect(&format!("no parent: {var}"))
            .id;
        let id = NodeId(self.nodes.len() as i32);
        let tn = TreeNode {
            index,
            id,
            parent,
            mv,
        };
        self.nodes.push(tn);
    }

    pub fn find_by_var(&self, var: &Variation) -> Option<&TreeNode> {
        // start at root
        // nodes where parent=node root with mv = variation[i]
        let mut tn = &ROOT;
        for mv in var.moves() {
            let children = self.children_of(tn.id);
            tn = children
                .iter()
                .filter(|&&tn| tn.mv == mv)
                .exactly_one()
                .ok()?;
        }
        Some(tn)
    }

    pub fn find_by_index(&self, i: usize) -> Option<&TreeNode> {
        self.nodes.iter().find(|&tn| tn.index == i)
    }

    pub fn find_by_id(&self, id: NodeId) -> &TreeNode {
        &self.nodes[id.0 as usize]
    }

    fn variation_of(&self, mut id: NodeId) -> Variation {
        let mut var = Variation::new();
        while id != ROOT.id && var.len() < LEN_PLY {
            let node = self.find_by_id(id);
            var.push_front(node.mv);
            id = node.parent;
        }
        assert!(var.len() < LEN_PLY, "unbounded var: {var}");
        var
    }

    fn children_of(&self, id: NodeId) -> Vec<&TreeNode> {
        // root has -1 as parent
        self.nodes
            .iter()
            .filter_map(|tn| if tn.parent == id { Some(tn) } else { None })
            .collect_vec()
    }
}

use std::fmt::Write;

fn display_node(
    tree: &Tree,
    f: &mut fmt::Formatter,
    payload: &impl Fn(&str, &TreeNode) -> String,
    indent: Vec<bool>,
    id: NodeId,
) -> fmt::Result {
    if id == ROOT.id {
        writeln!(f, "{}", payload("", tree.find_by_id(id)))?;
    }
    let node = tree.find_by_id(id);
    let children = tree.children_of(node.id);
    for (i, &child) in children.iter().enumerate() {
        let mut twig = String::new();
        let last = i >= children.len() - 1;
        // print single line
        for s in &indent {
            write!(&mut twig, "{}", if *s { "    " } else { "|   " })?;
        }
        write!(&mut twig, "{}", if last { "└── " } else { "├── " })?;
        writeln!(f, "{}", payload(&twig, tree.find_by_id(child.id)))?;

        // for _ in spaces.len()..5 {
        //     write!(f, "    ")?;
        // }

        if last && tree.children_of(child.id).len() == 0 {
            for s in &indent {
                write!(f, "{}", if *s { "    " } else { "|   " })?;
            }
            writeln!(f)?;
        }

        // recurse
        let mut spaces = indent.clone();
        spaces.push(last);
        display_node(tree, f, payload, spaces, child.id)?;
    }
    write!(f, "")
}

impl fmt::Display for Tree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let payload = |twig: &str, tn: &TreeNode| format!("{twig} {}", tn.mv.to_uci());
        display_node(self, f, &payload, vec![], ROOT.id)
    }
}

/// show variation
fn displayable<'a>(t: &'a Tree, bd: &'a Board) -> impl Fn(&mut fmt::Formatter) -> fmt::Result + 'a {
    fn format_data(tree: &Tree, bd: &Board, tn: &TreeNode, f: &mut fmt::Formatter) -> fmt::Result {
        let var = tree.variation_of(tn.id);
        if let Some(stem) = &var.stem() {
            let san = bd.make_moves_old(stem).to_san(tn.mv);
            write!(f, "{san}")?;
        }
        Ok(())
    }

    // the displayable function
    |f: &mut fmt::Formatter| -> fmt::Result {
        let payload = |twig: &str, tn: &TreeNode| {
            format!("{twig} {}", Displayable(|f| format_data(t, bd, tn, f)))
        };
        display_node(t, f, &payload, vec![], ROOT.id)
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct NodeDetails {
    pub n: Node,
    pub e: Event, // <- bit flags ?
    pub sc: Score,
    pub nt: NodeType,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct TreeCrit {
    pub enabled: bool,
    pub starts_with: Variation,
    pub max_ply: Ply,
}

impl TreeCrit {
    // enable with RUST_LOG=tree=info
    pub fn accept(&self, var: &Variation) -> bool {
        self.enabled && var.len() <= self.starts_with.len() + self.max_ply as usize
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ChessTree {
    board: Board,
    tree: Tree,
    arena: Vec<NodeDetails>,
}

impl ChessTree {
    pub fn new(board: Board) -> Self {
        let root = NodeDetails::default();
        Self {
            board,
            tree: Tree::new(),
            arena: vec![root],
        }
    }

    pub(crate) fn merge(&mut self, var: &Variation, details: NodeDetails) {
        // let sp = Span::current().field("trail").to_string_or("<na>");
        event!(target: "tree", tracing::Level::INFO, "E: {var:<20} {event} {sc}" ,var = var.to_uci(), event=details.e, sc=details.sc);
        if let Some(tn) = self.tree.find_by_var(&var.take(details.n.ply as usize)) {
            let nd = &mut self.arena[tn.index];
            nd.sc = details.sc;
            nd.n = details.n;
            nd.e = details.e;
            nd.nt = details.nt;
        } else {
            debug!(target: "tree", "TREE: adding at {var} {details:?} index {}", self.arena.len());
            self.tree.add(var, self.arena.len()); // use the arena index as the ID
            self.arena.push(details);
        }
    }
}

/// show variation and detail
fn displayable2<'a>(ct: &'a ChessTree) -> impl Fn(&mut fmt::Formatter) -> fmt::Result + 'a {
    fn format_data2(
        twig: &str,
        ct: &ChessTree,
        tn: &TreeNode,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {

        let var = ct.tree.variation_of(tn.id);
        let san = if let Some(stem) = &var.stem() {
            ct.board.make_moves_old(stem).to_san(tn.mv)
        } else {
            String::new()
        };
        let uci = tn.mv.to_uci();
        let nd = &ct.arena[tn.index];
        let a = nd.n.alpha;
        let b = nd.n.beta;
        let window = format!("[{:>5} {:>5}]", a.to_string(), b.to_string());
        let sc = nd.sc.to_string();
        let nt = nd.n.node_type(nd.sc);
        let p = nd.n.ply;
        let d = nd.n.depth;
        let nt = match nt {
            NodeType::ExactPv => "##",
            NodeType::LowerCut => "↑",
            NodeType::UpperAll => "↓",
            NodeType::Unused => "?",
        };
        let qs = match d <= 0 {
            true => '*',
            false => ' ',
        };
        if f.alternate() {
            let left = format!("{twig} {nt} {san}");
            write!(
                f,
                "{left:<30} {sc:<6} {window} {e} ({uci}) P{p}D{d} {qs}",
                e = nd.e
            )?;
        } else {
            let left = format!("{twig} {san}");
            write!(f, "{left}")?;
        }
        Ok(())
    }

    // the displayable function
    |f: &mut fmt::Formatter| -> fmt::Result {
        let payload2 = |twig: &str, tn: &TreeNode| {
            format!("{:#}", Displayable(|f| format_data2(twig, ct, tn, f)))
        };
        display_node(&ct.tree, f, &payload2, vec![], ROOT.id)
    }
}

impl fmt::Display for ChessTree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "\n{:L>}", self.board)?;
        // write!(f, "{}", Displayable(displayable(&self.tree, &self.board)))?;
        write!(f, "{:#}", Displayable(displayable2(&self)))?;
        Ok(())
    }
}

// line, branch or variation
#[derive(Clone)]
pub struct Trail {
    seldepth: Ply,
    path: Variation,
    pv_for_ply: Vec<Variation>,
    score_for_ply: Vec<Score>,
    root: Board,
    refutations: Vec<Variation>,
    refutation_scores: Vec<Score>,
    pub chess_tree: ChessTree,
    tree_crit: TreeCrit,
}

/// PV[i] has len >= i. as the cv to ply i is len i.
/// PV[i] has len = i when setting terminal/leaf node (such as eval/stalemate)
/// PV[1] on raised alpha: set to cv(len 1) + mv + pv[2].skip(2) (first 2 will have been cv to get to i=2)
/// PV[i] = CV + MV + PV[i+1].skip(i+1)
/// sp PV[i] now len >= i+1 (as not a terminal/leaf so has been extended)
///
/// when we push at ply 0, we become ply1, and we can clear pv[1]
impl Trail {
    pub fn new(root: Board) -> Self {
        Self {
            seldepth: 0,
            path: Variation::new(),
            pv_for_ply: vec![Variation::new(); LEN_PLY],
            score_for_ply: vec![Score::zero(); LEN_PLY],
            refutations: vec![],
            refutation_scores: vec![],
            tree_crit: TreeCrit::default(),
            chess_tree: ChessTree::new(root.clone()),
            root,
        }
    }

    pub fn clear(&mut self) {
        *self = Self::new(self.root.clone());
    }

    pub fn set_tree_crit(&mut self, crit: TreeCrit) {
        self.tree_crit = crit;
        if log::log_enabled!(target: "tree", log::Level::Info) {
            self.tree_crit.enabled = true;
        }
    }

    pub fn selective_depth(&self) -> Ply {
        self.seldepth
    }

    pub fn root(&self) -> &Board {
        &self.root
    }

    pub fn path(&self) -> &Variation {
        &self.path
    }

    fn board(&self, ply: i32) -> Board {
        let ply = ply as usize;
        self.root().make_moves_old(&self.path.take(ply))
    }

    pub fn pv(&self) -> &Variation {
        debug_assert!(self.pv_for_ply[0].validate(self.root()).is_ok(), "{self:#}");
        &self.pv_for_ply[0]
    }

    /// move goes from ply to ply+1
    /// null moves allowed
    pub fn push_move(&mut self, n: &Node, mv: Move) {
        let ply = n.ply;
        debug_assert_eq!(self.path.len(), ply as usize, "push move\n{self:#}");
        debug_assert!(
            mv.is_null()
                || self
                    .board(ply)
                    .validate_pseudo_legal_and_legal_move(mv)
                    .is_ok(),
            "push({ply}, {mv}) on {b}\n{self}",
            b = self.board(ply)
        );
        let ply = ply as usize;
        // self.boards[ply + 1] = self.boards[ply].make_move(&mv);
        // self.path.truncate(ply);
        self.path.push(mv);
        self.pv_for_ply[ply + 1].clear();
        self.seldepth = self.seldepth.max(self.path.len() as Ply);
        let n = Node {
            ply: n.ply + 1,
            depth: n.depth - 1,
            alpha: -n.beta,
            beta: -n.alpha,
        };

        if self.tree_crit.accept(&self.path) {
            self.chess_tree.merge(
                &self.path,
                NodeDetails {
                    n,
                    e: Event::MovePush,
                    sc: Score::INFINITY,
                    nt: NodeType::Unused,
                },
            )
        }
    }

    pub fn pop_move(&mut self, n: &Node, mv: Move) {
        debug_assert_eq!(self.path.last(), Some(mv));
        self.path.pop();
        debug_assert_eq!(n.ply as usize, self.path.len());
    }

    /// set pv to here (current pv)
    pub fn terminal(&mut self, n: &Node, sc: Score, e: Event) {
        let ply = n.ply as usize;
        self.pv_for_ply[ply] = self.path.clone();
        trace!("set_pv:\n{self}");
        if self.tree_crit.accept(&self.path) {
            self.chess_tree.merge(
                &self.path,
                NodeDetails {
                    n: n.clone(),
                    e,
                    sc,
                    nt: n.node_type(sc),
                },
            );
        }
    }

    /// mv at ply was good
    ///
    /// best pv for this ply
    ///
    /// PV[i] = CV + MV + PV[i+1].skip(i+1) )
    ///
    /// PV[i+1].skip(i+1) could be empty is its the first raise of that ply
    ///
    /// a move is an edge between nodes
    /// the first set of moves are at ply=0 so the var[0] takes you from ply0 to ply1
    /// the score of a "move" is really the score of the best position from that node,
    /// so is at move "from-ply"
    /// when we are processing a node, we are looking at candidate moves FROM THAT NODE
    ///
    pub fn alpha_raised(&mut self, n: &Node, sc: Score, mv: Move, e: Event) {
        let ply = n.ply as usize;
        // debug_assert_eq!(
        //     ply,
        //     self.path.len(),
        //     "update_pv: ply {ply} must == len(cv={})",
        //     self.path.display_san(self.root())
        // );

        let root = self.root();
        let path = &self.path;
        let mut var = path.clone();
        var.push(mv);
        // // debug_assert_eq!(var.last(), Some(&mv), "last moves don't match");
        let pv_above = &self.pv_for_ply[ply + 1];
        let extend = if !pv_above.is_empty() {
            self.pv_for_ply[ply + 1].skip(ply + 1)
        } else {
            Variation::new()
        };
        var.extend(&extend);
        debug_assert!(
            var.len() <= self.selective_depth() as usize,
            "seldepth {sd} var : {var} {self:#}",
            sd = self.selective_depth()
        );
        debug_assert!(
            var.validate(root).is_ok(),
            "update_pv: new pv at ply {ply}: {var} = cv ({path}) + mv ({mv}) + pv[{ply}+1][{ply}+1:] ({extend}) is invalid\nevent {e}\n{self:#}"
        );
        trace!("update_pv:\n{self}");
        if self.tree_crit.accept(&self.path) {
            self.chess_tree.merge(
                &var,
                NodeDetails {
                    n: n.clone(),
                    e,
                    sc,
                    nt: NodeType::ExactPv,
                },
            )
        }
        self.pv_for_ply[ply] = var;
    }

    pub fn ignore_move(&mut self, n: &Node, sc: Score, _mv: Move, _e: Event) {
        debug_assert_eq!(n.node_type(sc), NodeType::UpperAll, "node type {n} sc {sc}");
    }

    /// we dont actually make the move - we futility prune it first
    pub fn prune_move(&mut self, n: &Node, sc: Score, mv: Move, e: Event) {
        self.push_move(n, mv); // the move wont have been made
        if self.tree_crit.accept(&self.path) {
            self.chess_tree.merge(
                &self.path,
                NodeDetails {
                    n: n.clone(),
                    e,
                    sc,
                    nt: NodeType::UpperAll,
                },
            )
        }
        self.pop_move(n, mv);
    }
    pub fn prune_node(&mut self, n: &Node, sc: Score, e: Event) {
        debug_assert_eq!(self.path.len(), n.ply as usize, "{self:#}");
        if self.tree_crit.accept(&self.path) {
            self.chess_tree.merge(
                &self.path,
                NodeDetails {
                    n: n.clone(),
                    e,
                    sc,
                    nt: n.node_type(sc),
                },
            )
        }
    }

    /// low or high - look at bounds
    pub fn fail(&mut self, n: &Node, sc: Score, _mv: Move, e: Event) {
        // WORKS BUT SLOW
        // debug_assert!(
        //     n.node_type(sc) != NodeType::ExactPv,
        //     "node type {}",
        //     n.node_type(sc)
        // );
        // debug_assert!({
        //     let ply = n.ply as usize;
        //     let var = self.path.take(ply + 1); // up to and including
        //     debug_assert_eq!(
        //         var.validate(self.root()).map_err(|e| e.to_string()),
        //         Ok(()),
        //         "refuted(ply:{ply})\n{self}"
        //     );
        //     self.refutations.push(var);
        //     self.refutation_scores.push(sc);
        //     true
        // });
        if self.tree_crit.accept(&self.path) {
            self.chess_tree.merge(
                &self.path,
                NodeDetails {
                    n: n.clone(),
                    e,
                    sc,
                    nt: NodeType::LowerCut,
                },
            )
        }
    }

    pub fn take_tree(&mut self) -> ChessTree {
        let board = self.root().clone();
        mem::replace(&mut self.chess_tree, ChessTree::new(board))
    }
}

impl fmt::Debug for Trail {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "{}", self)?;
        } else {
            // this is used for tracing
            write!(f, "{}", self.path().to_uci())?;
        }
        Ok(())
    }
}

impl fmt::Display for Trail {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let root = self.root();
        let pv = &self.pv_for_ply[0];
        let path = &self.path;
        writeln!(f, "seld  {}", self.selective_depth())?;
        writeln!(f, "root  {root}")?;
        writeln!(f, "CV    {var} ({path})", var = path.display_san(root))?;
        writeln!(f, "PV    {var} ({pv})", var = pv.display_san(root))?;

        for (ply, pvp) in self.pv_for_ply.iter().enumerate() {
            if !pvp.is_empty() {
                writeln!(f, "PV{ply:>2}  {san} ({pvp})", san = pvp.display_san(root))?;
            }
        }
        for (i, refut) in self.refutations.iter().enumerate() {
            if !refut.is_empty() {
                match refut.validate(root) {
                    Ok(_) => writeln!(f, " R{i:>2}  {var}", var = refut.display_san(root)),
                    Err(e) => writeln!(f, " R{i:>2} {e}"),
                }?;
            }
        }
        if f.alternate() {
            writeln!(f, "{:#}", self.chess_tree)?;
        }
        writeln!(f)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Trail, Tree};
    use crate::{
        board::Board,
        catalog::Catalog,
        domain::{
            engine::Engine,
            trail::{displayable, NodeId, ROOT},
        },
        infra::utils::{Displayable, ToStringOr},
        search::{node::Node, timecontrol::TimeControl},
        variation::Variation,
        Algo,
    };
    use test_log::test;

    #[test]
    fn trail() {
        let board = Catalog::starting_board();
        let mut t = Trail::new(board.clone());
        println!("{t}");
        let n = Node::root(0);
        t.push_move(&n, board.parse_san_move("e4").unwrap());
        t.push_move(
            &n.new_child(),
            board.parse_san_variation("e4 e5").unwrap().last().unwrap(),
        );
        println!("e4 e5\n{t}");
    }

    #[test]
    fn display_tree() {
        let pos = Catalog::starting_position();
        let mut eng = Algo::new();
        eng.explainer.tree_crit.enabled = true;
        eng.explainer.tree_crit.max_ply = 5;
        let sr = eng.search(pos.clone(), TimeControl::Depth(1)).unwrap();
        println!(
            "score: {sc} {pv}",
            sc = sr.score().to_string_or("-"),
            pv = sr.pv().to_san(pos.board())
        );
        println!("tree...");
        {
            if let Some(tree) = sr.tree() {
                println!("{tree:#}");
                for nd in tree.arena.iter() {
                    println!("nd: {nd:?}");
                }
            }
        }
    }

    #[test]
    fn tree_basics() -> anyhow::Result<()> {
        let mut tree = Tree::new();
        let bd = Board::starting_pos();
        let a3 = bd.parse_san_variation("a3")?;
        let b3 = bd.parse_san_variation("b3")?;
        let a3e6 = bd.parse_san_variation("a3 e6")?;
        let a3e5 = bd.parse_san_variation("a3 e5")?;
        let a3a6 = bd.parse_san_variation("a3 a6")?;
        let a3a6b3 = bd.parse_san_variation("a3 a6 b3")?;

        tree.add(&a3, 2);
        tree.add(&a3e6, 4);
        tree.add(&a3e5, 6);
        tree.add(&a3a6, 8);
        tree.add(&b3, 10);
        tree.add(&a3a6b3, 12);

        let a3e6b3 = bd.parse_san_variation("a3 e6 b3")?;
        let a3e6b3a3 = bd.parse_san_variation("a3 e6 b3 Bxa3")?;
        tree.add(&a3e6b3, 22);
        tree.add(&a3e6b3a3, 24);

        println!("{tree:#?}");

        assert_eq!(tree.find_by_id(NodeId(0)), &ROOT);
        assert_eq!(tree.find_by_var(&Variation::new()), Some(&ROOT));
        assert_eq!(tree.find_by_index(0), Some(&ROOT));

        let var = bd.parse_san_variation("a3 a6 g3")?;
        assert_eq!(tree.find_by_var(&var), None);
        assert_eq!(tree.find_by_var(&a3).unwrap().index, 2);
        assert_eq!(tree.find_by_var(&a3a6b3).unwrap().index, 12);

        let id_of_a3 = tree.find_by_var(&a3).unwrap().id;
        assert_eq!(tree.children_of(ROOT.id).len(), 2);
        assert_eq!(tree.children_of(id_of_a3).len(), 3);

        let id_of_a3a6b3 = tree.find_by_var(&a3a6b3).unwrap();
        assert_eq!(tree.variation_of(id_of_a3a6b3.id), a3a6b3);

        println!("{tree}");
        println!(
            "{tree}",
            tree = Displayable(displayable(&tree, &Board::starting_pos()))
        );

        Ok(())
    }
}
