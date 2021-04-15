use create:algo::Algo;


pub fn search(&mut self, mut board: Board) -> Algo {
    self.search_stats = SearchStats::new();
    self.current_best = None;
    self.overall_best_move = Move::NULL_MOVE;
    self.score = Score::default();
    self.clock_checks = 0;
    self.task_control.set_running();
    self.range = if let TimeControl::Depth(depth) = self.move_time_estimator.time_control {
        if self.iterative_deepening {
            1..depth + 1
        } else {
            depth..depth + 1
        }
    } else {
        // regardless of iterative deeping, we apply it if no explicit depth given
        1..MAX_PLY as u32
    };

    for depth in self.range.clone() {
        self.set_iteration_depth(depth);
        let mut root_node = Node::new_root(&mut board);
        let stats = &mut self.search_stats;
        let mut sp = SearchProgress::from_search_stats(stats);
        self.move_time_estimator.calculate_etimates_for_ply(depth, stats);
        stats.record_time_estimate(depth, &self.move_time_estimator.time_estimate);
        
        if self.score.is_mate() || self.move_time_estimator.probable_timeout(stats) {
            break;
        }
        self.score = Score::default();
        self.pv_table = PvTable::new(MAX_PLY);
        self.search_stats.clear_node_stats();
        let clock = Clock::new();
        // println!("Iterative deepening... ply {}", depth);

        self.alphabeta(&mut root_node);
        
        self.search_stats.record_time_actual(depth, &clock.elapsed());
        if !self.task_control.is_cancelled() {
            self.score = root_node.score;
            self.pv = self.pv_table.extract_pv();
            self.pv_table = self.pv_table.clone();
            self.current_best = Some(self.pv[0]);
            sp = SearchProgress::from_search_stats(&self.search_stats());
            sp.pv = Some(self.pv.clone());
            sp.score = Some(self.score);
            self.task_control.invoke_callback(&sp);
        } else {
            self.task_control.invoke_callback(&sp);
            break;
        }
    }

    self.overall_best_move = self.pv()[0];
    let sp = SearchProgress::from_best_move(Some(self.overall_best_move()));
    self.task_control.invoke_callback(&sp);
    self.clone()
}