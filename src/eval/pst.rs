use crate::Bitboard;
use crate::infra::parsed_config::{Component};
use crate::bitboard::square::Square;
use crate::eval::weight::Weight;
use crate::types::{Color, Piece};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use std::fmt;




#[derive(Clone)]
// #[serde(from="PstProxy", into="PstProxy")]
pub struct Pst {
    pub enabled: bool,
    pub pawn_r5: Weight,
    pub pawn_r6: Weight,
    pub pawn_r7: Weight,
    pub rook_edge: Weight,

    array: [[Weight; 64]; Piece::len()],
}

impl Default for Pst {
    fn default() -> Self {
        let mut me = Self {
            enabled: true,
            pawn_r5: Weight::from_i32(14, 32),
            pawn_r6: Weight::from_i32(-14, 168),
            pawn_r7: Weight::from_i32(103, 224),
            rook_edge: Weight::from_i32(28, 13),

            
            array: [[Weight::default(); 64]; Piece::len()],

        };
        me.init_pst();
        me
    }
}

use std::collections::BTreeMap;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PstHelper {
    p: BTreeMap<String,Weight>,
    n: BTreeMap<String,Weight>,
    b: BTreeMap<String,Weight>,
    r: BTreeMap<String,Weight>,
    q: BTreeMap<String,Weight>,
    k: BTreeMap<String,Weight>,
}


impl Serialize for Pst {
    fn serialize<S:Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut h = PstHelper::default();
        for (i, &p) in Piece::ALL_BAR_NONE.iter().enumerate() {
            let map = &mut [&mut h.p, &mut h.n, &mut h.b, &mut h.r, &mut h.q, &mut h.k][i];
            for sq in Square::all() {
                map.insert(sq.uci(), self.array[p][sq]);
            }
        }   
        h.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Pst {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        let h: PstHelper = Deserialize::deserialize(deserializer)?;
        let mut pst = Pst::default();
        for (i, &p) in Piece::ALL_BAR_NONE.iter().enumerate() {
            let map = [&h.p, &h.n, &h.b, &h.r, &h.q, &h.k][i];
            for (k,&v) in map.iter() {
                let sq = Bitboard::parse_square(k).map_err(serde::de::Error::custom)?;
                pst.array[p][sq] = v;
            }
        }
        Ok(pst)
    }

}



#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PstProxy {
    p: [[Weight; 8]; 8],
    n: [[Weight; 8]; 8],
    b: [[Weight; 8]; 8],
    r: [[Weight; 8]; 8],
    q: [[Weight; 8]; 8],
    k: [[Weight; 8]; 8],
}

impl From<PstProxy> for Pst {
    fn from(pp: PstProxy) -> Self {
        let mut pst = Pst::default();
        for (i, &p) in Piece::ALL_BAR_NONE.iter().enumerate() {
            let b = [&pp.p, &pp.n, &pp.b, &pp.r, &pp.q, &pp.k][i];
            for sq in Square::all() {
              pst.array[p][sq] = b[sq.rank_index()][sq.file_index()];
            }
        }   
        pst
    }
}

impl Into<PstProxy> for Pst {
    fn into(self) -> PstProxy {
        let mut pp = PstProxy::default();
        for (i, &p) in Piece::ALL_BAR_NONE.iter().enumerate() {
            let b = &mut [&mut pp.p, &mut pp.n, &mut pp.b, &mut pp.r, &mut pp.q, &mut pp.k][i];
            for sq in Square::all() {
                b[sq.rank_index()][sq.file_index()] = self.array[p][sq];
            }
        }   
        pp
    }
}





impl Component for Pst {



    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {}
}



impl fmt::Display for Pst {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "rook_edge        : {}", self.rook_edge)?;
        for &p in &Piece::ALL_BAR_NONE {
            for phase in ["s", "e"] {
                writeln!(f, "PST: {}.{}", p, phase)?;
                for rank in (0..8).rev() {
                    for file in 0..8 {
                        let sq = Square::from_xy(file, rank);
                        let sq = sq.flip_vertical(); // white is stored upside down
                        let wt = self.array[p][sq];
                        let score = if phase == "s" { wt.s() } else { wt.e() };
                        let s = format!("{:>4}", score);
                        write!(f, "{:>6},", s)?;
                    }
                    writeln!(f)?;
                }
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

impl fmt::Debug for Pst {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Pst")
            .field("enabled", &self.enabled)
            .finish()
    }
}


impl Pst {
    pub fn new() -> Self {
        Self::default()
    }


    #[inline]
    pub fn w_eval_square(&self, c: Color, p: Piece, mut sq: Square) -> Weight {
        if c == Color::White {
            sq = sq.flip_vertical();
        }
        self.pst(p, sq)
    }


    // P(osition) S(quare) T(able)
    #[inline]
    pub fn pst(&self, p: Piece, sq: Square) -> Weight {
        self.array[p][sq]
    }


    fn init_pst(&mut self) {
        let r5 = self.pawn_r5.s() as i32;
        let r6 = self.pawn_r6.s() as i32;
        let r7 = self.pawn_r7.s() as i32;

        #[rustfmt::skip]
        let pawn_pst_mg: [i32; 64] = [
        0,  0,  0,  0,  0,  0,  0,  0,
        r7, r7, r7, r7, r7, r7, r7, r7,
        r6, r6, r6, r6, r6, r6, r6, r6,
        r5, r5, r5,r5+5,r5+5, r5, r5, r5,
        -9, 0,  0, 20, 20, -5,  -5, -9,
        -5,-5, -9,  0,  0, -9, -5, -5,
        4, 15, 15,-35,-35, 15, 15,  4,
        0,  0,  0,  0,  0,  0,  0,  0];

        let r5 = self.pawn_r5.e() as i32;
        let r6 = self.pawn_r6.e() as i32;
        let r7 = self.pawn_r7.e() as i32;
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
        -20,-10,-15,-10,-10,-15,-10,-20];

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

        let a = self.rook_edge.e() as i32;
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
        -20,-10,-10, 5, -5,-10,-10,-20];

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
        0,  0,  0,  -10,-20,-10,  0,  0,
        20, 30, 15, -20,  0,-20, 30, 10];

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
                self.array[p][sq] = Weight::from_i32(square_values_mg[p][sq], square_values_eg[p][sq]);
            }
        }
    }
}



#[cfg(test)]
mod tests {
    use test_env_log::test;
    use super::*;
    use crate::search::engine::Engine;
    use anyhow::Result;

    #[test]
    fn pst_serde_test() {
        let pst = Pst::default();
        let text = toml::to_string(&pst).unwrap();
        info!("toml\n{}", text);
        eprintln!("toml\n{}", text);
        let pst2: Pst = toml::from_str(&text).unwrap();
        eprintln!("from toml\n{}", pst2);
    }

    #[test]
    fn test_pst() {
        let pst = Pst::default();
        info!("{}", pst);
        let eng = Engine::new();
        info!("{}", eng.algo.eval.pst);
    }

    #[test]
    fn pst_config() -> Result<()> {
        let mut eng = Engine::default();
        eng.configment("eval.pst.p.a2", "{ s=6.5, e=7.6 }")?;
        eng.configment("eval.pst.p.a2.s", "6.5")?;
        eng.configment("eval.pst.p.a2.e", "7.5")?;
        let _text = toml::to_string(&eng)?;
        // eprintln!("toml\n{}", text);
        // let lookup = c1.weight("eval.pst.p.a2", &Weight::from_i32(1, 1));
        assert_eq!(eng.algo.eval.pst.pst(Piece::Pawn, Square::A2).s(), Weight::from_f32(6.5,7.5).s());
        assert_eq!(eng.algo.eval.pst.pst(Piece::Pawn, Square::A2).e(), Weight::from_f32(6.5,7.5).e());
        eng.configment("eval.pst.p.a2.e", "8.5")?;
        assert_eq!(eng.algo.eval.pst.pst(Piece::Pawn, Square::A2).e(), Weight::from_f32(6.5,8.5).e());
        Ok(())
    }
}


