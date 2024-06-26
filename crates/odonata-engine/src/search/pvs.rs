use std::fmt;

use odonata_base::domain::node::Node;
use odonata_base::domain::BoundType;
use odonata_base::infra::component::Component;
use odonata_base::prelude::*;

#[derive(Clone, Debug)]
pub struct Pvs {
    pub enabled:   bool,
    pub min_depth: Ply,
    pub min_ply:   Ply,
}

impl Component for Pvs {
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {}
}

impl Default for Pvs {
    fn default() -> Self {
        Pvs {
            enabled:   true,
            min_depth: 2,
            min_ply:   0,
        }
    }
}

impl Configurable for Pvs {
    fn set(&mut self, p: Param) -> Result<bool> {
        self.enabled.set(p.get("enabled"))?;
        self.min_depth.set(p.get("min_depth"))?;
        self.min_ply.set(p.get("min_ply"))?;
        Ok(p.is_modified())
    }
}

impl fmt::Display for Pvs {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{self:#?}")
    }
}

// once we have an alpha raising move, search remainder using null window and see if they raise alpha (or cut)
// re-search full-window if they do, to get a score
impl Pvs {
    pub fn permitted(&mut self, _nt: BoundType, _b: &Board, n: &Node, mv_num: u32) -> bool {
        if !self.enabled {
            return false;
        }
        if mv_num <= 1 {
            return false;
        }

        if n.ply < self.min_ply {
            return false;
        }

        // includes QS
        if n.depth < self.min_depth {
            return false;
        }
        // if nt != NodeType::ExactPv {
        //     return false;
        // }
        // if !n.alpha.is_numeric() {
        //     return false;
        // }
        // if n.is_zw() {
        //     // no PVS in PVS search
        //     return false;
        // }
        true
    }
}

#[cfg(test)]
mod tests {

    // #[test]
    // #[ignore]
    // fn test_pvs() {
    //     let mut engine = ThreadedSearch::new();

    //     let positions = &Catalog::example_game()[101..102];
    //     let mut node_count = 0;
    //     for pos in positions {
    //         engine.new_game();
    //         let suggested_depth = pos.int("acd").unwrap() as i32;
    //         engine
    //             .search
    //             .set_timing_method(TimeControl::Depth(suggested_depth - 1));
    //         engine.set_position(pos.clone());

    //         engine.search_sync();
    //         let mut results = engine.search.results_as_position().clone();
    //         let nodes = results.int("acn").unwrap();
    //         node_count += nodes;

    //         // just leave acd
    //         results.tags_mut().pv = None;
    //         results.tags_mut().supplied_move = None;
    //         results.tags_mut().best_moves = None;
    //         results.tags_mut().centipawn_evaluation = None;
    //         results.tags_mut().acn = None;
    //         println!("{nodes:>12} {node_count:>12} {results}");
    //     }
    // }
}
