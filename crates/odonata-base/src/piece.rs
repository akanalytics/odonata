use std::fmt;
use std::ops::{Index, IndexMut};
use std::str::FromStr;

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use strum_macros::EnumString;

use crate::bits::bitboard::{Bitboard, Dir};

pub const MAX_PLY: Ply = 128;
pub const LEN_PLY: usize = MAX_PLY as usize;
pub const MAX_LEGAL_MOVES: usize = 218;

pub type Ply = i32;
pub type Hash = u64;

pub trait FlipVertical {
    fn flip_vertical(self) -> Self;
}

pub trait FlipSide {
    fn flip_side(self) -> Self;
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
pub struct Dual<T> {
    pub white: T,
    pub black: T,
}

impl<T> Index<Color> for Dual<T> {
    type Output = T;

    fn index(&self, c: Color) -> &Self::Output {
        match c {
            Color::White => &self.white,
            Color::Black => &self.black,
        }
    }
}

impl<T> IndexMut<Color> for Dual<T> {
    fn index_mut(&mut self, c: Color) -> &mut Self::Output {
        match c {
            Color::White => &mut self.white,
            Color::Black => &mut self.black,
        }
    }
}

impl<T> FlipSide for Dual<T> {
    fn flip_side(self) -> Self {
        Dual {
            white: self.black,
            black: self.white,
        }
    }
}

impl<T> Dual<T> {
    pub fn new(white: T, black: T) -> Self {
        Dual { white, black }
    }
}

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

#[derive(Copy, Clone, Default, Serialize, Deserialize, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Color {
    #[default]
    #[serde(rename = "w")]
    White = 0,

    #[serde(rename = "b")]
    Black = 1,
} // numbering as per CPW

impl<T> std::ops::Index<Color> for [T; 2] {
    type Output = T;
    #[inline]
    fn index(&self, i: Color) -> &Self::Output {
        #[cfg(not(all(not(feature = "unchecked_indexing"), debug_assertions)))]
        unsafe {
            self.get_unchecked(i.index())
        }

        #[cfg(all(not(feature = "unchecked_indexing"), debug_assertions))]
        &self[i.index()]
    }
}

impl<T> std::ops::IndexMut<Color> for [T; 2] {
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
    pub const fn is_white(self) -> bool {
        matches!(self, Color::White)
    }

    #[inline]
    pub const fn chooser_wb<T: Copy>(self, white_thing: T, black_thing: T) -> T {
        match self {
            Color::White => white_thing,
            Color::Black => black_thing,
        }
    }

    #[inline]
    pub const fn forward(self) -> Dir {
        self.chooser_wb(Dir::N, Dir::S)
    }

    #[inline]
    pub fn backward(self) -> Dir {
        self.chooser_wb(Dir::S, Dir::N)
    }

    #[inline]
    pub fn double_push_dest_rank(self) -> Bitboard {
        [Bitboard::RANK_4, Bitboard::RANK_5][self]
    }

    #[inline]
    pub const fn pawn_capture_east(self) -> Dir {
        self.chooser_wb(Dir::NE, Dir::SE)
    }

    #[inline]
    pub const fn pawn_capture_west(self) -> Dir {
        self.chooser_wb(Dir::NW, Dir::SW)
    }

    #[inline]
    pub fn back_rank(self) -> Bitboard {
        [Bitboard::RANK_1, Bitboard::RANK_8][self]
    }

    #[inline]
    /// index runs from 0-7
    pub fn rank_as_white_index(self, index: usize) -> Bitboard {
        [Bitboard::RANKS[index], Bitboard::RANKS[7 - index]][self]
    }

    #[inline]
    /// num runs from 1-8
    pub fn rank_as_white_num(self, num: usize) -> Bitboard {
        [Bitboard::RANKS[num - 1], Bitboard::RANKS[8 - num]][self]
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

impl FlipSide for Color {
    #[inline]
    fn flip_side(self) -> Color {
        self.chooser_wb(Color::Black, Color::White)
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", chooser_wb(*self, 'w', 'b'))
    }
}

// #[repr(u8)]
#[derive(Copy, Default, Hash, Clone, Debug, PartialEq, Eq, Deserialize, Serialize, EnumString)]
pub enum Piece {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    #[default]
    King,
}

impl From<u8> for Piece {
    fn from(u: u8) -> Self {
        Piece::from_index(u as usize)
    }
}

impl<T> std::ops::Index<Piece> for [T; 6] {
    type Output = T;
    #[inline]
    fn index(&self, i: Piece) -> &Self::Output {
        #[cfg(not(debug_assertions))]
        unsafe {
            &self.get_unchecked(i.index())
        }

        #[cfg(debug_assertions)]
        &self[i.index()]
    }
}

impl<T> std::ops::IndexMut<Piece> for [T; 6] {
    #[inline]
    fn index_mut(&mut self, p: Piece) -> &mut Self::Output {
        &mut self[p.index()]
    }
}

/// as lower case
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

    pub const ALL_BAR_KING: [Piece; 5] = [Piece::Pawn, Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen];

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
        ["pawn", "knight", "bishop", "rook", "queen", "king"][self.index()]
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

use bitflags::bitflags;

bitflags! {
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
    pub struct MoveType : u32 {
        const Start = 1 << 0;
        const Hash = 1 << 1;
        const Initialize = 1 << 2;
        const Null = 1 << 3;
        const Unsorted = 1 << 4;
        const Capture = 1 << 5;
        const GoodCapture = 1 << 6;
        const GoodCaptureUpfrontSorted = 1 << 7;
        const Killer = 1 << 8;
        const CounterMove = 1 << 9;
        const Promo = 1 << 10;
        const QueenPromo = 1 <<11;
        const Evasion = 1 << 12;
        const Quiet = 1 << 13;
        const QuietOrPromo = 1 << 14;
        const QuietUnsorted = 1 << 15;
        const BadCapture = 1 << 16;
        const Remaining = 1 << 17;
        const End = 1 << 18;
    }
}
// pub type MoveTypes = BitFlags<MoveType>;

impl fmt::Display for MoveType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use bitflags::parser::to_writer;
        let mut s = String::new();
        to_writer(self, &mut s)?;
        f.write_str(&s)?;
        Ok(())
    }
}

impl FromStr for MoveType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Self::from_chars(s).with_context(|| format!("parsing MoveTypes from {s}"))
    }
}

impl MoveType {
    pub fn from_chars(s: &str) -> anyhow::Result<MoveType> {
        let mut mts = MoveType::empty();
        for c in s.chars() {
            mts |= Self::from_char(c)?;
        }
        Ok(mts)
    }

    pub fn to_string(mts: MoveType) -> String {
        mts.iter().map(|mt| mt.as_char()).collect()
    }

    pub fn index(self) -> usize {
        debug_assert!(self.bits().count_ones() == 1, "MoveType {self} invalid");
        self.bits().trailing_zeros() as usize
    }

    pub fn as_char(self) -> char {
        debug_assert!(self.bits().count_ones() == 1, "MoveType {self} invalid");
        match self {
            MoveType::Start => 'S',
            MoveType::Hash => 'H',
            MoveType::Initialize => 'I',
            MoveType::Null => 'N',
            MoveType::Unsorted => 'U',
            MoveType::Capture => 'C',
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
            MoveType::End => 'E',
            _ => panic!("unknown MoveType {self:?}"),
        }
    }

    pub fn from_char(c: char) -> Result<MoveType> {
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
            _ => Err(anyhow::anyhow!("'{c}' is unknown move type")),
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

    pub fn vec_from_string(move_types: &str) -> Result<Vec<MoveType>> {
        move_types
            .chars()
            .map(MoveType::from_char)
            .collect::<Result<Vec<_>, _>>()
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct Repeats {
    pub in_search: u16,
    pub in_played: u16,
}

impl Repeats {
    pub fn in_total(&self) -> u16 {
        self.in_played + self.in_search
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use serde_json::json;

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
        assert_eq!(serde_json::from_str::<Color>("\"w\"").unwrap(), Color::White);
    }

    // // panic is caused by debug assertions - so only run this test in debug
    // #[cfg(debug_assertions)]
    // #[test]
    // #[should_panic]
    // fn piece_panic1() {
    //     let array = [0u32; Piece::len() - 1];
    //     assert_eq!(array[Piece::King], 1); // triggers panic not mismatch
    // }

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
    fn test_move_type() -> Result<()> {
        let many = MoveType::vec_from_string("HIGKPqB").unwrap();
        assert_eq!(many[0], MoveType::Hash);
        assert_eq!(many.last(), Some(&MoveType::BadCapture));
        assert_eq!(MoveType::slice_to_string(&many), "HIGKPqB");
        assert_eq!(
            MoveType::from_chars("CHB")?,
            MoveType::Capture | MoveType::Hash | MoveType::BadCapture
        );
        assert_eq!(
            "HCB".to_string(),
            MoveType::to_string(MoveType::Capture | MoveType::Hash | MoveType::BadCapture)
        );
        assert_eq!(MoveType::Capture.to_string(), "Capture");
        assert_eq!(
            (MoveType::GoodCapture | MoveType::Capture).to_string(),
            "Capture | GoodCapture"
        );

        use bitflags::Flags;
        assert_eq!(MoveType::FLAGS.iter().len(), 19);
        assert_eq!(MoveType::Start.index(), 0);
        assert_eq!(MoveType::End.index(), 18);
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
