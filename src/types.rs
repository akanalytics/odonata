use crate::bitboard::bitboard::{Bitboard, Dir};
use std::fmt;

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



#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Color {
    White = 0,
    Black = 1,
} // numbering as per CPW

impl<T> std::ops::Index<Color> for [T] {
    type Output = T;
    #[inline]
    fn index(&self, c: Color) -> &Self::Output {
            // unsafe {
            //     &self.get_unchecked(c.index())
            // }
            unsafe { &self.get_unchecked(c.index()) }
        }
}

// impl<T> std::ops::Index<&Color> for [T] {
//     type Output = T;
//     #[inline]
//     fn index(&self, c: &Color) -> &Self::Output {
//             // unsafe {
//             //     &self.get_unchecked(c.index())
//             // }
//             &self[(c.index())]
//         }
// }

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
    pub const fn len() -> usize {
        Self::ALL.len()
    }


    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }

    #[inline]
    pub  fn chooser_wb<T:Copy>(self, white_thing: T, black_thing: T) -> T {
        match self {
            Color::White => white_thing,
            Color::Black => black_thing,
        }
    }

    #[inline]
    pub fn forward(self) -> Dir {
        self.chooser_wb(Dir::N, Dir::S)
    }

    pub fn backward(self) -> Dir {
        self.chooser_wb(Dir::S, Dir::N)
    }

    #[inline]
    pub  fn double_push_dest_rank(self) -> Bitboard {
        self.chooser_wb(Bitboard::RANK_4, Bitboard::RANK_5)
    }

    #[inline]
    pub  fn pawn_capture_east(self) -> Dir {
        self.chooser_wb(Dir::NE, Dir::SE)
    }

    #[inline]
    pub  fn pawn_capture_west(self) -> Dir {
        self.chooser_wb(Dir::NW, Dir::SW)
    }

    #[inline]
    pub  fn back_rank(self) -> Bitboard {
        self.chooser_wb(Bitboard::RANK_1, Bitboard::RANK_8)
    }

    #[inline]
    pub  fn opposite(self) -> Color {
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
    None = 0,
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

// impl<T> std::ops::Index<&Piece> for [T] {
//     type Output = T;
//     #[inline]
//     fn index(&self, p: &Piece) -> &Self::Output {
//         &self[p.index()]
//     }
// }






impl<T> std::ops::Index<Piece> for [T] {
    type Output = T;
    #[inline]
    fn index(&self, p: Piece) -> &Self::Output {
        unsafe { &self.get_unchecked(p.index()) }
    }
}

impl<T> std::ops::IndexMut<Piece> for [T] {
    #[inline]
    fn index_mut(&mut self, p: Piece) -> &mut Self::Output {
        &mut self[p.index()]
    }
}

impl Default for Piece {
    #[inline]
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
    pub const ALL: [Piece; 7] =
        [Piece::None, Piece::Pawn, Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen, Piece::King];

    pub const ALL_BAR_NONE: [Piece; 6] =
        [Piece::Pawn, Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen, Piece::King];

    pub const ALL_BAR_KING: [Piece; 5] =
    [Piece::Pawn, Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen];

        // pub fn to_upper_char(self) -> &char {
    //     ".PNBRQK".as_bytes()[self as usize] as char
    // }
    #[inline]
    pub const fn index(&self) -> usize {
        *self as usize
    }

    #[inline]
    pub const fn len() -> usize {
        Self::ALL.len()
    }

    pub fn is_line_piece(&self) -> bool {
        match self {
            Piece::Bishop| Piece::Rook| Piece::Queen => true,
            _ => false,
        }
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

    pub fn elo_differnce(&self) -> f32 {
        let score = self.w as f32 + self.d as f32 / 2.0;        
        let total = self.w as f32 + self.d as f32 + self.l as f32;
        let percentage = score / total;
        -400.0 * f32::ln(1.0 / percentage - 1.0) / f32::ln(10.0)
    }
    
    // pub fn difference(s1: &ScoreWdl, s2: &ScoreWdl) -> ScoreWdl {
    //     Self::new(s1.w - s2.w, s1.d + s2.d, s1.l-s2.l)
    // }
}

impl std::ops::SubAssign for ScoreWdl {
    fn sub_assign(&mut self, o: Self) {
        self.l += o.w;
        self.d += o.d;
        self.w += o.l;
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
        assert_eq!(wdl138 - wdl567, ScoreWdl::new(8, 9, 13));
        wdl138 += ScoreWdl::new(100, 200, 300);
        assert_eq!(wdl138, ScoreWdl::new(101, 203, 308));
        wdl138 -= ScoreWdl::new(1, 3, 8);
        assert_eq!(wdl138, ScoreWdl::new(109, 206, 309));
        assert_eq!(-wdl138, ScoreWdl::new(309, 206, 109));

        // checked by https://www.3dkingdoms.com/chess/elo.htm
        assert_eq!(format!("{:.02}", ScoreWdl::new(217, 77, 184).elo_differnce()), "24.02");
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
