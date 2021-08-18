use crate::config::{Component, Config};
use crate::eval::weight::Weight;
use crate::material::Material;
use crate::mv::Move;
use crate::types::{Color, Piece, ScoreWdl};
use static_init::dynamic;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicI16, Ordering};
use crate::{trace, info, logger::LogInit};
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;


#[derive(Clone)]
pub struct MaterialBalance {
    pub enabled: bool,
    pub filename: String,
    pub consistency: bool,
    pub draws_only: bool,
    pub min_games: i32,
    pub max_pawns: i32,
    pub trade_factor: i32,
    pub material_weights: [Weight; Piece::len()],
    pub bishop_pair: Weight,
}


impl Default for MaterialBalance {
    fn default() -> Self {
        let mb = Self {
            enabled: true,
            filename: String::new(),
            consistency: true,
            draws_only: true,
            min_games: 50,
            max_pawns: 4,
            trade_factor: 2,
            material_weights: [
                Weight::default(),
                Weight::new(100, 100),
                Weight::new(350, 350), // knights
                Weight::new(350, 350),
                Weight::new(600, 600),
                Weight::new(1100, 1100),
                Weight::new(0, 0), // king
            ],
            bishop_pair: Weight::new(40, 85),
        };
        mb
    }
}



impl Component for MaterialBalance {
    fn settings(&self, c: &mut Config) {
        c.set("mb.enabled", &format!("type check default {}", self.enabled));
        c.set("mb.filename", &format!("type string default {}", self.filename));
        c.set("mb.consistency", &format!("type check default {}", self.consistency));
        c.set("mb.draws.only", &format!("type check default {}", self.draws_only));
        c.set(
            "mb.min.games",
            &format!("type spin min 0 max 2000 default {}", self.min_games),
        );
        c.set(
            "mb.max.pawns",
            &format!("type spin min 0 max 8 default {}", self.max_pawns),
        );
        c.set(
            "mb.trade.factor",
            &format!("type spin min -500 max 2000 default {}", self.trade_factor),
        );
        c.set_weight("eval.bishop.pair", &self.bishop_pair);
        for &p in &Piece::ALL_BAR_KING {
            let mut name = "eval.".to_string();
            name.push(p.to_char(Some(Color::Black)));
            c.set_weight(&name, &self.material_weights[p]);
        }
    }

    fn configure(&mut self, c: &Config) {
        debug!("mb.configure");
        self.enabled = c.bool("mb.enabled").unwrap_or(self.enabled);
        self.filename = c.string("mb.filename").unwrap_or(self.filename.clone());
        self.consistency = c.bool("mb.consistency").unwrap_or(self.consistency);
        self.draws_only = c.bool("mb.draws.only").unwrap_or(self.draws_only);
        self.min_games = c.int("mb.min.games").unwrap_or(self.min_games as i64) as i32;
        self.max_pawns = c.int("mb.max.pawns").unwrap_or(self.max_pawns as i64) as i32;
        self.trade_factor = c.int("mb.trade.factor").unwrap_or(self.trade_factor as i64) as i32;

        self.bishop_pair = c.weight("eval.bishop.pair", &self.bishop_pair);
        for &p in &Piece::ALL_BAR_KING {
            let mut name = "eval.".to_string();
            name.push(p.to_char(Some(Color::Black)));
            self.material_weights[p] = c.weight(&name, &self.material_weights[p]);
        }

        // we relaculate the derived scores as the config may have amended 
        // some of the inputs to this calculation
        if self.enabled {
            self.init();  // likely re-init but happens outside of perf criticality
            DERIVED_SCORES_CALCULATED.store(true, Ordering::Relaxed);
        }
    }

    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {}
}



impl fmt::Display for MaterialBalance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "filename         : {}", self.filename)?;
        writeln!(f, "consistency      : {}", self.consistency)?;
        writeln!(f, "draws only       : {}", self.draws_only)?;
        writeln!(f, "min games        : {}", self.min_games)?;
        writeln!(f, "max pawns        : {}", self.max_pawns)?;
        writeln!(f, "trade factor     : {}", self.trade_factor)?;
        writeln!(f, "bishop pair      : {}", self.bishop_pair)?;
        writeln!(f, "table size       : {}", Material::HASH_VALUES)?;
        Ok(())
    }
}

impl fmt::Debug for MaterialBalance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("MaterialBalance")
            .field("enabled", &self.enabled)
            .field("filename", &self.filename)
            .field("consistency", &self.consistency)
            .field("draws_only", &self.draws_only)
            .field("min_games", &self.min_games)
            .field("max_pawns", &self.max_pawns)
            .field("trade_factor", &self.trade_factor)
            .field("bishop_pair", &self.bishop_pair)
            .field("size", &Material::HASH_VALUES)
            .finish()
    }
}
impl MaterialBalance {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn w_eval_material(&self, mat: &Material) -> Weight {
        if self.enabled {
            let weight = self.balance_lookup(mat);
            if weight != Weight::zero() {
                return weight;
            }
        }
        self.w_eval_simple(mat)
    }

    fn w_eval_simple(&self, mat: &Material) -> Weight {
        let mut weight = Piece::ALL_BAR_KING
            .iter()
            .map(|&p| (mat.counts(Color::White, p) - mat.counts(Color::Black, p)) * self.material_weights[p])
            .sum();

        // let mut weight = Weight::new(score, score);
        if mat.counts(Color::White, Piece::Bishop) >= 2 {
            weight = weight + self.bishop_pair
        }
        if mat.counts(Color::Black, Piece::Bishop) >= 2 {
            weight = weight - self.bishop_pair
        }
        weight
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



    #[inline]
    pub fn balance_lookup(&self, mat: &Material) -> Weight {
        self.ensure_init();
        let score = DERIVED_SCORES[mat.hash()].load(Ordering::Relaxed);
        if score != 0 {
            return Weight::new(score as i32, score as i32);
        }
        Weight::zero()
    }


    pub fn log_material_balance(&self) {
        self.ensure_init();
        info!("Examining material balances...");
        info!("{:>6} {:>32} {}", "Centi", "Material", "Hash");
        let mut count = 0;
        for hash in 0..Material::HASH_VALUES {
            let cp = DERIVED_SCORES[hash].load(Ordering::Relaxed);
            if cp > 0 {
                count += 1;
                let mut mat = Material::maybe_from_hash(hash);
                *mat.counts_mut(Color::White, Piece::King) = 0;
                *mat.counts_mut(Color::Black, Piece::King) = 0;
                info!("{:>6} {:>32} {}", cp, mat.to_string(), hash);
            }
        }
        info!("Total material balance entries {:>6}", count);
    }


    pub fn init_from_file<P>(&self, filename: P) -> Result<(), String>
    where P: AsRef<Path>, P: Clone {

        // zero existing ones
        for n in 0..Material::HASH_VALUES {
            DERIVED_SCORES[n].store(0,  Ordering::Relaxed);
        }

        let file = File::open(filename.clone()).map_err(|err| err.to_string())?;
        let lines = io::BufReader::new(file).lines();

        let mut count = 0;
        for (n, line) in lines.enumerate() {
            if n > 0 && n % 1000 == 0 {
                info!("Read {} lines from {:?}", n, filename.as_ref().display());
            }
            let s = line.map_err(|err| err.to_string())?;
            let s = s.trim();
            if s.is_empty() || s.starts_with("#") {
                continue;
            }

            count += 1;

            // vec.push(Self::parse_epd(&s).map_err(|err| format!("{} in epd {}", err, s))?);
            let vec: Vec<_> = s.trim().splitn(2, ",").collect();
            if vec.len() != 2 {
                return Err(format!("Failed parsing line {} in file {}: '{}'", n, filename.as_ref().display(), s))
            }
            let mat = Material::from_piece_str(vec[0].trim())?;
            let cp = vec[1].trim().parse::<i32>().map_err(|err| format!("{} - unable to parse number '{}'", err, vec[1]))?;

            DERIVED_SCORES[mat.hash()].store(cp as i16,  Ordering::Relaxed);
            DERIVED_SCORES[mat.flip().hash()].store(-cp as i16, Ordering::Relaxed);
            trace!("{:<20} = {:>5}", format!("mb[{}]",mat), cp);
        }
        info!("Read {} items from {:?}", count, filename.as_ref().display());
        Ok(())
    }





    #[inline]
    pub fn ensure_init(&self) {
        if !DERIVED_SCORES_CALCULATED.load(Ordering::Relaxed) {
            // thread safe as we don't care if we run multiple times, or have a half-written table during the process
            self.init();
            DERIVED_SCORES_CALCULATED.store(true, Ordering::Relaxed);
        }
    }

    // inerior mutability
    fn init(&self) {
        if self.filename.is_empty() {
            self.init_from_raw_stats(); 
        } else {
            self.init_from_file(self.filename.clone()).unwrap();
        }
    }



    fn init_from_raw_stats(&self) {

        info!("creating derived material balance table from {} raw internal statistics", RAW_STATS.len() );
        for n in 0..Material::HASH_VALUES {
            DERIVED_SCORES[n].store(0,  Ordering::Relaxed);
        }
        let mut sorted_raw_stats = RAW_STATS.clone();
        sorted_raw_stats.sort_by_cached_key(|(mat, _score) | mat.to_string().len() as i32 * 20000 + mat.centipawns() ); 
        for (mat, wdl) in sorted_raw_stats.iter() {
            let pawns = mat.counts(Color::White, Piece::Pawn) + mat.counts(Color::Black, Piece::Pawn);
            if wdl.total() >= self.min_games && pawns <= self.max_pawns {
                let mut cp;
                if wdl.w + wdl.d <= 5 {
                    // certain losing position
                    cp = -8000;
                    // let trade_down_penalty = self.trade_factor /100 * mat.phase(); // bigger if less material
                    // cp -= trade_down_penalty as i16;
                } else if wdl.l + wdl.d <= 5 {
                    // certain winning position
                    cp = 8000;
                    // let trade_down_bonus = self.trade_factor /100 * mat.phase(); // bigger if less material
                    // cp += trade_down_bonus as i16;
                } else {
                    let elo = wdl.elo();
                    // straight 1-1 approximation
                    cp = elo as i32;
                    if cp == 0 {
                        cp = 1;
                    }
                }
                // adj means that drawish positions still incentivise a material gain
                let adj = self.w_eval_simple(mat).e();
                cp = cp.clamp(-5000 + adj, 5000 + adj);

                if !self.draws_only || (-20 < cp && cp < 20) {  
                    DERIVED_SCORES[mat.hash()].store(cp as i16,  Ordering::Relaxed);
                    DERIVED_SCORES[mat.flip().hash()].store(-cp as i16, Ordering::Relaxed);
                    trace!("{:<20} = {:>5}       wdl: {:>5} {:>5} {:>5}", format!("mb[{}]",mat), cp, wdl.w, wdl.d, wdl.l);
                }
            }
        }

        if self.consistency {
            for pass in 1..10 {
                self.init_ensure_consistency(pass);
            }
        }
    }

    fn init_ensure_consistency(&self, pass: i32) {
        info!("\nMaterial balance consistency: pass {}\n\n", pass);
        let mut adjustments = 0;
        for (hash,_atom) in DERIVED_SCORES.iter().enumerate() {
            let mat = Material::maybe_from_hash(hash);
            let cp = DERIVED_SCORES[mat.hash()].load(Ordering::Relaxed) as i32;
            if cp > 0 {
                adjustments += self.ensure_single_entry_consistent(&mat, cp);
            }
        }
        info!("Material balance consistency: pass {} adjusted {} items\n", pass, adjustments);
    }

    fn ensure_single_entry_consistent(&self, mat: &Material, cp: i32) -> i32 {
        let material_diffs = [
            Material::from_piece_str("P").unwrap(),
            Material::from_piece_str("N").unwrap(),
            Material::from_piece_str("B").unwrap(),
            Material::from_piece_str("R").unwrap(),
            Material::from_piece_str("Q").unwrap(),
            Material::from_piece_str("p").unwrap(),
            Material::from_piece_str("n").unwrap(),
            Material::from_piece_str("b").unwrap(),
            Material::from_piece_str("r").unwrap(),
            Material::from_piece_str("q").unwrap(),
            -Material::from_piece_str("P").unwrap(),
            -Material::from_piece_str("N").unwrap(),
            -Material::from_piece_str("B").unwrap(),
            -Material::from_piece_str("R").unwrap(),
            -Material::from_piece_str("Q").unwrap(),
            -Material::from_piece_str("p").unwrap(),
            -Material::from_piece_str("n").unwrap(),
            -Material::from_piece_str("b").unwrap(),
            -Material::from_piece_str("r").unwrap(),
            -Material::from_piece_str("q").unwrap(),
            ];
        // let no_material = Material::default();
        let mut adjustments = 0;

        for diff in &material_diffs {

            // first examine loss of material;
            let other = mat - diff;
            if other.hash() == 0 {
                continue;
            }
            let mut other_cp = DERIVED_SCORES[other.hash()].load(Ordering::Relaxed) as i32;
            if other_cp == 0 {
                let weight: Weight = Piece::ALL_BAR_KING
                .iter()
                .map(|&p| (other.counts(Color::White, p) - other.counts(Color::Black, p)) * self.material_weights[p])
                .sum();        
                other_cp = std::cmp::max(weight.s(), weight.e());
            }

            // losing material - ensure lesser entries consistent
            let mut new_cp = cp;
            if cp < other_cp && mat.white() > other.white() {
                new_cp = other_cp + diff.centipawns() / 2;  
                trace!("Loss (white) {:>32}={:>5} < {:>30}={:>5} ----> [{}] = {} ({})", mat.to_string(), cp, other, other_cp, other, new_cp, new_cp - cp );
            }
            if cp > other_cp && mat.black() > other.black() {
                new_cp = other_cp + diff.centipawns() / 2;  // centipawns already negative for black
                trace!("Loss (black) {:>32}={:>5} > {:>30}={:>5} ----> [{}] = {} ({})", mat.to_string(), cp, other, other_cp, other, new_cp, new_cp - cp );
            }

            if new_cp != cp {
                adjustments += 1;
                // trace!("[{:>32}]={:>5}", mat, cp);
                DERIVED_SCORES[mat.hash()].store(new_cp as i16,  Ordering::Relaxed);
                DERIVED_SCORES[mat.flip().hash()].store(-new_cp as i16, Ordering::Relaxed);
            }

            // gaining material - ensure greater entries consistent
            let mut new_cp = other_cp;
            if cp > other_cp && mat.white() < other.white() {
                new_cp = other_cp - diff.centipawns() / 2;  
                trace!("Gain (white) {:>32}={:>5} > {:>30}={:>5} ----> [{}] = {} ({})", mat.to_string(), cp, other, other_cp, other, new_cp, new_cp - other_cp );
            }
            if cp < other_cp && mat.black() < other.black() {
                new_cp = other_cp - diff.centipawns() / 2;  // centipawns already negative for black
                trace!("Gain (black) {:>32}={:>5} < {:>30}={:>5} ----> [{}] = {} ({})", mat.to_string(), cp, other, other_cp, other, new_cp, new_cp - other_cp );
            }
            // amend the higher one
            if new_cp != other_cp {
                adjustments += 1;
                // trace!("[{:>32}]={:>5}", mat, cp);
                DERIVED_SCORES[other.hash()].store(other_cp as i16,  Ordering::Relaxed);
                DERIVED_SCORES[other.flip().hash()].store(-other_cp as i16, Ordering::Relaxed);
            }
        }
        adjustments
    }
}







// mutable derived scores
type DerivedScoresVec = Vec<AtomicI16>;

static DERIVED_SCORES_CALCULATED: AtomicBool = AtomicBool::new(false);

#[dynamic(lazy)]
static DERIVED_SCORES: DerivedScoresVec = {
    let mut m = DerivedScoresVec::new();
    m.resize_with(Material::HASH_VALUES, || AtomicI16::new(0));
    m
};



// immutable raw stats
type RawStatsVec = Vec<(Material, ScoreWdl)>;

fn data(m: &mut RawStatsVec, s: &str, w: i32, d: i32, l: i32) {
    m.push((Material::from_piece_str(s).unwrap(), ScoreWdl::new(w, d, l)));
}


#[cfg(test)]
mod tests {
    use super::*;
    // use crate::{debug, info, logger::LogInit};
    use crate::board::Board;
    use crate::board::boardbuf::BoardBuf;
    use crate::eval::score::Score;
    use crate::eval::eval::SimpleScorer;

 
    #[test]
    fn test_balance() {
        let mut mb = MaterialBalance::new();
        mb.configure(Config::from_env());
        mb.log_material_balance();
    }

    #[test]
    fn test_score_balance() {

        let eval = &mut SimpleScorer::new();
        eval.tempo = Weight::zero();
        let board = Board::parse_fen("K7/P7/8/8/8/8/8/k7 w - - 0 1").unwrap();
        assert_eq!(board.eval_material(eval), Score::from_cp(282));
        
        let board = Board::parse_fen("k7/p7/8/8/8/8/8/K7 w - - 0 1").unwrap();
        assert_eq!(board.eval_material(eval), -Score::from_cp(282));

        let board = Board::parse_fen("K7/PPPPPPPP/8/8/8/8/8/k7 w - - 0 1").unwrap();
        assert_eq!(board.eval_material(eval), Score::from_cp(800));

        let board = Board::parse_fen("K7/PPPPP3/8/8/8/8/8/k7 w - - 0 1").unwrap();
        assert_eq!(board.eval_material(eval), Score::from_cp(500));

        // losing a pawn from 5P to 4P increases score from 500 to 5400
        let board = Board::parse_fen("K7/PPPP4/8/8/8/8/8/k7 w - - 0 1").unwrap();
        assert_eq!(board.eval_material(eval), Score::from_cp(5400));

        let board = Board::parse_fen("K7/PPP5/8/8/8/8/8/k7 w - - 0 1").unwrap();
        assert_eq!(board.eval_material(eval), Score::from_cp(5300));

        let board = Board::parse_fen("K7/PP6/8/8/8/8/8/k7 w - - 0 1").unwrap();
        assert_eq!(board.eval_material(eval), Score::from_cp(735));

        // black exchanges knight for a bishop
        let board = Board::parse_fen("8/2p3p1/3r1pk1/R2Prnp1/P5P1/4BK2/R4P1P/8 b - - 0 50").unwrap();
        assert_eq!(board.eval_material(eval), Score::from_cp(-100));
        let board = Board::parse_fen("8/2p3p1/3r1pk1/R2Pr1p1/P5P1/4PK2/R6P/8 b - - 0 51").unwrap();
        assert_eq!(board.eval_material(eval), Score::from_cp(-100));
    }
}






#[dynamic]
static RAW_STATS: RawStatsVec = {
    let mut vec = RawStatsVec::new();
    let m = &mut vec;

    // generated code
    // 3,375 code lines from 1,427,999 sample positions

    data(m, "BBNNPPPPPPPPQRRbbnnpppppppqrr", 4340, 2162, 1083);
    data(m, "BBNNPPPPPPPPQRRbbnppppppppqrr", 1987, 30, 15);
    data(m, "BBNNPPPPPPPPQRRbnnppppppppqrr", 1847, 21, 11);
    data(m, "BBNNPPPPPPPPQRRbbnnppppppppqr", 94, 2, 1);
    data(m, "BBNNPPPPPPPPQRRbbnnpppppppprr", 540, 0, 0);
    data(m, "BBNNPPPPPPPPRRbbnnpppppppqrr", 0, 0, 276);
    data(m, "BBNNPPPPPPPPRRbnnppppppppqrr", 1, 1, 229);
    data(m, "BBNNPPPPPPPPRRbbnppppppppqrr", 0, 0, 307);
    data(m, "BBNPPPPPPPPQRRbbnnpppppppqrr", 67, 138, 2855);
    data(m, "BBNNPPPPPPPPQRbbnppppppppqrr", 9, 13, 39);
    data(m, "BNNPPPPPPPPQRRbbnnpppppppqrr", 20, 58, 1266);
    data(m, "BBNPPPPPPPPQRRbnnppppppppqrr", 1603, 2152, 1380);
    data(m, "BNNPPPPPPPPQRRbbnppppppppqrr", 1054, 1999, 1815);
    data(m, "BBNNPPPPPPPQRRbbnppppppppqrr", 2273, 141, 63);
    data(m, "BBNNPPPPPPPQRRbnnppppppppqrr", 1662, 58, 37);
    data(m, "BBNNPPPPPPPPQRRbbnnppppppqrr", 942, 166, 104);
    data(m, "BBNNPPPPPPPPQRRbnnpppppppqrr", 420, 9, 3);
    data(m, "BBNNPPPPPPPPQRRbbnpppppppqrr", 608, 9, 4);
    data(m, "BNNPPPPPPPPQRRbbnnpppppppprr", 217, 0, 0);
    data(m, "BBNPPPPPPPPQRRbbnnpppppppprr", 329, 0, 1);
    data(m, "BBNNPPPPPPPPQRRbnppppppppqrr", 158, 0, 0);
    data(m, "BBNNPPPPPPPPQRRbbnnpppppppqr", 121, 1, 10);
    data(m, "BBNNPPPPPPPQRRbbnnpppppppprr", 315, 0, 0);
    data(m, "BBNNPPPPPPPPQRRbbnnppppppprr", 123, 0, 0);
    data(m, "BBNNPPPPPPPPRRbbnnppppppqrr", 0, 0, 195);
    data(m, "BBNNPPPPPPPPRRbbnpppppppqrr", 0, 2, 203);
    data(m, "BBNNPPPPPPPPRRbnnpppppppqrr", 2, 2, 153);
    data(m, "BBNPPPPPPPPQRRbbnnppppppqrr", 95, 175, 981);
    data(m, "BNNPPPPPPPPQRRbbnnppppppqrr", 36, 53, 406);
    data(m, "BBNNPPPPPPPQRRbbnnppppppqrr", 6702, 3390, 1928);
    data(m, "BNNPPPPPPPPQRRbbnpppppppqrr", 1257, 850, 447);
    data(m, "BBNPPPPPPPPQRRbnnpppppppqrr", 1415, 562, 289);
    data(m, "BBNPPPPPPPPQRRbbnpppppppqrr", 2824, 1458, 616);
    data(m, "BNNPPPPPPPPQRRbnnpppppppqrr", 796, 387, 184);
    data(m, "BBNNPPPPPPPPRRbbnnppppppprr", 131, 81, 36);
    data(m, "BBNNPPPPPPPQRRbbnpppppppqrr", 4140, 106, 53);
    data(m, "BBNPPPPPPPPQRRbnppppppppqrr", 1554, 16, 8);
    data(m, "BBNNPPPPPPPQRRbnnpppppppqrr", 3013, 41, 25);
    data(m, "BNNPPPPPPPPQRRbnppppppppqrr", 830, 10, 4);
    data(m, "BNNPPPPPPPPQRRbbppppppppqrr", 184, 10, 3);
    data(m, "BBNNPPPPPPPPRRbbnpppppppprr", 50, 1, 0);
    data(m, "BNNPPPPPPPPQRRbbnnpppppppqr", 63, 15, 17);
    data(m, "BBNPPPPPPPPQRRnnppppppppqrr", 336, 6, 2);
    data(m, "BBNPPPPPPPPQRRbbppppppppqrr", 424, 7, 3);
    data(m, "BBNPPPPPPPPQRRbbnnpppppppqr", 162, 2, 2);
    data(m, "BNNPPPPPPPPQRRnnppppppppqrr", 120, 2, 1);
    data(m, "BBNNPPPPPPPPQRRbbnnpppppqrr", 69, 11, 4);
    data(m, "BBNPPPPPPPPQRRbnnppppppppqr", 135, 0, 0);
    data(m, "BBNNPPPPPPPPQRRbbnppppppqrr", 113, 0, 0);
    data(m, "BNNPPPPPPPPQRRbnnppppppppqr", 92, 3, 1);
    data(m, "BBNNPPPPPPPQRRbbppppppppqrr", 74, 0, 0);
    data(m, "BBNNPPPPPPPQRRbnppppppppqrr", 176, 2, 0);
    data(m, "BBNNPPPPPPPQRRbbnnpppppppqr", 303, 8, 4);
    data(m, "BBNPPPPPPPPQRRbbnppppppppqr", 50, 1, 1);
    data(m, "BBNPPPPPPPPQRRbbnnppppppprr", 303, 0, 0);
    data(m, "BNNPPPPPPPPQRRbbnnppppppprr", 205, 1, 1);
    data(m, "BNNPPPPPPPPQRRbbnpppppppprr", 343, 0, 0);
    data(m, "BBNNPPPPPPPQRRbbnnppppppprr", 1282, 0, 0);
    data(m, "BBNPPPPPPPPQRRbbnpppppppprr", 293, 0, 0);
    data(m, "BNNPPPPPPPPQRRbnnpppppppprr", 145, 0, 0);
    data(m, "BBNPPPPPPPPQRRbnnpppppppprr", 143, 0, 0);
    data(m, "BBNNPPPPPPPRRbbnnppppppqrr", 0, 0, 857);
    data(m, "BNNPPPPPPPPRRbbnpppppppqrr", 0, 0, 95);
    data(m, "BBNPPPPPPPPRRbbnpppppppqrr", 0, 0, 158);
    data(m, "BNNPPPPPPPPRRbnnpppppppqrr", 0, 0, 64);
    data(m, "BBNPPPPPPPPRRbnnpppppppqrr", 0, 0, 85);
    data(m, "BBNNPPPPPPPRRbnnpppppppqrr", 0, 1, 614);
    data(m, "BBNNPPPPPPPRRbbnpppppppqrr", 2, 3, 645);
    data(m, "BBNPPPPPPPPRRbnppppppppqrr", 1, 2, 229);
    data(m, "BNNPPPPPPPPRRbnppppppppqrr", 0, 0, 55);
    data(m, "BBNPPPPPPPPRRbbppppppppqrr", 0, 0, 60);
    data(m, "BBNPPPPPPPPRRnnppppppppqrr", 0, 1, 57);
    data(m, "BBNNPPPPPPPPRRbbnppppppqrr", 2, 1, 49);
    data(m, "BBNNPPPPPPPQRbbnnppppppqrr", 6, 5, 145);
    data(m, "BNPPPPPPPPQRRbbnnppppppqrr", 0, 0, 56);
    data(m, "BBPPPPPPPPQRRbnnpppppppqrr", 10, 20, 350);
    data(m, "NNPPPPPPPPQRRbbnpppppppqrr", 1, 2, 224);
    data(m, "BNPPPPPPPPQRRbbnpppppppqrr", 23, 35, 1285);
    data(m, "BBPPPPPPPPQRRbbnpppppppqrr", 4, 18, 584);
    data(m, "BBNPPPPPPPQRRbbnnppppppqrr", 62, 99, 2632);
    data(m, "BNPPPPPPPPQRRbnnpppppppqrr", 8, 19, 667);
    data(m, "BBNNPPPPPPPQRbbnpppppppqrr", 17, 21, 125);
    data(m, "BNNPPPPPPPQRRbbnnppppppqrr", 20, 38, 1455);
    data(m, "NNPPPPPPPPQRRbnnpppppppqrr", 1, 3, 114);
    data(m, "BBNNPPPPPPPQRbnnpppppppqrr", 27, 31, 136);
    data(m, "BBNPPPPPPPPQRbnppppppppqrr", 3, 20, 51);
    data(m, "BBNPPPPPPPQRRbnnpppppppqrr", 2623, 3496, 2389);
    data(m, "BNNPPPPPPPQRRbbnpppppppqrr", 1817, 3044, 2900);
    data(m, "BBPPPPPPPPQRRbnppppppppqrr", 301, 409, 233);
    data(m, "BBPPPPPPPPQRRnnppppppppqrr", 91, 138, 70);
    data(m, "BNPPPPPPPPQRRbbppppppppqrr", 209, 410, 376);
    data(m, "NNPPPPPPPPQRRbnppppppppqrr", 127, 261, 145);
    data(m, "BNPPPPPPPPQRRnnppppppppqrr", 164, 279, 158);
    data(m, "BBNNPPPPPPPQRbnppppppppqrr", 71, 19, 4);
    data(m, "BNNPPPPPPPPRRbbnpppppppprr", 23, 42, 15);
    data(m, "NNPPPPPPPPQRRbbppppppppqrr", 90, 137, 138);
    data(m, "BBNPPPPPPPPQRRbbnnpppppqrr", 30, 20, 54);
    data(m, "BNPPPPPPPPQRRbbnnpppppppqr", 3, 8, 49);
    data(m, "BBNPPPPPPPPRRbnnpppppppprr", 29, 48, 19);
    data(m, "BBNNPPPPPPQRRbnnpppppppqrr", 1810, 117, 49);
    data(m, "BBNNPPPPPPQRRbbnpppppppqrr", 2183, 130, 65);
    data(m, "BBNPPPPPPPQRRbbppppppppqrr", 515, 38, 7);
    data(m, "BBNPPPPPPPPQRRbnnppppppqrr", 287, 47, 25);
    data(m, "BNNPPPPPPPPQRRbbnppppppqrr", 266, 76, 40);
    data(m, "BBNPPPPPPPQRRbnppppppppqrr", 1259, 37, 28);
    data(m, "BBNPPPPPPPPQRRbbnppppppqrr", 441, 118, 53);
    data(m, "BNNPPPPPPPQRRnnppppppppqrr", 143, 4, 2);
    data(m, "BBNNPPPPPPPQRRbbnnpppppqrr", 922, 196, 146);
    data(m, "BBNPPPPPPPQRRnnppppppppqrr", 257, 2, 2);
    data(m, "BNNPPPPPPPQRRbnppppppppqrr", 554, 31, 19);
    data(m, "BNNPPPPPPPPQRRbnnppppppqrr", 117, 26, 8);
    data(m, "BBNPPPPPPPQRRbbnnpppppppqr", 116, 36, 17);
    data(m, "BNNPPPPPPPQRRbbnnpppppppqr", 126, 53, 29);
    data(m, "BNNPPPPPPPQRRbbppppppppqrr", 306, 34, 10);
    data(m, "BNPPPPPPPPQRRbbnppppppppqr", 54, 23, 22);
    data(m, "BNPPPPPPPPQRRbnnppppppppqr", 37, 10, 6);
    data(m, "BBNNPPPPPPPQRRbbnppppppqrr", 1116, 16, 20);
    data(m, "BBNPPPPPPPPQRRnnpppppppqrr", 61, 0, 1);
    data(m, "BBNNPPPPPPPQRRbnnppppppqrr", 546, 9, 11);
    data(m, "BBNPPPPPPPPQRRbnpppppppqrr", 349, 2, 0);
    data(m, "BBNNPPPPPPQRRbbnnpppppppqr", 107, 3, 9);
    data(m, "BNNPPPPPPPPQRRbbpppppppqrr", 80, 0, 0);
    data(m, "BBNPPPPPPPPQRRbbpppppppqrr", 134, 1, 1);
    data(m, "BNNPPPPPPPPQRRbnpppppppqrr", 196, 0, 0);
    data(m, "BNPPPPPPPPQRRbbnnppppppprr", 63, 3, 1);
    data(m, "BBNPPPPPPPPQRRbbnnppppppqr", 55, 3, 7);
    data(m, "BBNPPPPPPPQRRbbnnppppppprr", 816, 2, 1);
    data(m, "BBNNPPPPPPPQRRbnpppppppqrr", 245, 1, 0);
    data(m, "BBNPPPPPPPPQRRbnnpppppppqr", 57, 0, 0);
    data(m, "BNNPPPPPPPQRRbbnnppppppprr", 543, 0, 1);
    data(m, "BBNNPPPPPPPQRRbbnnppppppqr", 192, 4, 1);
    data(m, "BBNPPPPPPPPQRRbbnpppppppqr", 53, 0, 0);
    data(m, "BBNNPPPPPPPQRRbbpppppppqrr", 64, 0, 0);
    data(m, "NNPPPPPPPPQRRbbnpppppppprr", 50, 0, 0);
    data(m, "BBNPPPPPPPPQRRbppppppppqrr", 50, 0, 0);
    data(m, "BNPPPPPPPPQRRbbnpppppppprr", 353, 2, 1);
    data(m, "BNPPPPPPPPQRRbnnpppppppprr", 141, 0, 0);
    data(m, "BBPPPPPPPPQRRbbnpppppppprr", 67, 1, 0);
    data(m, "BBNNPPPPPPQRRbbnnppppppprr", 812, 0, 0);
    data(m, "BNNPPPPPPPQRRbbnpppppppprr", 125, 0, 0);
    data(m, "BBNPPPPPPPQRRbbnpppppppprr", 180, 0, 0);
    data(m, "BBNPPPPPPPPQRRbbnnpppppprr", 56, 0, 0);
    data(m, "BNNPPPPPPPQRRbnnpppppppprr", 70, 0, 0);
    data(m, "BBNPPPPPPPQRRbnnpppppppprr", 131, 0, 0);
    data(m, "BNNPPPPPPPPQRRbbnppppppprr", 88, 0, 0);
    data(m, "BBNNPPPPPPPQRRbbnnpppppprr", 196, 0, 1);
    data(m, "BNNPPPPPPPPQRRbnnppppppprr", 98, 0, 0);
    data(m, "BBNPPPPPPPPQRRbbnppppppprr", 102, 0, 0);
    data(m, "BBNNPPPPPPPQRRbbnppppppprr", 95, 0, 0);
    data(m, "BBNNPPPPPPPQRRbnnppppppprr", 79, 0, 0);
    data(m, "BNNPPPPPPPPRRbbnppppppqrr", 0, 0, 83);
    data(m, "BBNNPPPPPPPRRbbnnpppppqrr", 0, 0, 344);
    data(m, "BBNPPPPPPPPRRbbnppppppqrr", 0, 0, 95);
    data(m, "BBNPPPPPPPPRRbnnppppppqrr", 0, 0, 56);
    data(m, "BBNNPPPPPPPRRbbnppppppqrr", 1, 4, 454);
    data(m, "BBNNPPPPPPPRRbnnppppppqrr", 1, 1, 313);
    data(m, "BBNPPPPPPPPRRbnpppppppqrr", 0, 4, 204);
    data(m, "BNNPPPPPPPPRRbnpppppppqrr", 2, 0, 60);
    data(m, "BBNPPPPPPPPRRbbpppppppqrr", 0, 0, 58);
    data(m, "BBNNPPPPPPPRRbnpppppppqrr", 6, 4, 118);
    data(m, "BBNNPPPPPPPRRbbnnppppppqr", 0, 2, 141);
    data(m, "BNPPPPPPPPQRRbbnppppppqrr", 39, 58, 524);
    data(m, "BNNPPPPPPPQRRbbnnpppppqrr", 31, 60, 423);
    data(m, "BBNPPPPPPPQRRbbnnpppppqrr", 87, 140, 782);
    data(m, "BBPPPPPPPPQRRbnnppppppqrr", 22, 24, 101);
    data(m, "BBNNPPPPPPPQRbnnppppppqrr", 34, 36, 51);
    data(m, "BNPPPPPPPPQRRbnnppppppqrr", 24, 34, 267);
    data(m, "BNNPPPPPPPPQRbnpppppppqrr", 15, 16, 19);
    data(m, "BBPPPPPPPPQRRbbnppppppqrr", 30, 36, 375);
    data(m, "BBNPPPPPPPPRRbbnnpppppprr", 1, 7, 43);
    data(m, "BBNPPPPPPPPQRbnpppppppqrr", 29, 12, 17);
    data(m, "BNNPPPPPPPPRRbbnnpppppprr", 0, 8, 52);
    data(m, "BBNNPPPPPPPQRbbnppppppqrr", 16, 42, 36);
    data(m, "NNPPPPPPPPQRRbbnppppppqrr", 12, 8, 82);
    data(m, "BBNPPPPPPPQRRbnnppppppqrr", 2178, 905, 503);
    data(m, "BBNNPPPPPPQRRbbnnpppppqrr", 1893, 1024, 730);
    data(m, "BNNPPPPPPPQRRbbnppppppqrr", 2594, 1680, 1163);
    data(m, "BBNNPPPPPPPRRbbnnpppppprr", 1447, 959, 299);
    data(m, "BNPPPPPPPPQRRbnpppppppqrr", 1151, 661, 283);
    data(m, "BBNNPPPPPPPQRbnpppppppqrr", 57, 6, 1);
    data(m, "BBNPPPPPPPQRRbbnppppppqrr", 4915, 2849, 1561);
    data(m, "BNNPPPPPPPQRRbnnppppppqrr", 1415, 668, 396);
    data(m, "BNPPPPPPPPQRRbbpppppppqrr", 354, 262, 159);
    data(m, "BBPPPPPPPPQRRbbpppppppqrr", 364, 213, 74);
    data(m, "BNPPPPPPPPQRRnnpppppppqrr", 172, 91, 49);
    data(m, "BBNPPPPPPPPRRbbnppppppprr", 144, 100, 43);
    data(m, "BNPPPPPPPPQRRbbnnppppppqr", 14, 17, 35);
    data(m, "NNPPPPPPPPQRRbnpppppppqrr", 159, 81, 48);
    data(m, "BBPPPPPPPPQRRnnpppppppqrr", 88, 38, 21);
    data(m, "BBPPPPPPPPQRRbnpppppppqrr", 488, 218, 76);
    data(m, "BBNNPPPPPPPQRbbnnppppppqr", 177, 101, 38);
    data(m, "NNPPPPPPPPQRRbbpppppppqrr", 64, 46, 18);
    data(m, "BBNPPPPPPPPRRbnnppppppprr", 63, 32, 4);
    data(m, "BNNPPPPPPPPRRbbnppppppprr", 72, 65, 21);
    data(m, "BNNPPPPPPPPRRbnnppppppprr", 42, 30, 8);
    data(m, "NNPPPPPPPPQRRnnpppppppqrr", 39, 18, 3);
    data(m, "BNNPPPPPPPQRRbnpppppppqrr", 1471, 31, 14);
    data(m, "BBNPPPPPPPQRRbnpppppppqrr", 2606, 28, 25);
    data(m, "BBNNPPPPPPQRRbnnppppppqrr", 1100, 34, 18);
    data(m, "BBPPPPPPPPQRRnppppppppqrr", 117, 4, 3);
    data(m, "BBNNPPPPPPQRRbbnppppppqrr", 1713, 68, 47);
    data(m, "BBNPPPPPPPQRRbbpppppppqrr", 890, 20, 4);
    data(m, "BBNNPPPPPPPQRbnnpppppppqr", 60, 2, 0);
    data(m, "BNPPPPPPPPQRRbbnpppppppqr", 124, 16, 6);
    data(m, "BBPPPPPPPPQRRbppppppppqrr", 189, 3, 0);
    data(m, "BBNNPPPPPPPRRbnnppppppprr", 451, 3, 2);
    data(m, "BNNPPPPPPPQRRbbpppppppqrr", 499, 11, 10);
    data(m, "BBNNPPPPPPPRRbbnppppppprr", 471, 7, 3);
    data(m, "BBNPPPPPPPQRRnnpppppppqrr", 516, 3, 1);
    data(m, "BBNPPPPPPPQRRbbnnppppppqr", 231, 28, 14);
    data(m, "BNPPPPPPPPQRRbppppppppqrr", 302, 6, 3);
    data(m, "BBNPPPPPPPPRRbnpppppppprr", 89, 1, 0);
    data(m, "BNNPPPPPPPQRRnnpppppppqrr", 241, 9, 3);
    data(m, "BNNPPPPPPPQRRbbnnppppppqr", 175, 24, 27);
    data(m, "BNPPPPPPPPQRRbnnpppppppqr", 89, 15, 11);
    data(m, "NNPPPPPPPPQRRbppppppppqrr", 54, 1, 0);
    data(m, "BNPPPPPPPPQRRnppppppppqrr", 267, 10, 2);
    data(m, "BBNNPPPPPPQRRbbnnppppppqr", 223, 8, 5);
    data(m, "BBNNPPPPPPPRRbbnnpppppppr", 102, 0, 0);
    data(m, "BBNPPPPPPPQRRbnnpppppppqr", 160, 1, 1);
    data(m, "BNNPPPPPPPQRRbbnpppppppqr", 149, 6, 1);
    data(m, "BBNNPPPPPPQRRbnpppppppqrr", 159, 0, 0);
    data(m, "BBNPPPPPPPQRRbbnpppppppqr", 189, 5, 3);
    data(m, "BBNNPPPPPPQRRbbpppppppqrr", 80, 0, 0);
    data(m, "BNNPPPPPPPQRRbnnpppppppqr", 126, 1, 1);
    data(m, "BNPPPPPPPPQRRbnppppppppqr", 62, 0, 0);
    data(m, "BBNNPPPPPPPQRRbbnpppppqrr", 112, 0, 0);
    data(m, "BBNPPPPPPPQRRbbnnpppppprr", 474, 0, 1);
    data(m, "BNPPPPPPPPQRRbbnppppppprr", 259, 1, 0);
    data(m, "BBPPPPPPPPQRRbbnppppppprr", 90, 0, 0);
    data(m, "BNNPPPPPPPQRRbbnnpppppprr", 273, 0, 0);
    data(m, "BNPPPPPPPPQRRbnnppppppprr", 83, 0, 0);
    data(m, "BBNPPPPPPPQRRbnnppppppprr", 361, 0, 0);
    data(m, "BBNPPPPPPPQRRbbnppppppprr", 841, 0, 0);
    data(m, "BBNNPPPPPPQRRbbnnpppppprr", 569, 0, 0);
    data(m, "BNNPPPPPPPQRRbbnppppppprr", 615, 0, 0);
    data(m, "BNNPPPPPPPQRRbnnppppppprr", 315, 0, 1);
    data(m, "BNPPPPPPPPQRRbnpppppppprr", 190, 0, 0);
    data(m, "BBNNPPPPPPQRRbbnppppppprr", 120, 0, 0);
    data(m, "BBNNPPPPPPQRRbnnppppppprr", 86, 0, 0);
    data(m, "BNNPPPPPPPRRbnnppppppqrr", 0, 0, 210);
    data(m, "BBNPPPPPPPRRbbnppppppqrr", 0, 1, 451);
    data(m, "BNPPPPPPPPRRbnpppppppqrr", 0, 0, 107);
    data(m, "BBNPPPPPPPRRbnnppppppqrr", 0, 0, 252);
    data(m, "BNNPPPPPPPRRbbnppppppqrr", 0, 0, 206);
    data(m, "BBNNPPPPPPRRbbnnpppppqrr", 4, 0, 180);
    data(m, "BBNPPPPPPPRRnnpppppppqrr", 0, 1, 111);
    data(m, "BBNNPPPPPPRRbbnppppppqrr", 1, 0, 303);
    data(m, "BBNPPPPPPPRRbnpppppppqrr", 0, 0, 489);
    data(m, "BNNPPPPPPPRRbnpppppppqrr", 0, 0, 270);
    data(m, "BNNPPPPPPPRRbbpppppppqrr", 0, 0, 75);
    data(m, "BBNNPPPPPPRRbnnppppppqrr", 0, 0, 263);
    data(m, "BBNPPPPPPPRRbbpppppppqrr", 0, 1, 168);
    data(m, "BBNPPPPPPPQRbbnppppppqrr", 1, 0, 95);
    data(m, "BNNPPPPPPPQRbbnppppppqrr", 5, 1, 101);
    data(m, "BBNPPPPPPPRRbbnpppppppqr", 1, 0, 51);
    data(m, "BBNNPPPPPPRRbnpppppppqrr", 4, 3, 50);
    data(m, "BBNNPPPPPPPRRbnnpppppqrr", 0, 1, 60);
    data(m, "BBNNPPPPPPQRbbnnpppppqrr", 2, 2, 47);
    data(m, "BBNPPPPPPPQRbnnppppppqrr", 0, 1, 74);
    data(m, "BBNNPPPPPPRRbbnnppppppqr", 0, 0, 56);
    data(m, "BBNNPPPPPPPRRbbnpppppqrr", 0, 0, 56);
    data(m, "BNNPPPPPPQRRbbnnpppppqrr", 12, 17, 290);
    data(m, "NNPPPPPPPQRRbbnppppppqrr", 3, 7, 204);
    data(m, "BNPPPPPPPQRRbbnppppppqrr", 23, 44, 1451);
    data(m, "BBNPPPPPPPRRbbnnpppppprr", 2, 20, 417);
    data(m, "BNPPPPPPPQRRbnnppppppqrr", 9, 22, 909);
    data(m, "BBPPPPPPPQRRbbnppppppqrr", 35, 34, 709);
    data(m, "NPPPPPPPPQRRbnpppppppqrr", 3, 5, 204);
    data(m, "BBNPPPPPPPQRbnpppppppqrr", 36, 58, 149);
    data(m, "BNNPPPPPPPRRbbnnpppppprr", 3, 4, 277);
    data(m, "BNNPPPPPPPQRbnpppppppqrr", 29, 22, 103);
    data(m, "BBNPPPPPPQRRbbnnpppppqrr", 16, 26, 395);
    data(m, "BBPPPPPPPQRRbnnppppppqrr", 10, 14, 333);
    data(m, "BBNNPPPPPPQRbbnppppppqrr", 11, 18, 62);
    data(m, "BBNNPPPPPPQRbnnppppppqrr", 17, 21, 64);
    data(m, "NNPPPPPPPQRRbnnppppppqrr", 1, 4, 90);
    data(m, "BBNPPPPPPPQRbbpppppppqrr", 2, 9, 39);
    data(m, "NPPPPPPPPQRRnnpppppppqrr", 1, 2, 47);
    data(m, "BPPPPPPPPQRRbnpppppppqrr", 4, 6, 308);
    data(m, "NPPPPPPPPQRRbbpppppppqrr", 2, 5, 125);
    data(m, "BNPPPPPPPPRRbbnppppppprr", 1, 0, 99);
    data(m, "BPPPPPPPPQRRbbpppppppqrr", 3, 5, 190);
    data(m, "BNNPPPPPPPQRbbnnppppppqr", 0, 3, 55);
    data(m, "BNNPPPPPPPQRbbpppppppqrr", 2, 14, 37);
    data(m, "BNNPPPPPPPRRbbnppppppprr", 206, 487, 337);
    data(m, "BNNPPPPPPQRRbbnppppppqrr", 676, 1045, 1219);
    data(m, "BBPPPPPPPQRRnnpppppppqrr", 204, 255, 131);
    data(m, "BBNPPPPPPQRRbnnppppppqrr", 1099, 1223, 852);
    data(m, "BBNPPPPPPPRRbnnppppppprr", 382, 609, 277);
    data(m, "BNPPPPPPPQRRbbpppppppqrr", 537, 958, 893);
    data(m, "BBNNPPPPPPQRbnpppppppqrr", 92, 23, 13);
    data(m, "BBPPPPPPPQRRbnpppppppqrr", 774, 995, 576);
    data(m, "NPPPPPPPPQRRbppppppppqrr", 71, 129, 85);
    data(m, "BNPPPPPPPQRRnnpppppppqrr", 298, 497, 340);
    data(m, "NNPPPPPPPQRRbbpppppppqrr", 101, 227, 196);
    data(m, "BBNPPPPPPPQRbnnpppppppqr", 81, 109, 52);
    data(m, "BPPPPPPPPQRRnppppppppqrr", 94, 183, 117);
    data(m, "NNPPPPPPPQRRbnpppppppqrr", 251, 427, 302);
    data(m, "BNPPPPPPPQRRbbnnppppppqr", 6, 18, 97);
    data(m, "BNNPPPPPPPQRbbnpppppppqr", 57, 105, 65);
    data(m, "BNPPPPPPPPQRRbbnpppppqrr", 25, 10, 37);
    data(m, "BBNPPPPPPPQRRbbnnppppqrr", 18, 16, 51);
    data(m, "BNNPPPPPPQRRbbpppppppqrr", 283, 20, 6);
    data(m, "BBNNPPPPPPRRbnnppppppprr", 295, 1, 13);
    data(m, "BBNPPPPPPPQRRbbnpppppqrr", 641, 176, 123);
    data(m, "BNNPPPPPPQRRbnpppppppqrr", 746, 45, 17);
    data(m, "BBNPPPPPPPQRRbnnpppppqrr", 287, 48, 30);
    data(m, "BNPPPPPPPQRRbbnpppppppqr", 146, 69, 34);
    data(m, "BBNNPPPPPQRRbbnppppppqrr", 316, 35, 24);
    data(m, "BBNPPPPPPQRRbbpppppppqrr", 611, 27, 12);
    data(m, "BNPPPPPPPPQRRbnppppppqrr", 185, 32, 13);
    data(m, "BNPPPPPPPQRRnppppppppqrr", 222, 6, 5);
    data(m, "BNPPPPPPPQRRbnnpppppppqr", 118, 47, 27);
    data(m, "BBNNPPPPPQRRbnnppppppqrr", 260, 12, 11);
    data(m, "BBNNPPPPPPRRbbnppppppprr", 435, 16, 4);
    data(m, "BBNPPPPPPQRRnnpppppppqrr", 275, 5, 2);
    data(m, "BBNPPPPPPQRRbnpppppppqrr", 1571, 63, 49);
    data(m, "BNPPPPPPPQRRbppppppppqrr", 282, 8, 0);
    data(m, "BBNNPPPPPPPRRbbnnppppprr", 171, 42, 14);
    data(m, "BBNNPPPPPPQRRbbnnppppqrr", 167, 38, 45);
    data(m, "BBPPPPPPPQRRbbnpppppppqr", 56, 15, 15);
    data(m, "BNNPPPPPPPRRbbnnpppppppr", 53, 12, 9);
    data(m, "BNNPPPPPPPQRRbbnpppppqrr", 428, 148, 110);
    data(m, "BBNPPPPPPQRRbbnnppppppqr", 56, 19, 19);
    data(m, "BNNPPPPPPQRRnnpppppppqrr", 151, 6, 4);
    data(m, "BNNPPPPPPPQRRbnnpppppqrr", 164, 41, 34);
    data(m, "BBPPPPPPPQRRbppppppppqrr", 143, 10, 1);
    data(m, "BBNPPPPPPPRRbnpppppppprr", 86, 0, 0);
    data(m, "BNNPPPPPPQRRbbnnppppppqr", 67, 19, 17);
    data(m, "BNPPPPPPPPQRRnnppppppqrr", 45, 9, 4);
    data(m, "BBPPPPPPPPQRRbbppppppqrr", 47, 15, 5);
    data(m, "BBPPPPPPPPQRRbnppppppqrr", 82, 16, 6);
    data(m, "BBNPPPPPPPPRRbbnpppppprr", 60, 13, 1);
    data(m, "BNPPPPPPPPQRRbbppppppqrr", 53, 13, 15);
    data(m, "BBNNPPPPPPQRbbnpppppppqr", 51, 2, 0);
    data(m, "BBPPPPPPPQRRnppppppppqrr", 78, 2, 2);
    data(m, "BBPPPPPPPQRRbnnpppppppqr", 33, 13, 5);
    data(m, "BNPPPPPPPQRRbbnnpppppprr", 84, 2, 2);
    data(m, "BBNPPPPPPPQRRbbppppppqrr", 278, 2, 0);
    data(m, "BBNNPPPPPPPRRbbnpppppprr", 169, 2, 0);
    data(m, "BBNNPPPPPPQRRbnnpppppqrr", 193, 6, 3);
    data(m, "BNNPPPPPPPQRRbbppppppqrr", 152, 3, 0);
    data(m, "BNNPPPPPPPQRRbnppppppqrr", 385, 5, 7);
    data(m, "BBNNPPPPPPQRRbbnpppppqrr", 255, 5, 11);
    data(m, "BNPPPPPPPPQRRnpppppppqrr", 86, 0, 3);
    data(m, "BBNPPPPPPPQRRnnppppppqrr", 76, 0, 0);
    data(m, "BBNNPPPPPPPRRbnnpppppprr", 103, 0, 0);
    data(m, "BBNPPPPPPPQRRbnppppppqrr", 570, 6, 11);
    data(m, "BNNPPPPPPQRRbnnpppppppqr", 49, 0, 1);
    data(m, "BNNPPPPPPPQRRnnppppppqrr", 68, 1, 0);
    data(m, "BBNPPPPPPPQRRbbnnpppppqr", 62, 3, 4);
    data(m, "BBPPPPPPPPQRRbpppppppqrr", 50, 0, 0);
    data(m, "BNPPPPPPPPQRRbpppppppqrr", 71, 0, 1);
    data(m, "BBNPPPPPPQRRbnnpppppppqr", 81, 1, 1);
    data(m, "BBNNPPPPPQRRbnpppppppqrr", 55, 0, 1);
    data(m, "BNNPPPPPPPQRRbbnnpppppqr", 43, 2, 8);
    data(m, "BBNPPPPPPQRRbbnpppppppqr", 82, 2, 2);
    data(m, "BNNPPPPPPQRRbbnpppppppqr", 70, 0, 0);
    data(m, "BNPPPPPPPQRRbnnppppppprr", 247, 1, 0);
    data(m, "BNPPPPPPPQRRbbnppppppprr", 541, 2, 2);
    data(m, "BBNPPPPPPQRRbbnnpppppprr", 259, 1, 0);
    data(m, "BNNPPPPPPPQRRbbnppppppqr", 127, 2, 3);
    data(m, "BBPPPPPPPQRRbnnppppppprr", 93, 1, 0);
    data(m, "BBNNPPPPPPPRRbbnnppppppr", 137, 0, 1);
    data(m, "BBNPPPPPPPQRRbpppppppqrr", 114, 0, 1);
    data(m, "BBNPPPPPPPQRRbbnppppppqr", 140, 3, 0);
    data(m, "BBNNPPPPPPQRRbnppppppqrr", 67, 0, 0);
    data(m, "BBPPPPPPPQRRbbnppppppprr", 229, 0, 0);
    data(m, "BNNPPPPPPQRRbbnnpppppprr", 200, 0, 0);
    data(m, "BBNPPPPPPPQRRnpppppppqrr", 71, 0, 0);
    data(m, "NNPPPPPPPQRRbbnppppppprr", 72, 0, 0);
    data(m, "BNNPPPPPPPQRRbnnppppppqr", 69, 0, 1);
    data(m, "BBNPPPPPPPQRRbnnppppppqr", 57, 0, 1);
    data(m, "BBNNPPPPPPQRRbbnnpppppqr", 47, 3, 3);
    data(m, "BNNPPPPPPQRRbnnppppppprr", 145, 0, 0);
    data(m, "BBNPPPPPPQRRbnnppppppprr", 269, 0, 0);
    data(m, "BBNPPPPPPQRRbbnppppppprr", 489, 0, 0);
    data(m, "BBNNPPPPPQRRbbnnpppppprr", 106, 0, 0);
    data(m, "BNPPPPPPPQRRbnpppppppprr", 116, 0, 0);
    data(m, "BNNPPPPPPQRRbbnppppppprr", 254, 0, 0);
    data(m, "BBNPPPPPPPQRRbbnnppppprr", 87, 0, 0);
    data(m, "BNNPPPPPPPQRRbnnpppppprr", 90, 0, 0);
    data(m, "BNNPPPPPPPQRRbbnpppppprr", 171, 0, 0);
    data(m, "BBNPPPPPPPQRRbbnpppppprr", 304, 0, 0);
    data(m, "BBNNPPPPPPQRRbbnnppppprr", 122, 0, 0);
    data(m, "BBNPPPPPPPQRRbnnpppppprr", 128, 0, 0);
    data(m, "BNPPPPPPPPQRRbnppppppprr", 88, 0, 0);
    data(m, "BBNPPPPPPPQRRbnppppppprr", 95, 0, 0);
    data(m, "BBNNPPPPPPQRRbbnpppppprr", 68, 0, 0);
    data(m, "BNNPPPPPPPQRRbnppppppprr", 60, 0, 0);
    data(m, "BBNNPPPPPPRRbbnnppppqrr", 0, 0, 67);
    data(m, "BNNPPPPPPPRRbbnpppppqrr", 0, 0, 130);
    data(m, "BBNPPPPPPPRRbnnpppppqrr", 0, 0, 90);
    data(m, "BNNPPPPPPPRRbnnpppppqrr", 0, 0, 63);
    data(m, "BBNPPPPPPPRRbbnpppppqrr", 0, 0, 161);
    data(m, "BBNNPPPPPPRRbnnpppppqrr", 0, 0, 109);
    data(m, "BBNNPPPPPPRRbbnpppppqrr", 2, 0, 147);
    data(m, "BNNPPPPPPPRRbnppppppqrr", 0, 1, 259);
    data(m, "BBNPPPPPPPRRbbppppppqrr", 0, 1, 178);
    data(m, "BNPPPPPPPPRRbpppppppqrr", 0, 0, 64);
    data(m, "BBNPPPPPPPRRbnppppppqrr", 1, 1, 399);
    data(m, "BBNPPPPPPPRRnnppppppqrr", 1, 0, 59);
    data(m, "BNNPPPPPPPRRbbppppppqrr", 0, 0, 82);
    data(m, "BBNPPPPPPPRRbpppppppqrr", 6, 7, 60);
    data(m, "BBNPPPPPPPQRbbnpppppqrr", 4, 3, 63);
    data(m, "BBNNPPPPPPRRbnppppppqrr", 6, 4, 69);
    data(m, "BBNPPPPPPPRRbbnppppppqr", 0, 1, 102);
    data(m, "BBNPPPPPPPRRnpppppppqrr", 2, 4, 44);
    data(m, "BBNPPPPPPPRRbnnppppppqr", 0, 1, 61);
    data(m, "BBNPPPPPPPQRbnppppppqrr", 89, 77, 105);
    data(m, "BPPPPPPPPQRRbnppppppqrr", 8, 18, 204);
    data(m, "BNNPPPPPPQRRbbnnppppqrr", 7, 16, 79);
    data(m, "BBNNPPPPPPQRbbnpppppqrr", 20, 13, 24);
    data(m, "BBNPPPPPPPRRbnpppppppqr", 2, 8, 46);
    data(m, "BNPPPPPPPPQRbpppppppqrr", 17, 12, 21);
    data(m, "BNPPPPPPPQRRbbnpppppqrr", 49, 61, 609);
    data(m, "BBNPPPPPPPRRbbnnppppprr", 19, 41, 208);
    data(m, "BNNPPPPPPPRRbbnnppppprr", 5, 21, 154);
    data(m, "BBPPPPPPPQRRbnnpppppqrr", 12, 11, 92);
    data(m, "BPPPPPPPPQRRbbppppppqrr", 7, 6, 143);
    data(m, "BNPPPPPPPQRRbnnpppppqrr", 31, 36, 313);
    data(m, "BBNNPPPPPPQRbnnpppppqrr", 32, 19, 26);
    data(m, "BNPPPPPPPPRRbbnpppppprr", 1, 4, 80);
    data(m, "BBPPPPPPPQRRbbnpppppqrr", 36, 39, 329);
    data(m, "BBNPPPPPPPQRbbppppppqrr", 15, 25, 30);
    data(m, "BBPPPPPPPPRRbbnpppppprr", 7, 5, 49);
    data(m, "BBNPPPPPPQRRbbnnppppqrr", 15, 15, 87);
    data(m, "NNPPPPPPPQRRbnnpppppqrr", 3, 15, 39);
    data(m, "BNNPPPPPPPQRbnppppppqrr", 54, 50, 85);
    data(m, "NPPPPPPPPQRRbbppppppqrr", 1, 2, 61);
    data(m, "NNPPPPPPPQRRbbnpppppqrr", 4, 10, 76);
    data(m, "NPPPPPPPPQRRbnppppppqrr", 5, 7, 95);
    data(m, "BBNPPPPPPPRRbbnpppppprr", 1212, 790, 221);
    data(m, "BNNPPPPPPPRRbbnpppppprr", 490, 436, 150);
    data(m, "BNPPPPPPPQRRbnppppppqrr", 2633, 1506, 712);
    data(m, "BBNPPPPPPQRRbnnpppppqrr", 714, 282, 165);
    data(m, "BBNPPPPPPQRRbbnpppppqrr", 1669, 1006, 765);
    data(m, "BNNPPPPPPPQRbbnppppppqr", 119, 64, 36);
    data(m, "BNNPPPPPPQRRbbnpppppqrr", 921, 679, 431);
    data(m, "BNPPPPPPPQRRnnppppppqrr", 361, 212, 90);
    data(m, "BBNNPPPPPPQRbbnnpppppqr", 177, 82, 66);
    data(m, "BBPPPPPPPQRRbnppppppqrr", 1048, 463, 217);
    data(m, "BBNPPPPPPPQRbpppppppqrr", 47, 4, 0);
    data(m, "BBNNPPPPPPRRbbnnppppprr", 545, 346, 133);
    data(m, "BNNPPPPPPPRRbnnpppppprr", 400, 205, 63);
    data(m, "NNPPPPPPPQRRbnppppppqrr", 325, 184, 89);
    data(m, "BNPPPPPPPPRRbbppppppprr", 37, 48, 6);
    data(m, "NNPPPPPPPQRRbbppppppqrr", 144, 96, 58);
    data(m, "BBPPPPPPPQRRbbppppppqrr", 557, 359, 177);
    data(m, "BNPPPPPPPQRRbbppppppqrr", 1016, 810, 417);
    data(m, "BNNPPPPPPQRRbnnpppppqrr", 490, 292, 174);
    data(m, "BBPPPPPPPQRRnnppppppqrr", 179, 87, 24);
    data(m, "BBNNPPPPPQRRbbnnppppqrr", 135, 59, 40);
    data(m, "BBNPPPPPPPRRbnnpppppprr", 491, 222, 59);
    data(m, "BNPPPPPPPPRRbnppppppprr", 157, 87, 10);
    data(m, "BPPPPPPPPQRRnpppppppqrr", 160, 102, 39);
    data(m, "BBNPPPPPPPQRbbnppppppqr", 209, 111, 46);
    data(m, "BNPPPPPPPQRRbbnnpppppqr", 25, 24, 52);
    data(m, "BPPPPPPPPQRRbpppppppqrr", 349, 193, 42);
    data(m, "BBNPPPPPPPQRbnnppppppqr", 82, 29, 16);
    data(m, "BNNPPPPPPPQRbnnppppppqr", 54, 31, 22);
    data(m, "BBPPPPPPPPRRbbppppppprr", 43, 24, 5);
    data(m, "BBNNPPPPPPPRbbnnppppppr", 38, 24, 3);
    data(m, "BPPPPPPPPQRRbbnppppppqr", 9, 11, 32);
    data(m, "NPPPPPPPPQRRnpppppppqrr", 111, 53, 20);
    data(m, "NPPPPPPPPQRRbpppppppqrr", 165, 73, 25);
    data(m, "BBNNPPPPPPQRbnppppppqrr", 63, 12, 5);
    data(m, "NNPPPPPPPQRRnnppppppqrr", 77, 36, 13);
    data(m, "BBPPPPPPPPRRbnppppppprr", 39, 19, 3);
    data(m, "BNNPPPPPPQRRbnppppppqrr", 898, 14, 10);
    data(m, "BBNPPPPPPQRRbnppppppqrr", 1176, 22, 33);
    data(m, "BBNPPPPPPQRRnnppppppqrr", 230, 2, 1);
    data(m, "BNPPPPPPPQRRnpppppppqrr", 625, 11, 9);
    data(m, "BNPPPPPPPQRRbpppppppqrr", 882, 14, 6);
    data(m, "BBNPPPPPPQRRbbppppppqrr", 688, 15, 13);
    data(m, "BNNPPPPPPQRRbbppppppqrr", 260, 14, 4);
    data(m, "BBPPPPPPPQRRbbnppppppqr", 121, 9, 2);
    data(m, "BNNPPPPPPPQRbnpppppppqr", 70, 0, 0);
    data(m, "BBPPPPPPPQRRnpppppppqrr", 227, 5, 2);
    data(m, "BNNPPPPPPPRRbnppppppprr", 252, 3, 3);
    data(m, "BBNNPPPPPPRRbbnpppppprr", 499, 9, 2);
    data(m, "BNPPPPPPPQRRbbnppppppqr", 346, 37, 43);
    data(m, "BBNPPPPPPPQRbnpppppppqr", 100, 1, 1);
    data(m, "BPPPPPPPPQRRppppppppqrr", 101, 1, 0);
    data(m, "BBNNPPPPPPQRbbnppppppqr", 81, 0, 0);
    data(m, "BBNPPPPPPPRRbbnnppppppr", 54, 7, 2);
    data(m, "BBNNPPPPPQRRbnnpppppqrr", 100, 6, 4);
    data(m, "BBNNPPPPPPRRbnnpppppprr", 305, 2, 0);
    data(m, "BBPPPPPPPQRRbpppppppqrr", 393, 4, 4);
    data(m, "NNPPPPPPPQRRbpppppppqrr", 115, 1, 1);
    data(m, "BBNPPPPPPPRRbbppppppprr", 179, 2, 2);
    data(m, "BBNPPPPPPPRRbnppppppprr", 608, 4, 0);
    data(m, "BNPPPPPPPQRRbnnppppppqr", 138, 16, 19);
    data(m, "BBNNPPPPPQRRbbnpppppqrr", 207, 11, 7);
    data(m, "BNNPPPPPPQRRbbnnpppppqr", 61, 4, 13);
    data(m, "BNNPPPPPPQRRnnppppppqrr", 121, 8, 3);
    data(m, "BNNPPPPPPPRRbbppppppprr", 58, 4, 2);
    data(m, "BBNPPPPPPPRRnnppppppprr", 139, 1, 0);
    data(m, "NNPPPPPPPQRRnpppppppqrr", 100, 0, 0);
    data(m, "BBNPPPPPPQRRbbnnpppppqr", 66, 13, 7);
    data(m, "BPPPPPPPPQRRbnpppppppqr", 68, 5, 5);
    data(m, "BNNPPPPPPPRRbbnnppppppr", 37, 11, 6);
    data(m, "BBNNPPPPPPQRbnnppppppqr", 51, 1, 0);
    data(m, "NPPPPPPPPQRRbnpppppppqr", 44, 3, 4);
    data(m, "BBNPPPPPPQRRnpppppppqrr", 69, 0, 0);
    data(m, "BNPPPPPPPQRRbbnnppppprr", 65, 0, 2);
    data(m, "BBNPPPPPPPQRRbnpppppqrr", 75, 1, 0);
    data(m, "BNPPPPPPPQRRbnpppppppqr", 206, 2, 0);
    data(m, "BBNPPPPPPQRRbnnppppppqr", 128, 0, 3);
    data(m, "BNNPPPPPPQRRbbnppppppqr", 116, 1, 0);
    data(m, "BNNPPPPPPQRRbnnppppppqr", 111, 0, 2);
    data(m, "BBNNPPPPPPRRbbnnppppppr", 88, 1, 0);
    data(m, "BBNPPPPPPQRRbbnppppppqr", 229, 2, 5);
    data(m, "BBNPPPPPPPRRbbnpppppppr", 81, 0, 0);
    data(m, "BBNPPPPPPQRRbpppppppqrr", 89, 0, 0);
    data(m, "BBPPPPPPPQRRbnpppppppqr", 60, 0, 0);
    data(m, "BNPPPPPPPQRRbbpppppppqr", 67, 3, 2);
    data(m, "BBNNPPPPPQRRbbnnpppppqr", 68, 2, 2);
    data(m, "BNNPPPPPPQRRbbnnppppprr", 131, 0, 0);
    data(m, "BNPPPPPPPQRRbnnpppppprr", 204, 0, 0);
    data(m, "NNPPPPPPPQRRbbnpppppprr", 59, 0, 0);
    data(m, "BPPPPPPPPQRRbnppppppprr", 51, 0, 0);
    data(m, "BNPPPPPPPQRRbbnpppppprr", 467, 4, 1);
    data(m, "BBPPPPPPPQRRbnnpppppprr", 89, 0, 0);
    data(m, "BBPPPPPPPQRRbbnpppppprr", 188, 0, 0);
    data(m, "BBNPPPPPPQRRbbnnppppprr", 122, 0, 0);
    data(m, "BNNPPPPPPQRRbbnpppppprr", 302, 0, 0);
    data(m, "BBNPPPPPPQRRbbnpppppprr", 476, 0, 0);
    data(m, "BBNPPPPPPQRRbnnpppppprr", 254, 0, 0);
    data(m, "BNPPPPPPPQRRbbppppppprr", 183, 0, 0);
    data(m, "BPPPPPPPPQRRbpppppppprr", 60, 0, 0);
    data(m, "BNNPPPPPPQRRbnnpppppprr", 191, 0, 0);
    data(m, "BNPPPPPPPQRRbnppppppprr", 532, 0, 0);
    data(m, "BBPPPPPPPQRRbbppppppprr", 108, 0, 0);
    data(m, "BNPPPPPPPQRRnnppppppprr", 81, 0, 0);
    data(m, "BBPPPPPPPQRRbnppppppprr", 164, 0, 0);
    data(m, "NNPPPPPPPQRRbnppppppprr", 68, 0, 0);
    data(m, "BBNNPPPPPQRRbbnnppppprr", 72, 0, 0);
    data(m, "BBNNPPPPPQRRbbnpppppprr", 50, 0, 0);
    data(m, "BBNPPPPPPQRRbnppppppprr", 81, 0, 0);
    data(m, "BBNPPPPPPPQRRbnpppppprr", 55, 0, 0);
    data(m, "BBNPPPPPPRRbbnpppppqrr", 0, 0, 149);
    data(m, "BNPPPPPPPRRbnppppppqrr", 0, 0, 309);
    data(m, "BBNPPPPPPRRbnnpppppqrr", 0, 0, 118);
    data(m, "BBPPPPPPPRRbbppppppqrr", 0, 0, 53);
    data(m, "BBPPPPPPPRRbnppppppqrr", 0, 0, 101);
    data(m, "BNPPPPPPPRRbbppppppqrr", 0, 0, 115);
    data(m, "BNNPPPPPPRRbbnpppppqrr", 0, 1, 80);
    data(m, "BNNPPPPPPRRbnnpppppqrr", 0, 0, 64);
    data(m, "BNPPPPPPPRRbpppppppqrr", 0, 0, 170);
    data(m, "BBNPPPPPPRRbnppppppqrr", 1, 1, 289);
    data(m, "BNNPPPPPPRRbnppppppqrr", 0, 0, 118);
    data(m, "BBPPPPPPPRRbpppppppqrr", 0, 0, 83);
    data(m, "BBNPPPPPPRRbbppppppqrr", 0, 0, 106);
    data(m, "BNPPPPPPPRRnpppppppqrr", 1, 0, 138);
    data(m, "BBPPPPPPPRRnpppppppqrr", 0, 0, 55);
    data(m, "BBNPPPPPPQRbbnpppppqrr", 1, 2, 67);
    data(m, "BNPPPPPPPQRbnppppppqrr", 6, 2, 166);
    data(m, "BNPPPPPPPQRbbppppppqrr", 0, 0, 56);
    data(m, "BNNPPPPPPQRbbnpppppqrr", 1, 0, 51);
    data(m, "BNPPPPPPPRRbnpppppppqr", 0, 1, 75);
    data(m, "BBNPPPPPPPRRbnpppppqrr", 0, 1, 71);
    data(m, "BPPPPPPPQRRbbppppppqrr", 1, 8, 291);
    data(m, "BNPPPPPPQRRbbnpppppqrr", 28, 11, 364);
    data(m, "BBNPPPPPPRRbbnnppppprr", 6, 5, 186);
    data(m, "BPPPPPPPQRRnnppppppqrr", 0, 2, 76);
    data(m, "BNNPPPPPPQRbnppppppqrr", 13, 30, 112);
    data(m, "BPPPPPPPQRRbnppppppqrr", 12, 26, 597);
    data(m, "BNNPPPPPPRRbbnnppppprr", 0, 2, 60);
    data(m, "NPPPPPPPQRRbbppppppqrr", 3, 4, 175);
    data(m, "NPPPPPPPQRRbnppppppqrr", 5, 8, 376);
    data(m, "BNPPPPPPPQRbpppppppqrr", 11, 33, 122);
    data(m, "BBPPPPPPQRRbbnpppppqrr", 1, 8, 236);
    data(m, "BNPPPPPPPQRnpppppppqrr", 13, 22, 91);
    data(m, "BNPPPPPPQRRbnnpppppqrr", 2, 19, 242);
    data(m, "BBNPPPPPPQRbbppppppqrr", 3, 15, 57);
    data(m, "BBNPPPPPPPRbnppppppprr", 6, 17, 49);
    data(m, "BNPPPPPPPRRbbnpppppprr", 1, 4, 403);
    data(m, "BNPPPPPPPRRbnnpppppprr", 2, 4, 219);
    data(m, "BBPPPPPPPRRbbnpppppprr", 2, 6, 185);
    data(m, "NNPPPPPPQRRbbnpppppqrr", 2, 0, 54);
    data(m, "NPPPPPPPQRRnnppppppqrr", 0, 1, 79);
    data(m, "BNPPPPPPPQRbbnppppppqr", 1, 1, 65);
    data(m, "NNPPPPPPPRRbbnpppppprr", 0, 0, 52);
    data(m, "BBPPPPPPPQRbpppppppqrr", 12, 23, 58);
    data(m, "BBNPPPPPPQRbnppppppqrr", 32, 54, 151);
    data(m, "BBPPPPPPQRRbnnpppppqrr", 5, 6, 93);
    data(m, "BBNPPPPPPQRbbnnpppppqr", 2, 3, 61);
    data(m, "BBPPPPPPPRRbnnpppppprr", 0, 3, 79);
    data(m, "BBNNPPPPPQRbnnpppppqrr", 10, 9, 34);
    data(m, "PPPPPPPPQRRbpppppppqrr", 0, 3, 77);
    data(m, "BBNPPPPPQRRbbnnppppqrr", 3, 2, 48);
    data(m, "BBPPPPPPQRRbnppppppqrr", 584, 712, 393);
    data(m, "BPPPPPPPQRRnpppppppqrr", 271, 543, 381);
    data(m, "NPPPPPPPQRRbpppppppqrr", 270, 472, 250);
    data(m, "BBPPPPPPPRRbnppppppprr", 158, 291, 79);
    data(m, "BNPPPPPPQRRbbppppppqrr", 320, 623, 594);
    data(m, "BBPPPPPPQRRnnppppppqrr", 86, 109, 66);
    data(m, "BNPPPPPPPQRbbpppppppqr", 33, 67, 47);
    data(m, "NNPPPPPPPQRbnpppppppqr", 26, 36, 24);
    data(m, "BNPPPPPPQRRnnppppppqrr", 160, 257, 196);
    data(m, "BNPPPPPPPRRnnppppppprr", 37, 104, 35);
    data(m, "BBPPPPPPPRRnnppppppprr", 19, 38, 15);
    data(m, "BBNPPPPPQRRbnnpppppqrr", 157, 174, 136);
    data(m, "BNNPPPPPPRRbbnpppppprr", 182, 410, 296);
    data(m, "BNPPPPPPPQRRbbnppppqrr", 9, 11, 66);
    data(m, "BBNPPPPPPRRbnnpppppprr", 280, 403, 168);
    data(m, "BBNPPPPPPPRbnnpppppppr", 34, 62, 13);
    data(m, "NNPPPPPPQRRbnppppppqrr", 126, 238, 180);
    data(m, "BBNPPPPPPQRbnnppppppqr", 102, 140, 97);
    data(m, "BNPPPPPPPRRbbppppppprr", 58, 218, 126);
    data(m, "NNPPPPPPPRRbnppppppprr", 27, 80, 36);
    data(m, "BPPPPPPPQRRbbnppppppqr", 5, 9, 56);
    data(m, "BNPPPPPPPQRnnpppppppqr", 17, 39, 26);
    data(m, "BBPPPPPPPQRbnpppppppqr", 50, 76, 35);
    data(m, "BNNPPPPPQRRbbnpppppqrr", 128, 190, 286);
    data(m, "BNNPPPPPPQRbbnppppppqr", 70, 125, 98);
    data(m, "BBNPPPPPPQRbpppppppqrr", 57, 14, 6);
    data(m, "NNPPPPPPQRRbbppppppqrr", 54, 89, 106);
    data(m, "BNPPPPPPQRRbbnnpppppqr", 5, 6, 41);
    data(m, "BNNPPPPPPPRbbnpppppppr", 13, 35, 17);
    data(m, "NNPPPPPPPRRbbppppppprr", 15, 34, 21);
    data(m, "BBPPPPPPPQRRbbpppppqrr", 116, 20, 16);
    data(m, "BNNPPPPPPRRbnppppppprr", 140, 5, 6);
    data(m, "BBNPPPPPQRRbnppppppqrr", 495, 22, 18);
    data(m, "NPPPPPPPQRRbnpppppppqr", 77, 20, 3);
    data(m, "BBNPPPPPPPRRbnnppppprr", 108, 15, 9);
    data(m, "BNPPPPPPPQRRbnpppppqrr", 423, 99, 85);
    data(m, "BNNPPPPPPPRRbbnppppprr", 127, 36, 9);
    data(m, "BBPPPPPPQRRnpppppppqrr", 182, 4, 1);
    data(m, "BNNPPPPPQRRbnppppppqrr", 297, 29, 7);
    data(m, "BNPPPPPPPQRRbbpppppqrr", 224, 75, 61);
    data(m, "BBNPPPPPPPRRbbnppppprr", 193, 39, 10);
    data(m, "BNPPPPPPQRRnpppppppqrr", 385, 19, 20);
    data(m, "BBNPPPPPQRRnnppppppqrr", 45, 4, 4);
    data(m, "BBPPPPPPQRRbpppppppqrr", 263, 6, 3);
    data(m, "BNPPPPPPQRRbbnppppppqr", 159, 52, 46);
    data(m, "BNNPPPPPPPRRbnnppppprr", 73, 10, 2);
    data(m, "BNNPPPPPPQRRbbnppppqrr", 95, 45, 28);
    data(m, "BNPPPPPPQRRbpppppppqrr", 555, 17, 12);
    data(m, "BBNNPPPPPRRbnnpppppprr", 76, 2, 1);
    data(m, "BBNPPPPPPQRbnpppppppqr", 104, 5, 1);
    data(m, "BBNNPPPPPRRbbnpppppprr", 143, 11, 6);
    data(m, "BNNPPPPPQRRbbppppppqrr", 65, 11, 8);
    data(m, "BPPPPPPPQRRppppppppqrr", 104, 3, 2);
    data(m, "BBPPPPPPQRRbnnppppppqr", 45, 12, 1);
    data(m, "BBNPPPPPPQRRbnnppppqrr", 54, 13, 13);
    data(m, "BBNPPPPPPRRbbppppppprr", 143, 8, 0);
    data(m, "BBNPPPPPPPQRbbnpppppqr", 36, 13, 3);
    data(m, "NNPPPPPPPQRRbnpppppqrr", 43, 18, 11);
    data(m, "BBPPPPPPPQRRbnpppppqrr", 175, 33, 26);
    data(m, "BBNPPPPPPQRRbbnppppqrr", 206, 51, 54);
    data(m, "BBNPPPPPPRRbnppppppprr", 416, 13, 1);
    data(m, "BPPPPPPPQRRbbpppppppqr", 43, 16, 8);
    data(m, "BPPPPPPPQRRbnpppppppqr", 119, 24, 18);
    data(m, "BNPPPPPPPPRRbnpppppprr", 54, 11, 1);
    data(m, "BBPPPPPPQRRbbnppppppqr", 49, 17, 5);
    data(m, "BBNPPPPPQRRbbppppppqrr", 168, 8, 11);
    data(m, "BBNPPPPPPRRnnppppppprr", 55, 0, 0);
    data(m, "BNPPPPPPQRRbnnppppppqr", 100, 23, 11);
    data(m, "NNPPPPPPPQRRbbpppppqrr", 36, 18, 11);
    data(m, "BBNNPPPPPPRRbbnnpppprr", 77, 11, 2);
    data(m, "BNNPPPPPPQRRbnnppppqrr", 58, 8, 6);
    data(m, "BNPPPPPPPQRRnnpppppqrr", 59, 22, 6);
    data(m, "NNPPPPPPQRRbpppppppqrr", 83, 2, 3);
    data(m, "NPPPPPPPPQRRbppppppqrr", 53, 5, 2);
    data(m, "NNPPPPPPQRRnpppppppqrr", 74, 1, 5);
    data(m, "BPPPPPPPPQRRnppppppqrr", 35, 12, 5);
    data(m, "BPPPPPPPPQRRbppppppqrr", 64, 13, 2);
    data(m, "BNNPPPPPPRRbbppppppprr", 67, 3, 4);
    data(m, "BNPPPPPPPRRbbnpppppppr", 38, 9, 9);
    data(m, "BNPPPPPPPQRRnppppppqrr", 185, 4, 2);
    data(m, "BNNPPPPPPQRRbnpppppqrr", 172, 7, 3);
    data(m, "BBPPPPPPPQRRbppppppqrr", 148, 1, 0);
    data(m, "BBNPPPPPPRRbbnpppppppr", 46, 1, 10);
    data(m, "BNNPPPPPPPRRbbpppppprr", 54, 0, 0);
    data(m, "BBPPPPPPPQRRnppppppqrr", 83, 1, 0);
    data(m, "BBNPPPPPPPRRnnpppppprr", 70, 0, 0);
    data(m, "BBNPPPPPPPRRbnpppppprr", 175, 0, 0);
    data(m, "BNPPPPPPPQRRbbnpppppqr", 50, 8, 2);
    data(m, "BNPPPPPPPQRRbppppppqrr", 257, 3, 10);
    data(m, "BNNPPPPPPPRRbnpppppprr", 127, 0, 0);
    data(m, "BNPPPPPPQRRbnpppppppqr", 157, 2, 3);
    data(m, "BBNNPPPPPPRRbbnppppprr", 106, 2, 1);
    data(m, "BNPPPPPPPQRRbnnpppppqr", 42, 2, 8);
    data(m, "BBNPPPPPPQRRbbpppppqrr", 127, 2, 1);
    data(m, "BBNPPPPPPQRRbnpppppqrr", 246, 6, 10);
    data(m, "BNNPPPPPPQRRbbpppppqrr", 49, 2, 1);
    data(m, "BBNPPPPPQRRbbnppppppqr", 58, 1, 10);
    data(m, "BNPPPPPPPQRbnppppppprr", 56, 0, 1);
    data(m, "BBNPPPPPPPRRbbpppppprr", 90, 1, 5);
    data(m, "BBNPPPPPPQRRbbnpppppqr", 103, 0, 0);
    data(m, "BNPPPPPPQRRbnnpppppprr", 135, 0, 0);
    data(m, "BPPPPPPPQRRbnppppppprr", 189, 0, 0);
    data(m, "BBNPPPPPPPRRbbnppppppr", 64, 1, 1);
    data(m, "BBPPPPPPQRRbbnpppppprr", 97, 1, 0);
    data(m, "BNPPPPPPPQRRbnppppppqr", 106, 0, 0);
    data(m, "BNPPPPPPQRRbbnpppppprr", 254, 1, 0);
    data(m, "BNPPPPPPPQRRbbppppppqr", 86, 0, 0);
    data(m, "BPPPPPPPQRRbbppppppprr", 74, 0, 0);
    data(m, "BBNPPPPPPQRRbppppppqrr", 60, 0, 0);
    data(m, "NPPPPPPPQRRbnppppppprr", 110, 0, 0);
    data(m, "BBNPPPPPPQRRbnnpppppqr", 55, 0, 0);
    data(m, "BBNPPPPPQRRbnnpppppprr", 84, 0, 0);
    data(m, "BNPPPPPPPQRRbbnppppprr", 53, 0, 0);
    data(m, "BBNPPPPPQRRbbnpppppprr", 126, 0, 0);
    data(m, "BNPPPPPPQRRbnppppppprr", 318, 0, 0);
    data(m, "BNNPPPPPQRRbbnpppppprr", 58, 0, 0);
    data(m, "BBPPPPPPQRRbbppppppprr", 82, 0, 0);
    data(m, "BNPPPPPPQRRbbppppppprr", 86, 0, 0);
    data(m, "BBPPPPPPQRRbnppppppprr", 120, 0, 0);
    data(m, "NNPPPPPPQRRbnppppppprr", 50, 0, 0);
    data(m, "BNNPPPPPQRRbnnpppppprr", 58, 0, 0);
    data(m, "BNPPPPPPQRRnnppppppprr", 59, 0, 0);
    data(m, "BNPPPPPPPQRRbbpppppprr", 115, 0, 0);
    data(m, "BBPPPPPPPQRRbbpppppprr", 51, 0, 0);
    data(m, "BNPPPPPPPQRRbnpppppprr", 259, 0, 0);
    data(m, "BBPPPPPPPQRRbnpppppprr", 74, 0, 0);
    data(m, "BBNPPPPPPQRRbbnppppprr", 132, 0, 0);
    data(m, "BNNPPPPPPQRRbbnppppprr", 93, 0, 0);
    data(m, "BBNPPPPPPQRRbnpppppprr", 62, 0, 0);
    data(m, "BBNPPPPPPRRbbnppppqrr", 0, 0, 57);
    data(m, "BNPPPPPPPRRbbpppppqrr", 0, 0, 73);
    data(m, "BNPPPPPPPRRbnpppppqrr", 0, 0, 112);
    data(m, "BBNPPPPPPRRbnpppppqrr", 1, 0, 139);
    data(m, "BBNPPPPPPRRbbpppppqrr", 1, 0, 69);
    data(m, "BNPPPPPPPRRbppppppqrr", 5, 2, 158);
    data(m, "BNNPPPPPPRRbnpppppqrr", 0, 2, 99);
    data(m, "BNPPPPPPPRRnppppppqrr", 1, 0, 106);
    data(m, "BBPPPPPPPRRbppppppqrr", 0, 1, 81);
    data(m, "BNNPPPPPPRRbbnpppppqr", 0, 0, 58);
    data(m, "BNPPPPPPPRRbnppppppqr", 1, 1, 86);
    data(m, "BNPPPPPPPQRbnpppppqrr", 18, 3, 101);
    data(m, "BBNPPPPPPRRbbnpppppqr", 0, 1, 55);
    data(m, "NPPPPPPPQRRbnpppppqrr", 18, 28, 165);
    data(m, "BNPPPPPPPQRnppppppqrr", 36, 39, 79);
    data(m, "BNPPPPPPPRRbnnppppprr", 5, 10, 89);
    data(m, "BNPPPPPPPQRbbnpppppqr", 13, 8, 70);
    data(m, "BBNPPPPPPQRbbpppppqrr", 19, 14, 39);
    data(m, "BBPPPPPPPQRbppppppqrr", 21, 32, 34);
    data(m, "BPPPPPPPQRRbnpppppqrr", 23, 46, 312);
    data(m, "PPPPPPPPQRRbppppppqrr", 3, 10, 56);
    data(m, "BBPPPPPPPRRbbnppppprr", 7, 19, 127);
    data(m, "BBNPPPPPPQRbnpppppqrr", 66, 60, 86);
    data(m, "BNPPPPPPPRRbbnppppprr", 6, 15, 188);
    data(m, "NPPPPPPPQRRbbpppppqrr", 8, 12, 89);
    data(m, "BNPPPPPPPQRbppppppqrr", 45, 83, 124);
    data(m, "BBPPPPPPQRRbbnppppqrr", 7, 10, 79);
    data(m, "BNPPPPPPQRRbnnppppqrr", 11, 14, 92);
    data(m, "BPPPPPPPQRRbbpppppqrr", 7, 18, 280);
    data(m, "BNPPPPPPQRRbbnppppqrr", 31, 26, 143);
    data(m, "BPPPPPPPQRRnnpppppqrr", 5, 5, 44);
    data(m, "BNNPPPPPPQRbnpppppqrr", 40, 29, 77);
    data(m, "BNNPPPPPPQRbbpppppqrr", 15, 10, 26);
    data(m, "BBNPPPPPPPRbnpppppprr", 27, 25, 40);
    data(m, "BNNPPPPPPPRbnpppppprr", 16, 16, 24);
    data(m, "BBPPPPPPQRRbbpppppqrr", 399, 218, 144);
    data(m, "BBNPPPPPPRRbbnppppprr", 612, 436, 124);
    data(m, "BPPPPPPPQRRbppppppqrr", 1037, 627, 219);
    data(m, "BBPPPPPPPRRbbpppppprr", 249, 142, 32);
    data(m, "NPPPPPPPQRRbppppppqrr", 534, 274, 119);
    data(m, "BBPPPPPPQRRbnpppppqrr", 583, 284, 133);
    data(m, "BBNPPPPPQRRbbnppppqrr", 198, 102, 80);
    data(m, "NNPPPPPPQRRbnpppppqrr", 195, 114, 52);
    data(m, "BBNPPPPPPRRbnnppppprr", 297, 99, 32);
    data(m, "BNPPPPPPPRRbnpppppprr", 847, 500, 143);
    data(m, "BNPPPPPPQRRbnpppppqrr", 1365, 866, 509);
    data(m, "BBNPPPPPPQRbppppppqrr", 70, 3, 3);
    data(m, "BNPPPPPPPQRbnppppppqr", 316, 170, 76);
    data(m, "BNPPPPPPQRRbbpppppqrr", 628, 549, 367);
    data(m, "BBPPPPPPPQRbbppppppqr", 103, 51, 13);
    data(m, "NNPPPPPPQRRbbpppppqrr", 106, 64, 51);
    data(m, "BBPPPPPPPRRbnpppppprr", 344, 148, 25);
    data(m, "BBPPPPPPPQRbnppppppqr", 118, 56, 14);
    data(m, "BPPPPPPPQRRnppppppqrr", 559, 348, 157);
    data(m, "BBNPPPPPPQRbbnpppppqr", 269, 150, 87);
    data(m, "BNPPPPPPPRRnnpppppprr", 100, 53, 11);
    data(m, "BBPPPPPPQRRnnpppppqrr", 88, 29, 16);
    data(m, "BNNPPPPPPPRbbnppppppr", 41, 44, 14);
    data(m, "NNPPPPPPPRRbbpppppprr", 35, 37, 17);
    data(m, "NNPPPPPPPRRbnpppppprr", 120, 61, 17);
    data(m, "BNNPPPPPPRRbbnppppprr", 254, 247, 97);
    data(m, "BNNPPPPPPQRbnnpppppqr", 79, 46, 21);
    data(m, "BNPPPPPPPRRbbpppppprr", 251, 302, 92);
    data(m, "BBNPPPPPPQRbnnpppppqr", 111, 55, 24);
    data(m, "NNPPPPPPPRRnnpppppprr", 37, 14, 3);
    data(m, "BBNPPPPPPPRbbnppppppr", 95, 44, 22);
    data(m, "BPPPPPPPPRRbppppppprr", 75, 47, 7);
    data(m, "BBNPPPPPQRRbnnppppqrr", 95, 37, 30);
    data(m, "BNPPPPPPQRRnnpppppqrr", 212, 134, 84);
    data(m, "BNPPPPPPPQRbbppppppqr", 122, 91, 30);
    data(m, "NPPPPPPPQRRnppppppqrr", 408, 228, 94);
    data(m, "BNNPPPPPQRRbnnppppqrr", 74, 34, 14);
    data(m, "BNNPPPPPQRRbbnppppqrr", 122, 82, 49);
    data(m, "BBNNPPPPPRRbbnnpppprr", 48, 42, 6);
    data(m, "BNNPPPPPPRRbnnppppprr", 170, 116, 25);
    data(m, "BNNPPPPPPQRbbnpppppqr", 129, 84, 37);
    data(m, "PPPPPPPPQRRpppppppqrr", 67, 46, 20);
    data(m, "BBPPPPPPPRRnnpppppprr", 47, 27, 6);
    data(m, "NNPPPPPPPQRbnppppppqr", 50, 18, 6);
    data(m, "BBNNPPPPPQRbbnnppppqr", 34, 19, 10);
    data(m, "BBNPPPPPPPRbnnppppppr", 36, 16, 5);
    data(m, "BNPPPPPPPQRnnppppppqr", 48, 19, 13);
    data(m, "BPPPPPPPQRRbbnpppppqr", 17, 17, 29);
    data(m, "BBNNPPPPPPRbbnnpppppr", 50, 29, 9);
    data(m, "BNNPPPPPPPRbnnppppppr", 34, 19, 6);
    data(m, "NNPPPPPPQRRnnpppppqrr", 57, 30, 10);
    data(m, "BNPPPPPPQRRbppppppqrr", 896, 21, 15);
    data(m, "BNPPPPPPPRRbppppppprr", 226, 1, 2);
    data(m, "BNNPPPPPQRRbbpppppqrr", 48, 1, 3);
    data(m, "BBNPPPPPPRRbnpppppprr", 535, 6, 4);
    data(m, "BBPPPPPPQRRbppppppqrr", 330, 14, 12);
    data(m, "BNNPPPPPPRRbnpppppprr", 367, 9, 1);
    data(m, "BNPPPPPPPQRnpppppppqr", 67, 3, 1);
    data(m, "BBNPPPPPPRRnnpppppprr", 94, 1, 0);
    data(m, "BPPPPPPPQRRbbppppppqr", 91, 15, 3);
    data(m, "BBNPPPPPPQRbbppppppqr", 87, 0, 0);
    data(m, "BNPPPPPPQRRbnnpppppqr", 85, 9, 7);
    data(m, "BPPPPPPPQRRpppppppqrr", 323, 8, 5);
    data(m, "BNPPPPPPPRRbbnppppppr", 81, 9, 4);
    data(m, "BPPPPPPPQRRbnppppppqr", 225, 6, 9);
    data(m, "NPPPPPPPQRRbnppppppqr", 102, 8, 6);
    data(m, "BBNPPPPPPQRbnppppppqr", 186, 4, 4);
    data(m, "BBNPPPPPPRRbbpppppprr", 229, 10, 2);
    data(m, "BBNPPPPPQRRbnpppppqrr", 285, 15, 15);
    data(m, "BNPPPPPPQRRnppppppqrr", 529, 19, 7);
    data(m, "NPPPPPPPQRRpppppppqrr", 160, 2, 2);
    data(m, "BNPPPPPPQRRbbnpppppqr", 212, 29, 37);
    data(m, "BNNPPPPPPRRbbpppppprr", 83, 5, 0);
    data(m, "BBNPPPPPQRRbbpppppqrr", 160, 6, 9);
    data(m, "BBPPPPPPPQRbpppppppqr", 69, 4, 0);
    data(m, "BBPPPPPPQRRnppppppqrr", 204, 5, 4);
    data(m, "BBNPPPPPPPRbnpppppppr", 59, 1, 0);
    data(m, "BNPPPPPPPQRbpppppppqr", 68, 1, 2);
    data(m, "BBPPPPPPPRRbppppppprr", 132, 0, 0);
    data(m, "BNPPPPPPPRRnppppppprr", 228, 2, 1);
    data(m, "BNNPPPPPQRRbnpppppqrr", 185, 12, 10);
    data(m, "BNNPPPPPPRRnnpppppprr", 63, 1, 0);
    data(m, "BBNNPPPPPRRbbnppppprr", 92, 4, 0);
    data(m, "NNPPPPPPQRRbppppppqrr", 97, 2, 3);
    data(m, "NPPPPPPPQRRbbppppppqr", 61, 2, 4);
    data(m, "BNNPPPPPPQRbnppppppqr", 124, 1, 2);
    data(m, "BBPPPPPPPRRnppppppprr", 62, 1, 0);
    data(m, "NNPPPPPPQRRnppppppqrr", 87, 2, 0);
    data(m, "BBPPPPPPQRRbbnpppppqr", 66, 4, 6);
    data(m, "BNPPPPPPPRRbnnppppppr", 53, 3, 0);
    data(m, "BNPPPPPPPQRRbnppppqrr", 41, 5, 4);
    data(m, "BBNNPPPPPRRbnnppppprr", 60, 0, 0);
    data(m, "BNPPPPPPQRRbbppppppqr", 107, 3, 2);
    data(m, "BNPPPPPPPQRbnpppppprr", 87, 0, 0);
    data(m, "BPPPPPPPQRRbpppppppqr", 82, 2, 1);
    data(m, "BNNPPPPPPRRbbnppppppr", 72, 0, 2);
    data(m, "BNPPPPPPPRRbnpppppppr", 112, 0, 0);
    data(m, "BBNPPPPPPRRbbnppppppr", 139, 0, 1);
    data(m, "BNPPPPPPQRRbnppppppqr", 313, 3, 8);
    data(m, "BBNPPPPPQRRbbnpppppqr", 58, 0, 10);
    data(m, "BBPPPPPPQRRbnppppppqr", 123, 1, 2);
    data(m, "BNNPPPPPQRRbbnpppppqr", 48, 3, 2);
    data(m, "BBNPPPPPPRRbnnppppppr", 57, 0, 0);
    data(m, "NPPPPPPPQRRbpppppppqr", 53, 0, 1);
    data(m, "BNNPPPPPPQRbbnppppprr", 52, 0, 0);
    data(m, "BPPPPPPPQRRnpppppppqr", 60, 1, 0);
    data(m, "BPPPPPPPQRRbnpppppprr", 221, 0, 0);
    data(m, "BPPPPPPPQRRbbpppppprr", 129, 0, 0);
    data(m, "NPPPPPPPQRRbbpppppprr", 52, 0, 0);
    data(m, "BNPPPPPPQRRbbnppppprr", 200, 0, 0);
    data(m, "NPPPPPPPQRRbnpppppprr", 100, 0, 0);
    data(m, "BNPPPPPPQRRbnnppppprr", 77, 0, 0);
    data(m, "BBPPPPPPQRRbbnppppprr", 74, 0, 0);
    data(m, "BNPPPPPPQRRbnpppppprr", 464, 0, 1);
    data(m, "BNPPPPPPQRRbbpppppprr", 190, 0, 0);
    data(m, "BBPPPPPPQRRbbpppppprr", 88, 0, 0);
    data(m, "BPPPPPPPQRRbppppppprr", 211, 0, 0);
    data(m, "BPPPPPPPQRRnppppppprr", 109, 0, 0);
    data(m, "NPPPPPPPQRRbppppppprr", 91, 0, 0);
    data(m, "BBNPPPPPPQRbbnppppppr", 58, 0, 0);
    data(m, "NPPPPPPPQRRnppppppprr", 110, 0, 0);
    data(m, "BNNPPPPPQRRbbnppppprr", 93, 0, 2);
    data(m, "BBNPPPPPQRRbbnppppprr", 110, 0, 0);
    data(m, "BBPPPPPPQRRbnpppppprr", 140, 0, 0);
    data(m, "NNPPPPPPQRRbnpppppprr", 60, 0, 0);
    data(m, "BNPPPPPPQRRnnpppppprr", 76, 0, 0);
    data(m, "BPPPPPPPRRbppppppqrr", 0, 0, 131);
    data(m, "BPPPPPPPRRnppppppqrr", 0, 0, 51);
    data(m, "BNPPPPPPRRbnpppppqrr", 0, 0, 158);
    data(m, "NPPPPPPPRRnppppppqrr", 0, 0, 56);
    data(m, "NPPPPPPPRRbppppppqrr", 0, 0, 83);
    data(m, "BNPPPPPPRRbbpppppqrr", 0, 0, 58);
    data(m, "BBPPPPPPRRbnpppppqrr", 0, 0, 50);
    data(m, "BBPPPPPPRRbppppppqrr", 0, 0, 76);
    data(m, "BNPPPPPPRRbppppppqrr", 1, 0, 144);
    data(m, "BPPPPPPPRRpppppppqrr", 0, 0, 69);
    data(m, "BNPPPPPPRRnppppppqrr", 0, 0, 89);
    data(m, "BBPPPPPPRRnppppppqrr", 0, 0, 54);
    data(m, "BBNPPPPPRRbnpppppqrr", 0, 0, 64);
    data(m, "BPPPPPPPQRbppppppqrr", 0, 1, 94);
    data(m, "NPPPPPPPQRbppppppqrr", 3, 2, 62);
    data(m, "BNPPPPPPQRbnpppppqrr", 1, 2, 142);
    data(m, "BNPPPPPPRRbnppppppqr", 0, 2, 112);
    data(m, "BNPPPPPPPRbnpppppprr", 0, 0, 72);
    data(m, "BBNPPPPPQRbnpppppqrr", 22, 24, 75);
    data(m, "BPPPPPPPQRbnppppppqr", 1, 4, 95);
    data(m, "BNPPPPPPRRbbnppppprr", 0, 5, 186);
    data(m, "BNPPPPPPRRbnnppppprr", 1, 12, 71);
    data(m, "BNPPPPPPQRbppppppqrr", 12, 30, 185);
    data(m, "NPPPPPPQRRbnpppppqrr", 7, 16, 200);
    data(m, "BPPPPPPQRRbbpppppqrr", 4, 8, 155);
    data(m, "BNNPPPPPPRbnpppppprr", 3, 6, 45);
    data(m, "BNPPPPPPPRbbnppppppr", 0, 4, 51);
    data(m, "BNPPPPPPQRbbnpppppqr", 5, 4, 144);
    data(m, "BPPPPPPQRRbnpppppqrr", 12, 19, 327);
    data(m, "BPPPPPPPQRbbppppppqr", 1, 1, 65);
    data(m, "BNPPPPPPQRbnnpppppqr", 1, 7, 80);
    data(m, "BPPPPPPPRRbnpppppprr", 0, 5, 199);
    data(m, "PPPPPPPQRRnppppppqrr", 2, 10, 138);
    data(m, "BPPPPPPPRRbbpppppprr", 1, 1, 101);
    data(m, "NPPPPPPQRRbbpppppqrr", 2, 4, 86);
    data(m, "PPPPPPPQRRbppppppqrr", 5, 5, 228);
    data(m, "BBPPPPPPRRbbnppppprr", 0, 5, 95);
    data(m, "BNPPPPPQRRbbnppppqrr", 6, 2, 59);
    data(m, "BNNPPPPPQRbnpppppqrr", 7, 10, 41);
    data(m, "NPPPPPPQRRnnpppppqrr", 6, 4, 42);
    data(m, "BBPPPPPPQRnppppppqrr", 7, 15, 40);
    data(m, "BBNPPPPPPRbnpppppprr", 11, 23, 78);
    data(m, "BNPPPPPPQRnppppppqrr", 6, 18, 119);
    data(m, "NPPPPPPPRRbbpppppprr", 0, 1, 62);
    data(m, "NPPPPPPPRRbnpppppprr", 1, 4, 144);
    data(m, "NPPPPPPPQRbnppppppqr", 0, 5, 51);
    data(m, "BPPPPPPPQRpppppppqrr", 1, 5, 71);
    data(m, "BNPPPPPQRRbnnppppqrr", 4, 3, 59);
    data(m, "NPPPPPPPQRpppppppqrr", 2, 11, 48);
    data(m, "BBPPPPPPQRbppppppqrr", 17, 23, 72);
    data(m, "BNPPPPPPQRbbppppppqr", 84, 180, 136);
    data(m, "NNPPPPPPRRbnpppppprr", 64, 109, 47);
    data(m, "BBPPPPPPRRbnpppppprr", 189, 323, 97);
    data(m, "BPPPPPPPRRnppppppprr", 52, 153, 78);
    data(m, "BNNPPPPPQRbbnpppppqr", 37, 72, 60);
    data(m, "BBNPPPPPPRbnnppppppr", 49, 65, 25);
    data(m, "BNPPPPPPRRnnpppppprr", 46, 119, 49);
    data(m, "NPPPPPPQRRbppppppqrr", 249, 384, 300);
    data(m, "NNPPPPPPQRbbppppppqr", 12, 34, 10);
    data(m, "BPPPPPPQRRnppppppqrr", 247, 467, 281);
    data(m, "NPPPPPPPRRbppppppprr", 78, 173, 54);
    data(m, "BBPPPPPPPRbnpppppppr", 12, 58, 18);
    data(m, "PPPPPPPQRRbnppppppqr", 5, 10, 49);
    data(m, "BNNPPPPQRRbbnppppqrr", 21, 14, 22);
    data(m, "BBPPPPPPQRbnppppppqr", 138, 174, 95);
    data(m, "BBPPPPPQRRbnpppppqrr", 235, 175, 127);
    data(m, "BNNPPPPPPRbbnppppppr", 29, 85, 36);
    data(m, "BNPPPPPPRRbbpppppprr", 88, 314, 232);
    data(m, "BBPPPPPQRRnnpppppqrr", 35, 24, 16);
    data(m, "BPPPPPPPQRRbnppppqrr", 17, 13, 49);
    data(m, "BNPPPPPQRRbbpppppqrr", 121, 189, 213);
    data(m, "NNPPPPPQRRbnpppppqrr", 59, 63, 57);
    data(m, "NNPPPPPPQRbnppppppqr", 41, 86, 37);
    data(m, "BBNPPPPPQRbnnpppppqr", 57, 76, 29);
    data(m, "BBNPPPPPRRbnnppppprr", 77, 101, 49);
    data(m, "BPPPPPPPQRnpppppppqr", 39, 92, 43);
    data(m, "BBPPPPPPRRnnpppppprr", 34, 49, 28);
    data(m, "BNNPPPPPRRbbnppppprr", 40, 93, 64);
    data(m, "BNPPPPPPQRnnppppppqr", 40, 86, 46);
    data(m, "BNPPPPPQRRnnpppppqrr", 37, 43, 54);
    data(m, "BBNPPPPPQRbppppppqrr", 50, 2, 5);
    data(m, "BNPPPPPPPRbbpppppppr", 8, 42, 24);
    data(m, "NNPPPPPPRRbbpppppprr", 11, 49, 40);
    data(m, "NPPPPPPPQRbpppppppqr", 49, 81, 38);
    data(m, "NNPPPPPQRRbbpppppqrr", 18, 18, 16);
    data(m, "BNPPPPPPQRpppppppqrr", 40, 12, 10);
    data(m, "BBNPPPPQRRbnnppppqrr", 24, 17, 12);
    data(m, "BBPPPPPPQRnnppppppqr", 17, 26, 18);
    data(m, "BNPPPPPPPQRbpppppqrr", 22, 17, 21);
    data(m, "NPPPPPPQRRbnppppppqr", 114, 27, 14);
    data(m, "BNPPPPPPQRnpppppppqr", 64, 0, 4);
    data(m, "NPPPPPPQRRpppppppqrr", 125, 4, 1);
    data(m, "BNPPPPPQRRbppppppqrr", 354, 24, 12);
    data(m, "BBPPPPPPQRRbbppppqrr", 53, 21, 16);
    data(m, "BBPPPPPPRRbppppppprr", 120, 0, 1);
    data(m, "BNPPPPPPQRRbnppppqrr", 285, 53, 53);
    data(m, "BPPPPPPPQRRbpppppqrr", 228, 53, 29);
    data(m, "BNPPPPPPQRRbbppppqrr", 127, 30, 33);
    data(m, "BNNPPPPPRRbnpppppprr", 143, 5, 3);
    data(m, "BNPPPPPPPRRbbppppprr", 83, 26, 9);
    data(m, "BPPPPPPPQRRnpppppqrr", 139, 30, 22);
    data(m, "NPPPPPPPQRRbpppppqrr", 121, 31, 15);
    data(m, "BNPPPPPPRRbppppppprr", 194, 4, 2);
    data(m, "BBNPPPPPPRRbbnpppprr", 91, 14, 10);
    data(m, "BBPPPPPQRRbppppppqrr", 179, 6, 2);
    data(m, "BBPPPPPPRRnppppppprr", 51, 0, 0);
    data(m, "BBNPPPPPPQRbbnppppqr", 49, 5, 6);
    data(m, "BBNPPPPPRRbbpppppprr", 57, 3, 1);
    data(m, "BNPPPPPQRRbbnpppppqr", 72, 25, 22);
    data(m, "BBNPPPPPRRbnpppppprr", 231, 8, 1);
    data(m, "BNPPPPPQRRnppppppqrr", 178, 12, 14);
    data(m, "BNPPPPPPPQRbnpppppqr", 64, 14, 7);
    data(m, "BPPPPPPQRRpppppppqrr", 256, 7, 5);
    data(m, "BNPPPPPQRRbnnpppppqr", 46, 9, 7);
    data(m, "BBNPPPPPQRbnppppppqr", 110, 7, 4);
    data(m, "BBPPPPPPQRRbnppppqrr", 70, 12, 18);
    data(m, "BBPPPPPPPRRbbppppprr", 59, 7, 2);
    data(m, "BNPPPPPPRRbbnppppppr", 63, 20, 6);
    data(m, "PPPPPPPQRRbpppppppqr", 64, 12, 5);
    data(m, "BPPPPPPQRRbbppppppqr", 77, 45, 10);
    data(m, "BBPPPPPPPRRbnppppprr", 81, 14, 2);
    data(m, "PPPPPPPQRRnpppppppqr", 37, 17, 1);
    data(m, "BNPPPPPPRRnppppppprr", 150, 2, 2);
    data(m, "BNPPPPPPPRRbnppppprr", 230, 35, 5);
    data(m, "NPPPPPPQRRbbppppppqr", 41, 14, 4);
    data(m, "BNPPPPPPRRbnnppppppr", 49, 7, 5);
    data(m, "BPPPPPPQRRbnppppppqr", 186, 53, 19);
    data(m, "NPPPPPPPQRRnpppppqrr", 89, 17, 12);
    data(m, "BBNPPPPPPRbnpppppppr", 55, 1, 0);
    data(m, "BNNPPPPPPRRbbnpppprr", 63, 20, 6);
    data(m, "BBNPPPPQRRbnpppppqrr", 68, 9, 5);
    data(m, "BNPPPPPPPQRbbpppppqr", 45, 10, 5);
    data(m, "BPPPPPPPRRbnpppppppr", 59, 13, 3);
    data(m, "BNNPPPPPQRbnppppppqr", 47, 3, 7);
    data(m, "BNPPPPPPQRbpppppppqr", 98, 4, 2);
    data(m, "BBNPPPPPPRRbnnpppprr", 48, 5, 3);
    data(m, "BBPPPPPQRRnppppppqrr", 70, 2, 4);
    data(m, "BNPPPPPPQRRbpppppqrr", 254, 1, 5);
    data(m, "BNPPPPPPPRRbpppppprr", 124, 1, 0);
    data(m, "BBPPPPPPQRRbpppppqrr", 98, 3, 0);
    data(m, "NPPPPPPPQRRppppppqrr", 75, 3, 0);
    data(m, "BBNPPPPPPRRbbppppprr", 58, 2, 0);
    data(m, "BBPPPPPPPRRbpppppprr", 59, 0, 0);
    data(m, "BNPPPPPPRRbnpppppppr", 95, 0, 0);
    data(m, "BNPPPPPPQRRnpppppqrr", 135, 8, 8);
    data(m, "BBNPPPPPRRbbnppppppr", 58, 0, 0);
    data(m, "BBNPPPPPPRRbnppppprr", 138, 2, 1);
    data(m, "BPPPPPPPQRRppppppqrr", 154, 2, 0);
    data(m, "BNPPPPPPQRbnpppppprr", 70, 0, 0);
    data(m, "NPPPPPPQRRbpppppppqr", 51, 0, 0);
    data(m, "BNPPPPPQRRbnppppppqr", 114, 0, 6);
    data(m, "BBPPPPPQRRbnppppppqr", 57, 1, 1);
    data(m, "BPPPPPPQRRbpppppppqr", 85, 3, 2);
    data(m, "BPPPPPPQRRnpppppppqr", 59, 0, 0);
    data(m, "BNPPPPPPQRRbbnppppqr", 47, 2, 4);
    data(m, "BPPPPPPPQRRbnpppppqr", 45, 5, 2);
    data(m, "BNPPPPPPPRRnpppppprr", 111, 1, 0);
    data(m, "BNNPPPPPPRRbnppppprr", 63, 1, 0);
    data(m, "BBPPPPPPQRRnpppppqrr", 49, 0, 1);
    data(m, "BPPPPPPQRRbnpppppprr", 149, 0, 2);
    data(m, "PPPPPPPQRRbppppppprr", 66, 0, 0);
    data(m, "BNPPPPPPQRRbbpppppqr", 58, 2, 3);
    data(m, "NPPPPPPQRRbnpppppprr", 79, 0, 0);
    data(m, "BPPPPPPPQRRbppppppqr", 65, 0, 1);
    data(m, "BPPPPPPQRRbbpppppprr", 61, 0, 2);
    data(m, "BNPPPPPQRRbbnppppprr", 50, 0, 5);
    data(m, "BNPPPPPPPRRbnppppppr", 50, 0, 0);
    data(m, "BNPPPPPPQRRbnpppppqr", 94, 1, 2);
    data(m, "BPPPPPPQRRbppppppprr", 121, 0, 0);
    data(m, "BBPPPPPQRRbnpppppprr", 80, 0, 0);
    data(m, "BNPPPPPQRRbnpppppprr", 178, 0, 0);
    data(m, "NPPPPPPQRRnppppppprr", 57, 0, 0);
    data(m, "BNPPPPPQRRbbpppppprr", 51, 0, 0);
    data(m, "BPPPPPPPQRRbnppppprr", 55, 0, 0);
    data(m, "BPPPPPPQRRnppppppprr", 78, 0, 0);
    data(m, "BNPPPPPPQRRbnppppprr", 184, 0, 1);
    data(m, "BPPPPPPPQRRbpppppprr", 125, 0, 0);
    data(m, "BPPPPPPPQRRnpppppprr", 83, 0, 0);
    data(m, "BNPPPPPPQRRbbppppprr", 77, 0, 0);
    data(m, "BBPPPPPPQRRbnppppprr", 56, 0, 0);
    data(m, "NPPPPPPPRRbpppppqrr", 0, 0, 56);
    data(m, "BPPPPPPPRRbpppppqrr", 0, 0, 76);
    data(m, "BNPPPPPPRRbnppppqrr", 0, 0, 86);
    data(m, "BPPPPPPPRRppppppqrr", 0, 1, 82);
    data(m, "BNPPPPPPRRbpppppqrr", 1, 0, 112);
    data(m, "BBPPPPPPRRbpppppqrr", 0, 0, 59);
    data(m, "BNPPPPPPRRnpppppqrr", 0, 0, 84);
    data(m, "BNPPPPPPRRbnpppppqr", 0, 3, 110);
    data(m, "BNPPPPPPPRbnppppprr", 1, 0, 73);
    data(m, "BNPPPPPPQRbnppppqrr", 2, 2, 78);
    data(m, "BPPPPPPPQRbpppppqrr", 1, 1, 70);
    data(m, "BPPPPPPPRRbppppppqr", 0, 1, 55);
    data(m, "BPPPPPPQRRbnppppqrr", 25, 33, 161);
    data(m, "BNPPPPPPQRbpppppqrr", 52, 88, 187);
    data(m, "NPPPPPPPRRbnppppprr", 5, 17, 80);
    data(m, "BPPPPPPPQRppppppqrr", 24, 36, 70);
    data(m, "PPPPPPPQRRnpppppqrr", 7, 20, 109);
    data(m, "NPPPPPPQRRbbppppqrr", 3, 9, 67);
    data(m, "BBNPPPPPQRbnppppqrr", 19, 22, 38);
    data(m, "BPPPPPPPQRbnpppppqr", 11, 9, 75);
    data(m, "NPPPPPPPQRppppppqrr", 15, 37, 54);
    data(m, "BPPPPPPQRRbbppppqrr", 17, 10, 85);
    data(m, "BNPPPPPPPRbpppppprr", 18, 44, 68);
    data(m, "BNPPPPPPQRnpppppqrr", 37, 39, 134);
    data(m, "BPPPPPPPRRbnppppprr", 12, 15, 172);
    data(m, "BBPPPPPPQRbpppppqrr", 32, 50, 82);
    data(m, "BBNPPPPPPRbnppppprr", 25, 54, 54);
    data(m, "BNPPPPPPQRbbnppppqr", 7, 7, 52);
    data(m, "NPPPPPPPQRbnpppppqr", 6, 9, 35);
    data(m, "BBPPPPPPPRbpppppprr", 12, 24, 28);
    data(m, "BBPPPPPPRRbbnpppprr", 3, 7, 61);
    data(m, "NPPPPPPQRRbnppppqrr", 17, 19, 101);
    data(m, "PPPPPPPQRRbpppppqrr", 16, 21, 204);
    data(m, "BNNPPPPPPRbnppppprr", 15, 16, 40);
    data(m, "NPPPPPPPRRbbppppprr", 0, 2, 59);
    data(m, "BNPPPPPPPRbbnpppppr", 4, 7, 39);
    data(m, "BNPPPPPPRRbbnpppprr", 4, 6, 110);
    data(m, "BPPPPPPPRRbbppppprr", 3, 9, 84);
    data(m, "BBPPPPPPQRnpppppqrr", 21, 30, 29);
    data(m, "BNPPPPPPPRnpppppprr", 10, 15, 44);
    data(m, "BPPPPPPPQRbbpppppqr", 1, 8, 57);
    data(m, "NPPPPPPQRRnpppppqrr", 363, 197, 100);
    data(m, "BBPPPPPQRRbbppppqrr", 74, 47, 43);
    data(m, "BNPPPPPPPRbnppppppr", 226, 131, 44);
    data(m, "BBNPPPPPQRbbnppppqr", 88, 49, 35);
    data(m, "NPPPPPPPRRbpppppprr", 269, 127, 40);
    data(m, "BNPPPPPPQRbnpppppqr", 505, 278, 138);
    data(m, "NNPPPPPPRRbnppppprr", 116, 77, 30);
    data(m, "BNPPPPPPQRbbpppppqr", 225, 174, 102);
    data(m, "PPPPPPPQRRppppppqrr", 352, 227, 58);
    data(m, "BBPPPPPQRRbnppppqrr", 156, 48, 47);
    data(m, "BNPPPPPPQRnnpppppqr", 53, 38, 25);
    data(m, "BPPPPPPPRRnpppppprr", 270, 139, 43);
    data(m, "BNPPPPPQRRbnppppqrr", 391, 214, 161);
    data(m, "BBNPPPPPPRbbnpppppr", 144, 90, 25);
    data(m, "NPPPPPPQRRbpppppqrr", 486, 280, 126);
    data(m, "BPPPPPPQRRbpppppqrr", 796, 578, 259);
    data(m, "BBPPPPPPRRbnppppprr", 283, 147, 20);
    data(m, "BNPPPPPPRRbnppppprr", 807, 488, 141);
    data(m, "NPPPPPPPRRnpppppprr", 213, 115, 26);
    data(m, "BPPPPPPQRRnpppppqrr", 452, 267, 145);
    data(m, "BNPPPPPPRRbbppppprr", 229, 314, 104);
    data(m, "BPPPPPPPRRbpppppprr", 426, 298, 43);
    data(m, "NNPPPPPPRRbbppppprr", 40, 39, 23);
    data(m, "BBPPPPPPQRbbpppppqr", 136, 78, 44);
    data(m, "BNNPPPPPPRbbnpppppr", 66, 77, 26);
    data(m, "BPPPPPPPQRbppppppqr", 230, 160, 68);
    data(m, "BBPPPPPPQRbnpppppqr", 204, 92, 29);
    data(m, "BBNPPPPPRRbbnpppprr", 121, 80, 46);
    data(m, "BBPPPPPPPRbnppppppr", 72, 59, 8);
    data(m, "NPPPPPPPQRbppppppqr", 150, 52, 22);
    data(m, "BPPPPPPQRRbbnppppqr", 19, 18, 22);
    data(m, "BNPPPPPQRRbbppppqrr", 153, 127, 107);
    data(m, "BNPPPPPQRRnnppppqrr", 44, 25, 18);
    data(m, "BNNPPPPPQRbbnppppqr", 61, 37, 20);
    data(m, "BBNPPPPPQRbnnppppqr", 50, 14, 19);
    data(m, "BNNPPPPPRRbbnpppprr", 55, 62, 20);
    data(m, "BBPPPPPPRRbbppppprr", 183, 147, 41);
    data(m, "BBNPPPPPPRbnnpppppr", 59, 23, 6);
    data(m, "BNPPPPPPRRnnppppprr", 122, 61, 21);
    data(m, "BBNPPPPPQRbpppppqrr", 60, 9, 4);
    data(m, "BNPPPPPPQRppppppqrr", 84, 8, 4);
    data(m, "BBNPPPPPPQbbnpppppq", 33, 19, 3);
    data(m, "BNNPPPPPPRbnnpppppr", 41, 25, 6);
    data(m, "NNPPPPPQRRbnppppqrr", 50, 37, 42);
    data(m, "BPPPPPPPQRnppppppqr", 110, 68, 25);
    data(m, "BBPPPPPPPRbbppppppr", 57, 48, 6);
    data(m, "BBPPPPPPQRnnpppppqr", 37, 11, 8);
    data(m, "NPPPPPPPQRnppppppqr", 84, 55, 17);
    data(m, "BNPPPPPPPRbbppppppr", 70, 83, 17);
    data(m, "BNNPPPPPQRbnnppppqr", 31, 12, 7);
    data(m, "BBNPPPPPRRbnnpppprr", 58, 20, 12);
    data(m, "NNPPPPPPQRbbpppppqr", 21, 25, 13);
    data(m, "BBPPPPPPRRnnppppprr", 56, 15, 4);
    data(m, "NNPPPPPPQRbnpppppqr", 82, 42, 17);
    data(m, "PPPPPPPQRRbnpppppqr", 27, 12, 17);
    data(m, "BNNPPPPPRRbnnpppprr", 41, 29, 12);
    data(m, "BNPPPPPPPQbnppppppq", 48, 21, 4);
    data(m, "BBPPPPPPRRbpppppprr", 201, 0, 1);
    data(m, "BPPPPPPQRRbnpppppqr", 220, 23, 17);
    data(m, "BNPPPPPPQRbppppppqr", 245, 6, 3);
    data(m, "BBNPPPPPQRbnpppppqr", 123, 4, 5);
    data(m, "BBNPPPPPPRbbppppppr", 56, 3, 2);
    data(m, "NPPPPPPQRRppppppqrr", 241, 7, 7);
    data(m, "PPPPPPPQRRnppppppqr", 69, 7, 2);
    data(m, "BPPPPPPQRRbbpppppqr", 113, 14, 17);
    data(m, "BNPPPPPPRRbpppppprr", 456, 8, 2);
    data(m, "BPPPPPPPRRbnppppppr", 91, 3, 0);
    data(m, "BNPPPPPPRRnpppppprr", 329, 2, 0);
    data(m, "BBNPPPPPPRbnppppppr", 140, 2, 0);
    data(m, "BNPPPPPQRRbpppppqrr", 347, 12, 17);
    data(m, "BNPPPPPPRRbbnpppppr", 116, 13, 2);
    data(m, "BBNPPPPPRRbnppppprr", 195, 6, 0);
    data(m, "BNPPPPPQRRnpppppqrr", 204, 4, 18);
    data(m, "BNNPPPPPPRbnppppppr", 81, 3, 1);
    data(m, "BPPPPPPQRRppppppqrr", 398, 14, 5);
    data(m, "BBPPPPPQRRbpppppqrr", 126, 2, 5);
    data(m, "BNPPPPPPQRnppppppqr", 199, 4, 9);
    data(m, "PPPPPPPQRRbppppppqr", 118, 0, 7);
    data(m, "NPPPPPPQRRbbpppppqr", 51, 1, 3);
    data(m, "BBPPPPPPQRbppppppqr", 109, 1, 0);
    data(m, "NPPPPPPQRRbnpppppqr", 124, 8, 3);
    data(m, "BNNPPPPPRRbnppppprr", 101, 6, 1);
    data(m, "NPPPPPPPRRppppppprr", 61, 1, 0);
    data(m, "NPPPPPPPRRbnppppppr", 53, 3, 0);
    data(m, "BBNPPPPPQRbbpppppqr", 58, 2, 2);
    data(m, "BBPPPPPQRRnpppppqrr", 72, 3, 1);
    data(m, "BBPPPPPPRRbbnpppppr", 60, 2, 1);
    data(m, "BBNPPPPPRRbbppppprr", 80, 3, 2);
    data(m, "BNPPPPPPPRnpppppppr", 72, 2, 0);
    data(m, "BNNPPPPPQRbnpppppqr", 84, 3, 2);
    data(m, "BPPPPPPPRRppppppprr", 119, 1, 0);
    data(m, "BNPPPPPQRRbbnppppqr", 54, 10, 7);
    data(m, "BBPPPPPPQRnppppppqr", 55, 4, 1);
    data(m, "BPPPPPPPQRpppppppqr", 95, 0, 1);
    data(m, "BBPPPPPPPRbpppppppr", 52, 0, 0);
    data(m, "BNPPPPPPPRbpppppppr", 59, 0, 0);
    data(m, "BBPPPPPPRRnpppppprr", 120, 1, 0);
    data(m, "BPPPPPPPRRbbppppppr", 55, 3, 1);
    data(m, "BBPPPPPPRRbbppppppr", 65, 0, 0);
    data(m, "BNPPPPPPQRbnppppppq", 71, 2, 0);
    data(m, "BPPPPPPQRRnppppppqr", 131, 2, 3);
    data(m, "NPPPPPPQRRnppppppqr", 80, 0, 0);
    data(m, "BPPPPPPQRRbppppppqr", 189, 0, 4);
    data(m, "BNPPPPPPRRbnppppppr", 205, 1, 0);
    data(m, "BPPPPPPPRRbpppppppr", 66, 1, 0);
    data(m, "NPPPPPPQRRbppppppqr", 107, 1, 1);
    data(m, "BNPPPPPPRRbbppppppr", 99, 0, 0);
    data(m, "BNPPPPPQRRbbpppppqr", 47, 1, 2);
    data(m, "BNPPPPPQRRbnpppppqr", 141, 2, 5);
    data(m, "BBNPPPPPRRbbnpppppr", 76, 5, 0);
    data(m, "BNPPPPPPQRbnppppprr", 94, 1, 5);
    data(m, "BBPPPPPPRRbnppppppr", 74, 0, 0);
    data(m, "BBPPPPPQRRbnpppppqr", 52, 3, 1);
    data(m, "BPPPPPPQRRbbppppprr", 80, 0, 0);
    data(m, "PPPPPPPQRRbpppppprr", 92, 0, 0);
    data(m, "BNPPPPPPQRbpppppprr", 57, 0, 0);
    data(m, "BPPPPPPQRRbnppppprr", 138, 0, 0);
    data(m, "NPPPPPPQRRbnppppprr", 79, 0, 0);
    data(m, "NPPPPPPQRRbbppppprr", 64, 1, 0);
    data(m, "PPPPPPPQRRnpppppprr", 50, 0, 0);
    data(m, "NPPPPPPQRRnpppppprr", 132, 0, 0);
    data(m, "BBPPPPPQRRbnppppprr", 71, 0, 0);
    data(m, "BNPPPPPQRRbnppppprr", 189, 1, 0);
    data(m, "BPPPPPPQRRbpppppprr", 337, 0, 0);
    data(m, "BPPPPPPPQRbpppppppr", 50, 0, 1);
    data(m, "BPPPPPPQRRnpppppprr", 190, 0, 0);
    data(m, "PPPPPPPQRRppppppprr", 104, 0, 0);
    data(m, "NPPPPPPQRRbpppppprr", 134, 0, 1);
    data(m, "BNPPPPPPQRbnppppppr", 140, 0, 0);
    data(m, "BNPPPPPQRRbbppppprr", 82, 0, 0);
    data(m, "BBPPPPPQRRbbppppprr", 60, 0, 0);
    data(m, "BNPPPPPQRRbpppppprr", 55, 0, 0);
    data(m, "NPPPPPPRRbpppppqrr", 0, 0, 58);
    data(m, "BNPPPPPRRbnppppqrr", 0, 0, 60);
    data(m, "BPPPPPPRRbpppppqrr", 0, 0, 122);
    data(m, "BNPPPPPPRbnpppppqr", 0, 2, 70);
    data(m, "BPPPPPPRRppppppqrr", 0, 0, 58);
    data(m, "BNPPPPPPRbppppppqr", 0, 0, 51);
    data(m, "BNPPPPPRRbpppppqrr", 1, 0, 69);
    data(m, "BPPPPPPQRbpppppqrr", 7, 0, 90);
    data(m, "BPPPPPPPRbpppppprr", 0, 0, 53);
    data(m, "BNPPPPPPRbnppppprr", 0, 0, 90);
    data(m, "BPPPPPPRRbppppppqr", 1, 0, 70);
    data(m, "BNPPPPPRRbnpppppqr", 0, 0, 56);
    data(m, "NPPPPPPQRbpppppqrr", 2, 0, 51);
    data(m, "BNPPPPPQRnpppppqrr", 6, 14, 69);
    data(m, "BBNPPPPPRbnppppprr", 11, 13, 57);
    data(m, "NPPPPPPQRbnpppppqr", 5, 3, 103);
    data(m, "BPPPPPPQRppppppqrr", 8, 21, 133);
    data(m, "BNPPPPPRRbbnpppprr", 1, 3, 58);
    data(m, "BNPPPPPQRbpppppqrr", 19, 25, 127);
    data(m, "BNPPPPPPRnpppppprr", 2, 11, 79);
    data(m, "PPPPPPQRRnpppppqrr", 7, 8, 101);
    data(m, "BPPPPPPRRbbppppprr", 1, 4, 102);
    data(m, "NPPPPPQRRbnppppqrr", 3, 5, 57);
    data(m, "PPPPPPPQRbppppppqr", 2, 2, 68);
    data(m, "PPPPPPPRRnpppppprr", 1, 6, 67);
    data(m, "BNPPPPPPRbbnpppppr", 0, 2, 84);
    data(m, "NPPPPPPRRbnppppprr", 1, 5, 133);
    data(m, "NPPPPPPQRppppppqrr", 4, 15, 79);
    data(m, "BPPPPPQRRbnppppqrr", 9, 6, 109);
    data(m, "PPPPPPQRRbpppppqrr", 4, 9, 209);
    data(m, "NPPPPPPRRbbppppprr", 0, 0, 73);
    data(m, "BBPPPPPPRbpppppprr", 1, 14, 44);
    data(m, "BPPPPPPQRbbpppppqr", 1, 5, 102);
    data(m, "BPPPPPPRRbnppppprr", 5, 7, 218);
    data(m, "BPPPPPPPRbnppppppr", 3, 2, 76);
    data(m, "BPPPPPPQRbnpppppqr", 6, 11, 144);
    data(m, "BNPPPPPPRbpppppprr", 5, 14, 113);
    data(m, "PPPPPPPRRbpppppprr", 0, 3, 121);
    data(m, "BBPPPPPQRbpppppqrr", 14, 13, 48);
    data(m, "BNPPPPPQRbbnppppqr", 6, 3, 44);
    data(m, "BPPPPPPRRnpppppprr", 134, 297, 148);
    data(m, "BPPPPPQRRnpppppqrr", 130, 168, 134);
    data(m, "BPPPPPPPRnpppppppr", 20, 59, 38);
    data(m, "BNPPPPPPQRnppppqrr", 12, 19, 20);
    data(m, "NNPPPPPRRbnppppprr", 29, 67, 35);
    data(m, "NPPPPPPQRbppppppqr", 138, 250, 125);
    data(m, "BNPPPPPQRbbpppppqr", 57, 116, 102);
    data(m, "BNPPPPPPRnnppppppr", 27, 72, 29);
    data(m, "NNPPPPPQRbnpppppqr", 17, 40, 27);
    data(m, "BNPPPPPRRnnppppprr", 25, 47, 30);
    data(m, "BBPPPPPQRbnpppppqr", 97, 90, 58);
    data(m, "BPPPPPPQRnppppppqr", 106, 242, 140);
    data(m, "NPPPPPPRRbpppppprr", 142, 283, 148);
    data(m, "BNPPPPPPRbbppppppr", 43, 149, 93);
    data(m, "BBPPPPPRRbnppppprr", 98, 144, 53);
    data(m, "NPPPPPPPRbpppppppr", 35, 58, 30);
    data(m, "NPPPPPQRRbpppppqrr", 123, 151, 164);
    data(m, "BNPPPPPPPRbppppprr", 20, 20, 11);
    data(m, "BBNPPPPPRbnnpppppr", 33, 53, 29);
    data(m, "BNPPPPPRRbbppppprr", 35, 154, 118);
    data(m, "NNPPPPPPRbnppppppr", 28, 55, 24);
    data(m, "BBPPPPPPRbnppppppr", 71, 132, 37);
    data(m, "BNPPPPPQRppppppqrr", 37, 26, 13);
    data(m, "BNPPPPPPQRbppppqrr", 35, 27, 45);
    data(m, "BNPPPPPQRnnpppppqr", 23, 39, 27);
    data(m, "BBPPPPQRRbnppppqrr", 30, 25, 16);
    data(m, "BNPPPPQRRbbppppqrr", 25, 30, 48);
    data(m, "BNNPPPPPRbbnpppppr", 16, 44, 39);
    data(m, "PPPPPPPQRRbppppqrr", 9, 9, 33);
    data(m, "NNPPPPPRRbbppppprr", 8, 24, 29);
    data(m, "NPPPPPQRRppppppqrr", 113, 3, 4);
    data(m, "BNPPPPPPQRbbppppqr", 55, 20, 9);
    data(m, "NPPPPPPPRRbppppprr", 66, 13, 5);
    data(m, "BNPPPPPPQRbnppppqr", 130, 19, 15);
    data(m, "BBPPPPPRRbpppppprr", 117, 2, 3);
    data(m, "BNPPPPPPRRbnpppprr", 186, 29, 17);
    data(m, "BNPPPPPRRbnnpppppr", 35, 9, 6);
    data(m, "NPPPPPPPRRnppppprr", 59, 14, 3);
    data(m, "BNPPPPPRRnpppppprr", 152, 3, 2);
    data(m, "BNPPPPPRRbpppppprr", 184, 7, 2);
    data(m, "BBPPPPPRRnpppppprr", 62, 1, 3);
    data(m, "BPPPPPPRRbbppppppr", 54, 15, 2);
    data(m, "NPPPPPQRRbbpppppqr", 29, 9, 13);
    data(m, "BPPPPPPPRRbppppprr", 138, 29, 4);
    data(m, "BPPPPPQRRbnpppppqr", 113, 31, 17);
    data(m, "BPPPPPPQRRbppppqrr", 202, 54, 42);
    data(m, "PPPPPPQRRnppppppqr", 70, 17, 6);
    data(m, "BPPPPPQRRppppppqrr", 176, 18, 9);
    data(m, "BPPPPPPQRRnppppqrr", 109, 35, 18);
    data(m, "NPPPPPPRRppppppprr", 71, 0, 0);
    data(m, "BPPPPPPRRbnppppppr", 137, 13, 0);
    data(m, "BNPPPPPPRRbbpppprr", 83, 40, 14);
    data(m, "BPPPPPPRRppppppprr", 116, 3, 2);
    data(m, "BPPPPPPPQRbpppppqr", 87, 18, 8);
    data(m, "NPPPPPPPQRbpppppqr", 53, 12, 4);
    data(m, "BNPPPPPPRbpppppppr", 65, 0, 0);
    data(m, "BPPPPPPQRpppppppqr", 64, 3, 0);
    data(m, "BPPPPPQRRbbpppppqr", 42, 12, 18);
    data(m, "BPPPPPPPQRnpppppqr", 47, 8, 7);
    data(m, "NPPPPPPQRRbppppqrr", 99, 41, 25);
    data(m, "BNPPPPPQRbppppppqr", 169, 11, 8);
    data(m, "BNPPPPPQRRbnpppqrr", 51, 21, 8);
    data(m, "BNPPPPPRRbbnpppppr", 51, 11, 5);
    data(m, "BNPPPPPQRnppppppqr", 122, 5, 5);
    data(m, "BPPPPPPPRRnppppprr", 81, 8, 4);
    data(m, "NPPPPPQRRbnpppppqr", 57, 12, 14);
    data(m, "BNPPPPPPPRbnpppppr", 69, 13, 3);
    data(m, "PPPPPPQRRbppppppqr", 102, 21, 8);
    data(m, "BBPPPPPPRRbnpppprr", 59, 5, 4);
    data(m, "PPPPPPPQRRpppppqrr", 113, 35, 10);
    data(m, "BNPPPPQRRbpppppqrr", 81, 5, 11);
    data(m, "NPPPPPPRRbnppppppr", 73, 8, 3);
    data(m, "BBPPPPPQRbppppppqr", 82, 3, 5);
    data(m, "NPPPPPPQRRnppppqrr", 68, 25, 10);
    data(m, "BNPPPPQRRnpppppqrr", 42, 4, 5);
    data(m, "BNNPPPPPRbnppppppr", 49, 4, 0);
    data(m, "BBNPPPPPRbnppppppr", 82, 2, 0);
    data(m, "BNPPPPPRRbnppppppr", 105, 0, 3);
    data(m, "BNPPPPPPRRbppppprr", 175, 2, 1);
    data(m, "BPPPPPPQRbpppppprr", 75, 1, 0);
    data(m, "NPPPPPPQRRpppppqrr", 79, 0, 2);
    data(m, "BPPPPPPRRbpppppppr", 59, 1, 0);
    data(m, "BPPPPPQRRbppppppqr", 105, 1, 6);
    data(m, "BBPPPPPPRRbppppprr", 69, 1, 0);
    data(m, "BPPPPPPPRRpppppprr", 101, 0, 0);
    data(m, "BNPPPPPPQRnpppppqr", 59, 1, 3);
    data(m, "NPPPPPPPRRpppppprr", 54, 0, 0);
    data(m, "NPPPPPQRRbppppppqr", 57, 2, 2);
    data(m, "BPPPPPPQRRpppppqrr", 166, 3, 1);
    data(m, "BNPPPPPPQRbpppppqr", 106, 2, 1);
    data(m, "PPPPPPQRRpppppppqr", 52, 1, 0);
    data(m, "BNPPPPPPRRnppppprr", 119, 2, 0);
    data(m, "BNPPPPPQRRnppppqrr", 48, 3, 1);
    data(m, "BNPPPPPQRRbppppqrr", 88, 2, 6);
    data(m, "BPPPPPPPRRbppppppr", 58, 0, 0);
    data(m, "PPPPPPQRRbpppppprr", 69, 0, 0);
    data(m, "BNPPPPPPRRbnpppppr", 83, 0, 0);
    data(m, "BNPPPPPPRRbbpppppr", 65, 0, 0);
    data(m, "BPPPPPPQRRnpppppqr", 58, 0, 0);
    data(m, "BPPPPPPQRRbpppppqr", 93, 1, 1);
    data(m, "BPPPPPQRRbnppppprr", 63, 0, 0);
    data(m, "BPPPPPPQRbnppppppr", 54, 1, 0);
    data(m, "NPPPPPQRRbpppppprr", 58, 0, 0);
    data(m, "BPPPPPQRRbpppppprr", 114, 0, 1);
    data(m, "BNPPPPPQRbnppppppr", 74, 0, 0);
    data(m, "BPPPPPQRRnpppppprr", 57, 1, 0);
    data(m, "PPPPPPQRRppppppprr", 61, 0, 0);
    data(m, "BNPPPPQRRbnppppprr", 57, 0, 0);
    data(m, "BPPPPPPQRRnppppprr", 86, 0, 0);
    data(m, "BNPPPPPPQRbnpppppr", 81, 0, 0);
    data(m, "NPPPPPPQRRbppppprr", 79, 0, 0);
    data(m, "BPPPPPPQRRbppppprr", 138, 0, 0);
    data(m, "PPPPPPPQRRpppppprr", 69, 0, 0);
    data(m, "BNPPPPPQRRbnpppprr", 52, 0, 0);
    data(m, "BPPPPPPRRbppppqrr", 0, 0, 57);
    data(m, "BNPPPPPPRbpppppqr", 0, 0, 50);
    data(m, "BPPPPPPRRpppppqrr", 0, 0, 87);
    data(m, "NPPPPPPQRbppppqrr", 7, 0, 50);
    data(m, "BPPPPPPRRbpppppqr", 1, 6, 73);
    data(m, "BPPPPPPRRnpppppqr", 0, 2, 49);
    data(m, "BNPPPPPPRbnpppprr", 1, 2, 80);
    data(m, "BPPPPPPPRbppppprr", 0, 2, 59);
    data(m, "BPPPPPPQRbppppqrr", 4, 2, 69);
    data(m, "PPPPPPQRRbppppqrr", 15, 21, 132);
    data(m, "BPPPPPPRRbbpppprr", 2, 8, 76);
    data(m, "BNPPPPPPRbppppprr", 22, 49, 127);
    data(m, "BPPPPPQRRbnpppqrr", 11, 5, 43);
    data(m, "BBPPPPPPRbppppprr", 12, 39, 32);
    data(m, "BPPPPPPQRpppppqrr", 30, 52, 139);
    data(m, "BNPPPPPPRnppppprr", 15, 43, 80);
    data(m, "BPPPPPPRRbnpppprr", 8, 19, 124);
    data(m, "NPPPPPPQRpppppqrr", 22, 36, 87);
    data(m, "BPPPPPPQRbbppppqr", 7, 7, 53);
    data(m, "BPPPPPPQRbnppppqr", 11, 23, 69);
    data(m, "NPPPPPPPRpppppprr", 9, 19, 39);
    data(m, "BBPPPPPQRbppppqrr", 28, 20, 30);
    data(m, "NPPPPPPQRbnppppqr", 10, 18, 58);
    data(m, "PPPPPPPQRbpppppqr", 8, 10, 67);
    data(m, "BBPPPPPPRnppppprr", 11, 23, 25);
    data(m, "BNPPPPPPRbbnppppr", 0, 8, 62);
    data(m, "NPPPPPPRRbnpppprr", 3, 16, 89);
    data(m, "BNPPPPPQRbppppqrr", 32, 30, 91);
    data(m, "NPPPPPPPRbnpppppr", 3, 8, 48);
    data(m, "BNPPPPPQRnppppqrr", 23, 25, 39);
    data(m, "PPPPPPPRRnppppprr", 1, 5, 58);
    data(m, "BBNPPPPPRbnpppprr", 10, 22, 18);
    data(m, "BNPPPPPPQnpppppqr", 6, 8, 36);
    data(m, "BPPPPPPPRpppppprr", 13, 26, 61);
    data(m, "BNPPPPPPQbpppppqr", 7, 10, 41);
    data(m, "PPPPPPPRRbppppprr", 2, 5, 87);
    data(m, "PPPPPPQRRnppppqrr", 10, 15, 69);
    data(m, "BPPPPPPPRbnpppppr", 5, 12, 73);
    data(m, "NPPPPPPRRbbpppprr", 0, 2, 54);
    data(m, "NNPPPPPPRbnpppppr", 63, 34, 11);
    data(m, "BNPPPPPQRbnppppqr", 237, 120, 93);
    data(m, "NPPPPPPRRbppppprr", 347, 215, 55);
    data(m, "BNPPPPPPRbnpppppr", 402, 259, 84);
    data(m, "NPPPPPPQRbpppppqr", 277, 169, 69);
    data(m, "PPPPPPPQRppppppqr", 137, 80, 17);
    data(m, "BNPPPPPPPbnpppppp", 34, 25, 5);
    data(m, "PPPPPPQRRpppppqrr", 373, 252, 75);
    data(m, "NPPPPPPRRnppppprr", 268, 177, 40);
    data(m, "BPPPPPPRRnppppprr", 365, 227, 62);
    data(m, "BBPPPPPPRbnpppppr", 146, 65, 14);
    data(m, "NPPPPPQRRbppppqrr", 172, 104, 69);
    data(m, "BPPPPPPQRbpppppqr", 543, 357, 148);
    data(m, "BNPPPPPQRnnppppqr", 43, 17, 12);
    data(m, "BBNPPPPPRbnnppppr", 30, 17, 4);
    data(m, "BPPPPPPRRbppppprr", 569, 451, 88);
    data(m, "BNPPPPPPRbbpppppr", 139, 166, 46);
    data(m, "NPPPPPPPRnppppppr", 98, 47, 11);
    data(m, "BNPPPPPQRbbppppqr", 90, 90, 64);
    data(m, "BPPPPPPQRnpppppqr", 277, 139, 69);
    data(m, "NPPPPPPPRbppppppr", 125, 55, 16);
    data(m, "BNPPPPPPRpppppprr", 53, 8, 1);
    data(m, "BPPPPPPPRnppppppr", 118, 73, 17);
    data(m, "BBPPPPPRRbbpppprr", 78, 46, 17);
    data(m, "NPPPPPQRRnppppqrr", 124, 61, 56);
    data(m, "PPPPPPPRRpppppprr", 233, 123, 19);
    data(m, "BNPPPPPPQbnpppppq", 79, 53, 27);
    data(m, "BNPPPPQRRbnpppqrr", 56, 20, 20);
    data(m, "BNPPPPPRRbnpppprr", 282, 211, 55);
    data(m, "BBPPPPPRRbnpppprr", 119, 44, 12);
    data(m, "BBPPPPPQRbnppppqr", 92, 46, 39);
    data(m, "BPPPPPPPQbppppppq", 48, 38, 14);
    data(m, "BBNPPPPPRbbnppppr", 81, 55, 20);
    data(m, "BBPPPPPPRbbpppppr", 102, 81, 24);
    data(m, "BNPPPPPPRnnpppppr", 73, 50, 6);
    data(m, "PPPPPPQRRbnppppqr", 26, 15, 16);
    data(m, "BPPPPPQRRbppppqrr", 306, 197, 112);
    data(m, "BPPPPPPPRbppppppr", 181, 157, 27);
    data(m, "NPPPPPPQRnpppppqr", 206, 128, 43);
    data(m, "BNPPPPPQRpppppqrr", 44, 11, 16);
    data(m, "BPPPPPQRRnppppqrr", 155, 84, 59);
    data(m, "NNPPPPPRRbnpppprr", 30, 38, 10);
    data(m, "BNPPPPPRRbbpppprr", 92, 129, 59);
    data(m, "NNPPPPPPRbbpppppr", 31, 27, 10);
    data(m, "BNNPPPPPRbbnppppr", 41, 51, 8);
    data(m, "BNPPPPPPQbbpppppq", 34, 28, 11);
    data(m, "BBPPPPPQRbbppppqr", 66, 32, 21);
    data(m, "BNPPPPPRRnnpppprr", 53, 23, 11);
    data(m, "BNPPPPQRRbbpppqrr", 25, 17, 13);
    data(m, "BBPPPPPRRbppppprr", 101, 3, 1);
    data(m, "BNPPPPPPRbppppppr", 257, 1, 1);
    data(m, "BPPPPPPRRbbpppppr", 88, 10, 5);
    data(m, "NPPPPPPQRppppppqr", 129, 3, 1);
    data(m, "BBPPPPPRRnppppprr", 57, 1, 0);
    data(m, "NPPPPPPRRpppppprr", 167, 1, 0);
    data(m, "BPPPPPQRRpppppqrr", 215, 3, 9);
    data(m, "BNPPPPPQRbpppppqr", 200, 9, 14);
    data(m, "BNPPPPPQRnpppppqr", 139, 5, 3);
    data(m, "BNPPPPPRRbppppprr", 315, 9, 6);
    data(m, "PPPPPPPRRbppppppr", 70, 0, 1);
    data(m, "BPPPPPPRRpppppprr", 315, 6, 0);
    data(m, "BNPPPPPPRnppppppr", 186, 2, 0);
    data(m, "BBPPPPPQRbpppppqr", 85, 1, 3);
    data(m, "NPPPPPQRRpppppqrr", 147, 7, 3);
    data(m, "BPPPPPPQRppppppqr", 244, 4, 4);
    data(m, "BBPPPPPQRnpppppqr", 48, 1, 2);
    data(m, "BNPPPPPRRbbnppppr", 47, 9, 5);
    data(m, "BNPPPPQRRbppppqrr", 70, 7, 7);
    data(m, "BBPPPPPPRbppppppr", 136, 2, 0);
    data(m, "BPPPPPPPRpppppppr", 77, 0, 1);
    data(m, "BBNPPPPPRbbpppppr", 51, 2, 0);
    data(m, "BNNPPPPPRbnpppppr", 66, 4, 1);
    data(m, "PPPPPPQRRbpppppqr", 158, 11, 9);
    data(m, "BBNPPPPPRbnpppppr", 121, 1, 1);
    data(m, "NPPPPPQRRbnppppqr", 50, 7, 8);
    data(m, "BNPPPPPRRnppppprr", 183, 4, 1);
    data(m, "BPPPPPQRRbnppppqr", 101, 7, 10);
    data(m, "BPPPPPPRRbnpppppr", 163, 7, 1);
    data(m, "NPPPPPPRRbbpppppr", 53, 11, 2);
    data(m, "BPPPPPQRRbbppppqr", 45, 6, 3);
    data(m, "PPPPPPQRRnpppppqr", 90, 5, 6);
    data(m, "BBPPPPPPRnppppppr", 68, 0, 0);
    data(m, "NPPPPPPRRbnpppppr", 120, 2, 1);
    data(m, "BPPPPPPRRbppppppr", 217, 1, 0);
    data(m, "BNPPPPPQRbnpppppq", 58, 1, 2);
    data(m, "BNPPPPPPRbnpppppp", 80, 0, 0);
    data(m, "PPPPPPQRRppppppqr", 114, 2, 3);
    data(m, "NPPPPPQRRbpppppqr", 74, 0, 0);
    data(m, "BPPPPPQRRnpppppqr", 82, 1, 0);
    data(m, "NPPPPPPRRnppppppr", 88, 1, 0);
    data(m, "BNPPPPPRRbnpppppr", 180, 0, 0);
    data(m, "NPPPPPPRRbppppppr", 102, 0, 0);
    data(m, "BPPPPPQRRbpppppqr", 140, 2, 3);
    data(m, "BPPPPPPQRnppppprr", 53, 0, 0);
    data(m, "BPPPPPPRRnppppppr", 139, 0, 0);
    data(m, "BBPPPPPRRbnpppppr", 68, 0, 0);
    data(m, "BPPPPPPQRbppppprr", 84, 0, 0);
    data(m, "NPPPPPQRRnpppppqr", 60, 2, 0);
    data(m, "BNPPPPPRRbbpppppr", 63, 0, 0);
    data(m, "BPPPPPPQRbppppppq", 88, 1, 2);
    data(m, "BPPPPPQRRbnpppprr", 54, 0, 0);
    data(m, "PPPPPPQRRbppppprr", 105, 0, 1);
    data(m, "PPPPPPQRRnppppprr", 66, 0, 0);
    data(m, "BPPPPPPQRbnpppppr", 71, 0, 0);
    data(m, "NPPPPPPQRbnpppppr", 57, 0, 0);
    data(m, "BPPPPPPQRbppppppr", 161, 0, 0);
    data(m, "BPPPPPQRRbppppprr", 181, 0, 0);
    data(m, "BNPPPPPQRbbpppppr", 55, 0, 0);
    data(m, "NPPPPPPQRbppppppr", 65, 0, 0);
    data(m, "BNPPPPPQRbnpppppr", 127, 1, 0);
    data(m, "PPPPPPQRRpppppprr", 116, 0, 0);
    data(m, "NPPPPPPQRnppppppr", 79, 0, 0);
    data(m, "BPPPPPQRRnppppprr", 92, 0, 0);
    data(m, "NPPPPPQRRbppppprr", 86, 0, 1);
    data(m, "BPPPPPPQRnppppppr", 98, 0, 0);
    data(m, "NPPPPPQRRnppppprr", 65, 0, 0);
    data(m, "PPPPPPRRpppppqrr", 0, 0, 55);
    data(m, "BPPPPPPRbpppppqr", 0, 0, 79);
    data(m, "BPPPPPRRbpppppqr", 0, 2, 49);
    data(m, "NPPPPPPRnppppprr", 1, 0, 53);
    data(m, "BPPPPPQRbppppqrr", 4, 1, 53);
    data(m, "BPPPPPPRbppppprr", 1, 1, 101);
    data(m, "BNPPPPPRbnpppprr", 1, 1, 55);
    data(m, "BPPPPPPRnppppprr", 0, 0, 56);
    data(m, "NPPPPPPRbppppprr", 0, 0, 62);
    data(m, "PPPPPPQRpppppqrr", 1, 4, 51);
    data(m, "PPPPPPRRnppppprr", 0, 6, 108);
    data(m, "NPPPPPQRpppppqrr", 7, 12, 58);
    data(m, "BNPPPPPRbppppprr", 3, 10, 117);
    data(m, "BBPPPPPRbppppprr", 3, 15, 64);
    data(m, "BPPPPPRRbnpppprr", 3, 12, 87);
    data(m, "PPPPPPRRbppppprr", 3, 4, 143);
    data(m, "PPPPPPQRbpppppqr", 3, 8, 151);
    data(m, "BPPPPPPRbnpppppr", 5, 10, 132);
    data(m, "BPPPPPQRbnppppqr", 7, 4, 103);
    data(m, "BNPPPPPRnppppprr", 2, 13, 68);
    data(m, "NPPPPPPRpppppprr", 1, 7, 69);
    data(m, "PPPPPPPRbppppppr", 0, 0, 69);
    data(m, "BPPPPPPRpppppprr", 0, 11, 122);
    data(m, "NPPPPPQRbnppppqr", 6, 4, 56);
    data(m, "BPPPPPQRpppppqrr", 9, 13, 117);
    data(m, "BPPPPPPRbbpppppr", 1, 1, 84);
    data(m, "PPPPPPQRnpppppqr", 5, 3, 100);
    data(m, "PPPPPQRRbppppqrr", 8, 11, 84);
    data(m, "NPPPPPPRbnpppppr", 7, 3, 139);
    data(m, "NPPPPPRRbnpppprr", 0, 5, 52);
    data(m, "BPPPPPRRbbpppprr", 0, 2, 52);
    data(m, "BNPPPPQRbppppqrr", 11, 2, 43);
    data(m, "BPPPPPPQRppppqrr", 21, 28, 29);
    data(m, "BPPPPPQRnpppppqr", 116, 182, 134);
    data(m, "NPPPPPPQbppppppq", 12, 41, 21);
    data(m, "NPPPPPQRbpppppqr", 90, 149, 92);
    data(m, "BBPPPPPRbnpppppr", 54, 121, 21);
    data(m, "BPPPPPRRnppppprr", 114, 214, 102);
    data(m, "NNPPPPPRbnpppppr", 21, 53, 23);
    data(m, "NPPPPPRRbppppprr", 81, 205, 97);
    data(m, "NPPPPPPRbppppppr", 110, 210, 100);
    data(m, "BNPPPPPRnnpppppr", 20, 56, 19);
    data(m, "NPPPPQRRbppppqrr", 33, 54, 31);
    data(m, "BPPPPPPRnppppppr", 95, 222, 106);
    data(m, "BNPPPPQRbbppppqr", 19, 28, 20);
    data(m, "BBPPPPRRbnpppprr", 24, 34, 14);
    data(m, "BPPPPQRRnppppqrr", 39, 57, 33);
    data(m, "BNPPPPPRbbpppppr", 37, 140, 80);
    data(m, "BNPPPPPPRbpppprr", 16, 30, 35);
    data(m, "BBPPPPQRbnppppqr", 27, 29, 24);
    data(m, "BNPPPPRRbbpppprr", 18, 27, 33);
    data(m, "BBPPPPPPbnpppppp", 15, 33, 4);
    data(m, "BNPPPPPPRnpppprr", 12, 30, 13);
    data(m, "PPPPPQRRbnppppqr", 8, 14, 29);
    data(m, "BPPPPPPQnppppppq", 19, 47, 24);
    data(m, "BPPPPPPRRbpppprr", 205, 38, 9);
    data(m, "BPPPPPRRbnpppppr", 97, 21, 4);
    data(m, "BNPPPPPPRbnppppr", 122, 26, 6);
    data(m, "BPPPPPPRRnpppprr", 103, 24, 4);
    data(m, "PPPPPPPRRppppprr", 91, 10, 1);
    data(m, "BPPPPPQRRbpppqrr", 70, 16, 19);
    data(m, "NPPPPPPPRbpppppr", 60, 11, 2);
    data(m, "BNPPPPPRbppppppr", 163, 2, 0);
    data(m, "BNPPPPRRbppppprr", 88, 7, 4);
    data(m, "PPPPPPPQRpppppqr", 62, 13, 5);
    data(m, "NPPPPPPQRbppppqr", 74, 23, 7);
    data(m, "BNPPPPPRnppppppr", 98, 4, 1);
    data(m, "PPPPPPRRbppppppr", 95, 11, 2);
    data(m, "NPPPPPPRRnpppprr", 84, 13, 4);
    data(m, "BNPPPPPRRbnppprr", 50, 21, 7);
    data(m, "NPPPPPRRpppppprr", 113, 2, 3);
    data(m, "PPPPPQRRbpppppqr", 90, 21, 11);
    data(m, "NPPPPPRRbnpppppr", 60, 6, 2);
    data(m, "BNPPPPPQRbnpppqr", 57, 9, 17);
    data(m, "BPPPPPPQRnppppqr", 88, 12, 14);
    data(m, "BPPPPPPQRbppppqr", 132, 43, 12);
    data(m, "NPPPPPPQRnppppqr", 66, 12, 10);
    data(m, "NPPPPPPPRnpppppr", 55, 6, 1);
    data(m, "BPPPPQRRpppppqrr", 54, 7, 6);
    data(m, "BPPPPPRRpppppprr", 149, 9, 1);
    data(m, "BPPPPPPRpppppppr", 60, 2, 1);
    data(m, "NPPPPPQRRbpppqrr", 35, 17, 15);
    data(m, "BNPPPPPPRbbppppr", 61, 26, 3);
    data(m, "NPPPPPPRRbpppprr", 114, 26, 12);
    data(m, "NPPPPPQRppppppqr", 86, 7, 2);
    data(m, "PPPPPQRRnpppppqr", 65, 9, 8);
    data(m, "NPPPPQRRpppppqrr", 44, 4, 6);
    data(m, "PPPPPPQRRppppqrr", 125, 25, 23);
    data(m, "PPPPPPRRnppppppr", 54, 4, 0);
    data(m, "BPPPPPQRppppppqr", 150, 7, 8);
    data(m, "BPPPPPPPRnpppppr", 45, 10, 1);
    data(m, "BPPPPPRRbbpppppr", 60, 14, 6);
    data(m, "BPPPPPPPRbpppppr", 113, 20, 1);
    data(m, "BNPPPPRRnppppprr", 54, 4, 2);
    data(m, "BBPPPPPRnppppppr", 49, 0, 2);
    data(m, "BBPPPPQRbpppppqr", 49, 0, 1);
    data(m, "BBPPPPPRbppppppr", 84, 3, 2);
    data(m, "BNPPPPQRbpppppqr", 75, 6, 6);
    data(m, "BNPPPPPPRnpppppr", 80, 1, 0);
    data(m, "BNPPPPPPRbpppppr", 123, 2, 1);
    data(m, "BPPPPPPRRppppprr", 150, 3, 0);
    data(m, "BPPPPPPQRpppppqr", 102, 2, 6);
    data(m, "BNPPPPRRbnpppppr", 56, 1, 0);
    data(m, "BPPPPPRRbppppppr", 116, 0, 1);
    data(m, "PPPPPQRRppppppqr", 49, 2, 1);
    data(m, "NPPPPPPQRpppppqr", 68, 1, 2);
    data(m, "PPPPPPRRpppppppr", 50, 0, 0);
    data(m, "BPPPPPRRnppppppr", 78, 0, 0);
    data(m, "NPPPPPPRRppppprr", 73, 1, 0);
    data(m, "NPPPPPRRnppppppr", 55, 0, 0);
    data(m, "NPPPPPRRbppppppr", 73, 0, 1);
    data(m, "BNPPPPPQRbppppqr", 72, 3, 1);
    data(m, "BPPPPPQRRppppqrr", 83, 2, 4);
    data(m, "BPPPPPQRbppppprr", 47, 0, 4);
    data(m, "BBPPPPPPRbpppppr", 54, 0, 1);
    data(m, "BNPPPPPRRbpppprr", 76, 3, 0);
    data(m, "PPPPPPQRRpppppqr", 66, 3, 1);
    data(m, "NPPPPPPRRbpppppr", 54, 0, 0);
    data(m, "BPPPPPPRRbpppppr", 107, 0, 0);
    data(m, "BNPPPPPRRbnppppr", 58, 0, 0);
    data(m, "BPPPPPQRRbppppqr", 51, 1, 1);
    data(m, "PPPPPQRRbppppprr", 49, 1, 0);
    data(m, "BPPPPPPRRnpppppr", 60, 1, 0);
    data(m, "PPPPPQRRpppppprr", 51, 0, 0);
    data(m, "BPPPPPQRbppppppr", 82, 0, 0);
    data(m, "BPPPPPPQRbpppppr", 91, 0, 0);
    data(m, "BPPPPPQRRbpppprr", 60, 0, 0);
    data(m, "PPPPPPQRRppppprr", 69, 0, 0);
    data(m, "BPPPPPPRbppppqr", 0, 0, 59);
    data(m, "BPPPPPPRpppppqr", 0, 0, 58);
    data(m, "PPPPPPQRppppqrr", 4, 0, 76);
    data(m, "BPPPPPPRnpppprr", 1, 0, 50);
    data(m, "BPPPPPPRbpppprr", 0, 1, 100);
    data(m, "NPPPPPPRppppprr", 6, 23, 80);
    data(m, "NPPPPPQRppppqrr", 15, 16, 59);
    data(m, "NPPPPPPRbnppppr", 2, 15, 88);
    data(m, "BNPPPPPRbpppprr", 16, 36, 99);
    data(m, "BPPPPPPRbnppppr", 3, 20, 101);
    data(m, "BNPPPPPRnpppprr", 13, 31, 63);
    data(m, "PPPPPPRRnpppprr", 3, 12, 75);
    data(m, "BPPPPPPRbbppppr", 3, 7, 66);
    data(m, "PPPPPPPRbpppppr", 4, 11, 74);
    data(m, "PPPPPPQRnppppqr", 10, 14, 63);
    data(m, "BPPPPPQRppppqrr", 30, 52, 79);
    data(m, "BPPPPPPQpppppqr", 6, 10, 54);
    data(m, "BPPPPPRRbnppprr", 3, 8, 45);
    data(m, "BNPPPPPPbpppppr", 3, 12, 67);
    data(m, "BPPPPPPRppppprr", 18, 50, 102);
    data(m, "PPPPPQRRbpppqrr", 12, 3, 38);
    data(m, "BBPPPPPRbpppprr", 12, 30, 33);
    data(m, "PPPPPPQRbppppqr", 21, 23, 117);
    data(m, "BPPPPPQRbnpppqr", 9, 14, 42);
    data(m, "PPPPPPRRbpppprr", 8, 12, 136);
    data(m, "NPPPPPPQpppppqr", 6, 10, 37);
    data(m, "BPPPPPPRbpppppr", 521, 420, 66);
    data(m, "BPPPPQRRbpppqrr", 65, 44, 31);
    data(m, "BNPPPPQRbnpppqr", 48, 36, 32);
    data(m, "NPPPPPQRbppppqr", 194, 122, 74);
    data(m, "BBPPPPPPbbppppp", 44, 30, 5);
    data(m, "BNPPPPPRppppprr", 65, 16, 13);
    data(m, "BNPPPPPRbnppppr", 297, 204, 53);
    data(m, "BPPPPPRRbpppprr", 344, 296, 57);
    data(m, "BPPPPPRRnpppprr", 236, 150, 30);
    data(m, "BBPPPPPRbbppppr", 80, 59, 17);
    data(m, "NPPPPPRRnpppprr", 151, 144, 21);
    data(m, "PPPPPPQRpppppqr", 353, 218, 61);
    data(m, "BPPPPPQRbppppqr", 322, 221, 109);
    data(m, "NPPPPPPQnpppppq", 70, 26, 8);
    data(m, "NPPPPPPRnpppppr", 268, 173, 34);
    data(m, "PPPPPQRRppppqrr", 206, 168, 65);
    data(m, "PPPPPPRRppppprr", 424, 283, 42);
    data(m, "BNPPPPRRbbppprr", 17, 42, 15);
    data(m, "NPPPPPPRbpppppr", 325, 205, 41);
    data(m, "BNPPPPRRbnppprr", 58, 44, 14);
    data(m, "BPPPPPPQnpppppq", 50, 42, 10);
    data(m, "BNPPPPPPbnppppp", 90, 40, 4);
    data(m, "BPPPPPPRnpppppr", 327, 183, 45);
    data(m, "BNPPPPQRbbpppqr", 21, 22, 11);
    data(m, "NPPPPPRRbpppprr", 188, 173, 48);
    data(m, "NPPPPPPQbpppppq", 43, 30, 8);
    data(m, "NPPPPPQRnppppqr", 146, 76, 49);
    data(m, "BPPPPPPPbpppppp", 31, 25, 7);
    data(m, "PPPPPPRRbnppppr", 24, 29, 17);
    data(m, "BPPPPPQRnppppqr", 178, 102, 51);
    data(m, "BNPPPPPRbbppppr", 96, 159, 32);
    data(m, "BNPPPPPRnnppppr", 40, 16, 5);
    data(m, "PPPPPPPRppppppr", 114, 81, 13);
    data(m, "BBPPPPPPbnppppp", 36, 12, 6);
    data(m, "NPPPPQRRnpppqrr", 43, 9, 10);
    data(m, "BBPPPPPRbnppppr", 114, 68, 11);
    data(m, "NNPPPPPRbnppppr", 38, 48, 11);
    data(m, "BPPPPPPQbpppppq", 91, 70, 26);
    data(m, "BNPPPPPQbnppppq", 36, 23, 4);
    data(m, "BPPPPQRRnpppqrr", 35, 17, 16);
    data(m, "BNPPPPPPbbppppp", 38, 37, 6);
    data(m, "NPPPPQRRbpppqrr", 36, 18, 20);
    data(m, "BPPPPPQRpppppqr", 255, 8, 11);
    data(m, "NPPPPPQRpppppqr", 142, 5, 7);
    data(m, "PPPPPQRRbppppqr", 84, 11, 11);
    data(m, "BNPPPPPRnpppppr", 218, 4, 1);
    data(m, "BPPPPPPRppppppr", 291, 2, 1);
    data(m, "BNPPPPQRnppppqr", 50, 1, 3);
    data(m, "PPPPPQRRnppppqr", 45, 5, 5);
    data(m, "PPPPPPRRnpppppr", 117, 2, 0);
    data(m, "BPPPPQRRppppqrr", 76, 6, 5);
    data(m, "BNPPPPQRbppppqr", 87, 7, 5);
    data(m, "BNPPPPRRnpppprr", 52, 2, 0);
    data(m, "NPPPPPRRbnppppr", 66, 2, 1);
    data(m, "BPPPPPRRbnppppr", 115, 6, 2);
    data(m, "PPPPPPRRbpppppr", 173, 1, 3);
    data(m, "PPPPPPQRbpppppq", 52, 0, 1);
    data(m, "BPPPPPRRppppprr", 262, 3, 4);
    data(m, "NPPPPPPRppppppr", 174, 1, 2);
    data(m, "BNPPPPRRbpppprr", 103, 6, 5);
    data(m, "BBPPPPPRbpppppr", 115, 0, 0);
    data(m, "BNPPPPPRbpppppr", 289, 8, 6);
    data(m, "NPPPPPRRppppprr", 171, 3, 2);
    data(m, "BPPPPPRRbbppppr", 61, 4, 1);
    data(m, "BNPPPPPQbpppppq", 58, 0, 2);
    data(m, "BBPPPPPRnpppppr", 54, 0, 0);
    data(m, "PPPPPPRRppppppr", 141, 0, 0);
    data(m, "BPPPPPRRnpppppr", 138, 0, 0);
    data(m, "PPPPPPQRppppprr", 58, 1, 1);
    data(m, "NPPPPPRRnpppppr", 120, 0, 0);
    data(m, "PPPPPPQRppppppq", 58, 1, 0);
    data(m, "BPPPPPRRbpppppr", 224, 1, 0);
    data(m, "NPPPPPPRbpppppp", 58, 0, 0);
    data(m, "BPPPPPPRbpppppp", 112, 0, 0);
    data(m, "BNPPPPPRbnppppp", 103, 0, 0);
    data(m, "NPPPPPRRbpppppr", 122, 0, 0);
    data(m, "BPPPPPPRnpppppp", 72, 0, 0);
    data(m, "NPPPPPQRbpppppq", 56, 0, 0);
    data(m, "BPPPPQRRbppppqr", 53, 2, 4);
    data(m, "BPPPPPQRbpppppq", 79, 0, 1);
    data(m, "NPPPPPPRnpppppp", 58, 0, 0);
    data(m, "PPPPPQRRpppppqr", 92, 3, 4);
    data(m, "BPPPPPQRbpppprr", 53, 1, 1);
    data(m, "BPPPPPQRnpppppq", 56, 0, 1);
    data(m, "PPPPPPQRbpppppr", 55, 0, 0);
    data(m, "PPPPPPQRppppppr", 99, 0, 0);
    data(m, "BPPPPPQRnpppppr", 85, 0, 1);
    data(m, "BNPPPPQRbnppppr", 51, 0, 0);
    data(m, "BPPPPPQRbpppppr", 143, 0, 1);
    data(m, "PPPPPQRRppppprr", 86, 0, 2);
    data(m, "NPPPPPQRbpppppr", 90, 0, 0);
    data(m, "NPPPPPQRnpppppr", 65, 0, 0);
    data(m, "BPPPPQRRbpppprr", 56, 0, 0);
    data(m, "BPPPPPRbppppqr", 0, 0, 57);
    data(m, "PPPPPPRpppppqr", 0, 0, 66);
    data(m, "BPPPPPRpppppqr", 1, 0, 55);
    data(m, "BPPPPPPbpppppr", 0, 0, 68);
    data(m, "PPPPPPQpppppqr", 0, 0, 54);
    data(m, "PPPPPPRppppprr", 0, 1, 66);
    data(m, "BPPPPPRbpppprr", 1, 0, 97);
    data(m, "PPPPPPRnpppppr", 2, 3, 117);
    data(m, "PPPPPPRbpppppr", 4, 10, 212);
    data(m, "PPPPPRRnpppprr", 4, 2, 53);
    data(m, "BPPPPPRbnppppr", 7, 4, 119);
    data(m, "BNPPPPRbpppprr", 2, 4, 56);
    data(m, "PPPPPPQbpppppq", 1, 6, 52);
    data(m, "PPPPPQRnppppqr", 10, 9, 72);
    data(m, "BPPPPPRppppprr", 7, 12, 121);
    data(m, "BPPPPPQpppppqr", 3, 6, 50);
    data(m, "PPPPPRRbpppprr", 1, 2, 95);
    data(m, "NPPPPPRbnppppr", 1, 3, 85);
    data(m, "NPPPPPRppppprr", 1, 3, 70);
    data(m, "BPPPPPRbbppppr", 2, 1, 66);
    data(m, "BPPPPQRppppqrr", 6, 7, 42);
    data(m, "PPPPPQRbppppqr", 9, 4, 79);
    data(m, "NPPPPPPRpppprr", 8, 16, 27);
    data(m, "NNPPPPRbnppppr", 8, 37, 16);
    data(m, "NPPPPPRbpppppr", 106, 275, 112);
    data(m, "BPPPPPQnpppppq", 43, 54, 24);
    data(m, "BNPPPPRbbppppr", 12, 61, 42);
    data(m, "NPPPPQRbppppqr", 35, 73, 52);
    data(m, "BPPPPRRnpppprr", 54, 102, 31);
    data(m, "BPPPPPRnpppppr", 137, 277, 126);
    data(m, "BNPPPPPbbppppp", 1, 36, 23);
    data(m, "NPPPPPQbpppppq", 25, 45, 29);
    data(m, "NPPPPRRbpppprr", 27, 87, 43);
    data(m, "BBPPPPPbnppppp", 14, 45, 3);
    data(m, "BPPPPPPnpppppp", 25, 47, 23);
    data(m, "BNPPPPPRbppprr", 14, 28, 15);
    data(m, "BPPPPPPRpppprr", 19, 32, 22);
    data(m, "BBPPPPRbnppppr", 45, 60, 18);
    data(m, "BPPPPPQRpppqrr", 26, 18, 12);
    data(m, "NPPPPPPbpppppp", 29, 51, 17);
    data(m, "BPPPPQRnppppqr", 61, 72, 52);
    data(m, "BPPPPPPRbppppr", 251, 42, 11);
    data(m, "BPPPPPPQbppppq", 50, 13, 7);
    data(m, "PPPPPPPRpppppr", 100, 8, 4);
    data(m, "BPPPPPRppppppr", 236, 4, 4);
    data(m, "NPPPPRRppppprr", 66, 5, 0);
    data(m, "BPPPPPPRnppppr", 139, 13, 6);
    data(m, "BPPPPRRppppprr", 124, 1, 2);
    data(m, "PPPPPPRRpppprr", 177, 23, 5);
    data(m, "NPPPPPPRbppppr", 145, 17, 10);
    data(m, "PPPPQRRbppppqr", 34, 9, 10);
    data(m, "BPPPPQRpppppqr", 107, 7, 3);
    data(m, "NPPPPPPRnppppr", 109, 17, 2);
    data(m, "PPPPPPQRppppqr", 159, 26, 17);
    data(m, "PPPPPRRbpppppr", 132, 15, 3);
    data(m, "NPPPPQRpppppqr", 72, 1, 9);
    data(m, "PPPPPQRRpppqrr", 61, 21, 15);
    data(m, "BNPPPPRbpppppr", 112, 8, 4);
    data(m, "PPPPPRRnpppppr", 62, 5, 0);
    data(m, "NPPPPPRppppppr", 112, 3, 2);
    data(m, "BPPPPPRbnppppp", 55, 2, 0);
    data(m, "BNPPPPPRbbpppr", 35, 19, 3);
    data(m, "BPPPPPRRnppprr", 59, 10, 2);
    data(m, "NPPPPPRRbppprr", 65, 14, 6);
    data(m, "BPPPPRRbnppppr", 58, 7, 1);
    data(m, "BPPPPPRRbppprr", 120, 31, 7);
    data(m, "BPPPPPQRnpppqr", 52, 11, 12);
    data(m, "NPPPPPQRbpppqr", 58, 12, 12);
    data(m, "BNPPPPPRbnpppr", 81, 19, 4);
    data(m, "BPPPPPQRbpppqr", 111, 24, 24);
    data(m, "PPPPPQRbpppppq", 45, 4, 3);
    data(m, "NPPPPPQRnpppqr", 41, 5, 7);
    data(m, "BNPPPPRnpppppr", 81, 4, 3);
    data(m, "NPPPPPRRnppprr", 51, 13, 2);
    data(m, "BPPPPRRbpppppr", 89, 0, 1);
    data(m, "NPPPPPQRppppqr", 68, 1, 4);
    data(m, "BPPPPRRnpppppr", 50, 0, 0);
    data(m, "PPPPPRRppppppr", 96, 0, 0);
    data(m, "BPPPPPRRpppprr", 97, 0, 1);
    data(m, "PPPPPPRRbppppr", 68, 0, 0);
    data(m, "BNPPPPPRbppppr", 119, 0, 0);
    data(m, "BPPPPPQRppppqr", 102, 6, 7);
    data(m, "NPPPPPPRpppppr", 96, 0, 3);
    data(m, "NPPPPPRRpppprr", 60, 0, 2);
    data(m, "BNPPPPPRnppppr", 59, 1, 1);
    data(m, "BPPPPPPRpppppr", 161, 0, 1);
    data(m, "BPPPPPRbpppppp", 64, 0, 0);
    data(m, "PPPPPPRRpppppr", 106, 0, 0);
    data(m, "BPPPPPRRbppppr", 87, 0, 0);
    data(m, "PPPPPQRbpppppr", 55, 1, 2);
    data(m, "BPPPPPPRbppppp", 69, 0, 0);
    data(m, "BPPPPPQRbppppq", 55, 1, 1);
    data(m, "NPPPPPRRbppppr", 59, 0, 0);
    data(m, "BPPPPQRbpppppr", 69, 0, 0);
    data(m, "PPPPPQRppppppr", 53, 0, 0);
    data(m, "BPPPPPQRbppppr", 94, 0, 0);
    data(m, "PPPPPPQRpppppr", 82, 1, 0);
    data(m, "PPPPPQRRpppprr", 60, 0, 0);
    data(m, "PPPPPPRppppqr", 0, 0, 51);
    data(m, "BPPPPPRppppqr", 1, 1, 64);
    data(m, "BPPPPPPbppppr", 1, 2, 80);
    data(m, "PPPPPPRpppprr", 0, 2, 88);
    data(m, "BPPPPPRbppprr", 1, 0, 63);
    data(m, "PPPPPPQppppqr", 1, 2, 51);
    data(m, "BPPPPPRpppprr", 25, 58, 148);
    data(m, "BPPPPQRpppqrr", 8, 17, 40);
    data(m, "BNPPPPPbppppr", 4, 19, 65);
    data(m, "BPPPPPPpppppr", 6, 5, 97);
    data(m, "PPPPPPRbppppr", 15, 26, 188);
    data(m, "PPPPPPRnppppr", 7, 14, 114);
    data(m, "NPPPPPRpppprr", 8, 20, 76);
    data(m, "BNPPPPRbppprr", 12, 19, 31);
    data(m, "BPPPPPQppppqr", 15, 22, 60);
    data(m, "PPPPPRRbppprr", 5, 10, 69);
    data(m, "BPPPPPRbnpppr", 6, 15, 74);
    data(m, "NPPPPPPpppppr", 4, 7, 59);
    data(m, "PPPPPQRbpppqr", 18, 14, 76);
    data(m, "PPPPPQRnpppqr", 7, 10, 61);
    data(m, "PPPPPPQbppppq", 6, 11, 41);
    data(m, "BPPPPPRbbpppr", 8, 6, 43);
    data(m, "BNPPPPPnppppr", 3, 7, 44);
    data(m, "NPPPPPRbnpppr", 7, 13, 39);
    data(m, "NPPPPPQppppqr", 10, 14, 36);
    data(m, "BPPPPPRbppppr", 703, 559, 106);
    data(m, "BPPPPQRbpppqr", 161, 103, 79);
    data(m, "PPPPPPRpppppr", 588, 291, 57);
    data(m, "NPPPPPQbppppq", 55, 39, 14);
    data(m, "PPPPPRRpppprr", 381, 300, 48);
    data(m, "NPPPPPRnppppr", 318, 202, 46);
    data(m, "BPPPPQRnpppqr", 76, 43, 27);
    data(m, "BPPPPPPbppppp", 145, 140, 15);
    data(m, "NPPPPPRbppppr", 368, 227, 82);
    data(m, "NPPPPRRnppprr", 49, 54, 9);
    data(m, "BPPPPPRnppppr", 355, 218, 37);
    data(m, "NPPPPRRbppprr", 67, 79, 20);
    data(m, "PPPPPQRppppqr", 391, 206, 107);
    data(m, "NPPPPQRbpppqr", 68, 42, 17);
    data(m, "BNPPPPRbbpppr", 36, 72, 24);
    data(m, "BNPPPPRbnpppr", 115, 113, 29);
    data(m, "NPPPPPQnppppq", 64, 23, 10);
    data(m, "BBPPPPRbbpppr", 31, 28, 5);
    data(m, "PPPPQRRpppqrr", 76, 47, 39);
    data(m, "BBPPPPRbnpppr", 43, 24, 3);
    data(m, "BPPPPRRbppprr", 143, 105, 31);
    data(m, "BPPPPPQnppppq", 65, 40, 16);
    data(m, "BPPPPPQbppppq", 143, 134, 36);
    data(m, "PPPPPPQpppppq", 129, 69, 22);
    data(m, "NPPPPPPbppppp", 98, 31, 10);
    data(m, "NPPPPPPnppppp", 68, 41, 8);
    data(m, "BBPPPPPbnpppp", 56, 20, 1);
    data(m, "BPPPPPPnppppp", 67, 29, 8);
    data(m, "NPPPPQRnpppqr", 53, 36, 14);
    data(m, "BBPPPPPbbpppp", 50, 26, 5);
    data(m, "BNPPPPRpppprr", 44, 19, 6);
    data(m, "BNPPPPPbnpppp", 105, 55, 7);
    data(m, "BPPPPRRnppprr", 79, 50, 7);
    data(m, "BNPPPPPbbpppp", 35, 62, 3);
    data(m, "NNPPPPRbnpppr", 23, 21, 7);
    data(m, "BNPPPPRnppppr", 118, 2, 2);
    data(m, "NPPPPPRpppppr", 260, 4, 4);
    data(m, "BPPPPPPRbpppr", 56, 4, 2);
    data(m, "PPPPPPQRpppqr", 51, 2, 8);
    data(m, "BNPPPPPbppppp", 95, 2, 0);
    data(m, "BPPPPRRpppprr", 144, 4, 4);
    data(m, "BNPPPPRbppppr", 155, 9, 2);
    data(m, "NPPPPRRpppprr", 89, 6, 3);
    data(m, "BPPPPPRpppppr", 492, 8, 6);
    data(m, "PPPPPRRbppppr", 132, 3, 4);
    data(m, "BBPPPPRbppppr", 69, 1, 0);
    data(m, "BPPPPQRppppqr", 133, 5, 10);
    data(m, "NPPPPQRppppqr", 90, 8, 4);
    data(m, "PPPPPPRbppppp", 73, 1, 1);
    data(m, "BPPPPPQpppppq", 98, 11, 3);
    data(m, "PPPPPQRnppppq", 51, 3, 2);
    data(m, "PPPPPRRnppppr", 90, 2, 1);
    data(m, "BPPPPPPpppppp", 73, 1, 1);
    data(m, "BPPPPPRbnpppp", 53, 4, 2);
    data(m, "PPPPPQRbppppq", 65, 7, 1);
    data(m, "NPPPPPQpppppq", 62, 4, 1);
    data(m, "BBPPPPPbppppp", 68, 0, 0);
    data(m, "NPPPPPPRbpppr", 50, 0, 0);
    data(m, "BNPPPPPnppppp", 56, 0, 0);
    data(m, "PPPPQRRppppqr", 74, 2, 3);
    data(m, "PPPPPQRpppprr", 52, 1, 0);
    data(m, "BPPPPPRnppppp", 119, 0, 1);
    data(m, "BPPPPRRbppppr", 165, 0, 0);
    data(m, "PPPPPQRpppppq", 111, 3, 0);
    data(m, "BPPPPRRnppppr", 63, 0, 0);
    data(m, "BPPPPPRbppppp", 233, 1, 2);
    data(m, "PPPPPRRpppppr", 247, 4, 2);
    data(m, "PPPPPPRpppppp", 107, 0, 0);
    data(m, "NPPPPPRbppppp", 120, 0, 0);
    data(m, "NPPPPRRbppppr", 70, 1, 0);
    data(m, "BPPPPQRbppppq", 71, 2, 2);
    data(m, "NPPPPPRnppppp", 100, 0, 0);
    data(m, "BPPPPPPRppppr", 52, 1, 0);
    data(m, "BPPPPRRpppppr", 55, 1, 0);
    data(m, "PPPPPQRbppppr", 68, 0, 0);
    data(m, "BPPPPQRbppppr", 104, 0, 0);
    data(m, "PPPPQRRpppprr", 61, 0, 3);
    data(m, "PPPPPQRpppppr", 161, 0, 0);
    data(m, "NPPPPQRbppppr", 51, 0, 0);
    data(m, "BPPPPQRnppppr", 53, 0, 0);
    data(m, "BPPPPPQbppppp", 80, 0, 0);
    data(m, "PPPPPRppppqr", 0, 0, 79);
    data(m, "BPPPPPbppppr", 0, 0, 122);
    data(m, "PPPPPQppppqr", 2, 7, 73);
    data(m, "NPPPPPbppppr", 0, 0, 54);
    data(m, "BPPPPPnppppr", 1, 1, 63);
    data(m, "PPPPPRpppprr", 1, 1, 104);
    data(m, "NPPPPPnppppr", 0, 2, 57);
    data(m, "PPPPPPpppppr", 1, 0, 71);
    data(m, "BPPPPRbppprr", 0, 1, 50);
    data(m, "PPPPQRbpppqr", 4, 7, 48);
    data(m, "BPPPPPpppppr", 1, 1, 93);
    data(m, "PPPPPRbppppr", 7, 11, 233);
    data(m, "PPPPPRnppppr", 5, 7, 150);
    data(m, "BPPPPRbnpppr", 3, 2, 57);
    data(m, "BPPPPQppppqr", 5, 9, 49);
    data(m, "BPPPPRpppprr", 7, 15, 103);
    data(m, "NPPPPRpppprr", 2, 11, 59);
    data(m, "PPPPPQbppppq", 2, 11, 62);
    data(m, "NPPPPPpppppr", 2, 0, 59);
    data(m, "NPPPPPRppprr", 22, 20, 25);
    data(m, "BPPPPRnppppr", 125, 302, 115);
    data(m, "NPPPPPbppppp", 36, 85, 40);
    data(m, "BPPPPPRppprr", 45, 37, 30);
    data(m, "BPPPPPPppppr", 12, 26, 25);
    data(m, "BPPPPPnppppp", 56, 90, 58);
    data(m, "NPPPPRbppppr", 120, 270, 121);
    data(m, "BPPPPQnppppq", 32, 52, 20);
    data(m, "BNPPPPbbpppp", 8, 65, 29);
    data(m, "BBPPPPbnpppp", 22, 46, 6);
    data(m, "NPPPQRbpppqr", 18, 27, 19);
    data(m, "BPPPQRnpppqr", 21, 23, 18);
    data(m, "NPPPPPPppppr", 14, 20, 19);
    data(m, "NPPPRRbppprr", 13, 43, 26);
    data(m, "PPPPPPRnpppr", 13, 25, 12);
    data(m, "PPPPPPRbpppr", 14, 33, 26);
    data(m, "BPPPRRnppprr", 17, 40, 20);
    data(m, "BPPPPPQpppqr", 23, 29, 27);
    data(m, "BNPPPPPbpppr", 11, 24, 20);
    data(m, "NPPPPQbppppq", 18, 58, 30);
    data(m, "PPPPPQRpppqr", 161, 42, 40);
    data(m, "NPPPPPRbpppr", 222, 36, 13);
    data(m, "BPPPPPRnpppr", 198, 30, 9);
    data(m, "NPPPPPQbpppq", 45, 10, 7);
    data(m, "BPPPPPPnpppp", 53, 2, 2);
    data(m, "BPPPPPRbpppr", 417, 86, 17);
    data(m, "BPPPPRpppppr", 240, 16, 5);
    data(m, "NPPPPPPnpppp", 57, 1, 1);
    data(m, "PPPPPPRppppr", 352, 36, 13);
    data(m, "BPPPPRRbpprr", 68, 12, 3);
    data(m, "NPPPPPPbpppp", 86, 2, 6);
    data(m, "PPPPRRbppppr", 88, 9, 5);
    data(m, "PPPPQRbppppq", 53, 14, 2);
    data(m, "BPPPRRpppprr", 52, 2, 1);
    data(m, "NPPPPPRnpppr", 171, 35, 10);
    data(m, "BPPPPPpppppp", 51, 4, 0);
    data(m, "BPPPPQRbppqr", 61, 13, 13);
    data(m, "BPPPPPQbpppq", 89, 20, 7);
    data(m, "NPPPPRpppppr", 156, 12, 5);
    data(m, "PPPPPRRppprr", 176, 25, 8);
    data(m, "BPPPPPPbpppp", 122, 28, 0);
    data(m, "PPPPRRnppppr", 55, 1, 1);
    data(m, "BPPPPRbnpppp", 48, 5, 1);
    data(m, "BNPPPRbppppr", 60, 4, 4);
    data(m, "BNPPPPPbnppp", 48, 2, 0);
    data(m, "PPPPPRbppppp", 91, 3, 0);
    data(m, "PPPPPPQppppq", 74, 21, 1);
    data(m, "PPPPPRnppppp", 48, 1, 1);
    data(m, "BPPPQRppppqr", 46, 8, 4);
    data(m, "NPPPPPQppppq", 47, 3, 0);
    data(m, "PPPPRRpppppr", 105, 0, 1);
    data(m, "PPPPPRRbpppr", 72, 0, 0);
    data(m, "NPPPPPRppppr", 199, 3, 2);
    data(m, "BNPPPPRbpppr", 77, 1, 1);
    data(m, "BPPPPPRppppr", 251, 0, 2);
    data(m, "PPPPQRpppppq", 59, 5, 3);
    data(m, "BPPPPRbppppp", 120, 3, 0);
    data(m, "BPPPPRnppppp", 72, 2, 0);
    data(m, "BPPPPQRpppqr", 90, 6, 8);
    data(m, "PPPPPPRbpppp", 52, 1, 0);
    data(m, "BPPPPPPppppp", 51, 0, 1);
    data(m, "BNPPPPPbpppp", 54, 0, 1);
    data(m, "NPPPPRbppppp", 53, 1, 0);
    data(m, "BPPPPPQppppq", 73, 2, 0);
    data(m, "PPPPPRpppppp", 69, 0, 0);
    data(m, "BNPPPPRnpppr", 44, 3, 3);
    data(m, "BPPPPPRnpppp", 74, 0, 0);
    data(m, "PPPPPRRppppr", 163, 1, 1);
    data(m, "NPPPPPRnpppp", 63, 1, 0);
    data(m, "PPPPPPRppppp", 83, 0, 0);
    data(m, "BPPPPPRbpppp", 146, 1, 0);
    data(m, "PPPPPQRppppq", 81, 4, 0);
    data(m, "NPPPPPRbpppp", 88, 1, 1);
    data(m, "BPPPPRRbpppr", 49, 0, 2);
    data(m, "BPPPPPRppppp", 55, 0, 0);
    data(m, "PPPPQRpppppr", 87, 0, 0);
    data(m, "PPPPPQRppppr", 150, 0, 1);
    data(m, "PPPPPPQppppp", 54, 0, 0);
    data(m, "BPPPPPQbpppp", 76, 0, 0);
    data(m, "BPPPPQRbpppr", 59, 0, 0);
    data(m, "PPPPPRpppqr", 0, 0, 72);
    data(m, "BPPPPPppppq", 1, 0, 50);
    data(m, "BPPPPRpppqr", 1, 0, 53);
    data(m, "PPPPPPppppr", 4, 0, 94);
    data(m, "BPPPPPnpppr", 4, 2, 76);
    data(m, "NPPPPPnpppr", 0, 1, 54);
    data(m, "BPPPPPbpppr", 1, 2, 111);
    data(m, "PPPPPQpppqr", 2, 10, 96);
    data(m, "NPPPPPbpppr", 1, 1, 74);
    data(m, "PPPPPRppprr", 6, 1, 126);
    data(m, "PPPPPRppppq", 3, 1, 51);
    data(m, "PPPPPRnpppr", 22, 52, 134);
    data(m, "BPPPPQpppqr", 29, 46, 76);
    data(m, "BPPPPRppppq", 3, 15, 36);
    data(m, "BPPPPPppppr", 24, 38, 164);
    data(m, "BPPPPRppprr", 31, 72, 118);
    data(m, "PPPPPRbpppr", 30, 63, 215);
    data(m, "PPPPQRbppqr", 10, 8, 39);
    data(m, "NPPPPQpppqr", 12, 32, 45);
    data(m, "PPPPPQbpppq", 12, 35, 87);
    data(m, "NPPPPPppppr", 10, 23, 97);
    data(m, "BPPPPRbnppr", 7, 13, 37);
    data(m, "NPPPPRppprr", 11, 30, 70);
    data(m, "BNPPPPnpppr", 2, 16, 39);
    data(m, "PPPPPQnpppq", 2, 18, 43);
    data(m, "BNPPPPbpppr", 3, 21, 60);
    data(m, "PPPPPPbpppp", 19, 5, 55);
    data(m, "NPPPPPbpppp", 235, 88, 31);
    data(m, "BPPPPQbpppq", 154, 165, 49);
    data(m, "BPPPPRbpppr", 757, 727, 136);
    data(m, "NPPPPPnpppp", 196, 71, 19);
    data(m, "NPPPPRnpppr", 377, 308, 71);
    data(m, "PPPPRRppprr", 306, 333, 55);
    data(m, "BPPPRRbpprr", 54, 51, 19);
    data(m, "PPPPQRpppqr", 329, 194, 131);
    data(m, "NPPPPRbpppr", 377, 320, 60);
    data(m, "BNPPPPbbppp", 46, 103, 13);
    data(m, "PPPPPRppppr", 1164, 658, 108);
    data(m, "BPPPPRnpppr", 436, 266, 48);
    data(m, "BPPPPPbpppp", 398, 345, 52);
    data(m, "NPPPRRbpprr", 28, 32, 15);
    data(m, "BPPPPPnpppp", 193, 95, 20);
    data(m, "NPPPPQnpppq", 61, 43, 13);
    data(m, "BNPPPRbnppr", 39, 49, 7);
    data(m, "PPPPPQppppq", 270, 258, 48);
    data(m, "PPPPPPppppp", 213, 36, 33);
    data(m, "NPPPPQbpppq", 94, 49, 16);
    data(m, "BBPPPPbbppp", 46, 36, 6);
    data(m, "BPPPRRnpprr", 28, 21, 4);
    data(m, "BNPPPPppppr", 50, 21, 8);
    data(m, "BPPPPQnpppq", 80, 46, 17);
    data(m, "BNPPPRppprr", 39, 13, 3);
    data(m, "BNPPPPbnppp", 114, 97, 8);
    data(m, "BPPPQRbppqr", 42, 23, 21);
    data(m, "BBPPPPbnppp", 69, 32, 2);
    data(m, "PPPPRRppppq", 11, 33, 8);
    data(m, "BBPPPRbpppr", 50, 2, 1);
    data(m, "PPPPPRbpppp", 203, 2, 0);
    data(m, "PPPPQRbpppq", 65, 8, 6);
    data(m, "NPPPPRppppr", 409, 10, 7);
    data(m, "PPPPPQRppqr", 86, 13, 13);
    data(m, "BPPPPRppppr", 643, 13, 11);
    data(m, "NPPPPPRbppr", 77, 1, 2);
    data(m, "PPPPPPQpppq", 62, 7, 2);
    data(m, "PPPPRRnpppr", 100, 2, 1);
    data(m, "BNPPPRbpppr", 105, 5, 2);
    data(m, "NPPPPQppppq", 83, 14, 3);
    data(m, "BPPPRRppprr", 68, 4, 2);
    data(m, "BNPPPPbpppp", 120, 3, 1);
    data(m, "PPPPPPRpppr", 136, 3, 3);
    data(m, "BNPPPPnpppp", 68, 0, 0);
    data(m, "PPPPPRnpppp", 112, 1, 4);
    data(m, "BBPPPPbpppp", 83, 0, 0);
    data(m, "PPPPRRbpppr", 140, 7, 2);
    data(m, "BPPPPPPbppp", 57, 2, 0);
    data(m, "NPPPQRpppqr", 57, 5, 10);
    data(m, "BNPPPRnpppr", 55, 2, 1);
    data(m, "BPPPPPRnppr", 80, 5, 3);
    data(m, "BPPPPPRbppr", 155, 9, 1);
    data(m, "NPPPPPppppp", 124, 1, 8);
    data(m, "NPPPRRppprr", 55, 3, 3);
    data(m, "BPPPPQppppq", 153, 9, 6);
    data(m, "BPPPPPppppp", 175, 0, 8);
    data(m, "BPPPQRpppqr", 78, 4, 10);
    data(m, "NPPPPPRnppr", 57, 1, 1);
    data(m, "PPPPPRRpprr", 51, 1, 4);
    data(m, "BPPPPRbnppp", 57, 0, 2);
    data(m, "PPPPPRppppp", 321, 1, 0);
    data(m, "BPPPPRbpppp", 388, 3, 0);
    data(m, "PPPPPQppppr", 80, 3, 0);
    data(m, "BPPPPPRpppr", 143, 1, 0);
    data(m, "BPPPPRnpppp", 163, 0, 0);
    data(m, "NPPPPRnpppp", 149, 0, 1);
    data(m, "PPPPRRppppr", 256, 1, 1);
    data(m, "PPPPQRppppq", 172, 10, 5);
    data(m, "NPPPPPRpppr", 77, 1, 0);
    data(m, "NPPPPRbpppp", 185, 0, 1);
    data(m, "BPPPRRbpppr", 71, 1, 0);
    data(m, "BPPPRRnpppr", 49, 0, 1);
    data(m, "BPPPPPRbppp", 81, 0, 0);
    data(m, "PPPPQRbpppr", 74, 0, 0);
    data(m, "BPPPPQppppr", 54, 0, 0);
    data(m, "PPPPPPRpppp", 55, 0, 0);
    data(m, "PPPPPQRpppq", 58, 1, 1);
    data(m, "PPPPPQbpppp", 65, 1, 0);
    data(m, "PPPPPRRpppr", 64, 0, 0);
    data(m, "PPPPQRppppr", 240, 1, 2);
    data(m, "BPPPPQnpppp", 52, 1, 0);
    data(m, "BPPPQRbpppr", 66, 0, 1);
    data(m, "BPPPPQbpppp", 129, 0, 0);
    data(m, "BPPPPPRpppp", 65, 0, 0);
    data(m, "PPPPPQppppp", 130, 0, 0);
    data(m, "PPPPPQRpppr", 61, 0, 1);
    data(m, "PPPPPQRpppp", 54, 0, 0);
    data(m, "BPPPPbpppq", 0, 0, 70);
    data(m, "PPPPRpppqr", 1, 0, 107);
    data(m, "PPPPPppppq", 0, 0, 74);
    data(m, "BPPPPppppq", 0, 0, 58);
    data(m, "PPPPPppppr", 1, 1, 177);
    data(m, "BPPPPbpppr", 0, 0, 182);
    data(m, "PPPPRppprr", 1, 4, 127);
    data(m, "PPPPQpppqr", 3, 7, 94);
    data(m, "PPPPRppppq", 2, 0, 59);
    data(m, "BPPPPnpppr", 1, 1, 78);
    data(m, "NPPPPnpppr", 3, 1, 77);
    data(m, "NPPPPbpppr", 0, 1, 77);
    data(m, "PPPPRbpppr", 14, 27, 335);
    data(m, "PPPPQbpppq", 7, 19, 90);
    data(m, "BPPPQpppqr", 8, 16, 55);
    data(m, "PPPPRnpppr", 4, 22, 220);
    data(m, "BPPPPppppr", 4, 11, 197);
    data(m, "PPPPPnpppp", 9, 1, 62);
    data(m, "NPPPRppprr", 5, 13, 67);
    data(m, "BPPPPPbppr", 2, 5, 58);
    data(m, "BPPPRppprr", 8, 18, 99);
    data(m, "BNPPPbpppr", 3, 5, 53);
    data(m, "PPPPPbpppp", 8, 2, 127);
    data(m, "NPPPPppppr", 4, 3, 112);
    data(m, "PPPPPQppqr", 4, 9, 44);
    data(m, "PPPPQnpppq", 6, 10, 48);
    data(m, "BPPPPbnppp", 1, 4, 55);
    data(m, "NPPPQpppqr", 1, 9, 44);
    data(m, "BPPPPPpppr", 31, 69, 60);
    data(m, "BPPPRnpppr", 98, 360, 105);
    data(m, "PPPPPRnppr", 23, 46, 23);
    data(m, "BPPPPRpprr", 41, 81, 35);
    data(m, "BPPPQnpppq", 24, 40, 23);
    data(m, "NPPPPRpprr", 22, 36, 23);
    data(m, "BPPPPQppqr", 24, 36, 22);
    data(m, "NPPPPbpppp", 88, 157, 98);
    data(m, "NPPPRbpppr", 96, 363, 127);
    data(m, "NPPPQbpppq", 23, 57, 22);
    data(m, "BBPPPbnppp", 22, 57, 1);
    data(m, "BPPPPnpppp", 95, 171, 71);
    data(m, "BNPPPbbppp", 10, 56, 35);
    data(m, "PPPPPRbppr", 21, 55, 42);
    data(m, "PPPPPQbppq", 9, 27, 16);
    data(m, "PPPPPPbppp", 19, 14, 28);
    data(m, "NPPPPPpppr", 22, 34, 37);
    data(m, "PPPPRbnppp", 19, 37, 10);
    data(m, "BNPPPPbppr", 9, 26, 20);
    data(m, "PPPPRbpppp", 198, 6, 8);
    data(m, "NPPPRppppr", 188, 22, 8);
    data(m, "BPPPPRnppr", 267, 48, 6);
    data(m, "NPPPPRnppr", 251, 40, 12);
    data(m, "PPPPPQpppq", 237, 65, 6);
    data(m, "NPPPPQbppq", 49, 19, 10);
    data(m, "BPPPPPnppp", 176, 7, 2);
    data(m, "BPPPQppppq", 80, 12, 3);
    data(m, "BPPPPPbppp", 363, 106, 12);
    data(m, "NPPPPPnppp", 132, 15, 1);
    data(m, "BPPPPRbppr", 585, 135, 18);
    data(m, "PPPPRRpprr", 237, 43, 15);
    data(m, "NPPPPPbppp", 191, 12, 3);
    data(m, "BNPPPPbnpp", 79, 9, 3);
    data(m, "BPPPPQnppq", 47, 9, 5);
    data(m, "NPPPQppppq", 44, 14, 3);
    data(m, "BBPPPbpppp", 50, 0, 1);
    data(m, "PPPPPRpppr", 981, 79, 25);
    data(m, "PPPQRbpppq", 50, 18, 8);
    data(m, "NPPPPRbppr", 294, 52, 22);
    data(m, "BPPPPQbppq", 112, 33, 12);
    data(m, "NPPPPQnppq", 39, 16, 2);
    data(m, "BPPPRppppr", 339, 28, 11);
    data(m, "PPPPPPpppp", 180, 4, 6);
    data(m, "PPPRRnpppr", 64, 12, 3);
    data(m, "BNPPPPbbpp", 31, 22, 2);
    data(m, "BPPPPppppp", 109, 2, 7);
    data(m, "PPPRRbpppr", 89, 19, 3);
    data(m, "NPPPPppppp", 78, 1, 6);
    data(m, "PPPPQRppqr", 194, 38, 27);
    data(m, "BNPPPbpppp", 49, 5, 1);
    data(m, "PPPPRnpppp", 119, 3, 6);
    data(m, "BNPPPPpppr", 55, 5, 1);
    data(m, "BPPPRbnppp", 47, 2, 2);
    data(m, "PPPQRppppq", 81, 5, 3);
    data(m, "NPPPRnpppp", 77, 1, 1);
    data(m, "BPPPPRpppr", 549, 5, 5);
    data(m, "NPPPPRpppr", 340, 8, 1);
    data(m, "BPPPPQpppq", 131, 6, 2);
    data(m, "PPPPRppppp", 187, 0, 3);
    data(m, "PPPRRppppr", 147, 1, 2);
    data(m, "BNPPPPbppp", 108, 1, 2);
    data(m, "NPPPRbpppp", 107, 0, 0);
    data(m, "BPPPRnpppp", 87, 0, 1);
    data(m, "BNPPPRbppr", 63, 0, 0);
    data(m, "BPPPRbpppp", 166, 1, 2);
    data(m, "BPPPPPpppp", 178, 0, 6);
    data(m, "NPPPPQpppq", 93, 4, 3);
    data(m, "NPPPPPpppp", 97, 0, 3);
    data(m, "PPPPPRbppp", 137, 1, 0);
    data(m, "BBPPPPbppp", 66, 0, 0);
    data(m, "PPPPQppppr", 71, 0, 0);
    data(m, "PPPPPRnppp", 68, 0, 2);
    data(m, "PPPPPPRppr", 75, 0, 0);
    data(m, "PPPPRRbppr", 64, 2, 3);
    data(m, "BNPPPPnppp", 60, 0, 2);
    data(m, "PPPPRRnppr", 54, 1, 0);
    data(m, "BPPPPRbppp", 278, 0, 0);
    data(m, "PPPPRRpppr", 238, 1, 0);
    data(m, "PPPPQRpppq", 143, 9, 4);
    data(m, "PPPPPRpppp", 308, 0, 1);
    data(m, "NPPPPRbppp", 134, 0, 1);
    data(m, "BPPPPPRppr", 90, 1, 1);
    data(m, "BPPPPRnppp", 129, 0, 0);
    data(m, "NPPPPRnppp", 115, 0, 0);
    data(m, "PPPPPQpppr", 88, 0, 1);
    data(m, "BPPPPRpppp", 128, 0, 0);
    data(m, "PPPPPQbppp", 60, 0, 0);
    data(m, "PPPQRppppr", 94, 0, 0);
    data(m, "BPPPQbpppp", 65, 0, 0);
    data(m, "NPPPPRpppp", 70, 0, 0);
    data(m, "PPPPQppppp", 81, 1, 0);
    data(m, "PPPPPQpppp", 164, 0, 0);
    data(m, "PPPPQRpppr", 229, 0, 0);
    data(m, "BPPPPQbppp", 101, 0, 0);
    data(m, "PPPPQRbppp", 50, 0, 0);
    data(m, "BPPPPQpppp", 60, 0, 0);
    data(m, "PPPPQRpppp", 100, 0, 0);
    data(m, "PPPPPpppq", 0, 0, 165);
    data(m, "PPPPRppqr", 1, 1, 171);
    data(m, "BPPPPbppq", 1, 0, 90);
    data(m, "BPPPRppqr", 0, 2, 63);
    data(m, "BPPPPpppq", 0, 0, 122);
    data(m, "NPPPPpppq", 1, 2, 66);
    data(m, "BPPPPbppr", 3, 1, 164);
    data(m, "PPPPPpppr", 16, 2, 257);
    data(m, "PPPPRpppq", 2, 6, 144);
    data(m, "PPPPQppqr", 5, 13, 121);
    data(m, "PPPPRpprr", 3, 4, 161);
    data(m, "NPPPPbppr", 5, 4, 89);
    data(m, "NPPPPnppr", 2, 2, 81);
    data(m, "BPPPPnppr", 3, 1, 94);
    data(m, "PPPPQbppq", 12, 54, 85);
    data(m, "BPPPRpprr", 25, 99, 93);
    data(m, "NPPPQppqr", 18, 34, 40);
    data(m, "BPPPPpppr", 54, 107, 284);
    data(m, "NPPPPpppr", 21, 37, 153);
    data(m, "NPPPRpprr", 14, 68, 68);
    data(m, "PPPPRbppr", 37, 142, 299);
    data(m, "PPPPQnppq", 8, 42, 41);
    data(m, "PPPPPbppp", 35, 25, 164);
    data(m, "PPPPPnppp", 24, 26, 72);
    data(m, "BBPPPbppr", 6, 27, 28);
    data(m, "PPPPRnppr", 26, 106, 152);
    data(m, "BPPPRpppq", 7, 24, 72);
    data(m, "BNPPPbppr", 11, 32, 68);
    data(m, "NPPPPbnpp", 2, 20, 28);
    data(m, "BPPPPbnpp", 7, 29, 36);
    data(m, "BPPPQppqr", 19, 36, 46);
    data(m, "NPPPRpppq", 1, 12, 53);
    data(m, "BBPPPbbpp", 38, 47, 4);
    data(m, "NPPPRbppr", 409, 416, 104);
    data(m, "PPPPRpppr", 2360, 1927, 278);
    data(m, "BPPPRbppr", 743, 934, 110);
    data(m, "PPPPPpppp", 619, 78, 98);
    data(m, "BPPPPbppp", 733, 860, 87);
    data(m, "NPPPQbppq", 65, 49, 17);
    data(m, "BPPPRnppr", 416, 346, 46);
    data(m, "PPPPQpppq", 592, 545, 129);
    data(m, "NPPPRnppr", 369, 370, 72);
    data(m, "NPPPPbppp", 421, 194, 56);
    data(m, "PPPQRppqr", 217, 125, 93);
    data(m, "NPPPPnppp", 353, 211, 31);
    data(m, "BPPPQbppq", 117, 121, 33);
    data(m, "NPPPQnppq", 51, 36, 14);
    data(m, "BNPPPpppr", 71, 60, 22);
    data(m, "BNPPPbbpp", 31, 128, 14);
    data(m, "BPPPPnppp", 408, 151, 29);
    data(m, "BNPPPbnpp", 78, 103, 7);
    data(m, "PPPRRpprr", 331, 325, 53);
    data(m, "BPPPPPppr", 47, 29, 9);
    data(m, "PPPRRpppq", 19, 40, 22);
    data(m, "PPPPQpprr", 37, 10, 6);
    data(m, "BPPPQnppq", 56, 62, 19);
    data(m, "BBPPPpppr", 40, 12, 0);
    data(m, "BBPPPbnpp", 71, 28, 2);
    data(m, "PPPPRbnpp", 52, 27, 6);
    data(m, "BPPPPRprr", 33, 24, 5);
    data(m, "PPPPPRppr", 466, 8, 11);
    data(m, "NPPPRpppr", 667, 27, 12);
    data(m, "BNPPRbppr", 52, 6, 1);
    data(m, "NPPPPRbpr", 135, 6, 1);
    data(m, "PPPQRbppq", 64, 8, 8);
    data(m, "NPPPQpppq", 147, 13, 1);
    data(m, "BPPPPpppp", 456, 1, 9);
    data(m, "PPPPRbppp", 386, 3, 15);
    data(m, "PPPPQbppr", 65, 2, 2);
    data(m, "BPPPPPbpp", 151, 6, 2);
    data(m, "PPPQRnppq", 45, 5, 4);
    data(m, "PPPRRnppr", 87, 7, 2);
    data(m, "BBPPPbppp", 128, 4, 1);
    data(m, "PPPPPQppq", 135, 21, 2);
    data(m, "BPPPRpppr", 983, 34, 12);
    data(m, "BNPPPnppp", 83, 0, 1);
    data(m, "NPPPPPbpp", 113, 0, 2);
    data(m, "PPPPRnppp", 242, 2, 4);
    data(m, "BPPPQpppq", 207, 25, 7);
    data(m, "NPPPPpppp", 261, 0, 20);
    data(m, "PPPPQRpqr", 52, 2, 6);
    data(m, "BNPPPbppp", 164, 5, 4);
    data(m, "BPPPPPnpp", 74, 1, 2);
    data(m, "BPPPPRbpr", 228, 12, 2);
    data(m, "BPPPPRnpr", 106, 1, 4);
    data(m, "PPPRRbppr", 157, 6, 8);
    data(m, "BPPPRbnpp", 69, 3, 3);
    data(m, "BPPRRpprr", 52, 1, 1);
    data(m, "NPPPPPnpp", 72, 0, 0);
    data(m, "BNPPRnppr", 43, 6, 3);
    data(m, "PPPPPPppp", 114, 2, 2);
    data(m, "NPPPPRnpr", 87, 3, 1);
    data(m, "PPPPRRprr", 69, 6, 2);
    data(m, "PPPRRpppr", 359, 4, 3);
    data(m, "NPPPPQppq", 58, 2, 1);
    data(m, "BPPPPRppr", 354, 2, 0);
    data(m, "BPPPRbppp", 482, 1, 1);
    data(m, "BPPPPPppp", 135, 0, 1);
    data(m, "PPPPRpppp", 734, 1, 6);
    data(m, "PPPQRpppq", 221, 10, 3);
    data(m, "NPPPRnppp", 202, 3, 2);
    data(m, "PPPPPRbpp", 89, 0, 1);
    data(m, "PPPPQpppr", 231, 0, 0);
    data(m, "NPPPPRppr", 189, 4, 0);
    data(m, "NPPPRbppp", 230, 1, 1);
    data(m, "BPPPRnppp", 229, 2, 0);
    data(m, "BNPPPPbpp", 69, 1, 0);
    data(m, "NPPPPPppp", 75, 0, 3);
    data(m, "BPPPPQppq", 111, 6, 1);
    data(m, "PPPPPRppp", 196, 1, 0);
    data(m, "PPPPRRppr", 94, 1, 0);
    data(m, "NPPPPRnpp", 63, 0, 0);
    data(m, "PPPPQbppp", 129, 0, 1);
    data(m, "PPPPQRppq", 85, 5, 1);
    data(m, "BPPPPRnpp", 63, 0, 0);
    data(m, "BPPPPRbpp", 147, 0, 0);
    data(m, "BPPPRpppp", 128, 1, 0);
    data(m, "PPPPQnppp", 71, 0, 1);
    data(m, "NPPPRpppp", 67, 0, 0);
    data(m, "PPPRRbppp", 53, 0, 0);
    data(m, "BPPPQpppr", 80, 0, 0);
    data(m, "PPPPPQppr", 77, 0, 0);
    data(m, "NPPPPRbpp", 53, 0, 0);
    data(m, "PPPQRpppr", 317, 1, 0);
    data(m, "PPPRRpppp", 60, 0, 0);
    data(m, "PPPPQpppp", 410, 0, 0);
    data(m, "BPPPPRppp", 123, 0, 0);
    data(m, "BPPPQbppp", 163, 0, 0);
    data(m, "NPPPQbppp", 67, 0, 0);
    data(m, "BPPPQnppp", 80, 0, 0);
    data(m, "NPPPPRppp", 71, 0, 0);
    data(m, "PPPQRnppp", 52, 0, 0);
    data(m, "BPPPPQbpp", 66, 0, 0);
    data(m, "BPPQRpppr", 54, 0, 0);
    data(m, "PPPPQRppr", 144, 0, 0);
    data(m, "BPPPQpppp", 78, 0, 0);
    data(m, "PPPPPQppp", 186, 0, 0);
    data(m, "NPPPQpppp", 51, 0, 0);
    data(m, "PPPQRbppp", 70, 0, 0);
    data(m, "BPPPPQppp", 95, 0, 0);
    data(m, "PPPQRpppp", 112, 0, 0);
    data(m, "PPPPQRppp", 111, 0, 0);
    data(m, "PPPPppqr", 0, 0, 54);
    data(m, "PPPPpppq", 1, 0, 270);
    data(m, "PPPRppqr", 0, 1, 159);
    data(m, "BPPPbppq", 0, 1, 78);
    data(m, "PPPPRpqr", 1, 0, 100);
    data(m, "NPPPpppq", 1, 1, 68);
    data(m, "PPPPPppq", 0, 0, 80);
    data(m, "BPPPpppq", 0, 0, 124);
    data(m, "PPPPbppr", 0, 1, 51);
    data(m, "PPPRpppq", 3, 2, 156);
    data(m, "NPPPbppr", 3, 1, 98);
    data(m, "PPPPpppr", 6, 2, 497);
    data(m, "BPPPPppq", 1, 1, 98);
    data(m, "BPPPbppr", 3, 0, 215);
    data(m, "BPPPnppr", 0, 2, 112);
    data(m, "PPPRpprr", 0, 1, 190);
    data(m, "NPPPnppr", 0, 3, 80);
    data(m, "PPPQppqr", 2, 11, 106);
    data(m, "PPPPRprr", 5, 4, 43);
    data(m, "PPPPbppp", 21, 7, 296);
    data(m, "PPPRnppr", 18, 65, 254);
    data(m, "PPPRbppr", 15, 72, 404);
    data(m, "BPPRpppq", 2, 14, 42);
    data(m, "PPPPRppq", 4, 13, 92);
    data(m, "BPPQppqr", 6, 13, 33);
    data(m, "NPPPpppr", 11, 16, 190);
    data(m, "NPPRpprr", 4, 14, 77);
    data(m, "BNPPbppr", 3, 6, 47);
    data(m, "BPPPpppr", 16, 28, 320);
    data(m, "PPPPPppr", 9, 5, 137);
    data(m, "BPPRpprr", 6, 26, 106);
    data(m, "PPPQnppq", 6, 14, 48);
    data(m, "BPPPbnpp", 3, 11, 55);
    data(m, "PPPQbppq", 3, 22, 97);
    data(m, "PPPPQpqr", 3, 2, 50);
    data(m, "BPPPPbpr", 3, 6, 59);
    data(m, "PPPPnppp", 12, 15, 145);
    data(m, "NPPPPppr", 39, 75, 50);
    data(m, "NPPRbppr", 62, 344, 92);
    data(m, "PPPPQbpq", 6, 41, 14);
    data(m, "NPPPbppp", 105, 322, 150);
    data(m, "BPPPnppp", 137, 326, 99);
    data(m, "BPPPRprr", 56, 85, 20);
    data(m, "PPPPRnpr", 36, 89, 17);
    data(m, "BNPPPbpr", 10, 31, 12);
    data(m, "PPPPPbpp", 76, 53, 42);
    data(m, "BPPRnppr", 82, 334, 62);
    data(m, "BPPPPppr", 76, 152, 76);
    data(m, "PPPPRbpr", 47, 127, 42);
    data(m, "BBPPbnpp", 14, 51, 3);
    data(m, "NPPPRprr", 16, 44, 16);
    data(m, "NPPQbppq", 16, 35, 16);
    data(m, "PPPRbnpp", 28, 57, 11);
    data(m, "NPPPRppq", 8, 28, 24);
    data(m, "BPPPRppq", 7, 48, 45);
    data(m, "BNPPpppr", 9, 34, 30);
    data(m, "BNPPbbpp", 5, 63, 20);
    data(m, "PPPPPnpp", 35, 26, 8);
    data(m, "BPPQnppq", 18, 26, 9);
    data(m, "PPPPRppr", 2214, 290, 73);
    data(m, "BPPRpppr", 408, 54, 23);
    data(m, "NPPPPbpp", 358, 36, 11);
    data(m, "PPPPQppq", 537, 171, 39);
    data(m, "NPPPpppp", 134, 7, 17);
    data(m, "NPPPRnpr", 251, 82, 14);
    data(m, "PPPPPppp", 608, 7, 30);
    data(m, "PPPRbppp", 331, 30, 18);
    data(m, "NPPRpppr", 285, 54, 14);
    data(m, "NPPPRbpr", 271, 73, 10);
    data(m, "BPPPRbpr", 565, 190, 14);
    data(m, "NPPPPnpp", 295, 16, 8);
    data(m, "BPPPPbpp", 661, 230, 18);
    data(m, "PPPQRpqr", 88, 19, 21);
    data(m, "BPPPRnpr", 288, 68, 11);
    data(m, "BPPPpppp", 266, 14, 13);
    data(m, "BPPPQbpq", 83, 36, 5);
    data(m, "PPPRnppp", 185, 24, 13);
    data(m, "BPPPPnpp", 293, 25, 8);
    data(m, "BNPPbppp", 61, 9, 2);
    data(m, "BPPPQnpq", 35, 21, 1);
    data(m, "PPPRRprr", 208, 54, 18);
    data(m, "PPPQbppr", 45, 8, 2);
    data(m, "BBPPbppp", 47, 5, 3);
    data(m, "BBPPPppr", 45, 5, 0);
    data(m, "BPPQpppq", 103, 30, 4);
    data(m, "BNPPPbnp", 57, 21, 1);
    data(m, "PPRRbppr", 85, 21, 4);
    data(m, "PPRRnppr", 56, 13, 3);
    data(m, "BNPPPbbp", 30, 21, 3);
    data(m, "NPPPQbpq", 43, 18, 5);
    data(m, "BNPPPppr", 96, 29, 3);
    data(m, "NPPQpppq", 47, 11, 8);
    data(m, "BPPPPppp", 431, 1, 10);
    data(m, "BPPPRppr", 823, 9, 6);
    data(m, "PPPPRnpp", 170, 0, 4);
    data(m, "BPPRnppp", 88, 1, 0);
    data(m, "NPPPPppp", 261, 1, 6);
    data(m, "PPPRpppp", 471, 2, 6);
    data(m, "PPPPQbpr", 62, 1, 1);
    data(m, "NPPPRppr", 563, 11, 4);
    data(m, "PPRRpppr", 174, 7, 2);
    data(m, "BNPPPnpp", 59, 0, 2);
    data(m, "PPPPRbpp", 286, 1, 2);
    data(m, "BNPPPbpp", 131, 0, 4);
    data(m, "BBPPPbpp", 76, 0, 1);
    data(m, "BPPPQppq", 208, 19, 4);
    data(m, "PPPPPPpp", 78, 0, 2);
    data(m, "BPPRbppp", 200, 3, 0);
    data(m, "PPPPPQpq", 39, 8, 3);
    data(m, "PPPQpppr", 161, 2, 2);
    data(m, "PPPPPRpr", 195, 2, 0);
    data(m, "PPPRRnpr", 70, 2, 2);
    data(m, "BPPPPPbp", 63, 1, 1);
    data(m, "PPQRpppq", 94, 9, 3);
    data(m, "NPPRnppp", 102, 0, 1);
    data(m, "BPPPPRbr", 59, 0, 0);
    data(m, "PPPRRbpr", 83, 4, 0);
    data(m, "NPPPQppq", 117, 6, 1);
    data(m, "NPPRbppp", 99, 1, 1);
    data(m, "BPPPRbnp", 51, 1, 0);
    data(m, "PPPQRppq", 156, 8, 1);
    data(m, "PPPPRppp", 814, 1, 3);
    data(m, "BPPPRnpp", 154, 0, 0);
    data(m, "PPPPQppr", 211, 3, 2);
    data(m, "PPPRRppr", 293, 0, 1);
    data(m, "BPPPRbpp", 368, 2, 0);
    data(m, "NPPPPRpr", 129, 1, 0);
    data(m, "BNPPPppp", 64, 2, 1);
    data(m, "BPPPPQpq", 57, 2, 0);
    data(m, "NPPPRnpp", 159, 2, 0);
    data(m, "NPPPRbpp", 174, 1, 0);
    data(m, "PPPQbppp", 121, 0, 0);
    data(m, "BPPPPPpp", 83, 0, 0);
    data(m, "BPPPPRpr", 187, 0, 0);
    data(m, "BNPPRppr", 49, 1, 0);
    data(m, "PPPQnppp", 59, 0, 3);
    data(m, "BPPPRppp", 256, 0, 2);
    data(m, "PPPPQbpp", 126, 0, 0);
    data(m, "PPPPRRpr", 65, 0, 1);
    data(m, "PPPQpppp", 216, 0, 0);
    data(m, "BPPPPRbp", 66, 0, 0);
    data(m, "PPPPPQpr", 80, 0, 0);
    data(m, "PPPPPRpp", 113, 1, 0);
    data(m, "NPPPRppp", 137, 0, 0);
    data(m, "PPQRpppr", 176, 0, 2);
    data(m, "BPPPQppr", 87, 1, 1);
    data(m, "BPPQbppp", 74, 0, 0);
    data(m, "PPPPQRpq", 51, 2, 0);
    data(m, "PPPPQppp", 614, 0, 0);
    data(m, "NPPPQbpp", 55, 0, 0);
    data(m, "PPPQRppr", 242, 0, 0);
    data(m, "BPPPPRpp", 116, 0, 0);
    data(m, "BPPPQnpp", 56, 1, 0);
    data(m, "BPPPQbpp", 158, 0, 0);
    data(m, "PPPRRppp", 79, 0, 0);
    data(m, "NPPPPRpp", 52, 0, 0);
    data(m, "BPPPQppp", 185, 0, 0);
    data(m, "PPPQRbpp", 81, 0, 0);
    data(m, "NPPPQppp", 92, 0, 0);
    data(m, "PPPPPQpp", 154, 0, 0);
    data(m, "PPPPQRpr", 72, 0, 1);
    data(m, "BPPPPQpp", 131, 0, 0);
    data(m, "PPPQRppp", 222, 0, 0);
    data(m, "PPPPQRpp", 107, 0, 0);
    data(m, "PPPPppq", 0, 2, 344);
    data(m, "PPPRpqr", 1, 2, 301);
    data(m, "BPPPbpq", 0, 2, 118);
    data(m, "BPPPppq", 0, 3, 233);
    data(m, "NPPPppq", 0, 0, 125);
    data(m, "PPPPbpr", 0, 0, 61);
    data(m, "PPPPppr", 29, 13, 610);
    data(m, "PPPRppq", 3, 18, 273);
    data(m, "NPPPbpr", 1, 10, 105);
    data(m, "PPPQpqr", 4, 6, 96);
    data(m, "NPPPnpr", 1, 5, 74);
    data(m, "BPPPbpr", 4, 13, 192);
    data(m, "PPPRprr", 5, 8, 159);
    data(m, "BPPPnpr", 0, 5, 83);
    data(m, "PPPPRpq", 2, 21, 44);
    data(m, "NPPPppr", 38, 128, 261);
    data(m, "BPPPppr", 80, 257, 360);
    data(m, "PPPQnpq", 4, 40, 18);
    data(m, "PPPPnpp", 67, 93, 104);
    data(m, "PPPRbpr", 36, 300, 227);
    data(m, "PPPQbpq", 13, 85, 49);
    data(m, "NPPRprr", 16, 69, 52);
    data(m, "BPPQpqr", 12, 30, 28);
    data(m, "PPPPbpp", 72, 117, 232);
    data(m, "BNPPbpr", 8, 50, 34);
    data(m, "NPPRppq", 8, 20, 45);
    data(m, "PPPRnpr", 38, 258, 120);
    data(m, "BPPRprr", 25, 116, 80);
    data(m, "PPPPPpr", 8, 8, 41);
    data(m, "NPPPbnp", 6, 39, 14);
    data(m, "BBPPbpr", 1, 39, 10);
    data(m, "BPPRppq", 9, 61, 78);
    data(m, "BPPPbnp", 8, 46, 23);
    data(m, "BPPPbpp", 945, 1417, 115);
    data(m, "PPPRppr", 3649, 3781, 467);
    data(m, "PPRRprr", 232, 324, 53);
    data(m, "PPPPppp", 1378, 170, 222);
    data(m, "NPPPnpp", 459, 387, 48);
    data(m, "BPPPRrr", 41, 28, 2);
    data(m, "PPPQppq", 814, 858, 137);
    data(m, "PPRRppq", 21, 60, 22);
    data(m, "BPPPnpp", 506, 291, 38);
    data(m, "NPPRnpr", 222, 357, 38);
    data(m, "BPPRbpr", 494, 954, 68);
    data(m, "NPPPbpp", 475, 397, 82);
    data(m, "NPPQbpq", 18, 42, 8);
    data(m, "NPPRbpr", 236, 385, 41);
    data(m, "PPQRpqr", 75, 49, 36);
    data(m, "BPPRnpr", 269, 353, 27);
    data(m, "BNPPppr", 76, 114, 21);
    data(m, "BBPPbnp", 55, 31, 2);
    data(m, "PPPPRbr", 37, 20, 2);
    data(m, "BNPPbbp", 11, 83, 5);
    data(m, "PPPQprr", 33, 19, 5);
    data(m, "BPPPPpr", 86, 58, 9);
    data(m, "BPPQbpq", 66, 101, 22);
    data(m, "BBPPppr", 39, 23, 6);
    data(m, "BBPPbbp", 24, 52, 1);
    data(m, "PPPPPbp", 53, 7, 4);
    data(m, "BNPPbnp", 42, 82, 2);
    data(m, "NPPPPpr", 45, 42, 8);
    data(m, "PPPRbnp", 89, 33, 9);
    data(m, "BPPQnpq", 21, 50, 7);
    data(m, "NPPQnpq", 24, 31, 2);
    data(m, "BPPPRpq", 10, 31, 16);
    data(m, "PPPPRpr", 1086, 21, 15);
    data(m, "NPPRppr", 774, 55, 15);
    data(m, "NPPPppp", 475, 19, 40);
    data(m, "PPPPPpp", 348, 1, 13);
    data(m, "BPPQppq", 230, 31, 6);
    data(m, "PPPPQpq", 209, 37, 12);
    data(m, "PPPQbpr", 90, 3, 0);
    data(m, "BPPPRbr", 192, 9, 1);
    data(m, "BPPPppp", 794, 4, 27);
    data(m, "NPPPRbr", 92, 4, 1);
    data(m, "BPPPPbp", 324, 15, 3);
    data(m, "NPPQppq", 123, 22, 4);
    data(m, "PPPRbpp", 692, 17, 17);
    data(m, "BPPRppr", 1106, 52, 24);
    data(m, "BNPPPpr", 85, 8, 0);
    data(m, "PPPRnpp", 442, 11, 13);
    data(m, "BBPPbpp", 119, 2, 1);
    data(m, "NPPPRnr", 94, 1, 2);
    data(m, "PPRRnpr", 118, 7, 2);
    data(m, "BPPPPnp", 151, 2, 0);
    data(m, "NPPPPnp", 173, 4, 1);
    data(m, "BNPPnpp", 93, 4, 2);
    data(m, "PPRRbpr", 126, 10, 6);
    data(m, "PPPRRrr", 87, 3, 0);
    data(m, "BNPPbpp", 169, 7, 1);
    data(m, "NPPPPbp", 165, 4, 2);
    data(m, "BPPPRnr", 102, 4, 1);
    data(m, "BPPRbnp", 77, 2, 1);
    data(m, "PPQRbpq", 41, 4, 5);
    data(m, "BPPRnpp", 250, 4, 1);
    data(m, "PPPQppr", 381, 0, 3);
    data(m, "NPPPRpr", 332, 3, 4);
    data(m, "PPPRppp", 1614, 2, 13);
    data(m, "PPRRppr", 436, 9, 1);
    data(m, "BBPPPbp", 56, 0, 0);
    data(m, "BPPPRpr", 554, 4, 2);
    data(m, "PPQRppq", 184, 15, 1);
    data(m, "BPPPPpp", 332, 0, 3);
    data(m, "NPPRnpp", 245, 7, 0);
    data(m, "BPPRbpp", 550, 5, 2);
    data(m, "NPPPPpp", 212, 1, 4);
    data(m, "PPPPPRr", 70, 0, 0);
    data(m, "BNPPPbp", 68, 0, 0);
    data(m, "PPPPRbp", 219, 1, 2);
    data(m, "NPPRbpp", 217, 1, 3);
    data(m, "BPPPQpq", 119, 7, 2);
    data(m, "NPPPQpq", 63, 3, 2);
    data(m, "BNPPppp", 76, 0, 1);
    data(m, "PPPPRnp", 113, 0, 0);
    data(m, "NPPRppp", 151, 0, 1);
    data(m, "PPPPRpp", 557, 1, 1);
    data(m, "PPPQbpp", 255, 2, 0);
    data(m, "BPPRppp", 255, 0, 0);
    data(m, "PPPPQpr", 169, 0, 0);
    data(m, "BPPPRnp", 87, 0, 0);
    data(m, "BPPPPRr", 90, 0, 0);
    data(m, "BPPPPPp", 51, 0, 1);
    data(m, "BPPPRbp", 208, 0, 1);
    data(m, "PPPQnpp", 130, 2, 1);
    data(m, "BPPQppr", 99, 0, 0);
    data(m, "BNPPPpp", 62, 0, 0);
    data(m, "NPPPRnp", 73, 0, 0);
    data(m, "NPPPRbp", 84, 0, 0);
    data(m, "PPPRRpr", 156, 0, 0);
    data(m, "PPPQRpq", 91, 2, 0);
    data(m, "NPPPRpp", 152, 0, 0);
    data(m, "PPPQppp", 1013, 2, 0);
    data(m, "BPPPRpp", 272, 0, 0);
    data(m, "BPPQbpp", 208, 0, 0);
    data(m, "NPPQbpp", 75, 0, 0);
    data(m, "BPPPQpr", 74, 0, 0);
    data(m, "PPPPPRp", 67, 0, 0);
    data(m, "PPQRppr", 401, 1, 0);
    data(m, "PPPPQbp", 96, 0, 0);
    data(m, "BPPQnpp", 64, 1, 0);
    data(m, "NPPQnpp", 70, 0, 0);
    data(m, "PPPPPQr", 58, 0, 0);
    data(m, "PPRRppp", 89, 0, 0);
    data(m, "PPPQRpr", 146, 0, 0);
    data(m, "PPPPQpp", 619, 0, 0);
    data(m, "PPQRbpp", 83, 0, 0);
    data(m, "BPPPPRp", 88, 0, 0);
    data(m, "PPQRnpp", 63, 0, 0);
    data(m, "BPPPQbp", 100, 0, 0);
    data(m, "NPPQppp", 87, 0, 0);
    data(m, "BPPQppp", 173, 0, 0);
    data(m, "PPPRRpp", 78, 0, 0);
    data(m, "BPPPQpp", 268, 0, 0);
    data(m, "PPQRppp", 212, 0, 0);
    data(m, "NPPPQpp", 108, 0, 0);
    data(m, "PPPPPQp", 142, 0, 0);
    data(m, "PPPQRpp", 263, 0, 0);
    data(m, "BPPPPQp", 104, 0, 0);
    data(m, "NPPPPQp", 62, 0, 0);
    data(m, "PPPPQRp", 100, 0, 0);
    data(m, "BPPQRpp", 60, 0, 0);
    data(m, "BPPPQRp", 52, 0, 0);
    data(m, "PPPpqr", 0, 0, 69);
    data(m, "PPPbpq", 0, 0, 57);
    data(m, "PPPppq", 0, 0, 521);
    data(m, "PPRpqr", 1, 1, 225);
    data(m, "BPPbpq", 0, 0, 82);
    data(m, "PPPRqr", 5, 1, 152);
    data(m, "PPPnpr", 0, 1, 56);
    data(m, "BPPppq", 0, 0, 166);
    data(m, "BPPPbq", 0, 0, 53);
    data(m, "PPPPpq", 0, 1, 166);
    data(m, "NPPppq", 1, 0, 98);
    data(m, "PPPbpr", 0, 1, 74);
    data(m, "PPRppq", 3, 9, 249);
    data(m, "PPPppr", 14, 7, 889);
    data(m, "BPPbpr", 1, 4, 181);
    data(m, "NPPbpr", 1, 1, 70);
    data(m, "BPPPpq", 3, 4, 140);
    data(m, "PPQpqr", 0, 6, 72);
    data(m, "PPRprr", 0, 5, 192);
    data(m, "NPPnpr", 0, 3, 81);
    data(m, "NPPPpq", 3, 0, 109);
    data(m, "BPPnpr", 0, 2, 93);
    data(m, "NPPppr", 6, 37, 260);
    data(m, "PPPRpq", 6, 34, 172);
    data(m, "PPRbpr", 9, 142, 313);
    data(m, "BPPbnp", 2, 15, 56);
    data(m, "PPQbpq", 5, 38, 68);
    data(m, "PPRnpr", 11, 125, 193);
    data(m, "PPPbpp", 36, 45, 371);
    data(m, "BPPppr", 17, 91, 412);
    data(m, "PPPPpr", 34, 33, 249);
    data(m, "BPRprr", 1, 30, 53);
    data(m, "BPPPbr", 1, 17, 40);
    data(m, "PPPnpp", 33, 41, 223);
    data(m, "NPRprr", 3, 17, 53);
    data(m, "BPRppq", 2, 7, 54);
    data(m, "PPPRrr", 5, 13, 71);
    data(m, "PPRbnp", 27, 66, 7);
    data(m, "BPPRpq", 14, 70, 45);
    data(m, "PPPRbr", 60, 153, 7);
    data(m, "NPPbpp", 48, 361, 122);
    data(m, "BPRnpr", 28, 223, 18);
    data(m, "BPPnpp", 105, 379, 69);
    data(m, "NPPRrr", 19, 56, 8);
    data(m, "BPPPpr", 103, 314, 55);
    data(m, "PPPRnr", 45, 124, 5);
    data(m, "BPPRrr", 19, 94, 8);
    data(m, "NPRbpr", 29, 230, 30);
    data(m, "BNPppr", 5, 62, 10);
    data(m, "NPPPpr", 46, 161, 61);
    data(m, "NPPRpq", 7, 28, 20);
    data(m, "PPPPbp", 116, 85, 13);
    data(m, "PPPQbq", 9, 50, 7);
    data(m, "PPPPnp", 78, 59, 10);
    data(m, "BBPbnp", 4, 44, 2);
    data(m, "PPRbpp", 399, 78, 10);
    data(m, "PPPQpq", 609, 208, 40);
    data(m, "PPPRpr", 3683, 577, 120);
    data(m, "NPPppp", 210, 48, 29);
    data(m, "PPRRrr", 162, 41, 9);
    data(m, "BPPPbp", 917, 334, 17);
    data(m, "NPRppr", 197, 139, 15);
    data(m, "PPRnpp", 284, 32, 11);
    data(m, "BPPPnp", 370, 38, 4);
    data(m, "PPPPpp", 1173, 28, 91);
    data(m, "PPQbpr", 54, 11, 4);
    data(m, "NPPPnp", 444, 51, 13);
    data(m, "BPPppp", 388, 50, 24);
    data(m, "NPPRbr", 149, 62, 3);
    data(m, "NPPRnr", 179, 71, 5);
    data(m, "BNPnpp", 25, 26, 3);
    data(m, "BPRppr", 332, 115, 16);
    data(m, "NPPPbp", 440, 58, 19);
    data(m, "BPPRnr", 191, 59, 3);
    data(m, "BNPPpr", 126, 41, 5);
    data(m, "BPQppq", 56, 38, 3);
    data(m, "BPPRbr", 348, 149, 5);
    data(m, "PPQRqr", 36, 3, 11);
    data(m, "BNPbpp", 42, 27, 1);
    data(m, "PRRbpr", 50, 38, 2);
    data(m, "PRRnpr", 52, 16, 0);
    data(m, "PPRRpq", 20, 29, 4);
    data(m, "BBPPpr", 46, 9, 0);
    data(m, "BPPQbq", 37, 23, 4);
    data(m, "NPPPpp", 517, 3, 18);
    data(m, "PPRppp", 872, 7, 12);
    data(m, "PPPPRr", 387, 3, 0);
    data(m, "PPQppr", 247, 6, 4);
    data(m, "BPPPpp", 843, 2, 18);
    data(m, "BPPRpr", 950, 18, 6);
    data(m, "NPPRpr", 659, 11, 4);
    data(m, "NPPQpq", 109, 11, 3);
    data(m, "PPPRbp", 516, 4, 0);
    data(m, "PRRppr", 180, 6, 0);
    data(m, "BPRbpp", 213, 3, 0);
    data(m, "PQRppq", 66, 5, 3);
    data(m, "PPPRnp", 311, 4, 2);
    data(m, "PPPPPp", 134, 0, 6);
    data(m, "BPPQpq", 176, 15, 1);
    data(m, "BPRnpp", 68, 0, 1);
    data(m, "BBPPbp", 74, 2, 0);
    data(m, "BNPPbp", 128, 1, 2);
    data(m, "PPPQbr", 55, 0, 1);
    data(m, "NPRbpp", 69, 1, 0);
    data(m, "BNPPnp", 68, 0, 1);
    data(m, "PPPPQq", 85, 7, 0);
    data(m, "NPRnpp", 77, 3, 2);
    data(m, "BPPPPb", 77, 0, 0);
    data(m, "PPRRbr", 86, 3, 0);
    data(m, "PPRRnr", 64, 1, 0);
    data(m, "NPRppp", 58, 0, 0);
    data(m, "BPPRbp", 379, 1, 0);
    data(m, "PPQnpp", 95, 0, 0);
    data(m, "PPPRpp", 1561, 5, 6);
    data(m, "BPPRnp", 154, 0, 0);
    data(m, "BPRppp", 74, 0, 1);
    data(m, "PPPQpr", 366, 1, 2);
    data(m, "BPPPPp", 190, 0, 0);
    data(m, "PPRRpr", 327, 1, 0);
    data(m, "PPQbpp", 151, 1, 0);
    data(m, "BPPPRr", 313, 0, 0);
    data(m, "NPPPRr", 186, 1, 1);
    data(m, "NPPPPp", 133, 0, 1);
    data(m, "PPPPRb", 86, 0, 0);
    data(m, "PPQRpq", 118, 11, 0);
    data(m, "NPPRbp", 119, 2, 0);
    data(m, "BNPPpp", 83, 1, 3);
    data(m, "NPPRnp", 137, 0, 0);
    data(m, "PPPPRn", 58, 0, 0);
    data(m, "BBPPpp", 52, 1, 0);
    data(m, "BPPPQq", 86, 2, 0);
    data(m, "NPPRpp", 214, 3, 0);
    data(m, "PPPQbp", 201, 0, 0);
    data(m, "PPQppp", 494, 0, 0);
    data(m, "PPPPRp", 348, 0, 0);
    data(m, "PQRppr", 244, 1, 0);
    data(m, "BPPRpp", 360, 1, 0);
    data(m, "PPPPQr", 125, 0, 0);
    data(m, "NPPQpr", 52, 0, 1);
    data(m, "BPPPRb", 81, 0, 0);
    data(m, "BPPQpr", 78, 0, 0);
    data(m, "BPQbpp", 99, 0, 0);
    data(m, "BNPPPp", 59, 0, 0);
    data(m, "PPPRRr", 90, 0, 0);
    data(m, "PPPQnp", 129, 0, 0);
    data(m, "NPPPRn", 51, 0, 0);
    data(m, "PPPQpp", 1404, 0, 0);
    data(m, "BPPQnp", 62, 0, 0);
    data(m, "NPPPRp", 132, 0, 0);
    data(m, "BPPQbp", 207, 0, 0);
    data(m, "BPPPRp", 261, 0, 0);
    data(m, "BPQppp", 53, 0, 0);
    data(m, "PPPPQb", 58, 0, 0);
    data(m, "PPQRpr", 327, 1, 1);
    data(m, "NPPQbp", 75, 0, 0);
    data(m, "PPRRpp", 119, 0, 0);
    data(m, "NPPQnp", 62, 0, 0);
    data(m, "PPPPQp", 530, 0, 0);
    data(m, "BPPQpp", 340, 0, 0);
    data(m, "PQRppp", 77, 0, 0);
    data(m, "PPPQRr", 92, 0, 0);
    data(m, "PPQRbp", 81, 0, 0);
    data(m, "NPPQpp", 177, 0, 0);
    data(m, "BPPPPR", 67, 0, 0);
    data(m, "PPQRnp", 60, 0, 0);
    data(m, "PPPRRp", 59, 0, 0);
    data(m, "BPQRpr", 64, 0, 0);
    data(m, "PPQRpp", 373, 0, 0);
    data(m, "NPPPQp", 138, 0, 0);
    data(m, "BPPPQp", 277, 0, 0);
    data(m, "PPPPPQ", 92, 0, 0);
    data(m, "PPPQRp", 271, 0, 0);
    data(m, "BPPPPQ", 107, 0, 0);
    data(m, "BPQRpp", 52, 0, 0);
    data(m, "PPPPQR", 55, 0, 0);
    data(m, "BPPQRp", 89, 0, 0);
    data(m, "PPPqr", 0, 0, 93);
    data(m, "PPRqr", 1, 3, 379);
    data(m, "PPPpq", 0, 5, 658);
    data(m, "BPPbq", 0, 0, 108);
    data(m, "BPPpq", 0, 7, 252);
    data(m, "PPPbr", 0, 1, 75);
    data(m, "NPPpq", 2, 2, 179);
    data(m, "PPPPq", 0, 0, 58);
    data(m, "PPPpr", 58, 53, 982);
    data(m, "BPPbr", 1, 37, 132);
    data(m, "NPPnr", 2, 8, 64);
    data(m, "NPPbr", 1, 4, 49);
    data(m, "PPRpq", 11, 40, 431);
    data(m, "PPRrr", 3, 18, 174);
    data(m, "BPPnr", 1, 11, 62);
    data(m, "PPQqr", 6, 9, 54);
    data(m, "BPPPq", 1, 4, 48);
    data(m, "NPPpr", 41, 277, 230);
    data(m, "BPRpq", 4, 66, 65);
    data(m, "PPQbq", 8, 75, 13);
    data(m, "PPPbp", 128, 298, 191);
    data(m, "BPPpr", 85, 536, 319);
    data(m, "PPRbr", 32, 538, 13);
    data(m, "PPPnp", 121, 208, 87);
    data(m, "BPRrr", 3, 104, 11);
    data(m, "NPRpq", 4, 27, 42);
    data(m, "PPRnr", 55, 339, 13);
    data(m, "PPPPr", 48, 22, 58);
    data(m, "NPRrr", 3, 66, 13);
    data(m, "BPPbn", 3, 49, 0);
    data(m, "PPPRq", 2, 60, 36);
    data(m, "PPQnq", 3, 46, 5);
    data(m, "PPRpr", 4088, 6143, 413);
    data(m, "PPPpp", 2126, 307, 327);
    data(m, "BPRbr", 111, 644, 14);
    data(m, "BPPbp", 716, 1987, 54);
    data(m, "NPPnp", 396, 570, 51);
    data(m, "BPPnp", 404, 427, 18);
    data(m, "PPQpq", 660, 978, 131);
    data(m, "NPPbp", 334, 513, 38);
    data(m, "PQRqr", 71, 18, 57);
    data(m, "NPRnr", 76, 284, 6);
    data(m, "PRRpq", 9, 38, 9);
    data(m, "NPPPr", 73, 43, 0);
    data(m, "BPRnr", 59, 220, 7);
    data(m, "BPQbq", 26, 68, 5);
    data(m, "BNPpr", 46, 165, 11);
    data(m, "BPPPr", 112, 57, 3);
    data(m, "NPRbr", 45, 247, 2);
    data(m, "PRRrr", 68, 183, 18);
    data(m, "PPRbn", 50, 34, 1);
    data(m, "BBPpr", 27, 41, 5);
    data(m, "PPPPb", 66, 11, 0);
    data(m, "PPPPn", 63, 1, 0);
    data(m, "BPPRq", 3, 49, 9);
    data(m, "PPRnp", 534, 19, 3);
    data(m, "PPRbp", 911, 33, 8);
    data(m, "BPPPn", 150, 3, 0);
    data(m, "BPPpp", 1118, 38, 47);
    data(m, "PPPPp", 634, 3, 17);
    data(m, "NPPpp", 684, 45, 53);
    data(m, "NPRpr", 602, 135, 5);
    data(m, "PPPRr", 2019, 50, 7);
    data(m, "BPQpq", 142, 45, 4);
    data(m, "BNPPr", 63, 3, 1);
    data(m, "NPPPn", 215, 1, 0);
    data(m, "NPPPb", 152, 2, 0);
    data(m, "PPPQq", 281, 27, 3);
    data(m, "BPRpr", 932, 141, 9);
    data(m, "PRRnr", 54, 4, 0);
    data(m, "BPPPb", 335, 22, 0);
    data(m, "BBPbp", 76, 8, 1);
    data(m, "NPQpq", 70, 25, 5);
    data(m, "PRRbr", 74, 9, 1);
    data(m, "PPQbr", 74, 3, 2);
    data(m, "BNPbp", 138, 15, 2);
    data(m, "BNPnp", 54, 10, 0);
    data(m, "NPPPp", 383, 3, 7);
    data(m, "PPRpp", 2641, 13, 16);
    data(m, "BPRbp", 445, 8, 1);
    data(m, "BPPRr", 755, 5, 0);
    data(m, "PPPRb", 290, 0, 0);
    data(m, "PPPRn", 239, 1, 0);
    data(m, "PPQpr", 534, 5, 5);
    data(m, "PQRpq", 131, 16, 3);
    data(m, "NPPRr", 482, 1, 0);
    data(m, "BNPpp", 72, 0, 2);
    data(m, "BPPPp", 547, 3, 7);
    data(m, "BNPPb", 70, 0, 0);
    data(m, "PRRpr", 360, 4, 0);
    data(m, "BPRnp", 177, 7, 0);
    data(m, "BPPQq", 131, 2, 1);
    data(m, "NPRbp", 136, 1, 0);
    data(m, "NPRnp", 169, 6, 1);
    data(m, "NPPQq", 86, 4, 0);
    data(m, "PPPRp", 1187, 2, 1);
    data(m, "PPRRr", 254, 3, 0);
    data(m, "PPQnp", 207, 3, 0);
    data(m, "PPQbp", 338, 0, 0);
    data(m, "PRRbp", 50, 0, 0);
    data(m, "NPRpp", 179, 0, 1);
    data(m, "BNPPp", 97, 0, 0);
    data(m, "BPRpp", 299, 2, 1);
    data(m, "BBPPp", 65, 0, 0);
    data(m, "NPPRn", 72, 0, 0);
    data(m, "BPQpr", 96, 0, 0);
    data(m, "PPPQr", 233, 0, 0);
    data(m, "NPPRb", 79, 0, 0);
    data(m, "BPPRb", 172, 1, 0);
    data(m, "BPPRn", 84, 0, 0);
    data(m, "NPPPP", 67, 0, 0);
    data(m, "BPPPP", 83, 0, 0);
    data(m, "PPQRq", 58, 5, 1);
    data(m, "PQRpr", 571, 3, 1);
    data(m, "NPPRp", 222, 0, 0);
    data(m, "PPQpp", 1869, 0, 0);
    data(m, "PPPQn", 89, 1, 0);
    data(m, "BPPRp", 428, 1, 0);
    data(m, "NPQnp", 94, 0, 0);
    data(m, "PRRpp", 103, 0, 0);
    data(m, "PPPPR", 200, 0, 0);
    data(m, "BPQbp", 250, 1, 0);
    data(m, "PPPQb", 147, 0, 0);
    data(m, "BPPQr", 89, 1, 0);
    data(m, "NPQbp", 92, 0, 0);
    data(m, "BPQnp", 85, 1, 0);
    data(m, "BPPPR", 248, 0, 0);
    data(m, "PPPQp", 1286, 0, 0);
    data(m, "PQRnp", 71, 0, 0);
    data(m, "BPQpp", 289, 0, 0);
    data(m, "PPQRr", 352, 0, 0);
    data(m, "PQRbp", 83, 0, 0);
    data(m, "NPPPR", 114, 0, 0);
    data(m, "BPPQb", 151, 0, 0);
    data(m, "NPQpp", 141, 0, 0);
    data(m, "NPPQn", 53, 0, 0);
    data(m, "PPRRp", 104, 0, 0);
    data(m, "NPPQb", 65, 0, 0);
    data(m, "BPPQn", 66, 0, 0);
    data(m, "NPPQp", 224, 0, 0);
    data(m, "BPPQp", 526, 0, 0);
    data(m, "BPQRr", 89, 0, 0);
    data(m, "PPPPQ", 405, 0, 0);
    data(m, "PQRpp", 351, 0, 0);
    data(m, "NPQRr", 57, 0, 0);
    data(m, "PPQRn", 58, 0, 0);
    data(m, "PPQRb", 72, 0, 0);
    data(m, "PPPRR", 57, 0, 0);
    data(m, "PPQRp", 511, 0, 0);
    data(m, "BPPPQ", 276, 0, 0);
    data(m, "NPPPQ", 172, 0, 0);
    data(m, "PPPQR", 239, 0, 0);
    data(m, "BPQRp", 83, 0, 0);
    data(m, "NPPQR", 53, 0, 0);
    data(m, "BPPQR", 106, 0, 0);
    data(m, "PPqr", 0, 0, 148);
    data(m, "PPbq", 0, 0, 83);
    data(m, "BPbq", 0, 1, 95);
    data(m, "PPpq", 0, 1, 837);
    data(m, "PRqr", 2, 6, 304);
    data(m, "PPbr", 0, 1, 82);
    data(m, "BPpq", 0, 3, 209);
    data(m, "PPPq", 0, 5, 265);
    data(m, "NPpq", 3, 2, 125);
    data(m, "PPnr", 1, 2, 65);
    data(m, "PPpr", 16, 32, 1183);
    data(m, "NPPq", 1, 6, 90);
    data(m, "BPPq", 1, 3, 141);
    data(m, "PRpq", 4, 7, 318);
    data(m, "PQqr", 1, 9, 65);
    data(m, "PRrr", 0, 7, 121);
    data(m, "BPbr", 0, 16, 115);
    data(m, "PQnq", 3, 52, 6);
    data(m, "PPPr", 77, 105, 320);
    data(m, "NPpr", 3, 74, 269);
    data(m, "PRnr", 7, 332, 13);
    data(m, "BPpr", 10, 192, 384);
    data(m, "PPnp", 54, 129, 182);
    data(m, "PPRq", 15, 129, 186);
    data(m, "PRbr", 7, 409, 19);
    data(m, "PPbp", 54, 184, 355);
    data(m, "PQbq", 1, 51, 7);
    data(m, "BPbn", 0, 58, 1);
    data(m, "NPbp", 27, 421, 57);
    data(m, "BPnp", 56, 433, 10);
    data(m, "BRnr", 0, 107, 4);
    data(m, "NPRq", 5, 45, 11);
    data(m, "PRbn", 12, 49, 2);
    data(m, "BPRq", 7, 86, 15);
    data(m, "PPPn", 118, 56, 0);
    data(m, "PPPb", 152, 122, 0);
    data(m, "NPPr", 56, 249, 7);
    data(m, "BPPr", 102, 369, 10);
    data(m, "BNpr", 0, 66, 10);
    data(m, "NRbr", 0, 110, 5);
    data(m, "PPRr", 5285, 1116, 32);
    data(m, "BPpp", 320, 189, 53);
    data(m, "BPPb", 597, 377, 0);
    data(m, "PRnp", 223, 73, 5);
    data(m, "NPPn", 453, 105, 0);
    data(m, "PPPp", 1465, 28, 95);
    data(m, "BNPr", 91, 65, 0);
    data(m, "PRbp", 354, 158, 10);
    data(m, "PPQq", 684, 240, 14);
    data(m, "BPPn", 359, 85, 0);
    data(m, "BRpr", 13, 479, 8);
    data(m, "NPPb", 355, 97, 0);
    data(m, "NPpp", 155, 114, 48);
    data(m, "NRpr", 13, 329, 9);
    data(m, "BQpq", 8, 80, 0);
    data(m, "BNbp", 0, 58, 0);
    data(m, "PQbr", 45, 9, 1);
    data(m, "BPPp", 1002, 10, 19);
    data(m, "BNPb", 99, 4, 0);
    data(m, "BPRr", 1063, 65, 1);
    data(m, "NPPp", 645, 6, 19);
    data(m, "NRnp", 56, 2, 0);
    data(m, "NPRr", 773, 65, 0);
    data(m, "PRpp", 1097, 24, 18);
    data(m, "PPRn", 479, 3, 0);
    data(m, "PPRb", 572, 5, 0);
    data(m, "PQpr", 332, 14, 6);
    data(m, "BRnp", 47, 8, 0);
    data(m, "BRbp", 116, 14, 1);
    data(m, "PPPP", 198, 0, 0);
    data(m, "BPQq", 139, 31, 2);
    data(m, "RRpr", 93, 2, 0);
    data(m, "NPQq", 82, 20, 3);
    data(m, "QRpq", 52, 7, 2);
    data(m, "BBPb", 73, 0, 0);
    data(m, "BBPp", 58, 0, 0);
    data(m, "BPPP", 273, 0, 0);
    data(m, "BNPp", 109, 0, 1);
    data(m, "PQnp", 108, 1, 2);
    data(m, "PPRp", 2434, 7, 4);
    data(m, "BRpp", 81, 5, 0);
    data(m, "BPRb", 290, 0, 0);
    data(m, "NPPP", 219, 0, 0);
    data(m, "PRRr", 336, 3, 0);
    data(m, "PQRq", 132, 15, 4);
    data(m, "PPQr", 471, 1, 2);
    data(m, "NPRb", 104, 1, 0);
    data(m, "PQbp", 189, 1, 0);
    data(m, "BPRn", 119, 0, 0);
    data(m, "NPRn", 135, 0, 0);
    data(m, "QRpr", 271, 2, 3);
    data(m, "PQpp", 758, 1, 0);
    data(m, "PPQb", 272, 0, 0);
    data(m, "PPQn", 203, 2, 0);
    data(m, "BPRp", 479, 1, 0);
    data(m, "BNPP", 76, 0, 0);
    data(m, "NPQr", 51, 0, 0);
    data(m, "BQbp", 87, 0, 0);
    data(m, "PPPR", 828, 0, 0);
    data(m, "BPQr", 121, 0, 0);
    data(m, "NPRp", 216, 0, 0);
    data(m, "PQRr", 1110, 2, 1);
    data(m, "PPQp", 2316, 0, 0);
    data(m, "NPPR", 271, 0, 0);
    data(m, "NPQb", 113, 0, 0);
    data(m, "BPPR", 527, 0, 0);
    data(m, "BPQn", 107, 1, 0);
    data(m, "BPQb", 278, 0, 0);
    data(m, "PRRp", 133, 0, 0);
    data(m, "NPQn", 131, 1, 0);
    data(m, "BQpp", 69, 0, 0);
    data(m, "PPPQ", 1112, 0, 0);
    data(m, "BPQp", 683, 0, 0);
    data(m, "PQRb", 114, 0, 0);
    data(m, "NPQp", 314, 0, 0);
    data(m, "PQRn", 109, 0, 0);
    data(m, "PPRR", 126, 0, 0);
    data(m, "QRpp", 121, 0, 0);
    data(m, "NQRr", 71, 0, 0);
    data(m, "BQRr", 73, 0, 0);
    data(m, "BPPQ", 731, 0, 0);
    data(m, "NPPQ", 354, 0, 0);
    data(m, "PQRp", 616, 0, 0);
    data(m, "PPQR", 746, 0, 0);
    data(m, "BQRp", 63, 0, 0);
    data(m, "BNPQ", 86, 0, 0);
    data(m, "BPQR", 223, 0, 0);
    data(m, "NPQR", 111, 0, 0);
    data(m, "PPq", 0, 17, 959);
    data(m, "NPq", 2, 7, 196);
    data(m, "BPq", 0, 8, 275);
    data(m, "PPr", 119, 280, 1123);
    data(m, "PRq", 26, 121, 574);
    data(m, "NPr", 23, 523, 18);
    data(m, "PPb", 163, 496, 0);
    data(m, "PPn", 174, 316, 0);
    data(m, "BPr", 17, 896, 29);
    data(m, "BRq", 2, 97, 33);
    data(m, "NRq", 3, 50, 20);
    data(m, "PPp", 2112, 486, 303);
    data(m, "NPn", 209, 758, 0);
    data(m, "PQq", 689, 1240, 33);
    data(m, "PRr", 4546, 7771, 85);
    data(m, "BPb", 247, 1941, 0);
    data(m, "BPn", 200, 435, 0);
    data(m, "NPb", 205, 893, 0);
    data(m, "BNr", 3, 122, 0);
    data(m, "BBr", 1, 49, 0);
    data(m, "BRr", 108, 1852, 1);
    data(m, "NPp", 680, 151, 57);
    data(m, "NRr", 89, 1751, 1);
    data(m, "PPP", 715, 0, 0);
    data(m, "BPp", 1048, 264, 42);
    data(m, "PRn", 647, 48, 0);
    data(m, "BQq", 31, 222, 2);
    data(m, "PRb", 829, 74, 0);
    data(m, "BBb", 0, 89, 0);
    data(m, "BNn", 4, 76, 0);
    data(m, "BNb", 1, 177, 0);
    data(m, "NQq", 17, 112, 6);
    data(m, "PRp", 2986, 8, 3);
    data(m, "PQr", 1000, 10, 11);
    data(m, "BNp", 97, 8, 6);
    data(m, "NPP", 539, 0, 0);
    data(m, "BPP", 725, 2, 0);
    data(m, "BRn", 131, 8, 0);
    data(m, "QRq", 190, 41, 3);
    data(m, "NRb", 113, 3, 0);
    data(m, "RRr", 328, 3, 0);
    data(m, "NRn", 142, 4, 0);
    data(m, "BRb", 275, 5, 0);
    data(m, "PQb", 454, 0, 0);
    data(m, "PPR", 2139, 0, 0);
    data(m, "NRp", 212, 3, 0);
    data(m, "BNP", 146, 0, 0);
    data(m, "BRp", 295, 4, 0);
    data(m, "PQn", 398, 6, 0);
    data(m, "NQr", 91, 3, 0);
    data(m, "BQr", 131, 1, 1);
    data(m, "BBP", 88, 0, 0);
    data(m, "QRr", 1357, 16, 1);
    data(m, "PQp", 2838, 0, 0);
    data(m, "NQb", 150, 1, 0);
    data(m, "BPR", 794, 0, 0);
    data(m, "BQn", 134, 3, 0);
    data(m, "NPR", 447, 0, 0);
    data(m, "BQb", 278, 0, 0);
    data(m, "NQn", 114, 5, 0);
    data(m, "RRp", 95, 0, 0);
    data(m, "BQp", 473, 0, 0);
    data(m, "PPQ", 2423, 0, 0);
    data(m, "QRb", 117, 0, 0);
    data(m, "QRn", 101, 0, 0);
    data(m, "PRR", 185, 0, 0);
    data(m, "NQp", 222, 2, 0);
    data(m, "BPQ", 1515, 0, 0);
    data(m, "NPQ", 850, 0, 0);
    data(m, "QRp", 562, 0, 0);
    data(m, "PQR", 1661, 0, 0);
    data(m, "BNQ", 138, 0, 0);
    data(m, "BBQ", 57, 0, 0);
    data(m, "BQR", 300, 0, 0);
    data(m, "NQR", 182, 0, 0);
    data(m, "Pq", 0, 17, 1155);
    data(m, "Bq", 0, 2, 346);
    data(m, "Nq", 0, 11, 347);
    data(m, "Rq", 11, 36, 1046);
    data(m, "Pr", 5, 280, 1149);
    data(m, "Nr", 0, 602, 119);
    data(m, "Pn", 79, 407, 0);
    data(m, "Br", 0, 778, 54);
    data(m, "Pb", 52, 596, 0);
    data(m, "Bn", 0, 862, 0);
    data(m, "Nb", 0, 764, 0);
    data(m, "PP", 1700, 50, 0);
    data(m, "Bp", 1, 635, 45);
    data(m, "Rn", 110, 603, 0);
    data(m, "Rb", 46, 773, 0);
    data(m, "Np", 0, 383, 82);
    data(m, "NP", 960, 51, 0);
    data(m, "Qr", 978, 27, 6);
    data(m, "Rp", 1116, 262, 5);
    data(m, "BP", 1250, 147, 0);
    data(m, "Qn", 356, 4, 0);
    data(m, "PR", 3761, 0, 0);
    data(m, "NN", 0, 112, 0);
    data(m, "BN", 550, 5, 0);
    data(m, "BB", 216, 3, 0);
    data(m, "Qb", 285, 0, 0);
    data(m, "BR", 1326, 0, 0);
    data(m, "NR", 1059, 0, 0);
    data(m, "Qp", 990, 24, 0);
    data(m, "PQ", 4453, 0, 0);
    data(m, "RR", 236, 0, 0);
    data(m, "BQ", 2696, 0, 0);
    data(m, "NQ", 1760, 0, 0);
    data(m, "QR", 3120, 0, 0);
    data(m, "P", 1981, 969, 0);
    data(m, "N", 0, 1086, 0);
    data(m, "B", 0, 1236, 0);
    data(m, "R", 7830, 0, 0);
    data(m, "Q", 6572, 0, 0);

    // end of generated code
    vec
};



// mb[PPqr] = -6000  - W: 0  D: 0  L: 148
// mb[PPqb] = -6000  - W: 0  D: 0  L: 83
// mb[BPqb] = -6000  - W: 0  D: 1  L: 95
// mb[PPqp] = -6000  - W: 0  D: 1  L: 837
// mb[RPqr] = -715  - W: 2  D: 6  L: 304
// mb[PPrb] = -5750  - W: 0  D: 1  L: 82
// mb[BPqp] = -5750  - W: 0  D: 3  L: 209
// mb[PPPq] = -5800  - W: 0  D: 5  L: 265
// mb[NPqp] = -5750  - W: 3  D: 2  L: 125
// mb[PPrn] = -5750  - W: 1  D: 2  L: 65
// mb[PPrp] = -629  - W: 16  D: 32  L: 1183
// mb[NPPq] = -546  - W: 1  D: 6  L: 90
// mb[BPPq] = -5550  - W: 1  D: 3  L: 141
// mb[RPqp] = -652  - W: 4  D: 7  L: 318
// mb[QPqr] = -440  - W: 1  D: 9  L: 65
// mb[RPrr] = -620  - W: 0  D: 7  L: 121
// mb[BPrb] = -474  - W: 0  D: 16  L: 115
// mb[QPqn] = -17  - W: 3  D: 52  L: 6
// mb[PPPr] = -183  - W: 77  D: 105  L: 320
// mb[NPrp] = -353  - W: 3  D: 74  L: 269
// mb[RPrn] = -5  - W: 7  D: 332  L: 13
// mb[BPrp] = -262  - W: 10  D: 192  L: 384
// mb[PPnp] = -127  - W: 54  D: 129  L: 182
// mb[RPPq] = -199  - W: 15  D: 129  L: 186
// mb[RPrb] = -9  - W: 7  D: 409  L: 19
// mb[PPbp] = -194  - W: 54  D: 184  L: 355
// mb[QPqb] = -35  - W: 1  D: 51  L: 7
// mb[BPbn] = -5  - W: 0  D: 58  L: 1
// mb[NPbp] = -20  - W: 27  D: 421  L: 57
// mb[BPnp] = 32  - W: 56  D: 433  L: 10
// mb[RBrn] = -12  - W: 0  D: 107  L: 4
// mb[RNPq] = -34  - W: 5  D: 45  L: 11
// mb[RPbn] = 55  - W: 12  D: 49  L: 2
// mb[RBPq] = -25  - W: 7  D: 86  L: 15
// mb[PPPn] = 286  - W: 118  D: 56  L: 0
// mb[PPPb] = 217  - W: 152  D: 122  L: 0
// mb[NPPr] = 55  - W: 56  D: 249  L: 7
// mb[BPPr] = 67  - W: 102  D: 369  L: 10
// mb[BNrp] = -45  - W: 0  D: 66  L: 10
// mb[RNrb] = -15  - W: 0  D: 110  L: 5
// mb[RPPr] = 398  - W: 5285  D: 1116  L: 32
// mb[BPpp] = 179  - W: 320  D: 189  L: 53
// mb[BPPb] = 247  - W: 597  D: 377  L: 0
// mb[RPnp] = 318  - W: 223  D: 73  L: 5
// mb[NPPn] = 393  - W: 453  D: 105  L: 0
// mb[PPPp] = 453  - W: 1465  D: 28  L: 95
// mb[BNPr] = 231  - W: 91  D: 65  L: 0
// mb[RPbp] = 274  - W: 354  D: 158  L: 10
// mb[QPPq] = 311  - W: 684  D: 240  L: 14
// mb[BPPn] = 390  - W: 359  D: 85  L: 0
// mb[RBrp] = 3  - W: 13  D: 479  L: 8
// mb[NPPb] = 368  - W: 355  D: 97  L: 0
// mb[NPpp] = 122  - W: 155  D: 114  L: 48
// mb[RNrp] = 3  - W: 13  D: 329  L: 9
// mb[QBqp] = 31  - W: 8  D: 80  L: 0
// mb[BNbp] = 1  - W: 0  D: 58  L: 0
// mb[QPrb] = 381  - W: 45  D: 9  L: 1
// mb[BPPp] = 649  - W: 1002  D: 10  L: 19
// mb[BNPb] = 5450  - W: 99  D: 4  L: 0
// mb[RBPr] = 605  - W: 1063  D: 65  L: 1
// mb[NPPp] = 587  - W: 645  D: 6  L: 19
// mb[RNnp] = 5500  - W: 56  D: 2  L: 0
// mb[RNPr] = 557  - W: 773  D: 65  L: 0
// mb[RPpp] = 627  - W: 1097  D: 24  L: 18
// mb[RPPn] = 5450  - W: 479  D: 3  L: 0
// mb[RPPb] = 5450  - W: 572  D: 5  L: 0
// mb[QPrp] = 566  - W: 332  D: 14  L: 6
// mb[RBnp] = 442  - W: 47  D: 8  L: 0
// mb[RBbp] = 474  - W: 116  D: 14  L: 1
// mb[PPPP] = 5400  - W: 198  D: 0  L: 0
// mb[QBPq] = 378  - W: 139  D: 31  L: 2
// mb[RRrp] = 5500  - W: 93  D: 2  L: 0
// mb[QNPq] = 339  - W: 82  D: 20  L: 3
// mb[QRqp] = 401  - W: 52  D: 7  L: 2
// mb[BBPb] = 5535  - W: 73  D: 0  L: 0
// mb[BBPp] = 5785  - W: 58  D: 0  L: 0
// mb[BPPP] = 5650  - W: 273  D: 0  L: 0
// mb[BNPp] = 5700  - W: 109  D: 0  L: 1
// mb[QPnp] = 5750  - W: 108  D: 1  L: 2
// mb[RPPp] = 1004  - W: 2434  D: 7  L: 4
// mb[RBpp] = 5750  - W: 81  D: 5  L: 0
// mb[RBPb] = 5700  - W: 290  D: 0  L: 0
// mb[NPPP] = 5650  - W: 219  D: 0  L: 0
// mb[RRPr] = 5700  - W: 336  D: 3  L: 0
// mb[QRPq] = 433  - W: 132  D: 15  L: 4
// mb[QPPr] = 5700  - W: 471  D: 1  L: 2
// mb[RNPb] = 5700  - W: 104  D: 1  L: 0
// mb[QPbp] = 5750  - W: 189  D: 1  L: 0
// mb[RBPn] = 5700  - W: 119  D: 0  L: 0
// mb[RNPn] = 5700  - W: 135  D: 0  L: 0
// mb[QRrp] = 6000  - W: 271  D: 2  L: 3
// mb[QPpp] = 6000  - W: 758  D: 1  L: 0
// mb[QPPb] = 5950  - W: 272  D: 0  L: 0
// mb[QPPn] = 5950  - W: 203  D: 2  L: 0
// mb[RBPp] = 5950  - W: 479  D: 1  L: 0
// mb[BNPP] = 5900  - W: 76  D: 0  L: 0
// mb[QNPr] = 5950  - W: 51  D: 0  L: 0
// mb[QBbp] = 6000  - W: 87  D: 0  L: 0
// mb[RPPP] = 5900  - W: 828  D: 0  L: 0
// mb[QBPr] = 5950  - W: 121  D: 0  L: 0
// mb[RNPp] = 5950  - W: 216  D: 0  L: 0
// mb[QRPr] = 6000  - W: 1110  D: 2  L: 1
// mb[QPPp] = 6000  - W: 2316  D: 0  L: 0
// mb[RNPP] = 6000  - W: 271  D: 0  L: 0
// mb[QNPb] = 6000  - W: 113  D: 0  L: 0
// mb[RBPP] = 6000  - W: 527  D: 0  L: 0
// mb[QBPn] = 6000  - W: 107  D: 1  L: 0
// mb[QBPb] = 6000  - W: 278  D: 0  L: 0
// mb[RRPp] = 6000  - W: 133  D: 0  L: 0
// mb[QNPn] = 6000  - W: 131  D: 1  L: 0
// mb[QBpp] = 6000  - W: 69  D: 0  L: 0
// mb[QPPP] = 6000  - W: 1112  D: 0  L: 0
// mb[QBPp] = 6000  - W: 683  D: 0  L: 0
// mb[QRPb] = 6000  - W: 114  D: 0  L: 0
// mb[QNPp] = 6000  - W: 314  D: 0  L: 0
// mb[QRPn] = 6000  - W: 109  D: 0  L: 0
// mb[RRPP] = 6000  - W: 126  D: 0  L: 0
// mb[QRpp] = 6000  - W: 121  D: 0  L: 0
// mb[QRNr] = 6000  - W: 71  D: 0  L: 0
// mb[QRBr] = 6000  - W: 73  D: 0  L: 0
// mb[QBPP] = 6000  - W: 731  D: 0  L: 0
// mb[QNPP] = 6000  - W: 354  D: 0  L: 0
// mb[QRPp] = 6000  - W: 616  D: 0  L: 0
// mb[QRPP] = 6000  - W: 746  D: 0  L: 0
// mb[QRBp] = 6000  - W: 63  D: 0  L: 0
// mb[QBNP] = 6000  - W: 86  D: 0  L: 0
// mb[QRBP] = 6000  - W: 223  D: 0  L: 0
// mb[QRNP] = 6000  - W: 111  D: 0  L: 0
// mb[PPq] = -822  - W: 0  D: 17  L: 959
// mb[NPq] = -623  - W: 2  D: 7  L: 196
// mb[BPq] = -737  - W: 0  D: 8  L: 275
// mb[PPr] = -275  - W: 119  D: 280  L: 1123
// mb[RPq] = -346  - W: 26  D: 121  L: 574
// mb[NPr] = 3  - W: 23  D: 523  L: 18
// mb[PPb] = 87  - W: 163  D: 496  L: 0
// mb[PPn] = 128  - W: 174  D: 316  L: 0
// mb[BPr] = -4  - W: 17  D: 896  L: 29
// mb[RBq] = -83  - W: 2  D: 97  L: 33
// mb[RNq] = -82  - W: 3  D: 50  L: 20
// mb[PPp] = 253  - W: 2112  D: 486  L: 303
// mb[NPn] = 76  - W: 209  D: 758  L: 0
// mb[QPq] = 120  - W: 689  D: 1240  L: 33
// mb[RPr] = 130  - W: 4546  D: 7771  L: 85
// mb[BPb] = 39  - W: 247  D: 1941  L: 0
// mb[BPn] = 113  - W: 200  D: 435  L: 0
// mb[NPb] = 65  - W: 205  D: 893  L: 0
// mb[BNr] = 8  - W: 3  D: 122  L: 0
// mb[BBr] = 6  - W: 1  D: 49  L: 0
// mb[RBr] = 18  - W: 108  D: 1852  L: 1
// mb[NPp] = 302  - W: 680  D: 151  L: 57
// mb[RNr] = 16  - W: 89  D: 1751  L: 1
// mb[PPP] = 5300  - W: 715  D: 0  L: 0
// mb[BPp] = 332  - W: 1048  D: 264  L: 42
// mb[RPn] = 578  - W: 647  D: 48  L: 0
// mb[QBq] = 39  - W: 31  D: 222  L: 2
// mb[RPb] = 547  - W: 829  D: 74  L: 0
// mb[BBb] = 1  - W: 0  D: 89  L: 0
// mb[BNn] = 17  - W: 4  D: 76  L: 0
// mb[BNb] = 1  - W: 1  D: 177  L: 0
// mb[QNq] = 28  - W: 17  D: 112  L: 6
// mb[RPp] = 1052  - W: 2986  D: 8  L: 3
// mb[QPr] = 719  - W: 1000  D: 10  L: 11
// mb[BNp] = 401  - W: 97  D: 8  L: 6
// mb[NPP] = 5550  - W: 539  D: 0  L: 0
// mb[BPP] = 5550  - W: 725  D: 2  L: 0
// mb[RBn] = 611  - W: 131  D: 8  L: 0
// mb[QRq] = 380  - W: 190  D: 41  L: 3
// mb[RNb] = 5600  - W: 113  D: 3  L: 0
// mb[RRr] = 5600  - W: 328  D: 3  L: 0
// mb[RNn] = 5600  - W: 142  D: 4  L: 0
// mb[RBb] = 5600  - W: 275  D: 5  L: 0
// mb[QPb] = 5850  - W: 454  D: 0  L: 0
// mb[RPP] = 5800  - W: 2139  D: 0  L: 0
// mb[RNp] = 5850  - W: 212  D: 3  L: 0
// mb[BNP] = 5800  - W: 146  D: 0  L: 0
// mb[RBp] = 5850  - W: 295  D: 4  L: 0
// mb[QPn] = 850  - W: 398  D: 6  L: 0
// mb[QNr] = 5850  - W: 91  D: 3  L: 0
// mb[QBr] = 5850  - W: 131  D: 1  L: 1
// mb[BBP] = 5885  - W: 88  D: 0  L: 0
// mb[QRr] = 872  - W: 1357  D: 16  L: 1
// mb[QPp] = 6000  - W: 2838  D: 0  L: 0
// mb[QNb] = 6000  - W: 150  D: 1  L: 0
// mb[RBP] = 6000  - W: 794  D: 0  L: 0
// mb[QBn] = 6000  - W: 134  D: 3  L: 0
// mb[RNP] = 6000  - W: 447  D: 0  L: 0
// mb[QBb] = 6000  - W: 278  D: 0  L: 0
// mb[QNn] = 6000  - W: 114  D: 5  L: 0
// mb[RRp] = 6000  - W: 95  D: 0  L: 0
// mb[QBp] = 6000  - W: 473  D: 0  L: 0
// mb[QPP] = 6000  - W: 2423  D: 0  L: 0
// mb[QRb] = 6000  - W: 117  D: 0  L: 0
// mb[QRn] = 6000  - W: 101  D: 0  L: 0
// mb[RRP] = 6000  - W: 185  D: 0  L: 0
// mb[QNp] = 6000  - W: 222  D: 2  L: 0
// mb[QBP] = 6000  - W: 1515  D: 0  L: 0
// mb[QNP] = 6000  - W: 850  D: 0  L: 0
// mb[QRp] = 6000  - W: 562  D: 0  L: 0
// mb[QRP] = 6000  - W: 1661  D: 0  L: 0
// mb[QBN] = 6000  - W: 138  D: 0  L: 0
// mb[QBB] = 6000  - W: 57  D: 0  L: 0
// mb[QRB] = 6000  - W: 300  D: 0  L: 0
// mb[QRN] = 6000  - W: 182  D: 0  L: 0
// mb[Pq] = -854  - W: 0  D: 17  L: 1155
// mb[Bq] = -5750  - W: 0  D: 2  L: 346
// mb[Nq] = -722  - W: 0  D: 11  L: 347
// mb[Rq] = -625  - W: 11  D: 36  L: 1046
// mb[Pr] = -379  - W: 5  D: 280  L: 1149
// mb[Nr] = -57  - W: 0  D: 602  L: 119
// mb[Pn] = 56  - W: 79  D: 407  L: 0
// mb[Br] = -22  - W: 0  D: 778  L: 54
// mb[Pb] = 27  - W: 52  D: 596  L: 0
// mb[Bn] = 1  - W: 0  D: 862  L: 0
// mb[Nb] = 1  - W: 0  D: 764  L: 0
// mb[PP] = 735  - W: 1700  D: 50  L: 0
// mb[Bp] = -22  - W: 1  D: 635  L: 45
// mb[Rn] = 54  - W: 110  D: 603  L: 0
// mb[Rb] = 19  - W: 46  D: 773  L: 0
// mb[Np] = -61  - W: 0  D: 383  L: 82
// mb[NP] = 634  - W: 960  D: 51  L: 0
// mb[Qr] = 682  - W: 978  D: 27  L: 6
// mb[Rp] = 384  - W: 1116  D: 262  L: 5
// mb[BP] = 502  - W: 1250  D: 147  L: 0
// mb[Qn] = 5750  - W: 356  D: 4  L: 0
// mb[RP] = 5700  - W: 3761  D: 0  L: 0
// mb[NN] = 1  - W: 0  D: 112  L: 0
// mb[BN] = 5700  - W: 550  D: 5  L: 0
// mb[BB] = 5785  - W: 216  D: 3  L: 0
// mb[Qb] = 5750  - W: 285  D: 0  L: 0
// mb[RB] = 5950  - W: 1326  D: 0  L: 0
// mb[RN] = 5950  - W: 1059  D: 0  L: 0
// mb[Qp] = 768  - W: 990  D: 24  L: 0
// mb[QP] = 6000  - W: 4453  D: 0  L: 0
// mb[RR] = 6000  - W: 236  D: 0  L: 0
// mb[QB] = 6000  - W: 2696  D: 0  L: 0
// mb[QN] = 6000  - W: 1760  D: 0  L: 0
// mb[QR] = 6000  - W: 3120  D: 0  L: 0
// mb[P] = 282  - W: 1981  D: 969  L: 0
// mb[N] = 1  - W: 0  D: 1086  L: 0
// mb[B] = 1  - W: 0  D: 1236  L: 0
// mb[R] = 5600  - W: 7830  D: 0  L: 0
// mb[Q] = 6000  - W: 6572  D: 0  L: 0
