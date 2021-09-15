use crate::board::Board;
use crate::config::{Component, Config};
use crate::eval::material_balance::MaterialBalance;
use crate::eval::pst::Pst;
use crate::eval::model::ModelScore;
use crate::eval::model::Scorer;
use crate::eval::score::Score;
use crate::eval::switches::Switches;
use crate::eval::weight::Weight;
use crate::globals::counts;
use crate::mv::Move;
use crate::phaser::Phaser;
use crate::search::node::Node;
use crate::stat::{ArrayStat, Stat};
use crate::types::{Color, Piece};

use std::fmt;

use super::model::{ExplainScorer, Model};

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
    pub mb: MaterialBalance,
    pub pst: Pst,
    pub phaser: Phaser,
    pub material: bool,
    pub position: bool,
    pub mobility: bool,
    pub pawn: bool,
    pub safety: bool,
    pub contempt: bool,
    pub tempo: bool,
    pub mobility_phase_disable: u8,
    pub quantum: i32,

    pub min_depth_mob: u8,
    pub undefended_sq: Weight,
    pub undefended_piece: Weight,
    pub trapped_piece: Weight,
    pub partially_trapped_piece: Weight,
    pub defended_non_pawn: Weight,
    pub xrayed: Weight,

    pub pawn_doubled: Weight,
    pub pawn_isolated: Weight,
    pub pawn_passed: Weight,
    pub pawn_passed_r6: Weight,
    pub pawn_passed_r5: Weight,
    pub passers_on_rim: Weight,
    pub blockaded: Weight,
    pub blockaded_passers: Weight,

    pub bishop_pair: Weight,
    pub fianchetto: Weight,
    pub bishop_outposts: Weight,
    pub bishop_color_pawns: Weight,
    pub knight_forks: Weight,
    pub knight_outposts: Weight,

    pub rook_pair: Weight,
    pub rook_open_file: Weight,
    pub queen_open_file: Weight,


    // pub pawn_shield: Weight,
    pub pawn_adjacent_shield: Weight,
    pub pawn_nearby_shield: Weight,
    pub open_files_near_king: Weight,
    pub attacks_near_king: Weight,
    pub tropism_d1: Weight,
    pub tropism_d2: Weight,
    pub tropism_d3: Weight,

    pub castling_rights: Weight,

    pub phasing: bool,
    pub contempt_penalty: Weight,
    pub tempo_bonus: Weight,
    // pub cache: TranspositionTable,
    // pub qcache: TranspositionTable,
    // pub depth: Ply,
}


impl Default for SimpleScorer {
    fn default() -> Self {
        let me = Self {
            mb: MaterialBalance::default(),
            pst: Pst::default(),
            phaser: Phaser::default(),
            mobility: true,
            position: true,
            material: true,
            pawn: true,
            safety: true,
            contempt: true,
            tempo: true,
            phasing: true,
            mobility_phase_disable: 101,
            quantum: 1,
            min_depth_mob: 1,
            contempt_penalty: Weight::new(-30, 0), // typically -ve

            undefended_sq: Weight::new(4, 3),
            undefended_piece: Weight::new(-3, 49),
            trapped_piece: Weight::new(-17, -22),
            partially_trapped_piece: Weight::new(-7, -15),
            defended_non_pawn: Weight::new(0, 0),
            xrayed: Weight::new(0, 0),

            bishop_pair: Weight::new(62, 58),
            fianchetto: Weight::new(55, 27),
            bishop_outposts: Weight::new(0, 0),
            bishop_color_pawns: Weight::new(55, 27),
            knight_forks: Weight::new(0, 0),
            knight_outposts: Weight::new(0, 0),
            rook_pair: Weight::new(-1, -1),
            rook_open_file: Weight::new(59, -4),

            queen_open_file: Weight::new(-19, 37),

            pawn_doubled: Weight::new(19, -35),
            pawn_isolated: Weight::new(-35, -5),
            pawn_passed: Weight::new(15, 28),
            pawn_passed_r5: Weight::new(1, 50),
            pawn_passed_r6: Weight::new(8, 94),
            passers_on_rim: Weight::new(-10, -10),
            blockaded: Weight::new(-10, -10),
            blockaded_passers: Weight::new(-10, -10),

            tempo_bonus: Weight::new(40, 50),
            pawn_adjacent_shield: Weight::new(44, -15),
            pawn_nearby_shield: Weight::new(42, -14),
            open_files_near_king: Weight::new(-6, -1),
            attacks_near_king: Weight::new(-8, -2),
            tropism_d1: Weight::new(-40, 29),
            tropism_d2: Weight::new(-28, 11),
            tropism_d3: Weight::new(-5, 2),

            castling_rights: Weight::new(0, 0),
        };
        me
    }
}

impl Component for SimpleScorer {
    fn settings(&self, c: &mut Config) {
        self.mb.settings(c);
        self.pst.settings(c);
        self.phaser.settings(c);
        c.set("eval.safety", &format!("type check default {}", self.safety));
        c.set("eval.mobility", &format!("type check default {}", self.mobility));
        c.set("eval.pawn", &format!("type check default {}", self.pawn));
        c.set("eval.position", &format!("type check default {}", self.position));
        c.set("eval.material", &format!("type check default {}", self.material));
        c.set("eval.contempt", &format!("type check default {}", self.contempt));
        c.set("eval.tempo", &format!("type check default {}", self.tempo));
        c.set("eval.phasing", &format!("type check default {}", self.phasing));
        c.set(
            "eval.mobility.min_depth",
            &format!("type spin min 0 max 101 default {}", self.min_depth_mob),
        );
        c.set(
            "eval.quantum",
            &format!("type spin min 1 max 100 default {}", self.quantum),
        );
        c.set(
            "eval.mobility.phase.disable",
            &format!("type spin min 0 max 101 default {}", self.mobility_phase_disable),
        );
        c.set_weight("eval.pawn.doubled", &self.pawn_doubled);
        c.set_weight("eval.mobility.undef.sq", &self.undefended_sq);
        c.set_weight("eval.mobility.undef.piece", &self.undefended_piece);
        c.set_weight("eval.mobility.trapped.piece", &self.trapped_piece);
        c.set_weight(
            "eval.mobility.partially.trapped.piece",
            &self.partially_trapped_piece,
        );
        c.set_weight("eval.mobility.defended.np", &self.defended_non_pawn);
        c.set_weight("eval.mobility.xrayed", &self.defended_non_pawn);
        c.set_weight("eval.pawn.isolated", &self.pawn_isolated);
        c.set_weight("eval.pawn.passed", &self.pawn_passed);
        c.set_weight("eval.pawn.passed.r6", &self.pawn_passed_r6);
        c.set_weight("eval.pawn.passed.r5", &self.pawn_passed_r5);
        c.set_weight("eval.passers.on.rim", &self.passers_on_rim);
        c.set_weight("eval.blockaded", &self.blockaded);
        c.set_weight("eval.blockaded.passers", &self.blockaded_passers);

        c.set_weight("eval.bishop.pair", &self.bishop_pair);
        c.set_weight("eval.fianchetto", &self.fianchetto);
        c.set_weight("eval.bishop.outposts", &self.bishop_outposts);
        c.set_weight("eval.bishop.color.pawns", &self.bishop_color_pawns);
        c.set_weight("eval.knight.forks", &self.knight_forks);
        c.set_weight("eval.knight.outposts", &self.knight_outposts);

        c.set_weight("eval.rook.pair", &self.rook_pair);
        c.set_weight("eval.rook.open.file", &self.rook_open_file);

        c.set_weight("eval.queen.open.file", &self.queen_open_file);


        // c.set_weight("eval.pawn.shield", &self.pawn_shield);
        c.set_weight("eval.pawn.adjacent.shield", &self.pawn_adjacent_shield);
        c.set_weight("eval.pawn.nearby.shield", &self.pawn_nearby_shield);
        c.set_weight("eval.open.files.near.king", &self.open_files_near_king);
        c.set_weight("eval.tropism.d1", &self.tropism_d1);
        c.set_weight("eval.tropism.d2", &self.tropism_d2);
        c.set_weight("eval.tropism.d3", &self.tropism_d3);
        c.set_weight("eval.attacks.near.king", &self.attacks_near_king);

        c.set_weight("eval.castling.rights", &self.castling_rights);
        c.set_weight("eval.contempt.penalty", &self.contempt_penalty);
        c.set_weight("eval.tempo.bonus", &self.tempo_bonus);
    }

    fn configure(&mut self, c: &Config) {
        debug!("eval.configure");
        self.mb.configure(c);
        self.pst.configure(c);
        self.phaser.configure(c);
        self.mobility = c.bool("eval.mobility").unwrap_or(self.mobility);
        self.pawn = c.bool("eval.pawn").unwrap_or(self.pawn);
        self.safety = c.bool("eval.safety").unwrap_or(self.safety);
        self.position = c.bool("eval.position").unwrap_or(self.position);
        self.material = c.bool("eval.material").unwrap_or(self.material);
        self.phasing = c.bool("eval.phasing").unwrap_or(self.phasing);
        self.quantum = c
            .int("eval.quantum")
            .unwrap_or(self.quantum as i64) as i32;
        self.mobility_phase_disable = c
            .int("eval.mobility.phase.disable")
            .unwrap_or(self.mobility_phase_disable as i64) as u8;
        self.min_depth_mob = c
            .int("eval.mobility.min.depth")
            .unwrap_or(self.min_depth_mob as i64) as u8;

        self.undefended_sq = c.weight("eval.mobility.undef.sq", &self.undefended_sq);
        self.undefended_piece = c.weight("eval.mobility.undef.piece", &self.undefended_piece);
        self.trapped_piece = c.weight("eval.mobility.trapped.piece", &self.trapped_piece);
        self.partially_trapped_piece = c.weight(
            "eval.mobility.partially.trapped.piece",
            &self.partially_trapped_piece,
        );
        self.defended_non_pawn = c.weight(
            "eval.mobility.defended.np",
            &self.defended_non_pawn,
        );
        self.xrayed = c.weight(
            "eval.mobility.xrayed",
            &self.xrayed,
        );
        self.rook_open_file = c.weight("eval.rook.open.file", &self.rook_open_file);
        self.rook_pair = c.weight("eval.rook.pair", &self.rook_pair);

        self.queen_open_file = c.weight("eval.queen.open.file", &self.queen_open_file);

        self.fianchetto = c.weight("eval.fianchetto", &self.fianchetto);
        self.bishop_pair = c.weight("eval.bishop.pair", &self.bishop_pair);
        self.bishop_color_pawns = c.weight("eval.bishop.color.pawns", &self.bishop_color_pawns);
        self.knight_forks = c.weight("eval.knight.forks", &self.knight_forks);
        self.knight_outposts = c.weight("eval.knight.outposts", &self.knight_outposts);
        self.bishop_outposts = c.weight("eval.bishop.outposts", &self.bishop_outposts);

        self.pawn_doubled = c.weight("eval.pawn.doubled", &self.pawn_doubled);
        self.pawn_isolated = c.weight("eval.pawn.isolated", &self.pawn_isolated);
        self.pawn_passed = c.weight("eval.pawn.passed", &self.pawn_passed);
        self.pawn_passed_r6 = c.weight("eval.pawn.passed.r6", &self.pawn_passed_r6);
        self.pawn_passed_r5 = c.weight("eval.pawn.passed.r5", &self.pawn_passed_r5);
        self.passers_on_rim = c.weight("eval.passers.on.rim", &self.passers_on_rim);
        self.blockaded = c.weight("eval.blockaded", &self.blockaded);
        self.blockaded_passers = c.weight("eval.blockaded.passers", &self.blockaded_passers);

        self.pawn_adjacent_shield = c.weight("eval.pawn.adjacent.shield", &self.pawn_adjacent_shield);
        self.pawn_nearby_shield = c.weight("eval.pawn.nearby.shield", &self.pawn_nearby_shield);
        self.open_files_near_king = c.weight("eval.open.files.near.king", &self.open_files_near_king);
        self.tropism_d1 = c.weight("eval.tropism.d1", &self.tropism_d1);
        self.tropism_d2 = c.weight("eval.tropism.d2", &self.tropism_d2);
        self.tropism_d3 = c.weight("eval.tropism.d3", &self.tropism_d3);
        self.attacks_near_king = c.weight("eval.attacks.near.king", &self.attacks_near_king);

        self.castling_rights = c.weight("eval.castling.rights", &self.castling_rights);
        self.contempt_penalty = c.weight("eval.contempt.penalty", &self.contempt_penalty);
        self.tempo_bonus = c.weight("eval.tempo.bonus", &self.tempo_bonus);

    }

    fn new_game(&mut self) {
        self.mb.new_game();
        self.phaser.new_game();
    }

    fn new_position(&mut self) {
        self.mb.new_position();
        self.phaser.new_position();
    }
}

impl fmt::Display for SimpleScorer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "[material balance]\n{}", self.mb)?;
        writeln!(f, "[phaser]\n{}", self.phaser)?;
        writeln!(f, "material         : {}", self.material)?;
        writeln!(f, "position         : {}", self.position)?;
        writeln!(f, "mobility         : {}", self.mobility)?;
        writeln!(f, "pawn             : {}", self.pawn)?;
        writeln!(f, "safety           : {}", self.safety)?;
        writeln!(f, "contempt         : {}", self.contempt)?;
        writeln!(f, "tempo            : {}", self.tempo)?;
        writeln!(f, "phasing          : {}", self.phasing)?;
        writeln!(f, "mob.phase.disable: {}", self.mobility_phase_disable)?;
        writeln!(f, "mob.min.depth    : {}", self.min_depth_mob)?;
        writeln!(f, "undefended.piece : {}", self.undefended_piece)?;
        writeln!(f, "undefended.sq    : {}", self.undefended_sq)?;
        writeln!(f, "trapped.piece    : {}", self.trapped_piece)?;
        writeln!(f, "part.trap.piece  : {}", self.partially_trapped_piece)?;
        writeln!(f, "defended.non.pawn: {}", self.defended_non_pawn)?;
        writeln!(f, "castling.rights  : {}", self.castling_rights)?;
        // writeln!(f, "pawn.shield      : {}", self.pawn_shield)?;
        writeln!(f, "pawn.doubled     : {}", self.pawn_doubled)?;
        writeln!(f, "pawn.passed      : {}", self.pawn_passed)?;
        writeln!(f, "pawn.passed.r7   : {}", self.pawn_passed_r6)?;
        writeln!(f, "pawn.isolated    : {}", self.pawn_isolated)?;

        writeln!(f, "bishop pair      : {}", self.bishop_pair)?;
        writeln!(f, "rook pair        : {}", self.rook_pair)?;
        writeln!(f, "fianchetto       : {}", self.fianchetto)?;
        writeln!(f, "bishop outposts  : {}", self.bishop_outposts)?;
        writeln!(f, "knight outposts  : {}", self.knight_outposts)?;
        writeln!(f, "knight forks     : {}", self.knight_forks)?;

        writeln!(f, "rook.open.file   : {}", self.rook_open_file)?;

        writeln!(f, "queen.open.file  : {}", self.queen_open_file)?;
        writeln!(f, "pawn.nearby      : {}", self.pawn_nearby_shield)?;
        writeln!(f, "pawn.adjacent    : {}", self.pawn_adjacent_shield)?;
        writeln!(f, "tropism.d1       : {}", self.tropism_d1)?;
        writeln!(f, "tropism.d2       : {}", self.tropism_d2)?;
        writeln!(f, "tropism.d3       : {}", self.tropism_d3)?;
        writeln!(f, "contempt penalty : {}", self.contempt_penalty)?;
        writeln!(f, "tempo bonus      : {}", self.tempo_bonus)?;
        writeln!(f, "eval stats\n{}", EVAL_COUNTS)?;
        // writeln!(f, "cache\n{}", self.cache)?;
        // writeln!(f, "qcache\n{}", self.qcache)?;

        Ok(())
    }
}

// builder methods
impl SimpleScorer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_switches(&mut self, enabled: bool) {
        self.material = enabled;
        self.position = enabled;
        self.mobility = enabled;
        self.safety = enabled;
        self.pawn = enabled;
        self.contempt = enabled;
        self.tempo = enabled;
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
            self.tempo_bonus
        } else {
            -self.tempo_bonus
        }
    }

    pub fn w_eval_draw(&self, board: &Board, _node: &Node) -> Score {
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
        let contempt = self.contempt as i32
            * board
                .color_us()
                .chooser_wb(self.contempt_penalty, -self.contempt_penalty);
        return Score::from_f32(contempt.interpolate(board.phase(&self.phaser)));

        // FIXME! v33
        // let signum = 1 - (node.ply % 2) * 2; // ply=0 => 1  ply=1=> -1
        // let contempt = signum * self.contempt + board.signum();
        // return Score::from_cp(contempt);
    }

    pub fn w_evaluate(&self, board: &Board, node: &Node) -> Score {
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
        self.w_eval_some(board, Switches::ALL_SCORING)
    }

    // we dont care about stalemates or checkmates
    pub fn w_eval_qsearch(&self, board: &Board, _node: &Node) -> Score {
        counts::QEVAL_COUNT.increment();
        self.w_eval_some(board, Switches::ALL_SCORING | Switches::INSUFFICIENT_MATERIAL)
    }


    pub fn predict(&self, m: &Model, scorer: &mut impl Scorer) {
        if m.mat.is_insufficient() && m.switches.contains(Switches::INSUFFICIENT_MATERIAL) {
            if m.switches.contains(Switches::CONTEMPT) {
                let contempt = m.turn.chooser_wb(1, 0);
                scorer.contempt("insufficient mat", contempt, 1 - contempt, self.contempt_penalty);
            }
            return;
        }

        // material
        if self.material && m.switches.contains(Switches::MATERIAL) {
            if self.mb.enabled {
                self.mb.w_eval_material(&m.mat, scorer);
            } else {
                self.mb.w_eval_material_without_balance(&m.mat, scorer);
            }
        };

        let w = &m.white;
        let b = &m.black;

        if self.material && m.switches.contains(Switches::MATERIAL) {
            scorer.material(
                "bishop pair",
                w.has_bishop_pair as i32,
                b.has_bishop_pair as i32,
                self.bishop_pair,
            );
            scorer.material(
                "rook pair",
                w.has_rook_pair as i32,
                b.has_rook_pair as i32,
                self.rook_pair,
            );
        }

        // position
        if self.position && m.switches.contains(Switches::POSITION) {
            let board = &m.multiboard;
            // let mut sum = Weight::zero();
            for &p in &Piece::ALL_BAR_NONE {
                let w = (board.pieces(p) & board.white()).flip_vertical();
                let b = board.pieces(p) & board.black();

                let w = w.squares().map(|sq| self.pst.pst(p, sq)).sum::<Weight>();
                let b = b.squares().map(|sq| self.pst.pst(p, sq)).sum::<Weight>();

                let black = ["pst none", "pst p", "pst n", "pst b", "pst r", "pst q", "pst k"][p];
                let white = ["pst none", "pst P", "pst N", "pst B", "pst R", "pst Q", "pst K"][p];
                scorer.position(white, 1, 0, w);
                scorer.position(black, 0, 1, b);
                // sum = sum + w - b;
            }
            scorer.position("fianchetti", w.fianchetti, b.fianchetti, self.fianchetto);
            scorer.position("bishop color pawns", w.bishop_color_pawns, b.bishop_color_pawns, self.bishop_color_pawns);
            scorer.position("bishop outposts", w.bishop_outposts, b.bishop_outposts, self.bishop_outposts);
            scorer.position("knight forks", w.knight_forks, b.knight_forks, self.knight_forks);
            scorer.position("knight outposts", w.knight_outposts, b.knight_outposts, self.knight_outposts);

            // scorer.position("pst", 1, 0, w.psq.iter().map(|(p,sq)| self.pst(*p, *sq)).sum::<Weight>());
            // scorer.position("pst", 0, 1, b.psq.iter().map(|(p,sq)| self.pst(*p, *sq)).sum::<Weight>());
        }

        // pawn structure
        if self.pawn && m.switches.contains(Switches::PAWN) {
            scorer.pawn("doubled", w.doubled_pawns, b.doubled_pawns, self.pawn_doubled);
            scorer.pawn("isolated", w.isolated_pawns, b.isolated_pawns, self.pawn_isolated);
            scorer.pawn("passed", w.passed_pawns, b.passed_pawns, self.pawn_passed);
            scorer.pawn(
                "passed.r6",
                w.passed_pawns_on_r6,
                b.passed_pawns_on_r6,
                self.pawn_passed_r6,
            );
            scorer.pawn(
                "passed.r5",
                w.passed_pawns_on_r5,
                b.passed_pawns_on_r5,
                self.pawn_passed_r5,
            );
            scorer.pawn(
                "passers on rim",
                w.passers_on_rim,
                b.passers_on_rim,
                self.passers_on_rim,
            );
            scorer.pawn(
                "blockaded",
                w.blockaded,
                b.blockaded,
                self.blockaded,
            );
            scorer.pawn(
                "blockaded passers",
                w.blockaded_passers,
                b.blockaded_passers,
                self.blockaded_passers,
            );
        }

        //  bishop



        // king safety
        if self.safety && m.switches.contains(Switches::SAFETY) {
            // scorer.safety("nearby pawns", w.nearby_pawns, b.nearby_pawns, self.pawn_shield);
            scorer.safety(
                "adjacent shield",
                w.adjacent_shield,
                b.adjacent_shield,
                self.pawn_adjacent_shield,
            );
            scorer.safety(
                "nearby shield",
                w.nearby_shield,
                b.nearby_shield,
                self.pawn_nearby_shield,
            );
            scorer.safety(
                "open files near king",
                w.open_files_near_king,
                b.open_files_near_king,
                self.open_files_near_king,
            );
            scorer.safety(
                "tropism d1",
                w.king_tropism_d1,
                b.king_tropism_d1,
                self.tropism_d1,
            );
            scorer.safety(
                "tropism d2",
                w.king_tropism_d2,
                b.king_tropism_d2,
                self.tropism_d2,
            );
            scorer.safety(
                "tropism d3",
                w.king_tropism_d3,
                b.king_tropism_d3,
                self.tropism_d3,
            );
            scorer.safety(
                "attacks near king",
                // reversed!
                b.attacks_on_opponent_king_area,
                w.attacks_on_opponent_king_area,
                self.attacks_near_king,
            );
        }
        // w.castling_sides, b.castling_sides * self.;

        // mobility
        if scorer.phase() <= self.mobility_phase_disable as i32
            && self.mobility
            && m.switches.contains(Switches::MOBILITY)
        {
            scorer.mobility("move", w.move_squares, b.move_squares, self.undefended_sq);
            scorer.mobility(
                "undef piece",
                w.non_pawn_defended_moves,
                b.non_pawn_defended_moves,
                self.undefended_piece,
            );

            scorer.mobility(
                "trapped",
                w.fully_trapped_pieces,
                b.fully_trapped_pieces,
                self.trapped_piece,
            );
            scorer.mobility(
                "part trapped",
                w.partially_trapped_pieces,
                b.partially_trapped_pieces,
                self.partially_trapped_piece,
            );
            // scorer.mobility(
            //     "defended non pawn",
            //     w.defended_non_pawn,
            //     b.defended_non_pawn,
            //     self.defended_non_pawn,
            // );
            // scorer.mobility(
            //     "xrayed",
            //     w.xrayed,
            //     b.xrayed,
            //     self.xrayed,
            // );
            scorer.mobility(
                "rook open file",
                w.rooks_on_open_files,
                b.rooks_on_open_files,
                self.rook_open_file,
            );
            scorer.mobility(
                "queen open file",
                w.queens_on_open_files,
                b.queens_on_open_files,
                self.queen_open_file,
            );
        }
        if self.tempo && m.switches.contains(Switches::TEMPO) {
            scorer.tempo("tempo", w.has_tempo as i32, b.has_tempo as i32, self.tempo_bonus);
        }
        scorer.interpolate("interpolate");
    }

    pub fn w_eval_explain(&self, b: &Board) -> ExplainScorer {
        let model = Model::from_board(b, Switches::ALL_SCORING);
        let mut scorer = ExplainScorer::new(b.phase(&self.phaser));
        self.predict(&model, &mut scorer);
        scorer
    }

    pub fn w_eval_some(&self, b: &Board, switches: Switches) -> Score {
        let model = Model::from_board(b, switches);
        let mut scorer = ModelScore::new(b.phase(&self.phaser));
        self.predict(&model, &mut scorer);
        Score::from_cp(scorer.as_score().as_i16() as i32 / self.quantum * self.quantum)
    }


    // // updated on capture & promo
    // #[inline]
    // pub fn w_eval_material(&self, mat: &Material) -> Weight {
    //     self.mb.w_eval_material(mat)
    // }

    /// the value of the capture or promotion (or both for promo capture)
    #[inline]
    pub fn eval_move_material(&self, mv: &Move) -> i32 {
        self.mb.eval_move_material(mv)
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
    pub fn eval(&self, eval: &SimpleScorer, nd: &Node) -> Score {
        ALL.increment();
        self.signum() * eval.w_evaluate(self, nd)
    }

    #[inline]
    pub fn eval_some(&self, eval: &SimpleScorer, sw: Switches) -> Score {
        ALL.increment();
        self.signum() * eval.w_eval_some(self, sw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::BoardBuf;
    use crate::catalog::Catalog;
    use crate::test_env_log::test;

    #[test]
    fn test_score_material() {
        let board = Catalog::starting_board();
        let eval = &mut SimpleScorer::new();
        eval.tempo = false;
        info!("\n{}", eval.w_eval_explain(&board));
        assert_eq!(board.eval(eval, &Node::root(0)), Score::from_cp(0));

        let board_w = Catalog::white_starting_position();
        assert_eq!(board_w.phase(&eval.phaser), 50);
        eval.set_switches(false);
        eval.material = true;
        eval.contempt = true;
        info!("\n{}", eval.w_eval_explain(&board_w));
        let score = board_w.eval(eval, &Node::root(0));
        assert!(score > Score::from_cp(5048));
        assert!(score < Score::from_cp(8048));

        let board_b = Catalog::black_starting_position();
        info!("\n{}", eval.w_eval_explain(&board_b));
        assert_eq!(eval.w_eval_some(&board_b, Switches::ALL_SCORING), score.negate());
    }

    #[test]
    fn test_eval_configure() {
        let mut eval = SimpleScorer::new();
        eval.configure(&Config::new().set("eval.b.s", "700"));
        assert_eq!(
            Score::from_f32(eval.mb.material_weights[Piece::Bishop].s()),
            Score::from_cp(700)
        );

        let mut eval = SimpleScorer::new();
        eval.configure(&Config::new().set("eval.position", "false"));
        assert_eq!(eval.position, false);
    }

    #[test]
    fn test_score_position() {
        let mut eval = SimpleScorer::new();

        let bd = Board::parse_fen("8/P7/8/8/8/8/8/8 w - - 0 1").unwrap().as_board();
        eval.set_switches(false);
        eval.position = true;
        assert_eq!(bd.phase(&eval.phaser), 100);
        assert_eq!(bd.eval(&eval, &Node::root(0)), Score::from_f32(eval.pst.pawn_r7.e()), "{}", eval.w_eval_explain(&bd));

        let bd = Board::parse_fen("8/4p3/8/8/8/8/8/8 w - - 0 1")
            .unwrap()
            .as_board();
        assert_eq!(bd.phase(&eval.phaser), 100);

        assert_eq!(bd.eval(&eval, &Node::root(0)), Score::from_cp(0));

        let w = Catalog::white_starting_position();
        assert_eq!(w.phase(&eval.phaser), 50);
        let score = w.eval(&eval, &Node::root(0));
        assert!(score < Score::from_cp(-110), "{}", eval.w_eval_explain(&w));
        assert!(score > Score::from_cp(-120), "{}", eval.w_eval_explain(&w));

        let b = Catalog::black_starting_position();
        assert_eq!(
            w.eval(&eval, &Node::root(0)),
            eval.w_eval_some(&b, Switches::ALL_SCORING).negate()
        );

        // from blacks perspective to negate
        let bd = Board::parse_fen("8/8/8/8/8/8/p7/8 b - - 0 1").unwrap().as_board();
        assert_eq!(bd.eval(&eval, &Node::root(0)), Score::from_f32(eval.pst.pawn_r7.e()));
    }

    #[test]
    fn test_score_mobility() {
        let mut eval = SimpleScorer::new();
        eval.pawn_doubled = Weight::from_i32(-1);
        eval.pawn_isolated = Weight::zero();
        eval.mobility_phase_disable = 101;
        let b = Catalog::starting_board();
        eval.set_switches(false);
        eval.mobility = true;
        assert_eq!(eval.w_eval_some(&b, Switches::ALL_SCORING), Score::from_cp(0));
    }

    #[test]
    fn test_score_pawn() {
        let mut eval = SimpleScorer::new();
        eval.pawn_doubled = Weight::from_i32(-1);
        eval.pawn_isolated = Weight::zero();
        eval.mobility_phase_disable = 101;
        let _b = Catalog::starting_board();
        eval.set_switches(false);
        eval.pawn = true;
        // 1xw 4xb doubled pawns, 1xw 2xb isolated pawns, 1xb passed pawn
        let b = Board::parse_fen("8/pppp1p1p/pppp4/8/8/2P5/PPP4P/8 b - - 0 1")
            .unwrap()
            .as_board();
        eval.pawn_doubled = Weight::from_i32(-1);
        eval.pawn_isolated = Weight::zero();
        eval.pawn_passed = Weight::zero();
        assert_eq!(
            eval.w_eval_some(&b, Switches::ALL_SCORING),
            Score::from_cp(-1 - -4)
        );

        eval.pawn_doubled = Weight::zero();
        eval.pawn_isolated = Weight::from_i32(-1);
        eval.pawn_passed = Weight::zero();
        assert_eq!(
            eval.w_eval_some(&b, Switches::ALL_SCORING),
            Score::from_cp(-1 - -2)
        );

        eval.pawn_doubled = Weight::zero();
        eval.pawn_isolated = Weight::zero();
        eval.pawn_passed = Weight::from_i32(10);
        assert_eq!(
            eval.w_eval_some(&b, Switches::ALL_SCORING),
            Score::from_cp(0 - 10)
        );

        // 1xw (-1) 3xb doubled (+3), 1xb (+1) tripled pawns  2xw 1xb isolated
        let b = Board::parse_fen("8/pppp3p/ppp5/p7/8/2P5/PPP1P1P1/8 b - - 0 1")
            .unwrap()
            .as_board();

        eval.pawn_doubled = Weight::from_i32(-1);
        eval.pawn_isolated = Weight::zero();
        eval.set_switches(false);
        eval.pawn = true;
        assert_eq!(
            eval.w_eval_some(&b, Switches::ALL_SCORING),
            Score::from_cp(3),
            "{}",
            eval.w_eval_explain(&b).to_string()
        );

        eval.pawn_doubled = Weight::zero();
        eval.pawn_isolated = Weight::from_i32(-1);
        assert_eq!(
            eval.w_eval_some(&b, Switches::ALL_SCORING),
            Score::from_cp(-1),
            "{}",
            eval.w_eval_explain(&b).to_string()
        );
    }

    #[test]
    fn test_score_safety() {
        let mut eval = SimpleScorer::new();
        let b = Board::parse_fen("r7/8/8/8/8/1P6/PP6/K7 w - - 0 1")
            .unwrap()
            .as_board();

        eval.set_switches(false);
        eval.safety = true;
        eval.pawn_adjacent_shield = Weight::zero();
        eval.pawn_nearby_shield = Weight::zero();
        eval.attacks_near_king = Weight::zero();
        info!("{}\n{}", b, eval.w_eval_explain(&b));

        let e1 = eval.w_eval_some(&b, Switches::ALL_SCORING);
        eval.pawn_adjacent_shield = Weight::new(50, 50);
        let e2 = eval.w_eval_some(&b, Switches::ALL_SCORING);
        assert_eq!((e2 - e1), Score::from_cp(100), "{}", eval.w_eval_explain(&b)); // 2 pawns adjacent

        eval.pawn_nearby_shield = Weight::new(150, 150);
        let e3 = eval.w_eval_some(&b, Switches::ALL_SCORING);
        assert_eq!(e3 - e2, Score::from_cp(150), "{}", eval.w_eval_explain(&b)); // 2 pawns adjacent, 1 nearby

        eval.attacks_near_king = Weight::new(-75, -75);
        let att = eval.w_eval_some(&b, Switches::ALL_SCORING);
        assert_eq!((att - e3), Score::from_cp(-75)); // 1 attack on nearby pawn
        info!("{}\n{}", b, eval.w_eval_explain(&b));
    }

    #[test]
    fn test_eval_bug1() {
        let pos = &Catalog::bratko_kopec()[0];
        let b = pos.board();
        let mut eval = SimpleScorer::default();
        eval.mb.enabled = false;
        let explain = eval.w_eval_explain(b);
        println!("{}", explain);
    }

    #[test]
    #[ignore]
    fn test_eval_various() {
        let mut eval = SimpleScorer::default();
        eval.mb.enabled = false;
        let node = Node::root(0);
        for pos in Catalog::win_at_chess() {
            let b = pos.board();
            let _score = b.signum() * b.eval(&eval, &node);
            let explain = eval.w_eval_explain(b);
            info!("\n{}\n{}", pos, explain);
        }
    }
}
