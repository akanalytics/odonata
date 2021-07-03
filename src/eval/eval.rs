use crate::bitboard::attacks::{BitboardAttacks, BitboardDefault};
use crate::bitboard::square::Square;
use crate::board::Board;
use crate::search::node::Node;
use crate::config::{Config, Component};
use crate::eval::score::Score;
use crate::eval::weight::Weight;
use crate::globals::counts;
use crate::{debug, logger::LogInit};
use crate::material::Material;
use crate::mv::Move;
use crate::stat::{ArrayStat, Stat};
use crate::types::{Color, Piece, Ply};

use std::fmt;

// eval1 = bl.scoring.material(p=300, b=400, n=700)
// eval2 = bl.scoring.position(endgame)

// for m in legal_moves:
//     bl.evaluate(m)
//     bl.evaluate(board + m)
//     score1 = eval1(board + m)
//     print(score1)::write_str
// '''
//         w     b  total
// pawns   3     5   -200
// bishops 1     5   -400
// total   -     -   1100
// '''
// print(score1.total)
// print(score1 + score2)
// '''
//              w     b  total
// pawns        3     5   -200
// bishops      1     5   -400
// passed pawns 4     0     50
// total        -     -   1100

// EndGame/Midgame and interp
// Tempo
// default scores
// position is by white/black as directional

// https://www.chessprogramming.org/Simplified_Evaluation_Function
const SQUARE_VALUES_MG: [[i32; 64]; Piece::len()] = [
    PAWN_PST_EG,
    PAWN_PST_MG,
    KNIGHT_PST,
    BISHOP_PST,
    ROOK_PST,
    QUEEN_PST,
    KING_PST_MG,
];
const SQUARE_VALUES_EG: [[i32; 64]; Piece::len()] = [
    PAWN_PST_EG,
    PAWN_PST_EG,
    KNIGHT_PST,
    BISHOP_PST,
    ROOK_PST,
    QUEEN_PST,
    KING_PST_EG,
];

#[rustfmt::skip]
const PAWN_PST_MG: [i32; 64] = [
0,  0,  0,  0,  0,  0,  0,  0,
40, 40, 40, 40, 40, 40, 40, 40,
 10, 10, 10, 10, 10, 10, 10, 10,
  5,  5,  5, 10, 10,  5,  5,  5,
 -9, 0,  0, 20, 20, -5,  -5, -9,
 -5,-5, -9,  0,  0, -9, -5, -5,
 9, 15, 15,-35,-35, 15, 15,  10,
 0,  0,  0,  0,  0,  0,  0,  0];

#[rustfmt::skip]
 const PAWN_PST_EG: [i32; 64] = [
 0,  0,  0,  0,  0,  0,  0,  0,
 60, 60, 60, 60, 60, 60, 60, 60,
 40, 40, 40, 40, 40, 40, 40, 40,
 20, 20, 20, 20, 20, 20, 20, 20,
 10, 10, 10, 10, 10, 10, 10, 10,
  5,  5,  5,  5,  5,  5,  5,  5,
  0,  0,  0,  0,  0,  0,  0,  0,
  0,  0,  0,  0,  0,  0,  0,  0];

#[rustfmt::skip]
const KNIGHT_PST: [i32; 64] = [
 -50,-40,-30,-30,-30,-30,-40,-50,
 -40,-20,  0,  0,  0,  0,-20,-40,
 -30,  0, 10, 15, 15, 10,  0,-30,
 -30,  5, 15, 20, 20, 15,  5,-30,
 -30,  0, 15, 20, 20, 15,  0,-30,
 -30,  5, 10, 15, 15, 10,  5,-30,
 -40,-20,  0,  5,  5,  0,-20,-40,
 -50,-40,-30,-30,-30,-30,-40,-50];

#[rustfmt::skip]
const BISHOP_PST: [i32; 64] = [
-20,-10,-10,-10,-10,-10,-10,-20,
-10,  0,  0,  0,  0,  0,  0,-10,
-10,  0,  5, 10, 10,  5,  0,-10,
-10,  5,  5, 10, 10,  5,  5,-10,
-10,  0, 10, 10, 10, 10,  0,-10,
-10, 10, 10, 10, 10, 10, 10,-10,
-10,  5,  0,  0,  0,  0,  5,-10,
-20,-10,-10,-10,-10,-10,-10,-20];

#[rustfmt::skip]
const ROOK_PST: [i32; 64] = [
  0,  0,  0,  0,  0,  0,  0,  0,
  5, 10, 10, 10, 10, 10, 10,  5,
 -5,  0,  0,  0,  0,  0,  0, -5,
 -5,  0,  0,  0,  0,  0,  0, -5,
 -5,  0,  0,  0,  0,  0,  0, -5,
 -5,  0,  0,  0,  0,  0,  0, -5,
 -5,  0,  0,  0,  0,  0,  0, -5,
  0,  0,  3,  7,  7,  5, 0,  0];

#[rustfmt::skip]
const QUEEN_PST: [i32; 64] = [
-20,-10,-10, -5, -5,-10,-10,-20,
-10,  0,  0,  0,  0,  0,  0,-10,
-10,  0,  5,  5,  5,  5,  0,-10,
 -5,  0,  5,  5,  5,  5,  0, -5,
  0,  0,  5,  5,  5,  5,  0, -5,
-10,  5,  5,  5,  5,  5,  0,-10,
-10,  0,  5,  0,  0,  0,  0,-10,
-20,-10,-10, -5, -5,-10,-10,-20];

#[rustfmt::skip]
const KING_PST_MG: [i32; 64] = [
-30,-40,-40,-50,-50,-40,-40,-30,
-30,-40,-40,-50,-50,-40,-40,-30,
-30,-40,-40,-50,-50,-40,-40,-30,
-30,-40,-40,-50,-50,-40,-40,-30,
-20,-30,-30,-40,-40,-30,-30,-20,
-10,-20,-20,-20,-20,-20,-20,-10,
  0,  0,  0,  0,  0,  0,  0,  0,
 20, 30, 15,  0,  0,  5, 30, 10];

#[rustfmt::skip]
const KING_PST_EG: [i32; 64] = [
-50,-40,-30,-20,-20,-30,-40,-50,
-30,-20,-10,  0,  0,-10,-20,-30,
-30,-10, 20, 30, 30, 20,-10,-30,
-30,-10, 30, 40, 40, 30,-10,-30,
-30,-10, 30, 40, 40, 30,-10,-30,
-30,-10, 20, 30, 30, 20,-10,-30,
-30,-30,  0,  0,  0,  0,-30,-30,
-50,-30,-30,-30,-30,-30,-30,-50];

pub static ALL: Stat = Stat::new("ALL");
pub static QUIESCENCE: Stat = Stat::new("QUIESCENCE");
pub static MATERIAL: Stat = Stat::new("MATERIAL");
pub static POSITION: Stat = Stat::new("POSITION");
pub static MOBILITY: Stat = Stat::new("MOBILITY");
pub static SEE: Stat = Stat::new("SEE");
pub static MOVE: Stat = Stat::new("MOVE");

pub static EVAL_COUNTS: ArrayStat =
    ArrayStat(&[&ALL, &QUIESCENCE, &MATERIAL, &POSITION, &MOBILITY, &SEE, &MOVE]);

// pub trait Scorable<Strategy> {
//     fn signum(&self) -> i32;

//     fn eval_move_see(&self, eval: &SimpleScorer, mv: &Move) -> Score;
//     fn eval_move_material(&self, eval: &SimpleScorer, mv: &Move) -> Score;

//     fn eval(&self, eval: &SimpleScorer) -> Score;
//     fn eval_qsearch(&self, eval: &SimpleScorer) -> Score;
//     fn eval_material(&self, eval: &SimpleScorer) -> Score;
//     fn eval_position(&self, eval: &SimpleScorer) -> Score;
//     fn eval_mobility(&self, eval: &SimpleScorer) -> Score;
// }

#[derive(Clone, Debug)]
pub struct SimpleScorer {
    pub material: bool,
    pub position: bool,
    pub mobility: bool,
    pub min_depth_mob: u8,
    pub mobility_phase_disable: u8,
    pub undefended_sq: i32,
    pub undefended_piece: i32,
    pub trapped_piece: i32,
    pub pawn_doubled: i32,
    pub pawn_isolated: i32,
    pub rook_open_file: i32,
    pub phasing: bool,
    pub contempt: i32,
    pub tempo: i32,
    pub material_scores: [i32; Piece::len()],
    pub bishop_pair: i32,
    // pub cache: TranspositionTable,
    // pub qcache: TranspositionTable,
    pub cache_eval: bool,
    pub cache_qeval: bool,
    pst: [[Weight; 64]; Piece::len()],
    pub depth: Ply,
}

impl Default for SimpleScorer {
    fn default() -> Self {
        SimpleScorer::new()
    }
}

impl Component for SimpleScorer {
    fn settings(&self, c: &mut Config) {
        c.set("eval.cache.eval", "type check default true");
        c.set("eval.cache.qeval", "type check default true");
        c.set("eval.mobility", "type check default true");
        c.set("eval.position", "type check default true");
        c.set("eval.material", "type check default true");
        c.set("eval.phasing", "type check default true");
        c.set(
            "eval.mobility.min_depth",
            &format!("type spin min 0 max 101 default {}", self.min_depth_mob),
        );
        c.set(
            "eval.mobility.phase_disable",
            &format!("type spin min 0 max 101 default {}", self.mobility_phase_disable),
        );
        c.set(
            "eval.rook.open_file",
            &format!("type spin min -200 max 200 default {}", self.rook_open_file),
        );
        c.set(
            "eval.pawn.doubled",
            &format!("type spin min -200 max 200 default {}", self.pawn_doubled),
        );
        c.set(
            "eval.mobility.undef_sq",
            &format!("type spin min -200 max 200 default {}", self.undefended_sq),
        );
        c.set(
            "eval.mobility.undef_piece",
            &format!("type spin min -200 max 200 default {}", self.undefended_piece),
        );
        c.set(
            "eval.mobility.trapped_piece",
            &format!("type spin min -200 max 200 default {}", self.trapped_piece),
        );
        c.set(
            "eval.pawn.isolated",
            &format!("type spin min -200 max 200 default {}", self.pawn_isolated),
        );
        c.set(
            "eval.draw_score_contempt",
            &format!("type spin min -10000 max 10000 default {}", self.contempt),
        );
        c.set(
            "eval.bishop_pair",
            &format!("type spin min -10000 max 10000 default {}", self.bishop_pair),
        );
        c.set(
            "eval.tempo",
            &format!("type spin min -1000 max 1000 default {}", self.tempo),
        );
        c.set(
            "eval.p",
            &("type spin min -10000 max 10000 default ".to_string() + &Piece::Pawn.centipawns().to_string()),
        );
        c.set(
            "eval.n",
            &("type spin min -10000 max 10000 default ".to_string()
                + &Piece::Knight.centipawns().to_string()),
        );
        c.set(
            "eval.b",
            &("type spin min -10000 max 10000 default ".to_string()
                + &Piece::Bishop.centipawns().to_string()),
        );
        c.set(
            "eval.r",
            &("type spin min -10000 max 10000 default ".to_string() + &Piece::Rook.centipawns().to_string()),
        );
        c.set(
            "eval.q",
            &("type spin min -10000 max 10000 default ".to_string() + &Piece::Queen.centipawns().to_string()),
        );
    }

    fn configure(&mut self, c: &Config) {
        debug!("eval.configure");
        self.cache_eval = c.bool("eval.cache.eval").unwrap_or(self.cache_eval);
        self.cache_qeval = c.bool("eval.cache.qeval").unwrap_or(self.cache_qeval);
        self.mobility = c.bool("eval.mobility").unwrap_or(self.mobility);
        self.mobility_phase_disable = c.int("eval.mobility.phase_disable").unwrap_or(self.mobility_phase_disable as i64) as u8;
        self.min_depth_mob = c.int("eval.mobility.min_depth").unwrap_or(self.min_depth_mob as i64) as u8;
        self.position = c.bool("eval.position").unwrap_or(self.position);
        self.material = c.bool("eval.material").unwrap_or(self.material);
        self.phasing = c.bool("eval.phasing").unwrap_or(self.phasing);
        self.undefended_piece = c.int("eval.mobility.undef_piece").unwrap_or(self.undefended_piece as i64) as i32;
        self.bishop_pair = c.int("eval.material.bishop_pair").unwrap_or(self.bishop_pair as i64) as i32;
        self.trapped_piece = c.int("eval.mobility.trapped_piece").unwrap_or(self.trapped_piece as i64) as i32;
        self.undefended_sq = c.int("eval.mobility.undef_sq").unwrap_or(self.undefended_sq as i64) as i32;
        self.pawn_doubled = c.int("eval.pawn.doubled").unwrap_or(self.pawn_doubled as i64) as i32;
        self.pawn_isolated = c.int("eval.pawn.isolated").unwrap_or(self.pawn_isolated as i64) as i32;
        self.rook_open_file = c.int("eval.rook.open_file").unwrap_or(self.rook_open_file as i64) as i32;
        self.contempt = c.int("eval.draw_score_contempt").unwrap_or(self.contempt as i64) as i32;
        self.tempo = c.int("eval.tempo").unwrap_or(self.tempo as i64) as i32;

        for p in &Piece::ALL_BAR_NONE {
            let mut name = "eval.".to_string();
            name.push(p.to_char(Some(Color::Black)));
            if let Some(i) = c.int(&name) {
                self.material_scores[*p] = i as i32;
            }
        }
    }

    fn new_game(&mut self) {
    }

    fn new_search(&mut self) {
    }

}

impl fmt::Display for SimpleScorer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "cache eval       : {}", self.cache_eval)?;
        writeln!(f, "cache qeval      : {}", self.cache_qeval)?;
        writeln!(f, "material         : {}", self.material)?;
        writeln!(f, "position         : {}", self.position)?;
        writeln!(f, "mobility         : {}", self.mobility)?;
        writeln!(f, "mob.phase_disable: {}", self.mobility_phase_disable)?;
        writeln!(f, "mob.min_depth:     {}", self.min_depth_mob)?;
        writeln!(f, "phasing          : {}", self.phasing)?;
        writeln!(f, "bishiop_pair     : {}", self.bishop_pair)?;
        writeln!(f, "undefended_piece : {}", self.undefended_piece)?;
        writeln!(f, "undefended_sq    : {}", self.undefended_sq)?;
        writeln!(f, "trapped_peice    : {}", self.trapped_piece)?;
        writeln!(f, "pawn.doubled     : {}", self.pawn_doubled)?;
        writeln!(f, "pawn.isolated    : {}", self.pawn_isolated)?;
        writeln!(f, "rook.open_file   : {}", self.rook_open_file)?;
        writeln!(f, "contempt         : {}", self.contempt)?;
        writeln!(f, "tempo            : {}", self.tempo)?;
        writeln!(f, "material scores  : {:?}", self.material_scores)?;
        writeln!(f, "eval stats\n{}", EVAL_COUNTS)?;
        // writeln!(f, "cache\n{}", self.cache)?;
        // writeln!(f, "qcache\n{}", self.qcache)?;

        Ok(())
    }
}

const MATERIAL_SCORES: [i32; Piece::len()] = [
    0, // None
    Piece::Pawn.centipawns(),
    Piece::Knight.centipawns(),
    Piece::Bishop.centipawns(),
    Piece::Rook.centipawns(),
    Piece::Queen.centipawns(),
    0, // king,
];

// builder methods
impl SimpleScorer {
    pub fn new() -> Self {
        SimpleScorer {
            cache_eval: false,
            cache_qeval: false,
            mobility: true,
            position: true,
            material: true,
            phasing: true,
            mobility_phase_disable: 80,
            min_depth_mob: 1,
            bishop_pair: 30,
            undefended_piece: 6,
            undefended_sq: 3,
            trapped_piece: -10,
            pawn_doubled: -10,
            pawn_isolated: -10,
            rook_open_file: 20,
            contempt: -30, // typically -ve
            tempo: 15,
            material_scores: MATERIAL_SCORES,
            // cache: TranspositionTable::default(),
            // qcache: TranspositionTable::default(),
            pst: Self::calculate_pst(),
            depth: 0,
        }
    }

    fn calculate_pst() -> [[Weight; 64]; Piece::len()] {
        let mut pst = [[Weight::default(); 64]; Piece::len()];
        for &p in &Piece::ALL_BAR_NONE {
            for sq in Square::all() {
                pst[p][sq] = Weight::new(SQUARE_VALUES_MG[p][sq], SQUARE_VALUES_EG[p][sq]);
            }
        }
        pst
    }

    pub fn set_position(&mut self, enabled: bool) -> Self {
        self.position = enabled;
        self.clone()
    }
}

impl SimpleScorer {
    pub fn w_evaluate(&mut self, board: &Board, node: &Node) -> Score {
        counts::EVAL_COUNT.increment();
        let outcome = board.outcome();
        let score = if outcome.is_game_over() {
            Score::score_from_outcome(self.contempt, outcome, board.color_us(), node.ply)
        } else {
            self.w_eval_without_wdl(board, node)
        };
        score
    }

    pub fn w_eval_qsearch(&mut self, board: &Board, node :&Node) -> Score {
        counts::QEVAL_COUNT.increment();
        // we check for insufficient material and 50/75 move draws.
        let outcome = board.draw_outcome();
        let score = if let Some(outcome) = outcome {
            if outcome.is_game_over() {
                return Score::score_from_outcome(
                    self.contempt,
                    outcome,
                    board.color_us(),
                    node.ply,
                );
            } else {
                self.w_eval_without_wdl(board, node)
            }
        } else {
            self.w_eval_without_wdl(board, node)
        };
        score
    }

    fn w_eval_without_wdl(&mut self, board: &Board, _node: &Node) -> Score {
        // if self.cache_eval {
        //     if let Some(entry) = self.cache.probe_by_board(board) {
        //         counts::EVAL_CACHE_COUNT.increment();
        //         return entry.score;
        //     }
        // }

        let ma = if self.material {
            let mat = Material::from_board(board);
            self.w_eval_material(&mat)
        } else {
            0
        };
        let po = if self.position {
            self.w_eval_position(board)
        } else {
            0
        };
        let mo = if self.mobility && self.depth >= self.min_depth_mob as i32 {
            self.w_eval_mobility(board)
        } else {
            0
        };
        let te = Score::side_to_move_score(self.tempo, board.color_us());
        let score = Score::from_cp(ma + po + mo) + te;
        // if self.cache_eval {
        //     if let Some(entry) = self.cache.probe_by_board(board) {
        //         counts::EVAL_CACHE_COUNT.increment();
        //         debug_assert!(entry.score == score, "unmatched score for board {:#}", board);
        //     }
        // }
        // if self.cache_eval && board.fifty_halfmove_clock() <= self.qcache.hmvc_horizon {
        //     let entry = TtNode {
        //         score,
        //         depth: 0,
        //         node_type: NodeType::Pv,
        //         bm: Move::NULL_MOVE, // not set for NodeType::All
        //     };
        //     self.cache.store(board.hash(), entry);
        // }
        score
    }

    // always updated
    pub fn w_eval_mobility(&self, b: &Board) -> i32 {
        let mut score = 0;
        if b.phase() > self.mobility_phase_disable as i32 {
            return 0;
        }
        if self.pawn_doubled != 0 {
            score += self.pawn_doubled
                * (BitboardDefault::doubled_pawns(b.white() & b.pawns()).popcount()
                    - BitboardDefault::doubled_pawns(b.black() & b.pawns()).popcount());
        }
        if self.pawn_isolated != 0 {
            score += self.pawn_isolated
                * (BitboardDefault::isolated_pawns(b.white() & b.pawns()).popcount()
                    - BitboardDefault::isolated_pawns(b.black() & b.pawns()).popcount());
        }

        if self.rook_open_file != 0 {
            let open_files = BitboardDefault::open_files(b.pawns());
            score += self.rook_open_file
                * ((b.rooks() & b.white() & open_files).popcount()
                    - (b.rooks() & b.black() & open_files).popcount())
        }
        let mut piece_mobility = 0;
        if self.undefended_sq > 0 || self.undefended_sq > 0 {
            piece_mobility += self.piece_mobility(&b, Color::White);
            piece_mobility -= self.piece_mobility(&b, Color::Black);
        }
        score += Weight::new(piece_mobility, piece_mobility/10).interpolate(b.phase());
        score
    }

    pub fn piece_mobility(&self, b: &Board, our: Color) -> i32 {
        let us = b.color(our);
        let mut score = 0;
        let their = our.opposite();
        let them = b.color(their);
        let occ = them | us;
        let bb = BitboardDefault::default();
        let their_p = b.pawns() & them;
        let (pe, pw) = bb.pawn_attacks(their_p, their);
        let pa = pe | pw;
        let bi = b.bishops() & them;
        let ni = b.knights() & them;
        let r = b.rooks() & them;
        let q = b.queens() & them;

        for sq in ((b.knights() | b.bishops() | b.rooks() | b.queens()) & us).squares() {
            let p = b.piece_at(sq.as_bb());

            // non-pawn-defended empty or oppoent sq
            let our_attacks = bb.non_pawn_attacks(our, p, us, them, sq) - pa;
            let empties = (our_attacks - b.occupied()).popcount();

            // those attacks on enemy that arent pawn defended and cant attack back
            let non_pawn_defended = match p {
                Piece::Queen => (our_attacks & occ - q - r - bi).popcount(),
                Piece::Rook => (our_attacks & occ  - r).popcount(),
                Piece::Knight => (our_attacks & occ  - ni).popcount(),
                Piece::Bishop => (our_attacks & occ  - bi - q).popcount(),
                _ => 0,
            };
            // trapped piece
            if empties + non_pawn_defended == 1 {
                score += self.trapped_piece / 2;
            }
            if empties + non_pawn_defended == 0 {
                score += self.trapped_piece;
            }
            score += empties * self.undefended_sq + non_pawn_defended * self.undefended_piece;
        }
        score
    }

    // P(osition) S(quare) T(able)
    #[inline]
    pub fn pst(&self, p: Piece, sq: Square) -> Weight {
        self.pst[p][sq]
    }

    // #[inline]
    // pub fn pst_mg(p: Piece, sq: Square) -> i32 {
    //     SQUARE_VALUES_MG[p.index()][sq.index()]
    // }

    // #[inline]
    // pub fn pst_eg(p: Piece, sq: Square) -> i32 {
    //     SQUARE_VALUES_EG[p.index()][sq.index()]
    // }

    // piece positions, king safety, centre control
    // only updated for the colour thats moved - opponents(blockes) not relevant
    pub fn w_eval_position(&self, board: &Board) -> i32 {
        let mut sum = 0_i32;
        let phase = if self.phasing { board.phase() } else { 0 };
        for &p in &Piece::ALL_BAR_NONE {
            let w = (board.pieces(p) & board.white()).flip_vertical();
            let b = board.pieces(p) & board.black();

            let w = w.squares().map(|sq| self.pst(p, sq)).sum::<Weight>();
            let b = b.squares().map(|sq| self.pst(p, sq)).sum::<Weight>();

            sum += (w - b).interpolate(phase);
        }
        sum
    }

    // // piece positions, king safety, centre control
    // // only updated for the colour thats moved - opponents(blockes) not relevant
    // pub fn w_eval_position_old(&self, board: &Board) -> i32 {
    //     let mut sum = 0_i32;
    //     for &p in &Piece::ALL_BAR_NONE {
    //         let w = (board.pieces(p) & board.white()).flip_vertical();
    //         let b = board.pieces(p) & board.black();

    //         let w_mg: i32 = w.iter().map(|bb| Self::pst_mg(p, bb.first_square())).sum();
    //         let b_mg: i32 = b.iter().map(|bb| Self::pst_mg(p, bb.first_square())).sum();

    //         let w_eg: i32 = w.iter().map(|bb| Self::pst_eg(p, bb.first_square())).sum();
    //         let b_eg: i32 = b.iter().map(|bb| Self::pst_eg(p, bb.first_square())).sum();
    //         let eg_perc = if self.phasing { board.phase() } else { 0 };

    //         sum += ((w_mg - b_mg) * (100 - eg_perc) + (w_eg - b_eg) * eg_perc) / 100;
    //     }
    //     sum
    // }

    // updated on capture & promo
    pub fn w_eval_material(&self, mat: &Material) -> i32 {
        let mut score = Piece::ALL_BAR_KING
            .iter()
            .map(|&p| {
                self.material_scores[p] * (mat.counts(Color::White, p) - mat.counts(Color::Black, p))
            })
            .sum();

        if mat.counts(Color::White, Piece::Bishop) >= 2 {
            score += self.bishop_pair
        }
        if mat.counts(Color::Black, Piece::Bishop) >= 2 {
            score -= self.bishop_pair
        }
        score
        // let mut total = 0_i32;
        // }
        // for &p in &Piece::ALL_BAR_NONE {
        //     total +=
        //         self.material_scores[p.index()] * (mat.counts(Color::White, p) - mat.counts(Color::Black, p));
        // }
        // total
    }

    pub fn eval_move_material(&self, mv: &Move) -> i32 {
        let mut score = 0;
        if mv.is_capture() {
            score += self.material_scores[mv.capture_piece()];
        }
        if mv.is_promo() {
            score += self.material_scores[mv.promo_piece()];
        }
        score
    }
}

impl Board {
    #[inline]
    fn signum(&self) -> i32 {
        self.color_us().chooser_wb(1, -1)
    }

    #[inline]
    pub fn eval_qsearch(&self, eval: &mut SimpleScorer, nd: &Node) -> Score {
        QUIESCENCE.increment();
        self.signum() * eval.w_eval_qsearch(self, nd)
    }

    #[inline]
    pub fn eval_move_see(&self, eval: &SimpleScorer, mv: &Move) -> Score {
        SEE.increment();
        Score::from_cp(eval.eval_move_see(self, &mv))
    }

    #[inline]
    pub fn eval_move_material(&self, eval: &SimpleScorer, mv: &Move) -> Score {
        MOVE.increment();
        Score::from_cp(eval.eval_move_material(&mv))
    }

    #[inline]
    pub fn eval(&self, eval: &mut SimpleScorer, nd: &Node) -> Score {
        ALL.increment();
        self.signum() * eval.w_evaluate(self, nd)
    }

    #[inline]
    pub fn eval_material(&self, eval: &SimpleScorer) -> Score {
        MATERIAL.increment();
        let m = Material::from_board(self);
        let s = eval.w_eval_material(&m);
        Score::from_cp(self.signum() * s)
    }
    #[inline]
    pub fn eval_position(&self, eval: &SimpleScorer) -> Score {
        POSITION.increment();
        let s = eval.w_eval_position(self);
        Score::from_cp(self.signum() * s)
    }
    pub fn eval_mobility(&self, eval: &SimpleScorer) -> Score {
        MOBILITY.increment();
        let s = eval.w_eval_mobility(self);
        Score::from_cp(self.signum() * s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::BoardBuf;
    use crate::catalog::Catalog;

    #[test]
    fn test_score_material() {
        let board = Catalog::starting_position();
        let eval = &mut SimpleScorer::new();
        assert_eq!(board.eval(eval, &Node::root()), Score::from_cp(0));

        let starting_pos_score = 8 * 100 + 2 * 325 + 2 * 350 + 2 * 500 + 900;
        let board = Catalog::white_starting_position();
        assert_eq!(board.eval_material(eval), Score::from_cp(starting_pos_score));

        let board = Catalog::black_starting_position();
        assert_eq!(board.eval_material(eval), Score::from_cp(starting_pos_score).negate());
    }

    #[test]
    fn test_eval_configure() {
        let mut eval = SimpleScorer::new();
        eval.configure(&Config::new().set("eval.b", "700"));
        assert_eq!(eval.material_scores[Piece::Bishop], 700);

        let mut eval = SimpleScorer::new();
        eval.configure(&Config::new().set("eval.position", "false"));
        assert_eq!(eval.position, false);
    }

    #[test]
    fn test_score_position() {
        let eval = &SimpleScorer::new();

        let bd = Board::parse_fen("8/P7/8/8/8/8/8/8 w - - 0 1").unwrap().as_board();
        assert_eq!(bd.eval_position(eval), Score::from_cp(60));

        let bd = Board::parse_fen("8/4p3/8/8/8/8/8/8 w - - 0 1")
            .unwrap()
            .as_board();
        assert_eq!(bd.phase(), 100);
        assert_eq!(bd.eval_position(eval), Score::from_cp(0));

        let w = Catalog::white_starting_position();
        assert_eq!(w.eval_position(eval), Score::from_cp(-116));

        let b = Catalog::black_starting_position();
        assert_eq!(w.eval_position(eval), b.eval_position(eval).negate());

        // from blacks perspective to negate
        let bd = Board::parse_fen("8/8/8/8/8/8/p7/8 b - - 0 1").unwrap().as_board();
        assert_eq!(bd.eval_position(eval), -Score::from_cp(-60));
    }

    #[test]
    fn test_score_mobility() {
        let mut eval = SimpleScorer::new();
        eval.pawn_doubled = -1;
        eval.pawn_isolated = 0;
        eval.mobility_phase_disable = 101;
        let b = Catalog::starting_position();
        assert_eq!(eval.w_eval_mobility(&b), 0);

        // 1xw 4xb doubled pawns, 1xw 2xb isolated pawns
        let b = Board::parse_fen("8/pppp1p1p/pppp4/8/8/2P5/PPP4P/8 b - - 0 1")
            .unwrap()
            .as_board();
        eval.pawn_doubled = -1;
        eval.pawn_isolated = 0;
        assert_eq!(eval.w_eval_mobility(&b), 3);

        eval.pawn_doubled = 0;
        eval.pawn_isolated = -1;
        assert_eq!(eval.w_eval_mobility(&b), 1);

        // 1xw (-1) 3xb doubled (+3), 1xb (+1) tripled pawns  2xw 1xb isolated
        let b = Board::parse_fen("8/pppp3p/ppp5/p7/8/2P5/PPP1P1P1/8 b - - 0 1")
            .unwrap()
            .as_board();

        eval.pawn_doubled = -1;
        eval.pawn_isolated = 0;
        assert_eq!(eval.w_eval_mobility(&b), 3);

        eval.pawn_doubled = 0;
        eval.pawn_isolated = -1;
        assert_eq!(eval.w_eval_mobility(&b), -1);
    }
}
