use criterion::*;
use odonata::bitboard::bitboard::*;
use odonata::board::makemove::*;
use odonata::board::movegen::*;
use odonata::board::*;
use odonata::catalog::*;
use odonata::eval::eval::*;
use odonata::eval::score::*;
use odonata::globals::constants::*;
use odonata::hasher::*;
use odonata::material::*;
use odonata::movelist::*;
use odonata::outcome::*;
use odonata::perft::Perft;
use odonata::pvtable::*;
use odonata::tt::{TranspositionTable, Entry, NodeType};
use odonata::search::algo::Algo;
use odonata::search::timecontrol::TimeControl;
use odonata::types::*;
use odonata::utils::*;
use std::time::Instant;
use std::sync::atomic::{AtomicU64, Ordering};
use odonata::bitboard::attacks::{BitboardAttacks, BitboardDefault};
use odonata::bitboard::bb_classical::ClassicalBitboard;
use odonata::bitboard::bb_hyperbola::Hyperbola;



/*
Bitboard 2.7ns (a|b)&c
Chooser 1.7ns
PieceToUpperChar 622ns
Perft(5) 334ms
Makemove 26.25ns
legal_moves 966ns
pseudo_legal_moves 300ns
Score: 5 > 4  1ns

search/minmax Depth 5  | search/alphabeta  Depth 5
initial         = 482ms    5.7ms
game end logic  = 6.8s     48ms
& early fin - no effect
collect PV no effect

12s (6.5s without game end test) - forgot to short circuit legal moves on leaf node


search/alphabeta Depth 5  =
85 ms (48 ms without game end tests)

Array (int = 6.3, enum = 6.7)

Material.is_insufficient 9.8ns


Jons_problem
180s (9ply)
1 hr (10ply)
478s (looks at end of game)

*/

fn bitwise_handcrafted(c: &mut Criterion) {
    let n1 = 1u64 << 3;
    let n2 = 1u64 << 4;
    let n3 = 1u64 << 5;
    c.bench_function("bitwise_handcrafted", |b| {
        b.iter(|| {
            let a = black_box(n1) | black_box(n2);
            let b = a & black_box(n3);
            black_box(b);
        });
    });
}

fn bitwise_bitflags(c: &mut Criterion) {
    let n1 = Bitboard::D1;
    let n2 = Bitboard::E1;
    let n3 = Bitboard::F1;
    c.bench_function("bibitwise_bitflags", |b| {
        b.iter(|| {
            let a = black_box(n1) | black_box(n2);
            let b = a & black_box(n3);
            black_box(b);
        });
    });
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

fn benchmark_perft5(c: &mut Criterion) {
    let mut board = Catalog::starting_position();
    let mut group = c.benchmark_group("sample size 10");
    group.sample_size(10);
    group.bench_function("perft5", |b| {
        b.iter(|| {
            black_box(Perft::perft(&mut board, black_box(5)));
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
            black_box(Score::Cp(black_box(5)) > Score::Cp(black_box(4)));
        });
    });
    group.finish();
}

fn make_move(c: &mut Criterion) {
    let board = Catalog::starting_position();
    let mv1 = board.parse_uci_move("e2e4").unwrap();
    let mv2 = board.parse_uci_move("b1c3").unwrap();
    c.bench_function("makemove", |b| {
        b.iter(|| {
            black_box(board.make_move(black_box(&mv1)));
            black_box(board.make_move(black_box(&mv2)));
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
    let board = Catalog::starting_position();
    let hasher = Hasher::new(1);
    c.bench_function("hash_board", |b| {
        b.iter(|| {
            black_box(hasher.hash_board(black_box(&board)));
        });
    });
}

fn pseudo_legal_moves(c: &mut Criterion) {
    let board = Catalog::starting_position();
    c.bench_function("pseudo_legal_moves", |b| {
        b.iter(|| {
            black_box(black_box(&board).pseudo_legal_moves());
        });
    });
}

fn legal_moves(c: &mut Criterion) {
    let board = Catalog::starting_position();
    c.bench_function("legal_moves", |b| {
        b.iter(|| {
            black_box(black_box(&board).legal_moves());
        });
    });
}

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
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|p| {
                black_box(p.board().legal_moves());
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

    let bams: Vec<(Board, Move)> = positions
        .iter()
        .map(|p| (p.board().clone(), p.bm().unwrap()[0]))
        .collect();
    group.bench_function("will_check_them", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            bams.iter().cycle_n(n).for_each(|bam| {
                black_box(bam.0.will_check_them(&bam.1));
            });
            t.elapsed() / positions.len() as u32
        })
    });

    let mut tt = TranspositionTable::with_capacity(300_000);
    group.bench_function("tt_probe", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|p| {
                black_box(tt.probe_by_board(p.board()));
            });
            t.elapsed() / positions.len() as u32
        })
    });

    group.bench_function("tt_store", |b| {
        let entry = Entry {
            score : Score::Cp(100),
            depth: 1,
            node_type: NodeType::Pv,
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

    group.bench_function("pseudo_legal_moves", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|p| {
                black_box(p.board().pseudo_legal_moves());
            });
            t.elapsed() / positions.len() as u32
        })
    });

    let bams: Vec<(Board, Move)> = positions
        .iter()
        .map(|p| (p.board().clone(), p.bm().unwrap()[0]))
        .collect();
    group.bench_function("make_move + hash + clone", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            bams.iter().cycle_n(n).for_each(|bam| {
                black_box(bam.0.make_move(&bam.1));
            });
            t.elapsed() / positions.len() as u32
        })
    });

    let bams: Vec<(Board, Move)> = positions
        .iter()
        .map(|p| (p.board().clone(), p.bm().unwrap()[0]))
        .collect();
    group.bench_function("hash_move", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            bams.iter().cycle_n(n).for_each(|bam| {
                black_box(Hasher::default().hash_move(&bam.1, &bam.0));
            });
            t.elapsed() / positions.len() as u32
        })
    });
    let bams: Vec<(Board, Move)> = positions
        .iter()
        .map(|p| (p.board().clone(), p.bm().unwrap()[0]))
        .collect();
    group.bench_function("hash_board", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            bams.iter().cycle_n(n).for_each(|bam| {
                black_box(Hasher::default().hash_board(&bam.0));
            });
            t.elapsed() / positions.len() as u32
        })
    });
    let bams: Vec<(Board, Move)> = positions
        .iter()
        .map(|p| (p.board().clone(), p.bm().unwrap()[0]))
        .collect();
    group.bench_function("threats_to raw", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            bams.iter().cycle_n(n).for_each(|bam| {
                black_box(threats_to(black_box(&bam.0), Color::White));
                black_box(threats_to(black_box(&bam.0), Color::Black));
            });
            t.elapsed() / 2 / positions.len() as u32
        })
    });
    group.bench_function("threats_to (memoise)", |b| {
        let bams: Vec<(Board, Move)> = positions
            .iter()
            .map(|p| (p.board().clone(), p.bm().unwrap()[0]))
            .collect();
        b.iter_custom(|n| {
            let t = Instant::now();
            bams.iter().cycle_n(n).for_each(|bam| {
                black_box(&bam.0.threats_to(Color::White));
                black_box(&bam.0.threats_to(Color::Black));
            });
            t.elapsed() / 2 / positions.len() as u32
        })
    });
    let bams: Vec<(Board, Move)> = positions
        .iter()
        .map(|p| (p.board().clone(), p.bm().unwrap()[0]))
        .collect();
    group.bench_function("threats_to + clone", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            bams.iter().cycle_n(n).for_each(|bam| {
                black_box(&bam.0.clone().threats_to(Color::White));
                black_box(&bam.0.clone().threats_to(Color::Black));
            });
            t.elapsed() / 2 / positions.len() as u32
        })
    });
    group.finish();
}

fn benchmark_eval(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval");
    let positions = &Catalog::win_at_chess();
    let ef = &mut SimpleScorer::new();
    let ef_no_pos = &mut SimpleScorer::new().set_position(false);
    group.bench_function("material", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|p| {
                black_box(p.board().eval_material(black_box(ef)));
            });
            t.elapsed() / positions.len() as u32
        })
    });
    group.bench_function("position", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|p| {
                black_box(p.board().eval_position(black_box(ef)));
            });
            t.elapsed() / positions.len() as u32
        })
    });
    group.bench_function("mobility", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|p| {
                black_box(p.board().eval_mobility(black_box(ef)));
            });
            t.elapsed() / positions.len() as u32
        })
    });
    group.bench_function("all", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|p| {
                black_box(p.board().eval(black_box(ef)));
            });
            t.elapsed() / positions.len() as u32
        })
    });
    group.bench_function("qsearch", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|p| {
                black_box(p.board().eval_qsearch(black_box(ef)));
            });
            t.elapsed() / positions.len() as u32
        })
    });
    group.bench_function("all_less_pos", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|p| {
                black_box(p.board().eval(black_box(ef_no_pos)));
            });
            t.elapsed() / positions.len() as u32
        })
    });
    group.finish();
}


fn benchmark_attacks(c: &mut Criterion) {
    let mut group = c.benchmark_group("attacks");
    let positions = &Catalog::win_at_chess();
    let cb = ClassicalBitboard::default();
    let hq = Hyperbola::default();

    group.bench_function("classical.bishop", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            let mut count = 0;
            positions.iter().cycle_n(n).for_each(|p| {
                count  += p.board().bishops().popcount();
                let occ = p.board().black() | p.board().white();
                black_box(p.board().bishops().squares().map(|b| cb.bishop_attacks(occ, b).popcount() as i32).sum::<i32>());
            });
            t.elapsed() / (count as u32 / n as u32)
        })
    });

    group.bench_function("classical.rook", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            let mut count = 0;
            positions.iter().cycle_n(n).for_each(|p| {
                count  += p.board().rooks().popcount();
                let occ = p.board().black() | p.board().white();
                black_box(p.board().rooks().squares().map(|b| cb.rook_attacks(occ, b).popcount() as i32).sum::<i32>());
            });
            t.elapsed() / (count as u32 / n as u32)
        })
    });

    group.bench_function("classical.knight", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            let mut count = 0;
            positions.iter().cycle_n(n).for_each(|p| {
                count  += p.board().knights().popcount();
                let _occ = p.board().black() | p.board().white();
                black_box(p.board().knights().squares().map(|b| cb.knight_attacks(b).popcount() as i32).sum::<i32>());
            });
            t.elapsed() / (count as u32 / n as u32)  
        })
    });

    group.bench_function("hyperbola.bishop", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            let mut count = 0;
            positions.iter().cycle_n(n).for_each(|p| {
                count  += p.board().bishops().popcount();
                let occ = p.board().black() | p.board().white();
                black_box(p.board().bishops().squares().map(|b| hq.bishop_attacks(occ, b).popcount() as i32).sum::<i32>());
            });
            t.elapsed() / (count as u32 / n as u32)
        })
    });

    group.bench_function("hyperbola.rook", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            let mut count = 0;
            positions.iter().cycle_n(n).for_each(|p| {
                count  += p.board().rooks().popcount();
                let occ = p.board().black() | p.board().white();
                black_box(p.board().rooks().squares().map(|b| hq.rook_attacks(occ, b).popcount() as i32).sum::<i32>());
            });
            t.elapsed() / (count as u32 / n as u32)
        })
    });

    group.bench_function("hyperbola.knight", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            let mut count = 0;
            positions.iter().cycle_n(n).for_each(|p| {
                count  += p.board().knights().popcount();
                let _occ = p.board().black() | p.board().white();
                black_box(p.board().knights().squares().map(|b| hq.knight_attacks(b).popcount() as i32).sum::<i32>());
            });
            t.elapsed() / (count as u32 / n as u32)
        })
    });
    group.finish();
}


fn bench_chooser_array(c: &mut Criterion) {
    let white = Color::White;
    let black = Color::Black;
    c.bench_function("chooser_arr", |b| {
        b.iter(|| {
            black_box(chooser_array(
                black_box(white),
                &Bitboard::RANK_4,
                &Bitboard::RANK_5,
            ));
            black_box(chooser_array(
                black_box(black),
                &Bitboard::RANK_4,
                &Bitboard::RANK_5,
            ));
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
    let bd = &Catalog::starting_position();
    let m = Material::from_board(bd);
    group.bench_function("insufficient_material", |b| {
        b.iter(|| {
            black_box(black_box(m).is_insufficient());
        });
    });
    group.bench_function("insufficient_material2", |b| {
        b.iter(|| {
            black_box(Material::is_insufficient2(black_box(bd)));
        });
    });
    group.finish();
}

fn benchmark_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");
    group.sample_size(10);
    group.bench_function("alphabeta(5)", |b| {
        b.iter(|| {
            let board = Catalog::starting_position();
            let eval = SimpleScorer::new().set_position(false);
            let mut search = Algo::new()
                .set_timing_method(TimeControl::Depth(5))
                .set_minmax(false)
                .set_eval(eval)
                .build();
            search.new_game();
            black_box(search.search(&board));
        });
    });
    group.bench_function("minmax(5)", |b| {
        b.iter(|| {
            let board = Catalog::starting_position();
            let eval = SimpleScorer::new().set_position(false);
            let mut search = Algo::new()
                .set_timing_method(TimeControl::Depth(5))
                .set_minmax(true)
                .set_eval(eval)
                .set_qsearch(false)
                .build();
            search.new_game();
            black_box(search.search(&board));
        });
    });
    group.finish();
}

fn benchmark_mate_in_2(c: &mut Criterion) {
    let mut group = c.benchmark_group("mate2");
    group.sample_size(20);
    let board = Catalog::mate_in_2()[0].board().clone();
    let eval = SimpleScorer::new().set_position(false);
    group.bench_function("mate_in_2_ab", |b| {
        b.iter(|| {
            let mut search = Algo::new()
                .set_timing_method(TimeControl::Depth(3))
                .set_minmax(false)
                .set_eval(eval.clone())
                .set_iterative_deepening(false)
                .build();
            search.new_game();
            black_box(search.search(black_box(&board)));
            assert_eq!(search.pv_table.extract_pv().to_string(), "d5f6, g7f6, c4f7");
        });
    });
    let board = Catalog::mate_in_2()[0].board().clone();
    let eval = SimpleScorer::new().set_position(false);
    group.bench_function("mate_in_2_ab_iid", |b| {
        b.iter(|| {
            let mut search = Algo::new()
                .set_timing_method(TimeControl::Depth(3))
                .set_minmax(false)
                .set_eval(eval.clone())
                .set_iterative_deepening(true)
                .build();
            search.new_game();
            black_box(search.search(black_box(&board)));
            assert_eq!(search.pv_table.extract_pv().to_string(), "d5f6, g7f6, c4f7");
        });
    });
    group.finish();
}

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
            for c in &Color::ALL {
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

use rand::thread_rng;
use rand::seq::SliceRandom;


fn bench_shared_mem(c: &mut Criterion) {
    let mut group = c.benchmark_group("shared_mem");

    //let atomic_array: Vec<AtomicU64> = vec![];
    // let atomic = AtomicU64::new(0);

    const N :usize = 200_000;
    let mut vec = Vec::with_capacity(N);
    (0..N).into_iter().for_each(|i| vec.push(AtomicU64::new(i as u64)));
    let atomic_array = vec.into_boxed_slice();
    
    let mut rng = thread_rng();
    let mut shuf : Vec<usize> = (0..N).map(|i| i).collect();
    shuf.shuffle(&mut rng);
    
    
    group.bench_function("atomic_u64_relaxed", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            (0..100_000).cycle_n(n).for_each(|i| {
                black_box(atomic_array[black_box(shuf[i as usize])].store(black_box(1), Ordering::Relaxed));
                black_box(atomic_array[black_box(shuf[N-1-i as usize])].load(Ordering::Relaxed));
            });
            t.elapsed() / (2 * 100_000) as u32
        });
    });
    group.finish();
}


fn cache_eval(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_eval");
    group.sample_size(10);

    let positions = &Catalog::bratko_kopec()[..3];
    group.bench_function("cache_eval_on", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|p| {
                let mut algo = Algo::new().set_timing_method(TimeControl::Depth(4)).build();
                algo.search(p.board());
            });
            t.elapsed() / positions.len() as u32
        });
    });
    group.bench_function("cache_eval_off", |b| {
        b.iter_custom(|n| {
            let t = Instant::now();
            positions.iter().cycle_n(n).for_each(|p| {
                let mut algo = Algo::new().set_timing_method(TimeControl::Depth(4)).build();
                //algo.eval.cache.enabled = false;
                algo.search(p.board());
            });
            t.elapsed() / positions.len() as u32
        });
    });
    group.finish();
}



fn bench_moveordering(c: &mut Criterion) {
    let a1a2 = Move {
        from: a1,
        to: a2,
        ..Default::default()
    };
    let a1a3 = Move {
        from: a1,
        to: a3,
        ..Default::default()
    };
    let a1a4 = Move {
        from: a1,
        to: a4,
        ..Default::default()
    };
    let b1a2 = Move {
        from: b1,
        to: a2,
        ..Default::default()
    };
    let b1a3 = Move {
        from: b1,
        to: a3,
        ..Default::default()
    };
    let b1a4 = Move {
        from: b1,
        to: a4,
        ..Default::default()
    };
    let c1c2 = Move {
        from: c1,
        to: c2,
        ..Default::default()
    };
    let mut movelists = vec![MoveList::new(); 100];
    for i in 0..100 {
        movelists[i].extend([b1a2, b1a3, b1a4, a1a3, a1a4, a1a2].iter());
    }

    let mut pv = MoveList::new();
    pv.extend([a1a2, a1a3, a1a4].iter());
    let mut variation = MoveList::new();
    variation.extend([a1a2, a1a3, c1c2].iter());

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

criterion_group!(
    benches,
    bench_shared_mem,
    benchmark_mate_in_2,
    benchmark_search,
    benchmark_perft5,
    benchmark_eval,
    bb_calcs,
    board_calcs,
    benchmark_attacks,
    make_move,
    hash_move,
    hash_board,
    legal_moves,
    pseudo_legal_moves,
    bench_moveordering,
    bitwise_handcrafted,
    bitwise_bitflags,
    piece_to_upper_char,
    piece_to_char,
    bench_chooser_struct,
    bench_chooser_wb,
    bench_chooser_array,
    benchmark_score,
    benchmark_array,
    bench_insufficient_material,
    bench_pvtable,
    cache_eval
);
criterion_main!(benches);
