use crate::bitboard::precalc::BitboardDefault;
use crate::bitboard::square::Square;
use crate::board::Board;
use crate::config::{Component, Config};
use crate::eval::score::Score;
use crate::eval::weight::Weight;
use crate::globals::counts;
use crate::material::Material;
use crate::mv::Move;
use crate::search::node::Node;
use crate::stat::{ArrayStat, Stat};
use crate::types::{Color, Piece};
use crate::{debug, logger::LogInit};

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
    pub safety: bool,
    pub min_depth_mob: u8,
    pub mobility_phase_disable: u8,
    pub undefended_sq: i32,
    pub undefended_piece: i32,
    pub trapped_piece: i32,
    pub pawn_doubled: Weight,
    pub pawn_isolated: Weight,
    pub pawn_passed: Weight,
    pub pawn_shield: Weight,
    pub rook_edge: Weight,
    pub pawn_r5: Weight,
    pub pawn_r6: Weight,
    pub pawn_r7: Weight,
    pub rook_open_file: i32,
    pub phasing: bool,
    pub contempt: i32,
    pub tempo: Weight,
    pub material_weights: [Weight; Piece::len()],
    pub bishop_pair: Weight,
    // pub cache: TranspositionTable,
    // pub qcache: TranspositionTable,
    pub cache_eval: bool,
    pub cache_qeval: bool,
    pst: [[Weight; 64]; Piece::len()],
    // pub depth: Ply,
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
        c.set("eval.safety", "type check default true");
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
        c.set_weight("eval.pawn.doubled", &self.pawn_doubled);
        c.set(
            "eval.mobility.undef_sq",
            &format!("type spin min -200 max 200 default {}", self.undefended_sq),
        );
        c.set(
            "eval.mobility.undef_piece",
            &format!("type spin min -200 max 200 default {}", self.undefended_piece),
        );
        c.set(
            "eval.mobility.trapped.piece",
            &format!("type spin min -200 max 200 default {}", self.trapped_piece),
        );
        c.set_weight("eval.pawn.isolated", &self.pawn_isolated);
        c.set_weight("eval.pawn.passed", &self.pawn_passed);
        c.set_weight("eval.pawn.shield", &self.pawn_shield);
        c.set_weight("eval.rook.edge", &self.rook_edge);
        c.set_weight("eval.pawn.r5", &self.pawn_r5);
        c.set_weight("eval.pawn.r6", &self.pawn_r6);
        c.set_weight("eval.pawn.r7", &self.pawn_r7);
        c.set(
            "eval.draw.score.contempt",
            &format!("type spin min -10000 max 10000 default {}", self.contempt),
        );
        c.set_weight("eval.bishop.pair", &self.bishop_pair);
        c.set_weight("eval.tempo", &self.tempo);
        for &p in &Piece::ALL_BAR_KING {
            let mut name = "eval.".to_string();
            name.push(p.to_char(Some(Color::Black)));
            c.set_weight(&name, &self.material_weights[p]);
        }

    }

    fn configure(&mut self, c: &Config) {
        debug!("eval.configure");
        self.cache_eval = c.bool("eval.cache.eval").unwrap_or(self.cache_eval);
        self.cache_qeval = c.bool("eval.cache.qeval").unwrap_or(self.cache_qeval);
        self.mobility = c.bool("eval.mobility").unwrap_or(self.mobility);
        self.safety = c.bool("eval.safety").unwrap_or(self.safety);
        self.mobility_phase_disable = c
            .int("eval.mobility.phase.disable")
            .unwrap_or(self.mobility_phase_disable as i64) as u8;
        self.min_depth_mob = c
            .int("eval.mobility.min.depth")
            .unwrap_or(self.min_depth_mob as i64) as u8;
        self.position = c.bool("eval.position").unwrap_or(self.position);
        self.material = c.bool("eval.material").unwrap_or(self.material);
        self.phasing = c.bool("eval.phasing").unwrap_or(self.phasing);
        self.undefended_piece = c
            .int("eval.mobility.undef.piece")
            .unwrap_or(self.undefended_piece as i64) as i32;
        self.bishop_pair = c.weight("eval.material.bishop.pair", &self.bishop_pair);
        self.trapped_piece = c
            .int("eval.mobility.trapped.piece")
            .unwrap_or(self.trapped_piece as i64) as i32;
        self.undefended_sq = c
            .int("eval.mobility.undef_sq")
            .unwrap_or(self.undefended_sq as i64) as i32;
        self.pawn_doubled = c.weight("eval.pawn.doubled", &self.pawn_doubled);
        self.pawn_isolated = c.weight("eval.pawn.isolated", &self.pawn_isolated);
        self.pawn_passed = c.weight("eval.pawn.passed", &self.pawn_passed);
        self.pawn_shield = c.weight("eval.pawn.shield", &self.pawn_shield);
        self.rook_edge = c.weight("eval.rook.edge", &self.rook_edge);
        self.pawn_r5 = c.weight("eval.pawn.r5", &self.pawn_r5);
        self.pawn_r6 = c.weight("eval.pawn.r6", &self.pawn_r6);
        self.pawn_r7 = c.weight("eval.pawn.r7", &self.pawn_r7);
        self.rook_open_file = c.int("eval.rook.open.file").unwrap_or(self.rook_open_file as i64) as i32;
        self.contempt = c.int("eval.draw.score.contempt").unwrap_or(self.contempt as i64) as i32;
        self.tempo = c.weight("eval.tempo", &self.tempo);

        for &p in &Piece::ALL_BAR_KING {
            let mut name = "eval.".to_string();
            name.push(p.to_char(Some(Color::Black)));
            self.material_weights[p] = c.weight(&name, &self.material_weights[p]);
        }
        self.calculate_pst();
    }

    fn new_game(&mut self) {}

    fn new_position(&mut self) {}
}

impl fmt::Display for SimpleScorer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "cache eval       : {}", self.cache_eval)?;
        writeln!(f, "cache qeval      : {}", self.cache_qeval)?;
        writeln!(f, "material         : {}", self.material)?;
        writeln!(f, "position         : {}", self.position)?;
        writeln!(f, "mobility         : {}", self.mobility)?;
        writeln!(f, "safety           : {}", self.safety)?;
        writeln!(f, "mob.phase.disable: {}", self.mobility_phase_disable)?;
        writeln!(f, "mob.min.depth:     {}", self.min_depth_mob)?;
        writeln!(f, "phasing          : {}", self.phasing)?;
        writeln!(f, "bishiop.pair     : {}", self.bishop_pair)?;
        writeln!(f, "undefended.piece : {}", self.undefended_piece)?;
        writeln!(f, "undefended.sq    : {}", self.undefended_sq)?;
        writeln!(f, "trapped.piece    : {}", self.trapped_piece)?;
        writeln!(f, "pawn.shield      : {}", self.pawn_shield)?;
        writeln!(f, "pawn.doubled     : {}", self.pawn_doubled)?;
        writeln!(f, "pawn.passed      : {}", self.pawn_passed)?;
        writeln!(f, "pawn.isolated    : {}", self.pawn_isolated)?;
        writeln!(f, "rook_edge        : {}", self.rook_edge)?;
        writeln!(f, "rook.open.file   : {}", self.rook_open_file)?;
        writeln!(f, "contempt         : {}", self.contempt)?;
        writeln!(f, "tempo            : {}", self.tempo)?;
        writeln!(f, "material scores  : {:?}", self.material_weights)?;
        writeln!(f, "eval stats\n{}", EVAL_COUNTS)?;
        // writeln!(f, "cache\n{}", self.cache)?;
        // writeln!(f, "qcache\n{}", self.qcache)?;

        Ok(())
    }
}

// const MATERIAL_SCORES: [i32; Piece::len()] = [
//     0, // None
//     Piece::Pawn.centipawns(),
//     350, // Piece::Knight.centipawns(),
//     Piece::Bishop.centipawns(),
//     600,  // Piece::Rook.centipawns(),
//     1100, // Piece::Queen.centipawns(),
//     0,    // king,
// ];

// builder methods
impl SimpleScorer {
    pub fn new() -> Self {
        let mut me = SimpleScorer {
            cache_eval: false,
            cache_qeval: false,
            mobility: true,
            position: true,
            material: true,
            safety: true,
            phasing: true,
            mobility_phase_disable: 60,
            min_depth_mob: 1,
            bishop_pair: Weight::new(40, 85),
            undefended_piece: 6,
            undefended_sq: 3,
            trapped_piece: -10,
            pawn_doubled: Weight::new(-5, -50),
            pawn_isolated: Weight::new(-5, -50),
            pawn_passed: Weight::new(50, 80),
            pawn_shield: Weight::new(50, 0),
            rook_open_file: 20,
            rook_edge: Weight::new(0, 2),
            pawn_r5: Weight::new(5, 20),
            pawn_r6: Weight::new(10, 40),
            pawn_r7: Weight::new(40, 60),
            contempt: -30, // typically -ve
            tempo: Weight::new(16, 16),
            material_weights: [ Weight::default(),
                                Weight::new(100, 100),
                                Weight::new(350, 350), // knights
                                Weight::new(350, 350),
                                Weight::new(600, 625),
                                Weight::new(1100, 1300),
                                Weight::new(0, 0),   // king
                                ],
            // cache: TranspositionTable::default(),
            // qcache: TranspositionTable::default(),
            pst: [[Weight::default(); 64]; Piece::len()],
            // depth: 0,
        };
        me.calculate_pst();
        me
    }

    fn calculate_pst(&mut self) {
        
        let r5 = self.pawn_r5.s();
        let r6 = self.pawn_r6.s();
        let r7 = self.pawn_r7.s();

        #[rustfmt::skip]
        let pawn_pst_mg: [i32; 64] = [
          0,  0,  0,  0,  0,  0,  0,  0,
         r7, r7, r7, r7, r7, r7, r7, r7,
         r6, r6, r6, r6, r6, r6, r6, r6,
         r5, r5, r5,r5+5,r5+5, r5, r5, r5,
         -9, 0,  0, 20, 20, -5,  -5, -9,
         -5,-5, -9,  0,  0, -9, -5, -5,
         9, 15, 15,-35,-35, 15, 15,  10,
         0,  0,  0,  0,  0,  0,  0,  0];
        
        let r5 = self.pawn_r5.e();
        let r6 = self.pawn_r6.e();
        let r7 = self.pawn_r7.e();
         // FIXME! file A and H 
        #[rustfmt::skip]
         let pawn_pst_eg: [i32; 64] = [
         0,  0,  0,  0,  0,  0,  0,  0,
         r7, r7, r7, r7, r7, r7, r7, r7,
         r6, r6, r6, r6, r6, r6, r6, r6,
         r5, r5, r5, r5, r5, r5, r5, r5,
         10, 10, 10, 10, 10, 10, 10, 10,
          5,  5,  5,  5,  5,  5,  5,  5,
          0,  0,  0,  0,  0,  0,  0,  0,
          0,  0,  0,  0,  0,  0,  0,  0];
        
        
          #[rustfmt::skip]
        let knight_pst_mg: [i32; 64] = [
         -50,-40,-30,-30,-30,-30,-40,-50,
         -40,-20,  0,  0,  0,  0,-20,-40,
         -30,  0, 10, 15, 15, 10,  0,-30,
         -30,  5, 15, 20, 20, 15,  5,-30,
         -30,  0, 15, 20, 20, 15,  0,-30,
         -30,  5, 10, 15, 15, 10,  5,-30,
         -40,-20,  0,  5,  5,  0,-20,-40,
         -50,-40,-30,-30,-30,-30,-40,-50];
        
         #[rustfmt::skip]
        let knight_pst_eg: [i32; 64] = [
         -50,-40,-30,-30,-30,-30,-40,-50,
         -40,-20,  0,  0,  0,  0,-20,-40,
         -30,  0, 10, 15, 15, 10,  0,-30,
         -30,  5, 15, 20, 20, 15,  5,-30,
         -30,  0, 15, 20, 20, 15,  0,-30,
         -30,  5, 10, 15, 15, 10,  5,-30,
         -40,-20,  0,  5,  5,  0,-20,-40,
         -50,-40,-30,-30,-30,-30,-40,-50];

        
         #[rustfmt::skip]
        let bishop_pst_mg: [i32; 64] = [
        -20,-10,-10,-10,-10,-10,-10,-20,
        -10,  0,  0,  0,  0,  0,  0,-10,
        -10,  0,  5, 10, 10,  5,  0,-10,
        -10,  5,  5, 10, 10,  5,  5,-10,
        -10,  0, 10, 10, 10, 10,  0,-10,
        -10, 10, 10, 10, 10, 10, 10,-10,
        -10,  5,  0,  0,  0,  0,  5,-10,
        -20,-10,-10,-10,-10,-10,-10,-20];
        
        #[rustfmt::skip]
        let bishop_pst_eg: [i32; 64] = [
        -20,-10,-10,-10,-10,-10,-10,-20,
        -10,  0,  0,  0,  0,  0,  0,-10,
        -10,  0,  5, 10, 10,  5,  0,-10,
        -10,  5,  5, 10, 10,  5,  5,-10,
        -10,  0, 10, 10, 10, 10,  0,-10,
        -10, 10, 10, 10, 10, 10, 10,-10,
        -10,  5,  0,  0,  0,  0,  5,-10,
        -20,-10,-10,-10,-10,-10,-10,-20];

        
        #[rustfmt::skip]
        let rook_pst_mg: [i32; 64] = [
         0,  0,  0,  0,  0,  0,  0,  0,
         5, 10, 10, 10, 10, 10, 10,  5,
        -5,  0,  0,  0,  0,  0,  0, -5,
        -5,  0,  0,  0,  0,  0,  0, -5,
        -5,  0,  0,  0,  0,  0,  0, -5,
        -5,  0,  0,  0,  0,  0,  0, -5,
        -5,  0,  0,  0,  0,  0,  0, -5,
         0,  0,  3,  7,  7,  5,  0,  0];

        let a = self.rook_edge.e();
         #[rustfmt::skip]
        let rook_pst_eg: [i32; 64] = [
        a,  a,  a,  a,  a,  a,  a,  a,
        a,  0,  0,  0,  0,  0,  0,  a,
        a,  0,  0,  0,  0,  0,  0,  a,
        a,  0,  0,  0,  0,  0,  0,  a,
        a,  0,  0,  0,  0,  0,  0,  a,
        a,  0,  0,  0,  0,  0,  0,  a,
        a,  0,  0,  0,  0,  0,  0,  a,
        a,  a,  a,  a,  a,  a,  a,  a];

        
        #[rustfmt::skip]
        let queen_pst_mg: [i32; 64] = [
        -20,-10,-10, -5, -5,-10,-10,-20,
        -10,  0,  0,  0,  0,  0,  0,-10,
        -10,  0,  5,  5,  5,  5,  0,-10,
         -5,  0,  5,  5,  5,  5,  0, -5,
          0,  0,  5,  5,  5,  5,  0, -5,
        -10,  5,  5,  5,  5,  5,  0,-10,
        -10,  0,  5,  0,  0,  0,  0,-10,
        -20,-10,-10, -5, -5,-10,-10,-20];
        
        #[rustfmt::skip]
        let queen_pst_eg: [i32; 64] = [
        -20,-10,-10, -5, -5,-10,-10,-20,
        -10,  0,  0,  0,  0,  0,  0,-10,
        -10,  0,  5,  5,  5,  5,  0,-10,
         -5,  0,  5,  5,  5,  5,  0, -5,
          0,  0,  5,  5,  5,  5,  0, -5,
        -10,  5,  5,  5,  5,  5,  0,-10,
        -10,  0,  5,  0,  0,  0,  0,-10,
        -20,-10,-10, -5, -5,-10,-10,-20];


        #[rustfmt::skip]
        let king_pst_mg: [i32; 64] = [
        -30,-40,-40,-50,-50,-40,-40,-30,
        -30,-40,-40,-50,-50,-40,-40,-30,
        -30,-40,-40,-50,-50,-40,-40,-30,
        -30,-40,-40,-50,-50,-40,-40,-30,
        -20,-30,-30,-40,-40,-30,-30,-20,
        -10,-20,-20,-20,-20,-20,-20,-10,
          0,  0,  0,  0,  0,  0,  0,  0,
         20, 30, 15,  0,  0,  5, 30, 10];
        
        #[rustfmt::skip]
        let king_pst_eg: [i32; 64] = [
        -50,-40,-30,-20,-20,-30,-40,-50,
        -30,-20,-10,  0,  0,-10,-20,-30,
        -30,-10, 20, 30, 30, 20,-10,-30,
        -30,-10, 30, 40, 40, 30,-10,-30,
        -30,-10, 30, 40, 40, 30,-10,-30,
        -30,-10, 20, 30, 30, 20,-10,-30,
        -30,-30,  0,  0,  0,  0,-30,-30,
        -50,-30,-30,-30,-30,-30,-30,-50];
        



        let square_values_mg: [[i32; 64]; Piece::len()] = [
            pawn_pst_mg,
            pawn_pst_mg,
            knight_pst_mg,
            bishop_pst_mg,
            rook_pst_mg,
            queen_pst_mg,
            king_pst_mg,
        ];
        let square_values_eg: [[i32; 64]; Piece::len()] = [
            pawn_pst_eg,
            pawn_pst_eg,
            knight_pst_eg,
            bishop_pst_eg,
            rook_pst_eg,
            queen_pst_eg,
            king_pst_eg,
        ];

        for &p in &Piece::ALL_BAR_NONE {
            for sq in Square::all() {
                self.pst[p][sq] = Weight::new(square_values_mg[p][sq], square_values_eg[p][sq]);
            }
        }
    }

    pub fn set_position(&mut self, enabled: bool) -> Self {
        self.position = enabled;
        self.clone()
    }
}

impl SimpleScorer {
    pub fn w_tempo_adjustment(&self, us: Color) -> Weight {
        // axiom: we're white
        // white to move => advantage, black to move means white has a disadvantage
        if us == Color::White {
            self.tempo
        } else {
            -self.tempo
        }
    }

    pub fn w_eval_draw(&mut self, board: &Board, node: &Node) -> Score {
        // draw score is +ve for playing a stronger opponent (we want a draw), neg for weaker
        //
        //  Engine Col   |  search ply   |  value to searcher   | Score to white
        //     W               0                   +ve               +ve
        //     B               0                   +ve               -ve
        //     W               1 (oppo B)          -ve               +ve (a bonus to white opponet)
        //     B               1 (oppo W)          -ve               -ve
        // board.color_us() == Color::White => maximising
        // +ve contempt => +ve score => aim for draw => opponent stronger than us
        // board.color_us() == Color::Black => minimising
        // +ve contempt => -ve score => aim for draw => opponent stronger than us
        let signum = 1 - (node.ply % 2) * 2;   // ply=0 => 1  ply=1=> -1
        let contempt =  signum * self.contempt + board.signum();
        return Score::from_cp(contempt);
    }

    pub fn w_evaluate(&mut self, board: &Board, node: &Node) -> Score {
        counts::EVAL_COUNT.increment();
        let outcome = board.outcome();
        if outcome.is_game_over() {
            if outcome.is_draw() {
                return self.w_eval_draw(board, node);
            }
            if let Some(c) = outcome.winning_color() {
                return c.chooser_wb(Score::white_win(node.ply), Score::white_loss(node.ply));
            }
        }
        self.w_eval_without_wdl(board, node)
    }

    // we dont care about draws in qsearch
    pub fn w_eval_qsearch(&mut self, board: &Board, node: &Node) -> Score {
        counts::QEVAL_COUNT.increment();
        // we check for insufficient material and 50/75 move draws.
        // let outcome = board.draw_outcome();
        // let score = if let Some(outcome) = outcome {
        //     if outcome.is_game_over() {
        //         return Score::score_from_outcome(self.contempt, outcome, board.color_us(), node.ply);
        //     } else {
        //         self.w_eval_without_wdl(board, node)
        //     }
        // } else {
        self.w_eval_without_wdl(board, node)
        // };
        // score
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
            Weight::zero()
        };
        let po = if self.position {
            self.w_eval_position(board)
        } else {
            Weight::zero()
        };
        let mo = if self.mobility {
            self.w_eval_mobility(board)
        } else {
            Weight::zero()
        };
        let sa = if self.safety {
            self.w_eval_safety(board)
        } else {
            Weight::zero()
        };
        let te = self.w_tempo_adjustment(board.color_us());
        Score::from_cp((ma + po + mo + sa + te).interpolate(board.phase()))
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
    }

    pub fn eval_pawns(&self, c: Color, b: &Board) -> Weight {
        let mut score = Weight::zero();
        let bbd = BitboardDefault::default();

        let doubled = bbd.doubled_pawns(b.color(c) & b.pawns()).popcount();
        score += doubled * self.pawn_doubled;

        let isolated = bbd.isolated_pawns(b.color(c) & b.pawns()).popcount();
        score += isolated * self.pawn_isolated;

        let mut passed = 0;
        for p in (b.pawns() & b.color(c)).squares() {
            let doubled = p.is_in(bbd.doubled_pawns(b.color(c) & b.pawns()));
            let is_passed =
                (bbd.pawn_front_span(c, p) & b.pawns() & b.color(c.opposite())).is_empty() && !doubled;
            if is_passed {
                passed += 1;
            }
        }
        score += passed * self.pawn_passed;

        score
    }

    // always updated
    pub fn w_eval_mobility(&self, b: &Board) -> Weight {
        if b.phase() > self.mobility_phase_disable as i32 {
            return Weight::zero();
        }

        let mut score = self.eval_pawns(Color::White, b) - self.eval_pawns(Color::Black, b);

        if self.rook_open_file != 0 {
            let open_files = BitboardDefault::default().open_files(b.pawns());
            let s = self.rook_open_file
                * ((b.rooks() & b.white() & open_files).popcount()
                    - (b.rooks() & b.black() & open_files).popcount());
            score += Weight::new(s, s);
        }
        let mut piece_mobility = 0;
        if self.undefended_sq != 0 || self.undefended_piece != 0 {
            piece_mobility += self.piece_mobility(&b, Color::White);
            piece_mobility -= self.piece_mobility(&b, Color::Black);
        }
        score += Weight::new(piece_mobility, piece_mobility / 10);
        score
    }

    pub fn w_eval_safety(&self, b: &Board) -> Weight {
        let wp = b.pawns() & b.white();
        let bp = b.pawns() & b.black();
        let wk = b.kings() & b.white();
        let bk = b.kings() & b.black();
        let bb = BitboardDefault::default();
        let (mut w_nearby_pawns, mut b_nearby_pawns) = (0, 0);
        if wk.any() {
            w_nearby_pawns = (wp & bb.king_attacks(wk.square())).popcount();
        }
        if bk.any() {
            b_nearby_pawns = (bp & bb.king_attacks(bk.square())).popcount();
        }
        (w_nearby_pawns - b_nearby_pawns) * self.pawn_shield
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
                Piece::Rook => (our_attacks & occ - r).popcount(),
                Piece::Knight => (our_attacks & occ - ni).popcount(),
                Piece::Bishop => (our_attacks & occ - bi - q).popcount(),
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
    pub fn w_eval_position(&self, board: &Board) -> Weight {
        let mut sum = Weight::zero();
        for &p in &Piece::ALL_BAR_NONE {
            let w = (board.pieces(p) & board.white()).flip_vertical();
            let b = board.pieces(p) & board.black();

            let w = w.squares().map(|sq| self.pst(p, sq)).sum::<Weight>();
            let b = b.squares().map(|sq| self.pst(p, sq)).sum::<Weight>();

            sum = sum + w - b;
        }
        sum
    }

    #[inline]
    pub fn w_eval_square(&self, c: Color, p: Piece, mut sq: Square) -> Weight {
        if c == Color::White {
            sq = sq.flip_vertical();
        }
        self.pst(p, sq)
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
    //         let eg_perc = if self.phasing { board.phase() } else { 0 };.S

    //         sum += ((w_mg - b_mg) * (100 - eg_perc) + (w_eg - b_eg) * eg_perc) / 100;
    //     }
    //     sum
    // }

    // updated on capture & promo
    pub fn w_eval_material(&self, mat: &Material) -> Weight {
        let mut weight = Piece::ALL_BAR_KING
            .iter()
            .map(|&p| (mat.counts(Color::White, p) - mat.counts(Color::Black, p)) * self.material_weights[p] )
            .sum();

        // let mut weight = Weight::new(score, score);
        if mat.counts(Color::White, Piece::Bishop) >= 2 {
            weight = weight + self.bishop_pair
        }
        if mat.counts(Color::Black, Piece::Bishop) >= 2 {
            weight = weight - self.bishop_pair
        }
        weight
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
            score += self.material_weights[mv.capture_piece()].s();
        }
        if mv.is_promo() {
            score += self.material_weights[mv.promo_piece()].s();
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
    pub fn eval_draw(&self, eval: &mut SimpleScorer, nd: &Node) -> Score {
        self.signum() * eval.w_eval_draw(self, nd)
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
        let weight = eval.w_eval_material(&m);
        let s = weight.interpolate(self.phase());
        Score::from_cp(self.signum() * s)
    }
    #[inline]
    pub fn eval_position(&self, eval: &SimpleScorer) -> Score {
        POSITION.increment();
        let s = eval.w_eval_position(self).interpolate(self.phase());
        Score::from_cp(self.signum() * s)
    }
    pub fn eval_mobility(&self, eval: &SimpleScorer) -> Score {
        MOBILITY.increment();
        let s = eval.w_eval_mobility(self).interpolate(self.phase());
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
        let board = Catalog::starting_board();
        let eval = &mut SimpleScorer::new();
        eval.tempo = Weight::zero();
        assert_eq!(board.eval(eval, &Node::root(0)), Score::from_cp(0));

        let starting_pos_score = 8 * 100 + 2 * 350 + 2 * 350 + 2 * 600 + 1100 + (40 + 85) / 2; // (bishop pair, half the pieces)
        let board = Catalog::white_starting_position();
        assert_eq!(board.eval_material(eval), Score::from_cp(starting_pos_score));

        let board = Catalog::black_starting_position();
        assert_eq!(
            board.eval_material(eval),
            Score::from_cp(starting_pos_score).negate()
        );
    }

    #[test]
    fn test_eval_configure() {
        let mut eval = SimpleScorer::new();
        eval.configure(&Config::new().set("eval.b.s", "700"));
        assert_eq!(eval.material_weights[Piece::Bishop].s(), 700);

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
        assert_eq!(w.phase(), 50);
        assert_eq!(w.eval_position(eval), Score::from_cp(-113));

        let b = Catalog::black_starting_position();
        assert_eq!(w.eval_position(eval), b.eval_position(eval).negate());

        // from blacks perspective to negate
        let bd = Board::parse_fen("8/8/8/8/8/8/p7/8 b - - 0 1").unwrap().as_board();
        assert_eq!(bd.eval_position(eval), -Score::from_cp(-60));
    }

    #[test]
    fn test_score_mobility() {
        let mut eval = SimpleScorer::new();
        eval.pawn_doubled = Weight::new(-1, -1);
        eval.pawn_isolated = Weight::zero();
        eval.mobility_phase_disable = 101;
        let b = Catalog::starting_board();
        assert_eq!(eval.w_eval_mobility(&b), Weight::zero());

        // 1xw 4xb doubled pawns, 1xw 2xb isolated pawns, 1xb passed pawn
        let b = Board::parse_fen("8/pppp1p1p/pppp4/8/8/2P5/PPP4P/8 b - - 0 1")
            .unwrap()
            .as_board();
        eval.pawn_doubled = Weight::new(-1, -1);
        eval.pawn_isolated = Weight::zero();
        eval.pawn_passed = Weight::zero();
        assert_eq!(eval.eval_pawns(Color::White, &b), Weight::new(-1, -1));
        assert_eq!(eval.eval_pawns(Color::Black, &b), Weight::new(-4, -4));

        eval.pawn_doubled = Weight::zero();
        eval.pawn_isolated = Weight::new(-1, -1);
        eval.pawn_passed = Weight::zero();
        assert_eq!(eval.eval_pawns(Color::White, &b), Weight::new(-1, -1));
        assert_eq!(eval.eval_pawns(Color::Black, &b), Weight::new(-2, -2));

        eval.pawn_doubled = Weight::zero();
        eval.pawn_isolated = Weight::zero();
        eval.pawn_passed = Weight::new(10, 10);
        assert_eq!(eval.eval_pawns(Color::White, &b), Weight::new(0, 0));
        assert_eq!(eval.eval_pawns(Color::Black, &b), Weight::new(10, 10));

        // 1xw (-1) 3xb doubled (+3), 1xb (+1) tripled pawns  2xw 1xb isolated
        let b = Board::parse_fen("8/pppp3p/ppp5/p7/8/2P5/PPP1P1P1/8 b - - 0 1")
            .unwrap()
            .as_board();

        eval.pawn_doubled = Weight::new(-1, -1);
        eval.pawn_isolated = Weight::zero();
        assert_eq!(eval.w_eval_mobility(&b), Weight::new(3, 3));

        eval.pawn_doubled = Weight::zero();
        eval.pawn_isolated = Weight::new(-1, -1);
        assert_eq!(eval.w_eval_mobility(&b), Weight::new(-1, -1));
    }

    #[test]
    fn test_score_safety() {
        let mut eval = SimpleScorer::new();
        let b = Board::parse_fen("8/8/8/8/8/8/PPP5/K7 w - - 0 1")
            .unwrap()
            .as_board();
        eval.pawn_shield = Weight::zero();
        let e1 = eval.w_eval_safety(&b);
        eval.pawn_shield = Weight::new(50, 0);
        let e2 = eval.w_eval_safety(&b);

        assert_eq!((e2 - e1).s(), 100); // 2 pawns in front of king
    }
}
