use crate::bitboard::{Bitboard, Dir};
use std::fmt;
use std::iter::*;

pub const MAX_PLY: Ply = 128;

pub type Ply = i32;
pub type Hash = u64;


pub struct Chooser<T> {
    pub black: T,
    pub white: T,
}

// impl<T> Chooser<T> {
#[inline]
pub fn chooser_array<'a, T>(c: Color, white: &'a T, black: &'a T) -> &'a T {
    [&white, &black][c as usize]
}

#[inline]
pub fn chooser_wb_ref<'a, T>(c: Color, white: &'a T, black: &'a T) -> &'a T {
    match c {
        Color::White => white,
        Color::Black => black,
    }
}

#[inline]
pub fn chooser_wb<T>(c: Color, white: T, black: T) -> T {
    match c {
        Color::White => white,
        Color::Black => black,
    }
}

// pub fn chooser_wb<T>(c: Color, white: T, black: T) -> T {
//     match c {
//         Col::WHITE => { white },
//         Col::BLACK => { black }
//     }
// }

#[inline]
pub fn chooser_struct<'a, T>(c: Color, choices: &'a Chooser<&T>) -> &'a T {
    [&choices.white, &choices.black][c as usize]
}

bitflags! {
    pub struct CastlingRights: u8 {
        const NONE = 0;
        const WHITE_KING = 1 << 0;
        const WHITE_QUEEN = 1 << 1;
        const BLACK_KING = 1 << 2;
        const BLACK_QUEEN = 1 << 3;
        const ALL = Self::WHITE_KING.bits | Self::WHITE_QUEEN.bits | Self::BLACK_KING.bits | Self::BLACK_QUEEN.bits;
    }
}

impl Default for CastlingRights {
    fn default() -> Self {
        Self::NONE
    }
}

impl CastlingRights {

    #[inline]
    pub fn iter() -> &'static [Self] {
        &[ Self::WHITE_KING, Self::WHITE_QUEEN, Self::BLACK_KING, Self::BLACK_QUEEN ]
    }

    #[inline]
    pub fn index(&self) -> usize {
        match *self {
            Self::WHITE_KING => 0, 
            Self::WHITE_QUEEN => 1, 
            Self::BLACK_KING => 2, 
            Self::BLACK_QUEEN => 3,   
            _ => panic!("Castling index called on {}", self),          
        } 
    }


    pub fn parse(s: &str) -> Result<CastlingRights, String> {
        let mut castling = CastlingRights::NONE;
        for ch in s.chars() {
            match ch {
                '-' => break,
                'K' => castling |= Self::WHITE_KING,
                'Q' => castling |= Self::WHITE_QUEEN,
                'k' => castling |= Self::BLACK_KING,
                'q' => castling |= Self::BLACK_QUEEN,
                _ => return Err(format!("Invalid character '{}' in castling rights '{}'", ch, s)),
            }
        }
        Ok(castling)
    }

    // #[inline]
    // pub fn has_rights(&self, cr: CastlingRights) -> bool {
    //     self.contains(cr)
    // }

    pub fn king_side_right(c: Color) -> Self {
        chooser_wb(c, Self::WHITE_KING, Self::BLACK_KING)
    }

    pub fn queen_side_right(c: Color) -> Self {
        chooser_wb(c, Self::WHITE_QUEEN, Self::BLACK_QUEEN)
    }

    pub fn is_queen_side(&self) -> bool {
        self.intersects(Self::WHITE_QUEEN | Self::BLACK_QUEEN)
    }

    pub fn is_king_side(&self) -> bool {
        self.intersects(Self::WHITE_KING | Self::BLACK_KING)
    }

    pub fn king_side_squares(c: Color) -> Bitboard {
        chooser_wb(c, Bitboard::F1.or(Bitboard::G1), Bitboard::F8.or(Bitboard::G8))
    }

    pub fn queen_side_squares(c: Color) -> Bitboard {
        chooser_wb(
            c,
            Bitboard::D1.or(Bitboard::C1).or(Bitboard::B1),
            Bitboard::D8.or(Bitboard::C8).or(Bitboard::B8),
        )
    }
}

impl fmt::Display for CastlingRights {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_empty() {
            return write!(f, "-");
        }
        if self.contains(Self::WHITE_KING) {
            write!(f, "K")?
        }
        if self.contains(Self::WHITE_QUEEN) {
            write!(f, "Q")?
        }
        if self.contains(Self::BLACK_KING) {
            write!(f, "k")?
        }
        if self.contains(Self::BLACK_QUEEN) {
            write!(f, "q")?
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Color {
    White = 0,
    Black = 1,
} // numbering as per CPW

impl<T> std::ops::Index<Color> for [T] {
    type Output = T;
    #[inline]
    fn index(&self, c: Color) -> &Self::Output {
        &self[c.index()]
    }
}

impl<T> std::ops::IndexMut<Color> for [T] {
    #[inline]
    fn index_mut(&mut self, c: Color) -> &mut Self::Output {
        &mut self[c.index()]
    }
}

impl Default for Color {
    #[inline]
    fn default() -> Self {
        Color::White
    }
}

impl Color {
    pub const ALL: [Color; 2] = [Color::White, Color::Black];

    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }

    #[inline]
    pub fn chooser_wb<T>(self, white_thing: T, black_thing: T) -> T {
        match self {
            Color::White => white_thing,
            Color::Black => black_thing,
        }
    }

    #[inline]
    pub fn pawn_move(self) -> Dir {
        self.chooser_wb(Dir::N, Dir::S)
    }

    #[inline]
    pub fn double_push_dest_rank(self) -> Bitboard {
        self.chooser_wb(Bitboard::RANK_4, Bitboard::RANK_5)
    }

    #[inline]
    pub fn pawn_capture_east(self) -> Dir {
        self.chooser_wb(Dir::NE, Dir::SE)
    }

    #[inline]
    pub fn pawn_capture_west(self) -> Dir {
        self.chooser_wb(Dir::NW, Dir::SW)
    }

    #[inline]
    pub fn back_rank(self) -> Bitboard {
        self.chooser_wb(Bitboard::RANK_1, Bitboard::RANK_8)
    }

    #[inline]
    pub fn opposite(self) -> Color {
        self.chooser_wb(Color::Black, Color::White)
    }

    pub fn parse(s: &str) -> Result<Color, String> {
        match s {
            "w" => Ok(Color::White),
            "b" => Ok(Color::Black),
            _ => Err(format!("Invalid color: '{}'", s)),
        }
    }

    pub fn from_piece_char(ch: char) -> Result<Color, String> {
        if ch.is_lowercase() {
            return Ok(Color::Black);
        } else if ch.is_uppercase() {
            return Ok(Color::White);
        }
        Err(format!("Cannot get color for char '{}'", ch))
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", chooser_wb(*self, 'w', 'b'))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    Pawn = 0,
    Knight = 1,
    Bishop = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
    None = 6,
}

impl<T> std::ops::Index<Piece> for [T] {
    type Output = T;
    #[inline]
    fn index(&self, p: Piece) -> &Self::Output {
        &self[p.index()]
    }
}

impl<T> std::ops::IndexMut<Piece> for [T] {
    #[inline]
    fn index_mut(&mut self, p: Piece) -> &mut Self::Output {
        &mut self[p.index()]
    }
}

impl Default for Piece {
    fn default() -> Self {
        Piece::None
    }
}

impl fmt::Display for Piece {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_lower_char())
    }
}


impl Piece {
    pub const ALL: [Piece; 6] =
        [Piece::Pawn, Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen, Piece::King];

    #[inline]
    // pub fn to_upper_char(self) -> &char {
    //     ".PNBRQK".as_bytes()[self as usize] as char
    // }
    #[inline]
    pub fn index(&self) -> usize {
        *self as usize
    }

    #[inline]
    pub fn to_upper_char(&self) -> char {
        match self {
            Piece::None => '.',
            Piece::Pawn => 'P',
            Piece::Knight => 'N',
            Piece::Bishop => 'B',
            Piece::Rook => 'R',
            Piece::Queen => 'Q',
            Piece::King => 'K',
        }
    }

    #[inline]
    pub fn to_lower_char(&self) -> char {
        self.to_upper_char().to_ascii_lowercase()
    }


    /// coarse value in centipawns
    #[inline]
    pub const fn centipawns(&self) -> i32 {
        match self {
            Piece::None => 0,
            Piece::Pawn => 100,
            Piece::Knight => 325,
            Piece::Bishop => 350,
            Piece::Rook => 500,
            Piece::Queen => 900,
            Piece::King => 10000,
        }
    }

    #[inline]
    pub fn from_char(ch: char) -> Result<Piece, String> {
        Ok(match ch.to_ascii_uppercase() {
            '.' | ' ' => Piece::None,
            'P' => Piece::Pawn,
            'N' => Piece::Knight,
            'B' => Piece::Bishop,
            'R' => Piece::Rook,
            'Q' => Piece::Queen,
            'K' => Piece::King,
            _ => return Err(format!("Unknown piece '{}'", ch)),
        })
    }

    #[inline]
    pub fn to_char(&self, c: Option<Color>) -> char {
        match c {
            None => self.to_upper_char(),
            Some(c) if c == Color::White => self.to_upper_char(),
            Some(_) => self.to_upper_char().to_ascii_lowercase(),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, PartialOrd, Ord, Hash)]
pub struct ScoreWdl {
    pub w: i32,
    pub d: i32,
    pub l: i32,
}

impl fmt::Display for ScoreWdl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "W: {}  D: {}  L: {}", self.w, self.d, self.l)
    }
}

impl ScoreWdl {
    pub fn new(w: i32, d: i32, l: i32) -> ScoreWdl {
        ScoreWdl { w, d, l }
    }
}

impl std::ops::SubAssign for ScoreWdl {
    fn sub_assign(&mut self, o: Self) {
        self.w -= o.w;
        self.d -= o.d;
        self.l -= o.l;
    }
}

impl std::ops::Neg for ScoreWdl {
    type Output = Self;
    fn neg(self) -> Self {
        let mut z = Self::default();
        z -= self;
        z
    }
}

impl std::ops::AddAssign for ScoreWdl {
    fn add_assign(&mut self, o: Self) {
        *self -= -o;
    }
}

impl std::ops::Add for ScoreWdl {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        let mut z = Self::default();
        z += o;
        z += self;
        z
    }
}

impl std::ops::Sub for ScoreWdl {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        self + -o
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color() {
        assert_eq!(Color::parse("w"), Ok(Color::White));
        assert_eq!(Color::parse("b"), Ok(Color::Black));
        assert_eq!(Color::from_piece_char('n'), Ok(Color::Black));
        assert_eq!(Color::parse("B"), Err("Invalid color: 'B'".to_string()));
        assert_eq!(Piece::King.to_char(Some(Color::Black)), 'k');
        assert_eq!(Piece::King.to_char(None), 'K');
        let array = [1, 2];
        assert_eq!(array[Color::White], 1);
        assert_eq!(array[Color::Black], 2);
    }

    #[test]
    fn piece() {
        assert_eq!(Piece::Pawn.to_upper_char(), 'P');
        assert_eq!(Piece::King.to_char(Some(Color::Black)), 'k');
        assert_eq!(Piece::King.to_char(None), 'K');
    }

    #[test]
    fn test_score_wdl() {
        let mut wdl138 = ScoreWdl::new(1, 3, 8);
        let wdl567 = ScoreWdl::new(5, 6, 7);
        assert_eq!(wdl138 + wdl567, ScoreWdl::new(6, 9, 15));
        assert_eq!(wdl138 - wdl567, ScoreWdl::new(-4, -3, 1));
        wdl138 += ScoreWdl::new(100, 200, 300);
        assert_eq!(wdl138, ScoreWdl::new(101, 203, 308));
        wdl138 -= ScoreWdl::new(1, 3, 8);
        assert_eq!(wdl138, ScoreWdl::new(100, 200, 300));
        assert_eq!(-wdl138, ScoreWdl::new(-100, -200, -300));
    }

    #[test]
    fn choose() {
        let c = Color::White;
        const CHOICE: Chooser<&Bitboard> = Chooser { white: &Bitboard::RANK_4, black: &Bitboard::RANK_5 };

        let bb = chooser_array(c, &Bitboard::RANK_4, &Bitboard::RANK_5);
        assert_eq!(bb, &Bitboard::RANK_4);
        chooser_wb(c, &Bitboard::RANK_4, &Bitboard::RANK_5);
        chooser_struct(c, &CHOICE);
    }
}
