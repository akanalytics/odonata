use crate::bits::bitboard::{Bitboard, Dir};
use anyhow::{anyhow, bail, Result};
use enumflags2::BitFlags;
use serde::{Deserialize, Serialize};
use std::fmt;
use strum_macros::{Display, EnumCount};

pub const MAX_PLY: Ply = 128;
pub const LEN_PLY: usize = MAX_PLY as usize;
pub const MAX_LEGAL_MOVES: usize = 218;

pub type Ply = i32;
pub type Hash = u64;

pub struct Chooser<T> {
    pub black: T,
    pub white: T,
}

pub fn move_number_from_ply(p: Ply) -> (i32, Color) {
    ((p / 2) + 1, match p % 2 {
        0 => Color::White,
        _ => Color::Black,
    })
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

#[derive(
    Copy, Clone, Default, Serialize, Deserialize, Debug, Eq, PartialEq, Ord, PartialOrd, Hash,
)]
pub enum Color {
    #[default]
    #[serde(rename = "w")]
    White = 0,

    #[serde(rename = "b")]
    Black = 1,
} // numbering as per CPW

impl<T> std::ops::Index<Color> for [T] {
    type Output = T;
    #[inline]
    fn index(&self, i: Color) -> &Self::Output {
        #[cfg(not(all(not(feature = "unchecked_indexing"), debug_assertions)))]
        unsafe {
            &self.get_unchecked(i.index())
        }

        #[cfg(all(not(feature = "unchecked_indexing"), debug_assertions))]
        &self[(i.index())]
    }
}

impl<T> std::ops::IndexMut<Color> for [T] {
    #[inline]
    fn index_mut(&mut self, c: Color) -> &mut Self::Output {
        &mut self[c.index()]
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
    pub fn chooser_wb<T: Copy>(self, white_thing: T, black_thing: T) -> T {
        match self {
            Color::White => white_thing,
            Color::Black => black_thing,
        }
    }

    #[inline]
    pub fn forward(self) -> Dir {
        [Dir::N, Dir::S][self]
    }

    #[inline]
    pub fn backward(self) -> Dir {
        [Dir::S, Dir::N][self]
    }

    #[inline]
    pub fn double_push_dest_rank(self) -> Bitboard {
        [Bitboard::RANK_4, Bitboard::RANK_5][self]
    }

    #[inline]
    pub fn pawn_capture_east(self) -> Dir {
        [Dir::NE, Dir::SE][self]
    }

    #[inline]
    pub fn pawn_capture_west(self) -> Dir {
        [Dir::NW, Dir::SW][self]
    }

    #[inline]
    pub fn back_rank(self) -> Bitboard {
        [Bitboard::RANK_1, Bitboard::RANK_8][self]
    }

    #[inline]
    pub fn rank_as_white_index(self, index: usize) -> Bitboard {
        [Bitboard::RANKS[index], Bitboard::RANKS[7 - index]][self]
    }

    #[inline]
    pub fn rank_as_white_num(self, num: usize) -> Bitboard {
        [Bitboard::RANKS[num - 1], Bitboard::RANKS[8 - num]][self]
    }

    #[inline]
    pub fn opposite(self) -> Color {
        [Color::Black, Color::White][self]
    }

    pub fn parse(s: &str) -> Result<Color> {
        match s {
            "w" => Ok(Color::White),
            "b" => Ok(Color::Black),
            _ => Err(anyhow!("invalid color: '{}'", s)),
        }
    }

    #[inline]
    pub fn from_piece_char(ch: char) -> Result<Color> {
        if ch.is_lowercase() {
            return Ok(Color::Black);
        } else if ch.is_uppercase() {
            return Ok(Color::White);
        }
        Err(anyhow!("cannot get color for char '{}'", ch))
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", chooser_wb(*self, 'w', 'b'))
    }
}

// #[repr(u8)]
#[derive(Copy, Hash, Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum Piece {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl<T> std::ops::Index<Piece> for [T] {
    type Output = T;
    #[inline]
    fn index(&self, i: Piece) -> &Self::Output {
        #[cfg(not(debug_assertions))]
        unsafe {
            &self.get_unchecked(i.index())
        }

        #[cfg(debug_assertions)]
        &self[(i.index())]
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
        Piece::King
    }
}

impl fmt::Display for Piece {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_lower_char())
    }
}

impl Piece {
    pub const ALL: [Piece; 6] = [
        Piece::Pawn,
        Piece::Knight,
        Piece::Bishop,
        Piece::Rook,
        Piece::Queen,
        Piece::King,
    ];

    pub const ALL_BAR_KING: [Piece; 5] = [
        Piece::Pawn,
        Piece::Knight,
        Piece::Bishop,
        Piece::Rook,
        Piece::Queen,
    ];

    // pub fn to_upper_char(self) -> &char {
    //     ".PNBRQK".as_bytes()[self as usize] as char
    // }
    #[inline]
    pub const fn index(&self) -> usize {
        *self as usize
    }

    #[inline]
    pub const fn from_index(i: usize) -> Piece {
        Self::ALL[i]
    }

    #[inline]
    pub const fn len() -> usize {
        Self::ALL.len()
    }

    #[inline]
    pub const fn is_line_piece(&self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Piece::Bishop | Piece::Rook | Piece::Queen => true,
            _ => false,
        }
    }

    #[inline]
    pub const fn to_upper_char(&self) -> char {
        match self {
            Piece::Pawn => 'P',
            Piece::Knight => 'N',
            Piece::Bishop => 'B',
            Piece::Rook => 'R',
            Piece::Queen => 'Q',
            Piece::King => 'K',
        }
    }

    #[inline]
    pub const fn to_lower_char(&self) -> char {
        self.to_upper_char().to_ascii_lowercase()
    }

    #[inline]
    pub const fn name(&self) -> &'static str {
        ["none", "pawn", "knight", "bishop", "rook", "queen", "king"][self.index()]
    }

    /// coarse value in centipawns
    #[inline]
    pub const fn centipawns(&self) -> i32 {
        match self {
            Piece::Pawn => 100,
            Piece::Knight => 325,
            Piece::Bishop => 325,
            Piece::Rook => 500,
            Piece::Queen => 900,
            Piece::King => 10000,
        }
    }

    #[inline]
    pub fn from_char(ch: char) -> Result<Piece> {
        Ok(match ch.to_ascii_uppercase() {
            'P' => Piece::Pawn,
            'N' => Piece::Knight,
            'B' => Piece::Bishop,
            'R' => Piece::Rook,
            'Q' => Piece::Queen,
            'K' => Piece::King,
            _ => bail!("Unknown piece '{}'", ch),
        })
    }

    #[inline]
    pub const fn to_char(&self, c: Color) -> char {
        match c {
            Color::White => self.to_upper_char(),
            Color::Black => self.to_upper_char().to_ascii_lowercase(),
        }
    }

    pub fn parse(s: &str) -> Result<Piece> {
        if s.len() == 1 {
            Piece::from_char(s.chars().next().unwrap())
        } else {
            bail!("Unknown piece '{s}'");
        }
    }
}

#[enumflags2::bitflags]
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, EnumCount, Display, Serialize, Deserialize)]
pub enum MoveType {
    Start,
    Hash,
    Initialize,
    Null,
    Unsorted,
    Capture,
    GoodCapture,
    GoodCaptureUpfrontSorted,
    Killer,
    CounterMove,
    Promo,
    QueenPromo,
    Evasion,
    Quiet,
    QuietOrPromo,
    QuietUnsorted,
    BadCapture,
    Remaining,
    End,
}

pub type MoveTypes = BitFlags<MoveType>;

impl MoveType {
    pub fn new_from_str(s: &str) -> Result<MoveTypes, String> {
        let mut mts = MoveTypes::empty();
        for c in s.chars() {
            mts |= Self::from_char(c)?;
        }
        Ok(mts)
    }

    pub fn to_string(mts: MoveTypes) -> String {
        mts.iter().map(|mt| mt.as_char()).collect()
    }

    pub fn index(self) -> usize {
        (self as u16).trailing_zeros() as usize
    }

    pub fn as_char(self) -> char {
        match self {
            MoveType::Start => 'S',
            MoveType::Hash => 'H',
            MoveType::Initialize => 'I',
            MoveType::Null => 'N',
            MoveType::GoodCapture => 'g',
            MoveType::GoodCaptureUpfrontSorted => 'G',
            MoveType::Killer => 'K',
            MoveType::CounterMove => 'c',
            MoveType::Promo => 'P',
            MoveType::QueenPromo => 'p',
            MoveType::Evasion => 'V',
            MoveType::Quiet => 'q',
            MoveType::QuietOrPromo => 'A',
            MoveType::QuietUnsorted => 'Q',
            MoveType::BadCapture => 'B',
            MoveType::Remaining => 'R',
            MoveType::Unsorted => 'U',
            MoveType::Capture => 'C',
            MoveType::End => 'E',
        }
    }

    pub fn from_char(c: char) -> Result<MoveType, String> {
        match c {
            'S' => Ok(MoveType::Start),
            'H' => Ok(MoveType::Hash),
            'I' => Ok(MoveType::Initialize),
            'N' => Ok(MoveType::Null),
            'g' => Ok(MoveType::GoodCapture),
            'G' => Ok(MoveType::GoodCaptureUpfrontSorted),
            'K' => Ok(MoveType::Killer),
            'c' => Ok(MoveType::CounterMove),
            'P' => Ok(MoveType::Promo),
            'p' => Ok(MoveType::QueenPromo),
            'V' => Ok(MoveType::Evasion),
            'q' => Ok(MoveType::Quiet),
            'A' => Ok(MoveType::QuietOrPromo),
            'Q' => Ok(MoveType::QuietUnsorted),
            'B' => Ok(MoveType::BadCapture),
            'R' => Ok(MoveType::Remaining),
            'U' => Ok(MoveType::Unsorted),
            'C' => Ok(MoveType::Capture),
            'E' => Ok(MoveType::End),
            _ => Err(format!("'{}' is unknown move type", c)),
        }
    }

    pub fn slice_to_string(move_types: &[MoveType]) -> String {
        move_types
            .iter()
            .map(|&mt| mt.as_char())
            .collect::<Vec<char>>()
            .into_iter()
            .collect()
    }

    pub fn vec_from_string(move_types: &str) -> Result<Vec<MoveType>, String> {
        move_types
            .chars()
            .map(MoveType::from_char)
            .collect::<Result<Vec<_>, _>>()
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct Repeats {
    pub in_total:  u16,
    pub in_search: u16, // exclusive of root
}

impl Repeats {
    pub fn in_game(&self) -> u16 {
        self.in_total - self.in_search
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use serde_json::json;
    use strum::EnumCount;

    #[test]
    fn color() {
        assert_eq!(Color::parse("w").unwrap(), Color::White);
        assert_eq!(Color::parse("b").unwrap(), Color::Black);
        assert_eq!(Color::from_piece_char('n').unwrap(), Color::Black);
        assert_eq!(
            Color::parse("B").unwrap_err().to_string(),
            "invalid color: 'B'".to_string()
        );
        assert_eq!(Piece::King.to_char(Color::Black), 'k');
        assert_eq!(Piece::King.to_char(Color::White), 'K');
        let array = [1, 2];
        assert_eq!(array[Color::White], 1);
        assert_eq!(array[Color::Black], 2);
    }

    #[test]
    fn test_serde() {
        // let color = json!({"color" : "b"});
        assert_eq!(serde_json::to_string(&Color::Black).unwrap(), "\"b\"");
        assert_eq!(
            serde_json::from_str::<Color>("\"w\"").unwrap(),
            Color::White
        );
    }

    // panic is caused by debug assertions - so only run this test in debug
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn piece_panic1() {
        let array = [0u32; Piece::len() - 1];
        assert_eq!(array[Piece::King], 1); // triggers panic not mismatch
    }

    #[test]
    fn piece() {
        assert_eq!(Piece::Pawn.to_upper_char(), 'P');
        assert_eq!(Piece::King.to_char(Color::Black), 'k');
        assert_eq!(Piece::King.to_char(Color::White), 'K');
        for i in 0..Piece::len() {
            assert_eq!(Piece::from_index(i).index(), i);
        }
        assert_eq!(Piece::parse("P").unwrap(), Piece::Pawn);
        assert_eq!(Piece::parse("Q").unwrap(), Piece::Queen);
        assert_eq!(Piece::parse("q").unwrap(), Piece::Queen);
        assert_eq!(Piece::parse("").is_err(), true);
        assert_eq!(Piece::parse("X").is_err(), true);
        assert_eq!(Piece::parse("XX").is_err(), true);
    }

    #[test]
    fn test_move_type() -> Result<(), String> {
        let many = MoveType::vec_from_string("HIGKPqB").unwrap();
        assert_eq!(many[0], MoveType::Hash);
        assert_eq!(many.last(), Some(&MoveType::BadCapture));
        assert_eq!(MoveType::slice_to_string(&many), "HIGKPqB");
        assert_eq!(
            MoveType::new_from_str("CHB")?,
            MoveType::Capture | MoveType::Hash | MoveType::BadCapture
        );
        assert_eq!(
            "HCB".to_string(),
            MoveType::to_string(MoveType::Capture | MoveType::Hash | MoveType::BadCapture)
        );
        assert_eq!(MoveType::COUNT, 19);
        assert_eq!(MoveType::Start.index(), 0);
        assert_eq!(MoveType::End.index(), 16);
        Ok(())
    }

    #[test]
    fn choose() {
        let c = Color::White;
        const CHOICE: Chooser<&Bitboard> = Chooser {
            white: &Bitboard::RANK_4,
            black: &Bitboard::RANK_5,
        };

        let bb = chooser_array(c, &Bitboard::RANK_4, &Bitboard::RANK_5);
        assert_eq!(bb, &Bitboard::RANK_4);
        chooser_wb(c, &Bitboard::RANK_4, &Bitboard::RANK_5);
        chooser_struct(c, &CHOICE);
    }
}
