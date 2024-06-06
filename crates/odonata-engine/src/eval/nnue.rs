use std::cell::RefCell;
use std::fmt::{Debug, Display};
use std::path::Path;

use odonata_base::boards::Position;
use odonata_base::prelude::*;

use super::network::{Network, Network768xH2};

#[derive(Debug, Clone)]
pub enum Nnue {
    Nnue(NnueMixin<Network768xH2<f32>>),
    Nnue768H2(NnueMixin<Network768xH2<i16>>),
}

impl Display for Nnue {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let net = match self {
            Nnue::Nnue(nnue) => nnue.net.to_string(),
            Nnue::Nnue768H2(nnue) => nnue.net.to_string(),
        };
        write!(f, "nnue<{net}>")?;
        Ok(())
    }
}

impl Nnue {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Nnue> {
        let nnue_file = path.as_ref().to_path_buf();
        match nnue_file.to_string_lossy() {
            f if f.is_empty() => {
                let net = Network768xH2::<i16>::from_file("")?;
                let nnue = NnueMixin::<Network768xH2<i16>> {
                    wb_acc: RefCell::new(net.new_accumulators()),
                    last: RefCell::new(Board::new_empty()),
                    net,
                };
                Ok(Nnue::Nnue768H2(nnue))
            }
            f if f.ends_with(".bin") => {
                let net = Network768xH2::<i16>::from_file(nnue_file)?;
                let nnue = NnueMixin::<Network768xH2<i16>> {
                    wb_acc: RefCell::new(net.new_accumulators()),
                    last: RefCell::new(Board::new_empty()),
                    net,
                };
                Ok(Nnue::Nnue768H2(nnue))
            }
            f if f.ends_with("i16.yaml") => {
                let net = Network768xH2::<i16>::from_file(nnue_file)?;
                let nnue = NnueMixin::<Network768xH2<i16>> {
                    wb_acc: RefCell::new(net.new_accumulators()),
                    last: RefCell::new(Board::new_empty()),
                    net,
                };
                Ok(Nnue::Nnue768H2(nnue))
            }
            f if f.ends_with(".yaml") => {
                let net = Network768xH2::<f32>::from_file(nnue_file)?;
                let nnue = NnueMixin {
                    wb_acc: RefCell::new(net.new_accumulators()),
                    last: RefCell::new(Board::starting_pos()),
                    net,
                };
                Ok(Nnue::Nnue(nnue))
            }
            _ => unreachable!(),
        }
    }

    pub fn eval(&self, pos: &Position) -> i16 {
        match self {
            Nnue::Nnue(nnue) => nnue.eval(pos) as i16,
            Nnue::Nnue768H2(nnue) => nnue.eval(pos),
        }
    }

    pub fn eval_stateless(&self, b: &Board) -> i16 {
        match self {
            Nnue::Nnue(nnue) => nnue.eval_stateless(b) as i16,
            Nnue::Nnue768H2(nnue) => nnue.eval_stateless(b),
        }
    }

    pub fn new_game(&mut self) {
        match self {
            Nnue::Nnue(nnue) => nnue.new_game(),
            Nnue::Nnue768H2(nnue) => nnue.new_game(),
        }
    }
}

#[derive(Clone)]
pub struct NnueMixin<N: Network> {
    wb_acc: RefCell<N::Accumulators>,
    net:    Box<N>,
    last:   RefCell<Board>,
}

// impl<N: Network + Default> Default for NnueMixin<N> {
//     fn default() -> Self {
//         Self::from_file("").unwrap()
//     }
// }

impl<N: Network + Debug> Debug for NnueMixin<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "net         : {:?}", self.net)?;
        writeln!(f, "last eval bd: {}", self.last.borrow().to_fen())?;
        Ok(())
    }
}

impl<N: Network> NnueMixin<N> {

    pub fn new_game(&mut self) {
        let mut acc = self.net.new_accumulators();
        let b = Board::starting_pos();
        self.net.forward1(&mut acc, &b);
        self.wb_acc = RefCell::new(acc);
        self.last = RefCell::new(b);
    }

    #[cfg(test)]
    pub fn testing_eval_current(&self) -> N::Output {
        let turn = self.last.borrow().turn();
        self.net.forward2(turn, &self.wb_acc.borrow())
    }

    pub fn eval(&self, pos: &Position) -> N::Output {
        let b = pos.board();
        self.eval_stateless(b)
    }

    pub fn eval_stateless(&self, board: &Board) -> N::Output {
        let mut last = self.last.borrow_mut();
        let mut acc = self.wb_acc.borrow_mut();
        self.net.forward1_input(&mut acc, &last, board);
        *last = board.clone();
        self.net.forward2(board.turn(), &acc)
    }
}

#[cfg(test)]
mod tests {
    use odonata_base::infra::profiler::PerfProfiler;
    use odonata_base::Epd;
    use rand_chacha::rand_core::SeedableRng as _;
    use rand_chacha::ChaChaRng;
    use test_log::test;
    use tracing::{debug, info};

    use super::*;

    // #[test]
    // fn test_nn_state() -> Result<()> {
    //     let ns1 = NnueMixin::<Network768xH2<i16>>::default();
    //     let ns2 = ns1.clone();
    //     let b = Board::starting_pos();
    //     ns1.eval_stateless(&b);
    //     ns2.eval_stateless(&b);
    //     assert_eq!(ns1.testing_eval_current(), ns2.testing_eval_current());
    //     let mv = Move::parse_uci("d2d3", &b)?;
    //     let b1 = b.make_move(mv);

    //     ns1.eval_stateless(&b1);
    //     // ns2.push_move(mv);
    //     assert_eq!(
    //         ns1.testing_eval_current(),
    //         ns2.testing_eval_current(),
    //         "{:?} {:?}",
    //         ns1.wb_acc.borrow().0,
    //         ns2.wb_acc.borrow().0
    //     );

    //     Ok(())
    // }

    #[test]
    fn test_nn_random_moves() {
        let mut rand = ChaChaRng::seed_from_u64(1);
        let nnue1 = Nnue::from_file("").unwrap();
        let nnue2 = nnue1.clone();

        for i in 0..1000 {
            let epd = Epd::starting_pos();
            let var = epd.play_random_moves(i % 50, &mut rand);
            let Some(var) = var else { continue };
            let epd = Epd::from_var(epd.setup_board(), var.clone());

            debug!(
                "epd {i:<3}...{:<80} {}",
                epd.setup_board(),
                epd.played().to_san(&epd.setup_board())
            );

            let mut b = epd.setup_board();
            for mv in var.moves() {
                b.apply_move(mv);
                nnue2.eval_stateless(&b);
            }

            // final board
            let cp1 = nnue1.eval_stateless(&epd.board());
            let cp2 = nnue2.eval_stateless(&epd.board());
            assert_eq!(cp1, cp2, "#{i} (e) {epd} {nnue2:#?}");

            let setup = epd.setup_board();
            for i in 0..var.len() {
                let b = setup.make_moves(&var.take(i));
                nnue2.eval_stateless(&b);
            }
            // back to setup board
            let cp1 = nnue1.eval_stateless(&epd.setup_board());
            let cp2 = nnue2.eval_stateless(&epd.setup_board());
            assert_eq!(cp1, cp2, "#{i} (s) {epd} {nnue2:#?}");
        }
    }

    #[test]
    fn bench_nnue() {
        let mut board = Board::starting_pos();
        // let nnue = "../../crates/odonata-hce/resources/156789-256-epoch99.bin";
        // let nnue_128 = "../../crates/odonata-engine/resources/15678-128-epoch99.bin";
        // let nnue_256 = "../../crates/odonata-engine/resources/j5j29-256-epoch99.bin";
        let nnue_512 = "../../crates/odonata-engine/resources/r61-net.i16.bin";
        // let ns_128 = Nnue::from_file(nnue_128).unwrap();
        // let ns_256 = Nnue::from_file(nnue_256).unwrap();
        let ns_512 = Nnue::from_file(nnue_512).unwrap();
        // let _score = PerfProfiler::new("nnue128 startpos").bench(|| ns_128.eval_stateless(&board));
        // let _score = PerfProfiler::new("nnue256 startpos").bench(|| ns_256.eval_stateless(&board));
        let score = PerfProfiler::new("nnue512 startpos").bench(|| ns_512.eval_stateless(&board));
        info!("nnue score = {score} for startpos");
        let var = board.parse_san_variation("e4 e5 d4 exd4 Nf3 Bb4 c3").unwrap();
        let b = var
            .moves()
            .map(|m| {
                board.apply_move(m);
                board.clone()
            })
            .collect_vec();
        // var.moves().for_each(|_m| ns_128.pop_move());

        // let mut prof_mv0 = PerfProfiler::new("nnue mv0 (128)");
        // let mut prof_mv0_256 = PerfProfiler::new("nnue mv0 (256)");
        let mut prof_mv0 = PerfProfiler::new("nnue mv0 (512)");
        let mut prof_mv1 = PerfProfiler::new("nnue mv1");
        let mut prof_mv2 = PerfProfiler::new("nnue mv2");
        let mut prof_mv3c = PerfProfiler::new("nnue mv3c");
        let mut prof_mv4 = PerfProfiler::new("nnue mv4");
        let mut prof_mv5 = PerfProfiler::new("nnue mv5");
        let mut prof_mv6 = PerfProfiler::new("nnue mv6");
        let mut prof_pop6 = PerfProfiler::new("nnue pop6");
        let mut prof_pop5 = PerfProfiler::new("nnue pop5");
        let mut prof_pop4 = PerfProfiler::new("nnue pop4");
        // let mut prof_eval = PerfProfiler::new("nnue eval 128");
        let mut prof_eval_512 = PerfProfiler::new("nnue eval 512");
        // let mut prof_align = PerfProfiler::new("nnue align");
        // let mut prof_adj_6m = PerfProfiler::new("nnue adj-6");
        // let mut prof_adj_6p = PerfProfiler::new("nnue adj+6");

        prof_mv0.bench(|| ns_512.eval_stateless(&b[0]));
        prof_mv1.bench(|| ns_512.eval_stateless(&b[1]));
        prof_mv2.bench(|| ns_512.eval_stateless(&b[2]));
        prof_mv3c.bench(|| ns_512.eval_stateless(&b[3]));
        prof_mv4.bench(|| ns_512.eval_stateless(&b[4]));
        prof_mv5.bench(|| ns_512.eval_stateless(&b[5]));
        prof_mv6.bench(|| ns_512.eval_stateless(&b[6]));

        prof_pop6.bench(|| ns_512.eval_stateless(&b[5]));
        prof_pop5.bench(|| ns_512.eval_stateless(&b[4]));
        prof_pop4.bench(|| ns_512.eval_stateless(&b[3]));

        // prof_eval.bench(|| ns_512.eval_stateless(&b[3]));

        // prof_mv0_256.bench(|| ns_256.eval_stateless(&b[0]));
        // prof_mv0_512.bench(|| ns_512.eval_stateless(&b[0]));
        prof_eval_512.bench(|| ns_512.eval_stateless(&b[0]));
        // let mut pos2 = pos.clone();
        // pos2.eval_stateless(&b[0]);
        // pos2.eval_stateless(&b[1]);

        // prof_adj_6m.bench(|| ns_128.adj_move::<RM, ADD>(var[6]));
        // prof_adj_6p.bench(|| ns_128.adj_move::<ADD, RM>(var[6]));

        // prof_align.bench(|| ns_128.align(&pos2));
    }

    #[test]
    fn bench_profile_nnue() {
        let mut b = Board::starting_pos();
        let nnue = "../../crates/odonata-engine/resources/r61-net.i16.bin";
        let ns = Nnue::from_file(nnue).unwrap();
        let var = b.parse_san_variation("e4 e5 d4 exd4 Nf3 Bb4 c3").unwrap();
        let boards = var
            .moves()
            .map(|m| {
                b.apply_move(m);
                b.clone()
            })
            .collect_vec();
        let mut prof = PerfProfiler::new("nnue assorted");
        let tot = prof.bench(|| {
            let mut sc = 0;
            for b in boards.iter() {
                sc += ns.eval_stateless(b);
            }
            for b in boards.iter().rev() {
                sc -= ns.eval_stateless(b);
            }
            sc
        });
        assert_eq!(tot, 0);
    }
}
