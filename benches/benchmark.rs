use odonata::bitboard::bb_classical::ClassicalBitboard;
use odonata::bitboard::bb_hyperbola::Hyperbola;
use odonata::bitboard::bb_magic::*;
use odonata::bitboard::bb_sliders::*;
use odonata::bitboard::bitboard::*;
use odonata::bitboard::precalc::*;
use odonata::bitboard::square::*;
use odonata::board::boardcalcs::*;
use odonata::board::makemove::*;
use odonata::board::rules::*;
use odonata::board::*;
use odonata::bound::NodeType;
use odonata::cache::hasher::*;
use odonata::cache::tt2::*;
use odonata::catalog::*;
use odonata::domain::material::*;
use odonata::eval::eval::*;
use odonata::eval::score::*;
use odonata::eval::see::See;
use odonata::globals::constants::*;
use odonata::infra::component::*;
use odonata::movelist::*;
use odonata::mv::*;
use odonata::perft::Perft;
use odonata::phaser::Phaser;
use odonata::pvtable::*;
use odonata::search::algo::Algo;
use odonata::search::move_orderer::*;
use odonata::search::node::Node;
use odonata::search::timecontrol::TimeControl;
use odonata::types::*;
use odonata::utils::*;
use odonata::variation::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Instant;

use criterion::measurement::Measurement;
use criterion::*;
use log::*;

use odonata::infra::black_box;

criterion_group!(
    benches,
    benchmark_search,
    benchmark_thread,
    benchmark_perft,
    // benchmark_mate_in_2,
    benchmark_ordering,
    benchmark_eval,
    bb_calcs,
    sq_calcs,
    board_calcs,
    benchmark_attacks,
    make_move,
    hash_move,
    hash_board,
    legal_moves,
    bench_moveordering,
    benchmark_bitboard,
    piece_to_upper_char,
    piece_to_char,
    bench_logging,
    bench_chooser_struct,
    bench_chooser_wb,
    bench_chooser_array,
    benchmark_score,
    benchmark_array,
    bench_insufficient_material,
    bench_pvtable,
    bench_shared_mem,
);

// criterion_group!(
//     name = benches;
//     config = Criterion::default().with_measurement(PerfMeasurement::new(PerfMode::Instructions));
//     targets = benchmark_perft, benchmark_bitboard
// );

criterion_main!(benches);

fn bb_calcs(c: &mut Criterion) {
    let mut group = c.benchmark_group("bb_calcs");
    let bbs: Vec<Bitboard> = (0..64).into_iter().map(|sq| Bitboard::from_sq(sq)).collect();

    group.bench_function("first_square", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            for _ in 0..n {
                for bb in bbs.iter() {
                    black_box(black_box(bb).first_square());
                }
            }
            t.elapsed() / 64 as u32
        })
    });

    group.bench_function("first_squareX", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            for _ in 0..n {
                black_box(black_box(Bitboard::from_sq(0)).first_square());
                black_box(black_box(Bitboard::from_sq(8)).first_square());
                black_box(black_box(Bitboard::from_sq(16)).first_square());
                black_box(black_box(Bitboard::from_sq(24)).first_square());
                black_box(black_box(Bitboard::from_sq(32)).first_square());
                black_box(black_box(Bitboard::from_sq(40)).first_square());
                black_box(black_box(Bitboard::from_sq(48)).first_square());
                black_box(black_box(Bitboard::from_sq(56)).first_square());
            }
            t.elapsed() / 8 as u32
        })
    });

    group.bench_function("last_square", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            for _ in 0..n {
                for bb in bbs.iter() {
                    black_box(black_box(bb).last_square());
                }
            }
            t.elapsed() / 64 as u32
        })
    });
    group.bench_function("popcount", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            for _ in 0..n {
                for bb in bbs.iter() {
                    black_box(black_box(bb).popcount());
                }
            }
            t.elapsed() / 64 as u32
        })
    });

    group.finish();
}

fn benchmark_bitboard<T: Measurement>(c: &mut Criterion<T>) {
    let mut g = c.benchmark_group("bitboard");
    let n1 = 1u64 << 3;
    let n2 = 1u64 << 4;
    let n3 = 1u64 << 5;
    g.bench_function("bitwise_handcrafted", |b| {
        b.iter(|| {
            let a = black_box(n1) | black_box(n2);
            let b = a & black_box(n3);
            black_box(b);
        });
    });
    let n1 = Bitboard::D1;
    let n2 = Bitboard::E1;
    let n3 = Bitboard::F1;
    g.bench_function("bitwise_bitboard", |b| {
        b.iter(|| {
            let a = black_box(n1) | black_box(n2);
            let b = a & black_box(n3);
            black_box(b);
        });
    });
    g.bench_function("shl", |b| {
        b.iter(|| {
            black_box(Bitboard::A3 << 2);
        });
    });
    g.bench_function("wrapping_shl", |b| {
        b.iter(|| {
            black_box(4u64.wrapping_shl(2));
        });
    });
    g.finish();
}

fn piece_to_upper_char(c: &mut Criterion) {
    c.bench_function("piece_to_upper_char", |b| {
        b.iter(|| {
            black_box(Piece::Pawn.to_upper_char());
            black_box(Piece::Knight.to_upper_char());
        });
    });
}

fn piece_to_char(c: &mut Criterion) {
    c.bench_function("piece_to_char", |b| {
        b.iter(|| {
            black_box(Piece::Pawn.to_char(Some(Color::White)));
            black_box(Piece::Knight.to_char(Some(Color::Black)));
        });
    });
}

fn benchmark_perft<T: Measurement>(c: &mut Criterion<T>) {
    let mut pos = Catalog::starting_position();
    let mut group = c.benchmark_group("perft");
    group.sample_size(10);
    group.bench_function("perft5", |b| {
        b.iter(|| {
            black_box(Perft::perft(&mut pos.board_mut(), black_box(5)));
        });
    });
    group.bench_function("perft6", |b| {
        b.iter(|| {
            black_box(Perft::perft(&mut pos.board_mut(), black_box(6)));
        });
    });
    group.finish();
}

fn benchmark_score(c: &mut Criterion) {
    let mut group = c.benchmark_group("score");
    group.bench_function("handcrafted", |b| {
        b.iter(|| {
            black_box(black_box(5) > black_box(4));
        });
    });
    group.bench_function("score enum", |b| {
        b.iter(|| {
            black_box(Score::from_cp(black_box(5)) > Score::from_cp(black_box(4)));
        });
    });
    group.finish();
}

fn make_move(c: &mut Criterion) {
    let pos = Catalog::starting_position();
    let mv1 = pos.board().parse_uci_move("e2e4").unwrap();
    let mv2 = pos.board().parse_uci_move("b1c3").unwrap();
    c.bench_function("makemove", |b| {
        b.iter(|| {
            black_box(pos.board().make_move(black_box(&mv1)));
            black_box(pos.board().make_move(black_box(&mv2)));
        });
    });
}

fn hash_move(c: &mut Criterion) {
    let board = Catalog::perft_cpw_number3().0;
    let moves = board.legal_moves();
    //let hasher = Hasher::new(1);
    c.bench_function("hash_move", |b| {
        b.iter(|| {
            for mv in moves.iter() {
                let hasher = black_box(Hasher::default());
                black_box(hasher.hash_move(black_box(mv), black_box(&board)));
            }
        });
    });
}

fn hash_board(c: &mut Criterion) {
    let pos = Catalog::starting_position();
    let board = pos.board();
    let hasher = Hasher::new(1);
    c.bench_function("hash_board", |b| {
        b.iter(|| {
            black_box(hasher.hash_board(black_box(&board)));
        });
    });
}

fn legal_moves(c: &mut Criterion) {
    let pos = Catalog::starting_position();
    let board = pos.board();
    let mut ml = MoveList::new();
    c.bench_function("legal_moves_into", |b| {
        b.iter(|| {
            black_box(black_box(&board).legal_moves_into(&mut ml));
            ml.clear();
        });
    });
}

fn sq_calcs(c: &mut Criterion) {
    let mut group = c.benchmark_group("sq_calcs");
    // let bbs: Vec<Bitboard> = (0..64).into_iter().map(|sq| Bitboard::from_sq(sq)).collect();

    group.bench_function("between", |b| {
        let bb = BitboardDefault::default();
        b.iter_custom(|n| {
            let t = Instant::now();
            for _ in 0..n {
                for s1 in Bitboard::all().squares() {
                    black_box(Bitboard::all().squares().map(|s2| black_box(bb.between(s1, s2))).count());
                }
            }
            t.elapsed() / (64 * 64) as u32
        });
    });
    group.bench_function("calc_line_through", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            for _ in 0..n {
                for s1 in Bitboard::all().squares() {
                    black_box(
                        Bitboard::all()
                            .squares()
                            .map(|s2| black_box(Square::calc_line_through(s1, s2)))
                            .count(),
                    );
                }
            }
            t.elapsed() / (64 * 64) as u32
        });
    });
    group.bench_function("bounding_rectangle", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            for _ in 0..n {
                for s1 in Bitboard::all().squares() {
                    black_box(
                        Bitboard::all()
                            .squares()
                            .map(|s2| black_box(Square::bounding_rectangle(s1, s2)))
                            .count(),
                    );
                }
            }
            t.elapsed() / (64 * 64) as u32
        });
    });
    group.bench_function("calc_chebyshev_distance", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            for _ in 0..n {
                for s1 in Bitboard::all().squares() {
                    black_box(
                        Bitboard::all()
                            .squares()
                            .map(|s2| black_box(Square::calc_chebyshev_distance(s1, s2)))
                            .count(),
                    );
                }
            }
            t.elapsed() / (64 * 64) as u32
        });
    });
    group.finish();
}

fn board_calcs(c: &mut Criterion) {
    let mut group = c.benchmark_group("board_calcs");
    let positions = &Catalog::win_at_chess();

    group.bench_function("clone", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|p| {
                black_box(p.board().clone());
            });
            t.elapsed() / positions.len() as u32
        })
    });

    group.bench_function("draw_outcome", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|p| {
                black_box(p.board().draw_outcome());
            });
            t.elapsed() / positions.len() as u32
        })
    });

    group.bench_function("legal_moves", |b| {
        let mut ml = MoveList::new();
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|p| {
                black_box(p.board().legal_moves_into(&mut ml));
                ml.clear();
            });
            t.elapsed() / positions.len() as u32
        })
    });

    group.bench_function("has_legal_moves", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|p| {
                black_box(p.board().has_legal_moves());
            });
            t.elapsed() / positions.len() as u32
        })
    });

    group.bench_function("is_in_check", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|p| {
                black_box(p.board().is_in_check(Color::White));
                black_box(p.board().is_in_check(Color::Black));
            });
            t.elapsed() / 2 / positions.len() as u32
        })
    });

    let bams: Vec<(Board, Move)> = positions.iter().map(|p| (p.board().clone(), p.bm().unwrap()[0])).collect();
    group.bench_function("will_check_them", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            bams.iter().cycle_n(n).for_each(|bam| {
                black_box(bam.0.will_check_them(&bam.1));
            });
            t.elapsed() / positions.len() as u32
        })
    });

    let mut tt = TranspositionTable2::default();
    tt.new_game();
    group.bench_function("tt_probe", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|p| {
                black_box(tt.probe_by_board(p.board(), 4, 0));
            });
            t.elapsed() / positions.len() as u32
        })
    });

    group.bench_function("tt_store", |b| {
        let entry = TtNode {
            score: Score::from_cp(100),
            depth: 1,
            nt: NodeType::ExactPv,
            bm: Move::NULL_MOVE,
        };
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|p| {
                black_box(tt.store(p.board().hash(), entry));
            });
            t.elapsed() / positions.len() as u32
        })
    });

    let bams: Vec<(Board, Move)> = positions.iter().map(|p| (p.board().clone(), p.bm().unwrap()[0])).collect();
    group.bench_function("make_move + hash + clone", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            bams.iter().cycle_n(n).for_each(|bam| {
                black_box(bam.0.make_move(&bam.1));
            });
            t.elapsed() / positions.len() as u32
        })
    });

    group.bench_function("is_pseudo_legal_move", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            bams.iter().cycle_n(n).for_each(|bam| {
                black_box(bam.0.is_pseudo_legal_move(&bam.1));
            });
            t.elapsed() / positions.len() as u32
        })
    });

    group.bench_function("is_legal_move", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            bams.iter().cycle_n(n).for_each(|bam| {
                black_box(bam.0.is_legal_move(&bam.1));
            });
            t.elapsed() / positions.len() as u32
        })
    });

    let bams: Vec<(Board, Move)> = positions.iter().map(|p| (p.board().clone(), p.bm().unwrap()[0])).collect();
    group.bench_function("hash_move", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            bams.iter().cycle_n(n).for_each(|bam| {
                black_box(Hasher::default().hash_move(&bam.1, &bam.0));
            });
            t.elapsed() / positions.len() as u32
        })
    });
    let bams: Vec<(Board, Move)> = positions.iter().map(|p| (p.board().clone(), p.bm().unwrap()[0])).collect();
    group.bench_function("hash_board", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            bams.iter().cycle_n(n).for_each(|bam| {
                black_box(Hasher::default().hash_board(&bam.0));
            });
            t.elapsed() / positions.len() as u32
        })
    });
    let bams: Vec<(Board, Move)> = positions.iter().map(|p| (p.board().clone(), p.bm().unwrap()[0])).collect();
    group.bench_function("threats_to raw", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            bams.iter().cycle_n(n).for_each(|bam| {
                black_box(BoardCalcs::all_attacks_on(black_box(&bam.0), Color::White, bam.0.occupied()));
                black_box(BoardCalcs::all_attacks_on(black_box(&bam.0), Color::Black, bam.0.occupied()));
            });
            t.elapsed() / 2 / positions.len() as u32
        })
    });
    group.bench_function("threats_to (memoise)", |b| {
        let bams: Vec<(Board, Move)> = positions.iter().map(|p| (p.board().clone(), p.bm().unwrap()[0])).collect();
        b.iter_custom(|n| {
            let t = Instant::now();
            bams.iter().cycle_n(n).for_each(|bam| {
                black_box(&bam.0.all_attacks_on(Color::White));
                black_box(&bam.0.all_attacks_on(Color::Black));
            });
            t.elapsed() / 2 / positions.len() as u32
        })
    });
    let bams: Vec<(Board, Move)> = positions.iter().map(|p| (p.board().clone(), p.bm().unwrap()[0])).collect();
    group.bench_function("threats_to + clone", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            bams.iter().cycle_n(n).for_each(|bam| {
                black_box(&bam.0.clone().all_attacks_on(Color::White));
                black_box(&bam.0.clone().all_attacks_on(Color::Black));
            });
            t.elapsed() / 2 / positions.len() as u32
        })
    });
    group.finish();
}

fn benchmark_ordering(c: &mut Criterion) {
    let mut group = c.benchmark_group("ordering");
    let positions = &Catalog::win_at_chess();
    let see = See::default();
    let movelists: Vec<MoveList> = positions.iter().map(|p| p.board().legal_moves()).collect();
    group.bench_function("clone", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            movelists.iter().cycle_n(n).for_each(|ml| {
                black_box(ml.clone());
            });
            t.elapsed() / movelists.len() as u32
        })
    });
    group.bench_function("reverse + clone", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            movelists.iter().cycle_n(n).for_each(|ml| {
                black_box(black_box(ml.clone()).reverse());
            });
            t.elapsed() / movelists.len() as u32
        })
    });
    group.bench_function("sort + clone", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            movelists.iter().cycle_n(n).for_each(|ml| {
                black_box(black_box(ml.clone()).sort_unstable_by_key(Move::mvv_lva_score));
            });
            t.elapsed() / movelists.len() as u32
        })
    });
    group.bench_function("sort_by_cached_key + clone", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            movelists.iter().cycle_n(n).for_each(|ml| {
                black_box(black_box(ml.clone()).sort_by_cached_key(Move::mvv_lva_score));
            });
            t.elapsed() / movelists.len() as u32
        })
    });
    group.bench_function("see", |b| {
        b.iter_custom(|n| {
            let mut count = 0;
            let t = Instant::now();
            movelists.iter().enumerate().cycle_n(n).for_each(|(i, ml)| {
                let pos = &positions[i];
                for mv in ml.iter().filter(|&mv| mv.is_capture()) {
                    see.eval_move_see(pos.board(), *mv);
                    count += 1;
                }
            });
            t.elapsed() / (count / n) as u32
        })
    });
    let mut orderer = MoveOrderer::new();
    let mut algo = Algo::new();
    // const PLY: Ply = 3;
    let node = Node::root(3);

    const TT_MOVE: Move = Move::NULL_MOVE;
    orderer.order = MoveType::vec_from_string("SHIGKPQBE").unwrap();
    group.bench_function("SHIGKPQBE", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|pos| {
                let mut sorted_moves = orderer.create_sorted_moves(node, pos.board(), TT_MOVE, Move::NULL_MOVE);
                while let Some(mv) = sorted_moves.next_move(pos.board(), &mut algo) {
                    black_box(&mv);
                }
            });
            t.elapsed() / positions.len() as u32
        })
    });
    orderer.order = MoveType::vec_from_string("SHIgKPQBE").unwrap();
    group.bench_function("SHIgKPQBE - deferred sort", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|pos| {
                let mut sorted_moves = orderer.create_sorted_moves(node, pos.board(), TT_MOVE, Move::NULL_MOVE);
                while let Some(mv) = sorted_moves.next_move(pos.board(), &mut algo) {
                    black_box(&mv);
                }
            });
            t.elapsed() / positions.len() as u32
        })
    });
    orderer.order = MoveType::vec_from_string("SHIgKPqBE").unwrap();
    group.bench_function("SHIgKPqBE", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|pos| {
                let mut sorted_moves = orderer.create_sorted_moves(node, pos.board(), TT_MOVE, Move::NULL_MOVE);
                while let Some(mv) = sorted_moves.next_move(pos.board(), &mut algo) {
                    black_box(&mv);
                }
            });
            t.elapsed() / positions.len() as u32
        })
    });

    group.finish();
}

fn benchmark_eval(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval");
    let positions = &Catalog::win_at_chess();
    let ef = &mut Eval::new();
    let _phaser = Phaser::default();
    let ef_no_pos = &mut Eval::new().set_position(false);
    group.bench_function("material", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|p| {
                black_box(p.board().eval_some(black_box(ef)));
            });
            t.elapsed() / positions.len() as u32
        })
    });
    group.bench_function("position", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|p| {
                black_box(p.board().eval_some(black_box(ef)));
            });
            t.elapsed() / positions.len() as u32
        })
    });
    group.bench_function("mobility", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|p| {
                black_box(p.board().eval_some(black_box(ef)));
            });
            t.elapsed() / positions.len() as u32
        })
    });
    group.bench_function("safety", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|p| {
                black_box(p.board().eval_some(black_box(ef)));
            });
            t.elapsed() / positions.len() as u32
        })
    });
    group.bench_function("all_eval", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|p| {
                black_box(p.board().eval_with_outcome(black_box(ef), &Node::root(0)));
            });
            t.elapsed() / positions.len() as u32
        })
    });
    // group.bench_function("all_model_eval", |b| {
    //     b.iter_custom(|n| {
    //         let t = Instant::now();
    //         positions.iter().cycle_n(n).for_each(|p| {
    //             let mut model_score = ModelScore::new(phaser.phase(&p.board().material()), p.board().fifty_halfmove_clock());
    //             black_box(ef.predict(&Model::from_board(p.board(), Switches::ALL_SCORING), &mut model_score));
    //         });
    //         t.elapsed() / positions.len() as u32
    //     })
    // });
    group.bench_function("all_less_pos", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|p| {
                black_box(p.board().eval_with_outcome(black_box(ef_no_pos), &Node::root(0)));
            });
            t.elapsed() / positions.len() as u32
        })
    });
    group.finish();
}

fn benchmark_attacks(c: &mut Criterion) {
    let mut group = c.benchmark_group("attacks");
    let positions = &Catalog::win_at_chess();
    let pc = PreCalc::default();
    let cb = ClassicalBitboard::default();
    let hq = Hyperbola::default();
    let mg = Magic::default();

    group.bench_function("rules.king_legal", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            let mut list = MoveList::new();
            let mut count = 0;
            positions.iter().cycle_n(n).for_each(|p| {
                count += 1;
                let b = p.board().clone(); // caching
                black_box(Rules::king_legal(&b, &mut list));
                list.clear();
            });
            t.elapsed() / (count as u32 / n as u32)
        })
    });

    group.bench_function("rules.castles", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            let mut list = MoveList::new();
            let mut count = 0;
            positions.iter().cycle_n(n).for_each(|p| {
                count += 1;
                let b = p.board().clone(); // caching
                black_box(Rules::castles(&b, &mut list));
                list.clear();
            });
            t.elapsed() / (count as u32 / n as u32)
        })
    });

    group.bench_function("boardcalcs.pinned", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            let mut count = 0;
            positions.iter().cycle_n(n).for_each(|p| {
                count += 2;
                black_box(BoardCalcs::pinned_and_unmaskers(p.board(), Color::White)).0;
                black_box(BoardCalcs::pinned_and_unmaskers(p.board(), Color::Black)).0;
            });
            t.elapsed() / (count as u32 / n as u32)
        })
    });

    group.bench_function("boardcalcs.checkers_of", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            let mut count = 0;
            positions.iter().cycle_n(n).for_each(|p| {
                count += 2;
                black_box(BoardCalcs::checkers_of(p.board(), Color::White));
                black_box(BoardCalcs::checkers_of(p.board(), Color::Black));
            });
            t.elapsed() / (count as u32 / n as u32)
        })
    });

    group.bench_function("classical.bishop", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            let mut count = 0;
            positions.iter().cycle_n(n).for_each(|p| {
                count += p.board().bishops().popcount();
                let occ = p.board().black() | p.board().white();
                black_box(
                    p.board()
                        .bishops()
                        .squares()
                        .map(|b| cb.bishop_attacks(occ, b).popcount() as i32)
                        .sum::<i32>(),
                );
            });
            t.elapsed() / (count as u32 / n as u32)
        })
    });

    group.bench_function("classical.rook", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            let mut count = 0;
            positions.iter().cycle_n(n).for_each(|p| {
                count += p.board().rooks().popcount();
                let occ = p.board().black() | p.board().white();
                black_box(
                    p.board()
                        .rooks()
                        .squares()
                        .map(|b| cb.rook_attacks(occ, b).popcount() as i32)
                        .sum::<i32>(),
                );
            });
            t.elapsed() / (count as u32 / n as u32)
        })
    });

    // group.bench_function("classical.knight", |b| {
    //     b.iter_custom(|n| {
    //         let t = Instant::now();
    //         let mut count = 0;
    //         positions.iter().cycle_n(n).for_each(|p| {
    //             count += p.board().knights().popcount();
    //             let _occ = p.board().black() | p.board().white();
    //             black_box(p.board().knights().squares().for_each(|b| {
    //                 cb.knight_attacks(b);
    //             }));
    //         });
    //         t.elapsed() / (count as u32 / n as u32)
    //     })
    // });

    group.bench_function("hyperbola.bishop", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            let mut count = 0;
            positions.iter().cycle_n(n).for_each(|p| {
                count += p.board().bishops().popcount();
                let occ = p.board().black() | p.board().white();
                black_box(p.board().bishops().squares().for_each(|b| {
                    hq.bishop_attacks(occ, b);
                }));
            });
            t.elapsed() / (count as u32 / n as u32)
        })
    });

    group.bench_function("hyperbola.rook", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            let mut count = 0;
            positions.iter().cycle_n(n).for_each(|p| {
                count += p.board().rooks().popcount();
                let occ = p.board().black() | p.board().white();
                black_box(p.board().rooks().squares().for_each(|b| {
                    black_box(hq.rook_attacks(occ, b).popcount());
                }));
            });
            t.elapsed() / (count as u32 / n as u32)
        })
    });

    group.bench_function("hyperbola.queen", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            let mut count = 0;
            positions.iter().cycle_n(n).for_each(|p| {
                count += p.board().queens().popcount();
                let occ = p.board().black() | p.board().white();
                black_box(p.board().rooks().squares().for_each(|b| {
                    let _ = hq.rook_attacks(occ, b) | hq.rook_attacks(occ, b);
                }));
            });
            t.elapsed() / (count as u32 / n as u32)
        })
    });

    group.bench_function("hyperbola.king", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            let mut count = 0;
            positions.iter().cycle_n(n).for_each(|p| {
                count += p.board().kings().popcount();
                black_box(p.board().kings().squares().for_each(|b| {
                    pc.king_attacks(b);
                }));
            });
            t.elapsed() / (count as u32 / n as u32)
        })
    });

    group.bench_function("hyperbola.pawn", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            let mut count = 0;
            positions.iter().cycle_n(n).for_each(|p| {
                let pawns = p.board().pawns() & p.board().us();
                count += pawns.popcount();
                black_box(pawns.squares().for_each(|s| {
                    pc.pawn_attacks_ext(p.board().color_us(), p.board().us(), p.board().them(), s);
                }));
            });
            t.elapsed() / (count as u32 / n as u32)
        })
    });

    group.bench_function("hyperbola.knight", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            let mut count = 0;
            positions.iter().cycle_n(n).for_each(|p| {
                count += p.board().knights().popcount();
                let _occ = p.board().black() | p.board().white();
                black_box(p.board().knights().squares().for_each(|b| {
                    pc.knight_attacks(b);
                }));
            });
            t.elapsed() / (count as u32 / n as u32)
        })
    });

    group.bench_function("magic.bishop", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            let mut count = 0;
            positions.iter().cycle_n(n).for_each(|p| {
                count += p.board().bishops().popcount();
                let occ = p.board().black() | p.board().white();
                black_box(p.board().bishops().squares().for_each(|b| {
                    mg.bishop_attacks(occ, b);
                }));
            });
            t.elapsed() / (count as u32 / n as u32)
        })
    });

    group.bench_function("magic.rook", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            let mut count = 0;
            positions.iter().cycle_n(n).for_each(|p| {
                count += p.board().rooks().popcount();
                let occ = p.board().black() | p.board().white();
                black_box(p.board().rooks().squares().for_each(|b| {
                    mg.rook_attacks(occ, b);
                }));
            });
            t.elapsed() / (count as u32 / n as u32)
        })
    });

    group.bench_function("magic.queen", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            let mut count = 0;
            positions.iter().cycle_n(n).for_each(|p| {
                count += p.board().queens().popcount();
                let occ = p.board().black() | p.board().white();
                black_box(p.board().rooks().squares().for_each(|b| {
                    let _ = mg.rook_attacks(occ, b) | mg.bishop_attacks(occ, b);
                }));
            });
            t.elapsed() / (count as u32 / n as u32)
        })
    });

    group.finish();
}

fn bench_logging(c: &mut Criterion) {
    c.bench_function("logging", |b| {
        b.iter(|| {
            black_box(debug!("The cat sat on the mat and counted {} {} {}", 1, 2, black_box(3)));
        });
    });
}

fn bench_chooser_array(c: &mut Criterion) {
    let white = Color::White;
    let black = Color::Black;
    c.bench_function("chooser_arr", |b| {
        b.iter(|| {
            black_box([Bitboard::RANK_4, Bitboard::RANK_5][black_box(white)]);
            black_box([Bitboard::RANK_4, Bitboard::RANK_5][black_box(black)]);
        });
    });
}

fn bench_chooser_wb(c: &mut Criterion) {
    let white = Color::White;
    let black = Color::Black;
    c.bench_function("chooser_wb_ref", |b| {
        b.iter(|| {
            black_box(chooser_wb(black_box(white), &Bitboard::RANK_4, &Bitboard::RANK_5));
            black_box(chooser_wb(black_box(black), &Bitboard::RANK_4, &Bitboard::RANK_5));
        });
    });
}

fn bench_chooser_struct(c: &mut Criterion) {
    let white = Color::White;
    let black = Color::Black;
    const CHOICE: Chooser<&Bitboard> = Chooser {
        white: &Bitboard::RANK_4,
        black: &Bitboard::RANK_5,
    };
    c.bench_function("chooser_struct", |b| {
        b.iter(|| {
            black_box(chooser_struct(black_box(white), &CHOICE));
            black_box(chooser_struct(black_box(black), &CHOICE));
        });
    });
}

fn bench_insufficient_material(c: &mut Criterion) {
    let mut group = c.benchmark_group("insufficient");
    let bd = &Catalog::starting_board();
    let m = Material::from_board(bd);
    group.bench_function("insufficient_material", |b| {
        b.iter(|| {
            black_box(black_box(m).is_insufficient());
        });
    });
}

fn benchmark_thread(c: &mut Criterion) {
    let mut group = c.benchmark_group("thread");
    group.sample_size(10);
    let pos = Catalog::test_position();

    //let hasher2 = Hasher::new(3141592653589793);
    let board = Catalog::perft_cpw_number3().0;
    let moves = board.legal_moves();
    let mv = moves[0];
    // let mv2 = mv1.clone();
    // let mv3 = mv2.clone();
    // let board1 = Catalog::perft_cpw_number3().0.clone();
    // let board2 = Catalog::perft_cpw_number3().0.clone();
    // let board3 = Catalog::perft_cpw_number3().0.clone();

    //let hasher = Hasher::new(1);
    let cl = |_h: &Hasher, board: &Board, mv: &Move| {
        // let moves = board.legal_moves();
        // let hasher = Hasher::default();
        for _i in 1..1_000_000 {
            black_box(board.make_move(&mv));
            let mut moves = MoveList::new();
            board.legal_moves_into(&mut moves);
            black_box(moves);
            // black_box(h.hash_move(&mv, &board));
            // for mv in moves.iter() {
            //     black_box(hasher.hash_move(black_box(mv), black_box(&board)));
            // }
        }
    };
    // let cl2 = || {
    //     // let moves = board.legal_moves();
    //     // let hasher = &hasher2;
    //     for _i in 1..1000000 {
    //         black_box(board2.make_move(&mv2));
    //         // for mv in moves.iter() {
    //         //     black_box(hasher.hash_move(black_box(mv), black_box(&board)));
    //         // }
    //     }
    // };

    // let cl3 = || {
    //     // let moves = board.legal_moves();
    //     // let hasher = &hasher2;
    //     for _i in 1..1000000 {
    //         black_box(board3.make_move(&mv3));
    //         // for mv in moves.iter() {
    //         //     black_box(hasher.hash_move(black_box(mv), black_box(&board)));
    //         // }
    //     }
    // };

    group.bench_function("perft-1t", |b| {
        b.iter(|| {
            let pos1 = Catalog::starting_position();
            let pos2 = pos1.clone();
            let handle = thread::spawn(move || {
                black_box(Perft::perft(&mut pos1.board().clone(), black_box(6)));
            });
            // black_box(Perft::perft(&mut board1, black_box(6)));
            handle.join().unwrap();
            black_box(pos2.board());
        });
    });
    group.bench_function("perft-2t", |b| {
        b.iter(|| {
            let mut pos1 = Catalog::starting_position();
            let mut pos2 = pos1.clone();
            let handle = thread::spawn(move || {
                black_box(Perft::perft(&mut pos1.board_mut(), black_box(6)));
            });
            black_box(Perft::perft(&mut pos2.board_mut(), black_box(6)));
            handle.join().unwrap();
        });
    });
    group.bench_function("perft-3t", |b| {
        b.iter(|| {
            let mut pos1 = Catalog::starting_position();
            let mut pos2 = pos1.clone();
            let mut pos3 = pos1.clone();
            let handle1 = thread::spawn(move || {
                black_box(Perft::perft(&mut pos1.board_mut(), black_box(6)));
            });
            let handle2 = thread::spawn(move || {
                black_box(Perft::perft(&mut pos2.board_mut(), black_box(6)));
            });
            black_box(Perft::perft(&mut pos3.board_mut(), black_box(6)));
            handle1.join().unwrap();
            handle2.join().unwrap();
        });
    });

    group.bench_function("search-1t", |b| {
        b.iter(|| {
            let mut algo1 = Algo::new();
            algo1.set_timing_method(TimeControl::Depth(6));
            algo1.new_game();

            black_box({
                algo1.set_position(pos.clone());
                algo1.search()
            });
        });
    });
    group.bench_function("search-2t", |b| {
        b.iter(|| {
            let mut algo1 = Algo::new();
            algo1.set_timing_method(TimeControl::Depth(6));
            algo1.board = pos.board().clone();
            algo1.new_game();

            let mut algo2 = Algo::new();
            algo2.set_timing_method(TimeControl::Depth(6));
            algo2.board = pos.board().clone();
            algo2.new_game();

            let handle = thread::spawn(move || {
                black_box(algo1.search_iteratively());
            });
            black_box({
                algo2.set_position(pos.clone());
                algo2.search()
            });
            handle.join().unwrap();
        });
    });

    group.bench_function("hash_move-t1", |b| {
        b.iter(|| {
            let hash1 = Hasher::default().clone();
            cl(&hash1, &board.clone(), &mv.clone())
        });
    });

    group.bench_function("hash_move-t2", |b| {
        b.iter(|| {
            let board_clone = board.clone();
            let hash1 = Hasher::default().clone();
            let handle = thread::spawn(move || cl(&hash1, &board_clone, &mv.clone()));
            let hash3 = Hasher::default().clone();
            cl(&hash3, &board.clone(), &mv.clone());
            handle.join().unwrap();
        });
    });

    group.bench_function("hash_move-t3", |b| {
        b.iter(|| {
            let board1_clone = board.clone();
            let board2_clone = board.clone();
            let hash1 = Hasher::default().clone();
            let hash2 = Hasher::default().clone();
            let handle1 = thread::spawn(move || cl(&hash1, &board1_clone, &mv.clone()));
            let handle2 = thread::spawn(move || cl(&hash2, &board2_clone, &mv.clone()));
            let hash3 = Hasher::default().clone();
            cl(&hash3, &board.clone(), &mv.clone());
            handle1.join().unwrap();
            handle2.join().unwrap();
        });
    });

    group.bench_function("loop-1t", |b| {
        b.iter(|| {
            black_box(calc_sum(1000));
        });
    });
    group.bench_function("loop-2t", |b| {
        b.iter(|| {
            let handle = thread::spawn(move || {
                black_box(calc_sum(1000));
            });
            black_box(calc_sum(1000));
            handle.join().unwrap();
        });
    });

    group.finish();
}

fn calc_sum(n: u64) -> u64 {
    let mut sum = 0;
    for j in 1..30000 {
        for i in 1..n {
            sum += i;
            sum += (i % 3) * (i % 2);
        }
        sum += j;
    }
    sum
}

fn benchmark_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");
    group.sample_size(10);
    let pos = Catalog::test_position();
    let mut algo = Algo::new();
    algo.set_timing_method(TimeControl::Depth(5));
    group.bench_function("test(5)", |b| {
        b.iter(|| {
            // let eval = SimpleScorer::new().set_position(false);
            algo.new_game();
            black_box({
                algo.set_position(pos.clone());
                algo.search()
            });
        });
    });
    let pos = Catalog::starting_position();
    group.bench_function("starting(5)", |b| {
        b.iter(|| {
            // let eval = SimpleScorer::new().set_position();
            algo.new_game();
            black_box({
                algo.set_position(pos.clone());
                algo.search()
            });
        });
    });

    let pos = Catalog::mate_in_2()[0].clone();
    let mut algo1 = Algo::new();
    algo1.set_timing_method(TimeControl::Depth(3));
    algo1.ids.enabled = false;
    group.bench_function("mate_in_2_ab", |b| {
        b.iter(|| {
            algo1.new_game();
            black_box({
                algo1.set_position(pos.clone());
                algo1.search()
            });
            assert_eq!(algo1.pv_table.extract_pv().to_string(), "d5f6, g7f6, c4f7");
        });
    });
    let pos = Catalog::mate_in_2()[0].clone();
    let mut algo2 = Algo::new();
    algo2.set_timing_method(TimeControl::Depth(3));
    group.bench_function("mate_in_2_ab_ids", |b| {
        b.iter(|| {
            algo2.new_game();
            black_box({
                algo2.set_position(pos.clone());
                algo2.search()
            });
            assert_eq!(algo2.pv_table.extract_pv().to_string(), "d5f6, g7f6, c4f7");
        });
    });
    group.finish();
}

// fn benchmark_mate_in_2(c: &mut Criterion) {
//     let mut group = c.benchmark_group("mate2");
//     group.sample_size(20);
//     let board = Catalog::mate_in_2()[0].board().clone();
//     let mut algo1 = Algo::new().set_timing_method(TimeControl::Depth(3));
//     algo1.ids.enabled = false;
//     group.bench_function("mate_in_2_ab", |b| {
//         b.iter(|| {
//             algo1.new_game();
//             black_box(algo1.search(black_box(&board)));
//             assert_eq!(algo1.pv_table.extract_pv().to_string(), "d5f6, g7f6, c4f7");
//         });
//     });
//     let board = Catalog::mate_in_2()[0].board().clone();
//     let mut algo2 = Algo::new().set_timing_method(TimeControl::Depth(3));
//     group.bench_function("mate_in_2_ab_ids", |b| {
//         b.iter(|| {
//             algo2.new_game();
//             black_box(algo2.search(black_box(&board)));
//             assert_eq!(algo2.pv_table.extract_pv().to_string(), "d5f6, g7f6, c4f7");
//         });
//     });
//     group.finish();
// }

fn benchmark_array(c: &mut Criterion) {
    let mut group = c.benchmark_group("array");
    let array = [1, 2];
    group.bench_function("int_index", |b| {
        b.iter(|| {
            for c in &Color::ALL {
                black_box(array[c.opposite().index()]);
                black_box(array[c.index()]);
                black_box(array[c.opposite().index()]);
                black_box(array[c.index()]);
            }
            black_box(array[black_box(0)]);
            black_box(array[black_box(1)]);
            black_box(array[black_box(0)]);
            black_box(array[black_box(1)]);
        });
    });
    group.bench_function("enum_index", |b| {
        b.iter(|| {
            for &c in &Color::ALL {
                black_box(array[c.opposite()]);
                black_box(array[c]);
                black_box(array[c.opposite()]);
                black_box(array[c]);
            }
            black_box(array[black_box(Color::White)]);
            black_box(array[black_box(Color::Black)]);
            black_box(array[black_box(Color::White)]);
            black_box(array[black_box(Color::Black)]);
        });
    });
    group.finish();
}

fn bench_pvtable(c: &mut Criterion) {
    let mut pv_table = PvTable::new(MAX_PLY as usize);
    c.bench_function("pv_table", |b| {
        b.iter(|| {
            for i in 1..7 {
                pv_table.set(i, black_box(&Move::new_null()), false);
                pv_table.propagate_from(i);
            }
        });
    });
}

use rand::seq::SliceRandom;
use rand::thread_rng;

fn bench_shared_mem(c: &mut Criterion) {
    let mut group = c.benchmark_group("shared_mem");

    //let atomic_array: Vec<AtomicU64> = vec![];
    // let atomic = AtomicU64::new(0);

    const N: usize = 200_000;
    let mut vec = Vec::with_capacity(N);
    (0..N).into_iter().for_each(|i| vec.push(AtomicU64::new(i as u64)));
    let atomic_array = vec.into_boxed_slice();
    let mut rng = thread_rng();
    let mut shuf: Vec<usize> = (0..N).map(|i| i).collect();
    shuf.shuffle(&mut rng);
    group.bench_function("atomic_u64_relaxed", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            (0..100_000).cycle_n(n).for_each(|i| {
                black_box(atomic_array[black_box(shuf[i as usize])].store(black_box(1), Ordering::Relaxed));
                black_box(atomic_array[black_box(shuf[N - 1 - i as usize])].load(Ordering::Relaxed));
            });
            t.elapsed() / (2 * 100_000) as u32
        });
    });
    group.finish();
}

// fn cache_eval(c: &mut Criterion) {
//     let mut group = c.benchmark_group("cache_eval");
//     group.sample_size(10);

//     let positions = &Catalog::bratko_kopec()[..3];
//     group.bench_function("cache_eval_on", |b| {
//         b.iter_custom(|n| {
//             let t = Instant::now();
//             positions.iter().cycle_n(n).for_each(|p| {
//                 let mut algo = Algo::new().set_timing_method(TimeControl::Depth(4));
//                 algo.set_position(p.clone());
//                 algo.search();
//             });
//             t.elapsed() / positions.len() as u32
//         });
//     });
//     // group.bench_function("cache_eval_off", |b| {
//     //     b.iter_custom(|n| {
//     //         let t = Instant::now();
//     //         positions.iter().cycle_n(n).for_each(|p| {
//     //             let mut algo = Algo::new().set_timing_method(TimeControl::Depth(4));
//     //             //algo.eval.cache.enabled = false;
//     //             algo.set_position(p.clone());
//     //             algo.search();
//     //         });
//     //         t.elapsed() / positions.len() as u32
//     //     });
//     // });
//     // group.finish();
// }

fn bench_moveordering(c: &mut Criterion) {
    let a1a2 = Move::new_quiet(Piece::Queen, a1.square(), a2.square());
    let a1a3 = Move::new_quiet(Piece::Queen, a1.square(), a3.square());
    let a1a4 = Move::new_quiet(Piece::Queen, a1.square(), a4.square());
    let b1a2 = Move::new_quiet(Piece::Queen, b1.square(), a2.square());
    let b1a3 = Move::new_quiet(Piece::Queen, b1.square(), a3.square());
    let b1a4 = Move::new_quiet(Piece::Queen, b1.square(), a4.square());
    let c1c2 = Move::new_quiet(Piece::Queen, c1.square(), c2.square());
    let mut movelists = vec![MoveList::new(); 100];
    for i in 0..100 {
        movelists[i].extend(vec![b1a2, b1a3, b1a4, a1a3, a1a4, a1a2]);
    }

    let mut pv = Variation::new();
    pv.extend(vec![a1a2, a1a3, a1a4]);
    let mut variation = Variation::new();
    variation.extend(vec![a1a2, a1a3, c1c2]);

    c.bench_function("move_orderer", |b| {
        b.iter(|| {
            for i in 0..100 {
                black_box(Algo::order_from_prior_pv(
                    black_box(&mut movelists[i]),
                    black_box(&variation),
                    black_box(&pv),
                ));
            }
        });
    });
}
