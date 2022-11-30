use std::fmt;

use itertools::Itertools;

use crate::{
    board::Board,
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

#[derive(Clone)]
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
    parent: NodeId(0),
    mv: Move::NULL_MOVE,
};

impl Tree {
    fn new() -> Self {
        Tree { nodes: vec![ROOT] }
    }

    fn add(&mut self, var: &Variation, index: usize) {
        let mv = *var.last().expect("variation empty - cannot add root node");
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
                .filter(|&&tn| tn.mv == *mv)
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
        // root has 0 as parent, so filter it out (via skip) as it can never be a child
        self.nodes
            .iter()
            .skip(1)
            .filter_map(|tn| if tn.parent == id { Some(tn) } else { None })
            .collect_vec()
    }
}

fn display_node(
    tree: &Tree,
    f: &mut fmt::Formatter,
    payload: &impl Fn(&TreeNode) -> String,
    indent: Vec<bool>,
    id: NodeId,
) -> fmt::Result {
    let node = tree.find_by_id(id);
    let children = tree.children_of(node.id);
    for (i, &child) in children.iter().enumerate() {
        let last = i >= children.len() - 1;
        // print single line
        for s in &indent {
            write!(f, "{}", if *s { "    " } else { "|   " })?;
        }
        write!(f, "{}", if last { "└── " } else { "├── " })?;
        writeln!(f, "{}", payload(tree.find_by_id(child.id)))?;

        // for _ in spaces.len()..5 {
        //     write!(f, "    ")?;
        // }

        // var.push(node.mv);
        // writeln!(f, "{}  --> {}", node, var.to_san(b))?;

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
        let payload = |tn: &TreeNode| format!("{}", tn.mv.to_uci());
        display_node(self, f, &payload, vec![], ROOT.id)
    }
}

fn displayable<'a>(t: &'a Tree, bd: &'a Board) -> impl Fn(&mut fmt::Formatter) -> fmt::Result + 'a {
    fn format_data(tree: &Tree, bd: &Board, tn: &TreeNode, f: &mut fmt::Formatter) -> fmt::Result {
        let var = tree.variation_of(tn.id);
        if let Some(stem) = &var.stem() {
            let san = bd.make_moves_old(stem).to_san(&tn.mv);
            write!(f, "{san}")?;
        }
        Ok(())
    }

    // the displayable function
    |f: &mut fmt::Formatter| -> fmt::Result {
        let payload = |tn: &TreeNode| format!("{}", Displayable(|f| format_data(t, bd, tn, f)));
        display_node(t, f, &payload, vec![], ROOT.id)
    }
}

// line, branch or variation
#[derive(Clone, Debug)]
pub struct Trail {
    seldepth: Ply,
    path: Variation,
    pv_for_ply: Vec<Variation>,
    score_for_ply: Vec<Score>,
    root: Board,
    refutations: Vec<Variation>,
    refutation_scores: Vec<Score>,
}

impl Trail {
    pub fn new(root: Board) -> Self {
        Self {
            seldepth: 0,
            path: Variation::new(),
            pv_for_ply: vec![Variation::new(); LEN_PLY],
            score_for_ply: vec![Score::zero(); LEN_PLY],
            root,
            refutations: vec![],
            refutation_scores: vec![],
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
        debug_assert!(self.pv_for_ply[0].validate(self.root()).is_ok(), "{self}");
        &self.pv_for_ply[0]
    }

    /// move goes from ply to ply+1
    /// null moves allowed
    pub fn push(&mut self, n: &Node, mv: Move) {
        let ply = n.ply;
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
        self.path.truncate(ply);
        self.path.push(mv);
        self.seldepth = self.seldepth.max(self.path.len() as Ply);
    }

    /// set pv to here (current pv)
    pub fn terminal(&mut self, n: &Node, sc: Score, e: Event) {
        let ply = n.ply as usize;
        self.pv_for_ply[ply] = self.path.skip(ply);
        trace!("set_pv:\n{self}")
    }

    /// mv at ply was good
    ///
    /// best pv for this ply
    ///
    /// pv[ply] = cv[0..ply] (not stored) + cv[ply] + pv[ply+1]
    ///
    pub fn alpha_raised(&mut self, n: &Node, s: Score, mv: Move, e: Event) {
        let ply = n.ply as usize;
        debug_assert!(
            ply < self.path.len(),
            "update_pv: ply {ply} must < len(cv={})",
            self.path.display_san(self.root())
        );
        let mut var = self.path.skip(ply).take(1);
        var.extend(&self.pv_for_ply[ply + 1]);
        debug_assert!(
            self.pv_for_ply[ply]
                .validate(&self.board(ply as i32))
                .is_ok(),
            "update_pv: ply {ply} has invalid cv[0..ply] + pv\n{self}"
        );
        self.pv_for_ply[ply] = var;
        trace!("update_pv:\n{self}")
    }

    pub fn ignore_move(&mut self, n: &Node, sc: Score, mv: Move, e: Event) {}

    pub fn prune_node(&mut self, n: &Node, sc: Score, e: Event) {}

    /// low or high - look at bounds
    pub fn fail(&mut self, n: &Node, sc: Score, mv: Move, e: Event) {
        debug_assert!({
            let ply = n.ply as usize;
            let var = self.path.take(ply + 1); // up to and including
            debug_assert_eq!(
                var.validate(self.root()).map_err(|e| e.to_string()),
                Ok(()),
                "refuted(ply:{ply})\n{self}"
            );
            self.refutations.push(var);
            self.refutation_scores.push(sc);
            true
        });
    }
}

impl fmt::Display for Trail {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "root {board}", board = self.root())?;
        let pv = &self.pv_for_ply[0];
        writeln!(f, "CV    {var}", var = self.path.display_san(self.root()))?;
        writeln!(f, "PV    {var}", var = pv.display_san(self.root()))?;

        for (ply, pvp) in self.pv_for_ply.iter().enumerate() {
            if !pvp.is_empty() && ply <= self.path.len() {
                let pv_stem = self.path.take(ply); // <-- safe as tested above

                // let board_at_ply = self.root().make_moves_old(&pv_stem);
                write!(f, "PV{ply:>2}  ")?;
                write!(f, "{pv_stem}.", pv_stem = pv_stem.display_san(&self.root()))?;
                if pv_stem.validate(&self.root()).is_ok()
                    && pvp.validate(&self.root().make_moves_old(&pv_stem)).is_ok()
                {
                    writeln!(
                        f,
                        "{pvp}",
                        pvp = pvp.display_san(&self.root().make_moves_old(&pv_stem))
                    )?;
                } else {
                    writeln!(f, "[{pvp}]", pvp = pvp.to_uci())?;
                }
                // let board_at_ply = self.root().make_moves_old(&pv_stem);

                // pv_stem.extend(pvp);
                // writeln!(f,
                //     "| {pv_stem}"
                //     pv_stem = pv_stem.display_san(&self.root()))?;

                // // var = pvp.display_san(&board_at_ply)
                // )?;
            }
        }
        for (i, refut) in self.refutations.iter().enumerate() {
            if !refut.is_empty() {
                match refut.validate(self.root()) {
                    Ok(_) => writeln!(f, " R{i:>2}  {var}", var = refut.display_san(self.root())),
                    Err(e) => writeln!(f, " R{i:>2} {e}"),
                }?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Trail, Tree};
    use crate::{
        board::Board,
        catalog::Catalog,
        domain::trail::{displayable, NodeId, ROOT},
        infra::utils::Displayable,
        search::node::Node,
        variation::Variation,
    };

    #[test]
    fn trail() {
        let board = Catalog::starting_board();
        let mut t = Trail::new(board.clone());
        println!("{t}");
        t.push(&Node::root(0), board.parse_san_move("e4").unwrap());
        t.push(
            &Node::root(1),
            *board.parse_san_variation("e4 e5").unwrap().last().unwrap(),
        );
        println!("e4 e5\n{t}");
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
