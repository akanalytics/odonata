
use crate::board::Board;
use crate::search::node::Node;
use crate::eval::score::Score;
use crate::bound::NodeType;
use crate::infra::parsed_config::{Component};
// use crate::{debug, logger::LogInit};
use crate::types::Ply;
use std::fmt;
use serde::{Deserialize, Serialize};






#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
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
impl Pvs {
    pub fn permitted(&self, nt: NodeType,_b: &Board, node: &Node) -> bool {
        if !self.enabled {
            return false;
        }
        if node.depth < self.min_depth {
            return false;
        }
        if nt !=  NodeType::Pv {
            return false;
        }
        if !node.alpha.is_numeric()  {
            // no PVS in PVS search
            return false;
        }
        if node.alpha + Score::from_cp(1) == node.beta  {
            // no PVS in PVS search
            return false;
        }
 
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
