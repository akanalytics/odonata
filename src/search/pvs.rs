
use crate::{Algo, board::Board};
use crate::search::node::Node;
use crate::bound::NodeType;
use crate::infra::component::Component;
use crate::types::Ply;
use std::fmt;
use serde::{Deserialize, Serialize};

use super::node::Category;






#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Pvs {
    pub enabled: bool,
    pub min_depth: Ply,
}

impl Component for Pvs {
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {
    }
}

impl Default for Pvs {
    fn default() -> Self {
        Pvs {
            enabled: true,
            min_depth: 2, 
        }
    }
}

// once we have an alpha raising move, search remainder using null window and see if they raise alpha (or cut)
// re-search full-window if they do, to get a score
impl Algo {
    pub fn pvs_permitted(&mut self, nt: NodeType,_b: &Board, n: &Node, mv_num: u32) -> bool {
        if !self.pvs.enabled {
            return false;
        }
        if mv_num <= 1 {
            return false;
        }
        if n.depth < self.pvs.min_depth {
            return false;
        }
        if nt !=  NodeType::ExactPv {
            return false;
        }
        if !n.alpha.is_numeric()  {
            return false;
        }
        if n.is_zw()  {
            // no PVS in PVS search
            return false;
        }
        self.counts.inc(n, Category::Pvs);
        true
    }

}




impl fmt::Display for Pvs {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:#?}", self)?;
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::utils::*;
    use crate::search::timecontrol::*;
    use crate::tags::*;
    use crate::search::engine::*;

    // use crate::search::timecontrol::*;


    #[test]
    #[ignore]
    fn test_pvs() {
        let mut engine = Engine::new();

        let positions = &Catalog::example_game()[101..102];
        let mut node_count = 0;
        for pos in positions {
            engine.new_game();
            let suggested_depth = pos.acd().unwrap();
            engine.algo.set_timing_method(TimeControl::Depth(suggested_depth-1));
            engine.set_position(pos.clone());

            engine.search();
            let mut results = engine.algo.results_as_position().clone();
            let nodes = results.acn().unwrap();
            node_count += nodes;

            // just leave acd
            results.tags_mut().remove(Tag::PV);
            results.tags_mut().remove(Tag::SM);
            results.tags_mut().remove(Tag::BM);
            results.tags_mut().remove(Tag::CE);
            results.tags_mut().remove(Tag::ACN);
            println!("{:>12} {:>12} {}", Formatting::format_u128(nodes), Formatting::format_u128(node_count), results);
        }
    }
}
