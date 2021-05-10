
pub struct HyperbolaBitboard {
    rays: [[Bitboard; 8]; 64],
    king_moves: [Bitboard; 64],
    knight_moves: [Bitboard; 64],
}

impl Default for HyperbolaBitboard {
    fn default() -> Self {
        Self::new()
    }
}

impl HyperbolaBitboard {
    pub fn new() -> HyperbolaBitboard {
        // let mut attacks = [[Bitboard::EMPTY; 8]; 64];
        let mut classical = HyperbolaBitboard {
            rays: [[Bitboard::EMPTY; 8]; 64],
            king_moves: [Bitboard::EMPTY; 64],
            knight_moves: [Bitboard::EMPTY; 64],
        };
        for sq in 0..64_usize {
            for &dir in Dir::ALL.iter() {
                let bb = Bitboard::from_sq(sq as u32);
                let mask = Self::ray(dir, bb);
                classical.rays[sq][dir.index] = mask;
                classical.king_moves[sq] |= bb.shift(dir);

                // for example a night attack might be step N followed by step NE
                let next_dir = Dir::ALL[(dir.index + 1) % 8];
                classical.knight_moves[sq] |= bb.shift(dir).shift(next_dir);
            }
        }
        classical
    }

    fn sliding_attacks(&self, occupied: Bitboard, from: Square, dir: &Dir) -> Bitboard {
        let attacks = self.rays[from.index()][dir.index];
        let blockers = attacks & occupied;

        if blockers.is_empty() {
            return attacks;
        }
        let blocker_sq = if dir.shift > 0 { blockers.first_square() } else { blockers.last_square() };
        // println!("attcks::: dir:{}, from:sq:{} blockers: {:?} blocker_sq:{} \n",  dir.index, from_sq, blockers, blocker_sq);
        // println!("blockers:\n{} \nattacks:\n{} \n",blockers, attacks);
        // println!("minus\n{}\n", self.attacks[blocker_sq][dir.index]);
        // remove attacks from blocker sq and beyond
        attacks - self.rays[blocker_sq.index()][dir.index]
    }
}

impl BitboardAttacks for HyperbolaBitboard {


    fn rook_attacks(&self, occ: Bitboard, from: Square) -> Bitboard {
        self.sliding_attacks(occ, from, &Dir::N)
            | self.sliding_attacks(occ, from, &Dir::E)
            | self.sliding_attacks(occ, from, &Dir::S)
            | self.sliding_attacks(occ, from, &Dir::W)
    }

    fn bishop_attacks(&self, occ: Bitboard, from: Square) -> Bitboard {
        self.sliding_attacks(occ, from, &Dir::NE)
            | self.sliding_attacks(occ, from, &Dir::SE)
            | self.sliding_attacks(occ, from, &Dir::SW)
            | self.sliding_attacks(occ, from, &Dir::NW)
    }

    fn king_attacks(&self, from: Square) -> Bitboard {
        self.king_moves[from.index()]
    }

    fn knight_attacks(&self, from: Square) -> Bitboard {
        self.knight_moves[from.index()]
    }
}
