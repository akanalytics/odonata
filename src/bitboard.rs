use std::fmt::{self, Write};


#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Dir {
    shift: i32,
    mask:  Bitboard  // mask for oppsoite edge(s)
}

impl Dir {
    pub const N:  Self = Dir{ shift:  8, mask: Bitboard::RANK_8 };
    pub const S:  Self = Dir{ shift: -8, mask: Bitboard::RANK_1 };
    pub const E:  Self = Dir{ shift:  1, mask: Bitboard::FILE_H };
    pub const W:  Self = Dir{ shift: -1, mask: Bitboard::FILE_A };
    pub const NE: Self = Dir{ shift:  9, mask: Bitboard::RANK_8.or(Bitboard::FILE_H) };
    pub const SE: Self = Dir{ shift: -7, mask: Bitboard::RANK_1.or(Bitboard::FILE_H) };
    pub const SW: Self = Dir{ shift: -9, mask: Bitboard::RANK_1.or(Bitboard::FILE_A) };
    pub const NW: Self = Dir{ shift:  7, mask: Bitboard::RANK_8.or(Bitboard::FILE_A) };

    pub const ALL: [Self; 8] = [ Self::N, Self::NE, Self::E, Self::SE, Self::S, Self::SW, Self::W, Self::NW ];

}


// generated from https://docs.google.com/spreadsheets/d/1TB2TKX04VsR10CLNLDIvrufm6wSJOttXOyPNKndU4N0/edit?usp=sharing
bitflags! {
    pub struct Bitboard: u64 {
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

impl Bitboard {
    #[inline]
    pub fn from_xy(x: u32, y: u32) -> Bitboard {
        let bit = 1 << y * 8 + x;
        Bitboard::from_bits_truncate(bit)
    }

    #[inline]
    pub fn from_sq(index: u32) -> Bitboard {
        let bit = 1 << index;
        Bitboard::from_bits_truncate(bit)
    }


    #[inline]
    pub fn shift(self, dir: &Dir) -> Bitboard {
        let bb = self - dir.mask;
        if dir.shift > 0 {
            Bitboard::from_bits_truncate(bb.bits << dir.shift)
        } 
        else {
            Bitboard::from_bits_truncate(bb.bits >> -dir.shift)
        }
    }

    // bitflags & doesnt seem to be declared const
    #[inline]
    pub const fn or(self, other: Bitboard) -> Bitboard {
        Bitboard::from_bits_truncate(self.bits | other.bits)
    }

    #[inline]
    pub fn len(self) -> u32 {
        self.bits.count_ones()
    }

    #[inline]
    pub fn swap_bytes(self) -> Self {
        Bitboard::from_bits_truncate(self.bits.swap_bytes())
    }

    #[inline]
    pub fn last(self) -> Self {
        // MSB
        Bitboard::from_bits_truncate(1 << (63 - self.bits.leading_zeros()))
    }

    #[inline]
    pub fn first(self) -> Self {
        // LSB
        Bitboard::from_bits_truncate(1 << self.bits.trailing_zeros())
    }
}

impl Iterator for Bitboard {
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

impl fmt::Display for Bitboard {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        for r in (0..8).rev() {
            for f in 0..8 {
                let bit = 1 << r * 8 + f;
                fmt.write_char(if self.contains(Bitboard::from_bits_truncate(bit)) {
                    '1'
                } else {
                    '.'
                })?;
                fmt.write_char(' ')?;
            }
            fmt.write_char('\n')?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let a1b2 = Bitboard::A1 | Bitboard::B2;
        assert!(a1b2.contains(Bitboard::A1));
        assert!(a1b2 & Bitboard::C1 == Bitboard::A1 - Bitboard::A1);
        assert!(!a1b2.is_empty());
        assert!(a1b2.intersects(Bitboard::B2));
        assert!(Bitboard::FILE_A.contains(Bitboard::A4));
        assert_eq!(Bitboard::FILE_A.len(), 8);
        assert_eq!(Bitboard::from_xy(4, 7), Bitboard::E8);
        assert_eq!(Bitboard::from_sq(63), Bitboard::H8);
        assert_eq!(Bitboard::from_sq(8), Bitboard::A2);
        assert_eq!(a1b2.first(), Bitboard::A1);
        assert_eq!(a1b2.last(), Bitboard::B2);
        let a2b3 = a1b2.shift(&Dir::N);
        assert_eq!(a2b3, Bitboard::A2 | Bitboard::B3 );
        assert!(Bitboard::D8.shift(&Dir::N).is_empty() );
        assert_eq!(Bitboard::D8.shift(&Dir::E), Bitboard::E8 );

        assert_eq!(format!("{}", a1b2), ". . . . . . . . \n. . . . . . . . \n. . . . . . . . \n. . . . . . . . \n. . . . . . . . \n. . . . . . . . \n. 1 . . . . . . \n1 . . . . . . . \n");
        assert_eq!(format!("{:?}", a1b2), "A1 | B2");
        assert_eq!(
            format!("{:?}", Bitboard::FILE_A),
            "A1 | A2 | A3 | A4 | A5 | A6 | A7 | A8 | FILE_A"
        );
        assert_eq!(format!("{:b}", a1b2), "1000000001");

        println!("\n{}", Bitboard::FILE_B | Bitboard::RANK_7);
        println!("\n{}", (Bitboard::FILE_B | Bitboard::RANK_7).swap_bytes());
        let a = Bitboard::RANK_7;
        for item in a {
            print!("({:?})", item);
        }
        println!("{}", a);
    }


    #[test]
    fn test_direction() {
        let dir = Dir::N;
        assert_eq!(dir.shift, 8);
        assert_eq!( Dir::ALL[0], Dir::N );
        assert_eq!( format!("{:?}", Dir::N), "Dir { shift: 8, mask: A8 | B8 | C8 | D8 | E8 | F8 | G8 | H8 | RANK_8 }" );
    }
}
    
