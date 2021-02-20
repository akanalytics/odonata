use std::fmt::{self, Write};





// generated from https://docs.google.com/spreadsheets/d/1TB2TKX04VsR10CLNLDIvrufm6wSJOttXOyPNKndU4N0/edit?usp=sharing

bitflags! {
    pub struct Bitboard2: u64 {
        const EMPTY = 0; 
        const A1 = 1 << 0; 	const B1 = 1 << 1; const	C1 = 1 << 2; const	D1 = 1 << 3; const	E1 = 1 << 4; const	F1 = 1 << 5; const	G1 = 1 << 6; const	H1 = 1 << 7; const
        A2 = 1 << 8; const	B2 = 1 << 9; const	C2 = 1 << 10; const	D2 = 1 << 11; const	E2 = 1 << 12; const	F2 = 1 << 13; const	G2 = 1 << 14; const	H2 = 1 << 15; const
        A3 = 1 << 16; const	B3 = 1 << 17; const	C3 = 1 << 18; const	D3 = 1 << 19; const	E3 = 1 << 20; const	F3 = 1 << 21; const	G3 = 1 << 22; const	H3 = 1 << 23; const
        A4 = 1 << 24; const	B4 = 1 << 25; const	C4 = 1 << 26; const	D4 = 1 << 27; const	E4 = 1 << 28; const	F4 = 1 << 29; const	G4 = 1 << 30; const	H4 = 1 << 31; const
        A5 = 1 << 32; const	B5 = 1 << 33; const	C5 = 1 << 34; const	D5 = 1 << 35; const	E5 = 1 << 36; const	F5 = 1 << 37; const	G5 = 1 << 38; const	H5 = 1 << 39; const
        A6 = 1 << 40; const	B6 = 1 << 41; const	C6 = 1 << 42; const	D6 = 1 << 43; const	E6 = 1 << 44; const	F6 = 1 << 45; const	G6 = 1 << 46; const	H6 = 1 << 47; const
        A7 = 1 << 48; const	B7 = 1 << 49; const	C7 = 1 << 50; const	D7 = 1 << 51; const	E7 = 1 << 52; const	F7 = 1 << 53; const	G7 = 1 << 54; const	H7 = 1 << 55; const
        A8 = 1 << 56; const	B8 = 1 << 57; const	C8 = 1 << 58; const	D8 = 1 << 59; const	E8 = 1 << 60; const	F8 = 1 << 61; const	G8 = 1 << 62; const	H8 = 1 << 63;

        const FILE_A = Self::A1.bits | Self::A2.bits | Self::A3.bits | Self::A4.bits | Self::A5.bits | Self::A6.bits | Self::A7.bits | Self::A8.bits; 
        const RANK_1 = Self::A1.bits | Self::B1.bits | Self::C1.bits | Self::D1.bits | Self::E1.bits | Self::F1.bits | Self::G1.bits | Self::H1.bits; 
        const FILE_B = Self::FILE_A.bits << 1; const RANK_2 = Self::RANK_1.bits << 1*8;
        const FILE_C = Self::FILE_A.bits << 2; const RANK_3 = Self::RANK_1.bits << 2*8;
        const FILE_D = Self::FILE_A.bits << 3; const RANK_4 = Self::RANK_1.bits << 3*8;
        const FILE_E = Self::FILE_A.bits << 4; const RANK_5 = Self::RANK_1.bits << 4*8;
        const FILE_F = Self::FILE_A.bits << 5; const RANK_6 = Self::RANK_1.bits << 5*8;
        const FILE_G = Self::FILE_A.bits << 6; const RANK_7 = Self::RANK_1.bits << 6*8;
        const FILE_H = Self::FILE_A.bits << 7; const RANK_8 = Self::RANK_1.bits << 7*8;
    }
}

impl Bitboard2 {
    #[inline]
    pub fn len(self) -> u32 {
        self.bits.count_ones()
    }
    
    #[inline]
    pub fn swap_bytes(self) -> Self {
        Bitboard2::from_bits_truncate(self.bits.swap_bytes())
    }

    #[inline]
    pub fn first(self) -> Self {
        Bitboard2::from_bits_truncate(1 << (63 - self.bits.leading_zeros()))
    }
}

impl Iterator for Bitboard2 {
    type Item = Self;
    #[inline]
    fn next(&mut self) -> Option<Self> {
        if self.is_empty() {
            None
        } else {
            let sq = self.first();
            *self ^= sq;
            Some(sq)
        }
    }
}


impl fmt::Display for Bitboard2 {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        for r in (0..8).rev() {
            for f in 0..8 {
                let bit = 1 << r * 8 + f;
                fmt.write_char(if self.contains(Bitboard2::from_bits_truncate(bit)) { '1' } else { '.' })?;
                fmt.write_char(' ')?;
            }
            fmt.write_char('\n')?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

 
    #[test]
    fn test_bb2() {
        let bb1 = Bitboard2::A1 | Bitboard2::B2;
        assert!( bb1.contains(Bitboard2::A1) );
        assert!( bb1 & Bitboard2::C1 == Bitboard2::A1 - Bitboard2::A1 );
        assert!( ! bb1.is_empty() );
        assert!( bb1.intersects( Bitboard2::B2) );
        assert!( bb1.intersects( Bitboard2::B2) );
        assert!( Bitboard2::FILE_A.contains( Bitboard2::A4) );
        assert_eq!(format!("{}", bb1), ". . . . . . . . \n. . . . . . . . \n. . . . . . . . \n. . . . . . . . \n. . . . . . . . \n. . . . . . . . \n. 1 . . . . . . \n1 . . . . . . . \n");
        assert_eq!(format!("{:?}", bb1), "A1 | B2");
        assert_eq!(format!("{:?}", Bitboard2::FILE_A), "A1 | A2 | A3 | A4 | A5 | A6 | A7 | A8 | FILE_A");
        assert_eq!(format!("{:#?}", bb1), "A1 | B2");
        assert_eq!(format!("{:064b}", bb1), "0000000000000000000000000000000000000000000000000000001000000001");
        println!("\n{}", Bitboard2::FILE_B | Bitboard2::RANK_7);
        assert_eq!( Bitboard2::FILE_A.len(), 8);
        println!("\n{}", (Bitboard2::FILE_B | Bitboard2::RANK_7).swap_bytes());
        let a = Bitboard2::RANK_7;
        for item in a {
            print!("({:?})", item);
        }
        println!("{}", a);
        // # Binary, Debug, LowerHex, Octal and UpperHex
    }
}
