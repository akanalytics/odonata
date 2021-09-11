use crate::config::{Component, Config};
use crate::bitboard::square::Square;
use crate::eval::weight::Weight;
use crate::types::{Color, Piece};

use std::fmt;




#[derive(Clone)]
pub struct Pst {
    pub enabled: bool,
    pub pawn_r5: Weight,
    pub pawn_r6: Weight,
    pub pawn_r7: Weight,
    pub rook_edge: Weight,
    
    pst: [[Weight; 64]; Piece::len()],
}


impl Default for Pst {
    fn default() -> Self {
        let mut me = Self {
            enabled: false,
            pawn_r5: Weight::new(13, 32),
            pawn_r6: Weight::new(5, 86),
            pawn_r7: Weight::new(24, 304),
            rook_edge: Weight::new(0, 0),

            
            pst: [[Weight::default(); 64]; Piece::len()],

        };
        me.init_pst();

        me
    }
}



impl Component for Pst {
    fn settings(&self, c: &mut Config) {
        c.set("mb.enabled", &format!("type check default {}", self.enabled));
        c.set_weight("eval.rook.edge", &self.rook_edge);
        c.set_weight("eval.pawn.r5", &self.pawn_r5);
        c.set_weight("eval.pawn.r6", &self.pawn_r6);
        c.set_weight("eval.pawn.r7", &self.pawn_r7);


        // for &p in &Piece::ALL_BAR_KING {
        //     let mut name = "eval.".to_string();
        //     name.push(p.to_char(Some(Color::Black)));
        //     c.set_weight(&name, &self.material_weights[p]);
        // }

        // c.set("eval.mb.all", "type string default \"\"");  // cutechess can send "eval.mb=KPPk:100,KPk:56" etc
        // self.ensure_init();
        // for hash in 0..Material::HASH_VALUES {
        //     let cp = self.derived_load(hash);
        //     if let Some(cp) = cp {
        //         let mut mat = Material::maybe_from_hash(hash);
        //         *mat.counts_mut(Color::White, Piece::King) = 0;
        //         *mat.counts_mut(Color::Black, Piece::King) = 0;
        //         c.set(&format!("eval.mb.material.{} type string default",mat.to_string()), &cp.to_string());
        //     }
        // }
    }

    fn configure(&mut self, c: &Config) {
        debug!("mb.configure");
        self.enabled = c.bool("mb.enabled").unwrap_or(self.enabled);
        self.pawn_r5 = c.weight("eval.pawn.r5", &self.pawn_r5);
        self.pawn_r6 = c.weight("eval.pawn.r6", &self.pawn_r6);
        self.pawn_r7 = c.weight("eval.pawn.r7", &self.pawn_r7);
        self.rook_edge = c.weight("eval.rook.edge", &self.rook_edge);

        // for &p in &Piece::ALL_BAR_KING {
        //     let mut name = "eval.".to_string();
        //     name.push(p.to_char(Some(Color::Black)));
        //     self.material_weights[p] = c.weight(&name, &self.material_weights[p]);
        // }

        // let mut reconfigure = false;
        // for (k, v) in c.iter() {
        //     if let Some(k) = k.strip_prefix("eval.mb.material.") {
        //         info!("config fetch eval.mb.material.{} = [mb] {}", k, v);
        //         self.parse_and_store(k, v).unwrap();
        //         reconfigure = true;
        //     }
        // }
        self.init_pst();

    }


    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {}
}



impl fmt::Display for Pst {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "rook_edge        : {}", self.rook_edge)?;

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
        self.pst[p][sq]
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
        9, 15, 15,-35,-35, 15, 15,  10,
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
}


