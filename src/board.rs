use crate::bits::bitboard::Bitboard;
use crate::bits::castling::CastlingRights;
use crate::bits::square::Square;
use crate::cache::hasher::Hasher;
use crate::domain::material::Material;
use crate::mv::Move;
use crate::piece::{Color, Hash, Piece, Ply, Repeats};
use anyhow::Result;
use anyhow::{bail, Context};
use serde::{Serialize, Serializer};
use serde_with::DeserializeFromStr;
use std::cell::Cell;
use std::fmt::{self, Write};
use std::iter::*;
use std::str::FromStr;

pub mod analysis;
pub mod boardbuf;
pub mod boardcalcs;
pub mod makemove;
pub mod movegen;
pub mod rules;

pub use boardcalcs::BoardCalcs;

unsafe impl Send for Board {}
unsafe impl Sync for Board {}

#[derive(Clone, PartialEq, Eq, DeserializeFromStr)]
pub struct Board {
    pieces: [Bitboard; Piece::len()],
    colors: [Bitboard; Color::len()],

    fullmove_number: u16,
    turn: Color,
    repetition_count: Cell<Repeats>,
    hash: Hash,
    ply: Ply,

    castling: CastlingRights,
    en_passant: Bitboard,
    fifty_clock: u16,
    threats_to: [Cell<Bitboard>; Color::len()],
    checkers_of: [Cell<Bitboard>; Color::len()],
    pinned: [Cell<Bitboard>; Color::len()],
    discoverer: [Cell<Bitboard>; Color::len()],
}

impl Serialize for Board {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_fen())
    }
}

// impl<'de> Deserialize<'de> for Board {
//     fn deserialize<D>(deserializer: D) -> Result<Board, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         deserializer.deserialize_str()
//         Ok(Board::new_empty())
//     }
// }

impl fmt::Debug for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Board")
            .field("fen", &self.to_fen())
            .finish()
    }
}

impl FromStr for Board {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        Board::parse_fen(s)
    }
}

impl Board {
    /// white to move, no castling rights or en passant
    #[inline]
    pub fn new_empty() -> Board {
        Default::default()
    }

    #[inline]
    pub fn ply(&self) -> Ply {
        self.ply
    }

    #[inline]
    pub fn pieces(&self, p: Piece) -> Bitboard {
        self.pieces[p]
    }

    // bishops, rooks and queens
    #[inline]
    pub fn line_pieces(&self) -> Bitboard {
        self.rooks() | self.bishops() | self.queens()
    }

    #[inline]
    pub fn non_line_pieces(&self) -> Bitboard {
        self.pawns() | self.knights() | self.kings()
    }

    #[inline]
    pub fn pawns(&self) -> Bitboard {
        self.pieces(Piece::Pawn)
    }

    #[inline]
    pub fn knights(&self) -> Bitboard {
        self.pieces(Piece::Knight)
    }

    #[inline]
    pub fn bishops(&self) -> Bitboard {
        self.pieces(Piece::Bishop)
    }

    #[inline]
    pub fn rooks(&self) -> Bitboard {
        self.pieces(Piece::Rook)
    }

    #[inline]
    pub fn queens(&self) -> Bitboard {
        self.pieces(Piece::Queen)
    }

    #[inline]
    pub fn rooks_or_queens(&self) -> Bitboard {
        self.rooks() | self.queens()
    }

    #[inline]
    pub fn kings(&self) -> Bitboard {
        self.pieces(Piece::King)
    }

    #[inline]
    pub fn color(&self, c: Color) -> Bitboard {
        self.colors[c.index()]
    }

    #[inline]
    pub fn occupied(&self) -> Bitboard {
        self.black() | self.white()
    }

    #[inline]
    pub fn white(&self) -> Bitboard {
        self.colors[Color::White.index()]
    }

    #[inline]
    pub fn black(&self) -> Bitboard {
        self.colors[Color::Black.index()]
    }

    #[inline]
    pub fn piece(&self, sq: Square) -> Option<Piece> {
        match sq {
            _ if sq.is_in(self.pawns()) => Some(Piece::Pawn),
            _ if sq.is_in(self.knights()) => Some(Piece::Knight),
            _ if sq.is_in(self.bishops()) => Some(Piece::Bishop),
            _ if sq.is_in(self.rooks()) => Some(Piece::Rook),
            _ if sq.is_in(self.queens()) => Some(Piece::Queen),
            _ if sq.is_in(self.kings()) => Some(Piece::King),
            _ => None,
        }
    }

    #[inline]
    pub fn piece_unchecked(&self, sq: Square) -> Piece {
        self.piece(sq)
            .unwrap_or_else(|| panic!("No piece found on {} of {} ", sq, self.to_fen()))
    }

    #[inline]
    pub fn remove_piece(&mut self, sq: Bitboard, p: Piece, c: Color) {
        self.pieces[p].remove(sq);
        self.colors[c].remove(sq);
    }

    #[inline]
    pub fn move_piece(&mut self, from_sq: Bitboard, to_sq: Bitboard, p: Piece, c: Color) {
        self.pieces[p] ^= from_sq | to_sq;
        self.colors[c] ^= from_sq | to_sq;
    }

    #[inline]
    pub fn change_piece(&mut self, sq: Bitboard, from: Piece, to: Piece) {
        self.pieces[from].remove(sq);
        self.pieces[to].insert(sq);
    }

    #[inline]
    pub fn set_piece_at(&mut self, sq: Square, p: Option<Piece>) {
        for bb in self.pieces.iter_mut() {
            bb.remove(sq.as_bb());
        }
        // self.0.pieces(p).remove(sq);
        if let Some(p) = p {
            self.pieces[p].insert(sq.as_bb());
        }
        self.calculate_internals();
    }

    #[inline]
    pub fn set_color_at(&mut self, sq: Bitboard, c: Option<Color>) {
        if let Some(c) = c {
            self.colors[c.opposite()].remove(sq);
            self.colors[c].insert(sq);
        } else {
            self.colors[Color::White].remove(sq);
            self.colors[Color::Black].remove(sq);
        }
        self.calculate_internals();
    }
}

impl Board {
    #[inline]
    pub fn repetition_count(&self) -> Repeats {
        self.repetition_count.get()
    }

    pub fn set_repetition_count(&self, reps: Repeats) {
        self.repetition_count.set(reps);
    }

    #[inline]
    fn calculate_internals(&mut self) {
        self.hash = Hasher::default().hash_board(self);
        // self.material.set(Material::niche());
        self.pinned = [
            Cell::<_>::new(Bitboard::niche()),
            Cell::<_>::new(Bitboard::niche()),
        ];
        self.discoverer = [
            Cell::<_>::new(Bitboard::niche()),
            Cell::<_>::new(Bitboard::niche()),
        ];
        self.threats_to = [
            Cell::<_>::new(Bitboard::niche()),
            Cell::<_>::new(Bitboard::niche()),
        ];
        self.checkers_of = [
            Cell::<_>::new(Bitboard::niche()),
            Cell::<_>::new(Bitboard::niche()),
        ];
    }

    #[inline]
    pub fn hash(&self) -> Hash {
        self.hash
    }

    #[inline]
    pub fn castling(&self) -> CastlingRights {
        self.castling
    }

    #[inline]
    pub fn color_us(&self) -> Color {
        self.turn
    }

    #[inline]
    pub fn color_them(&self) -> Color {
        self.turn.opposite()
    }

    #[inline]
    pub fn them(&self) -> Bitboard {
        self.color(self.turn.opposite())
    }

    #[inline]
    pub fn us(&self) -> Bitboard {
        self.color(self.turn)
    }

    #[inline]
    pub fn en_passant(&self) -> Bitboard {
        self.en_passant
    }

    #[inline]
    pub fn fifty_halfmove_clock(&self) -> i32 {
        self.fifty_clock.into()
    }

    #[inline]
    pub fn fullmove_number(&self) -> i32 {
        self.fullmove_number as i32
    }

    #[inline]
    pub fn total_halfmoves(&self) -> Ply {
        2 * self.fullmove_number() as Ply + self.color_us().chooser_wb(0, 1) - 2
    }

    #[inline]
    pub fn material(&self) -> Material {
        // let mut mat = self.material.get();
        // if mat == Material::niche() {
        // mat = Material::from_board(self);
        //     self.material.set(mat);
        // }
        // mat
        Material::from_board(self)
    }

    #[inline]
    pub fn least_valuable_piece(&self, region: Bitboard) -> Bitboard {
        // cannot use b.turn as this flips during see!
        // the king is an attacker too!
        let non_promo_pawns =
            (self.pawns() & self.white() & region & (Bitboard::all().xor(Bitboard::RANK_7)))
                | (self.pawns() & self.black() & region & (Bitboard::all().xor(Bitboard::RANK_2)));
        if non_promo_pawns.any() {
            return non_promo_pawns.first();
        }
        let p = self.knights() & region;
        if p.any() {
            return p.first();
        }
        let p = self.bishops() & region;
        if p.any() {
            return p.first();
        }
        let p = self.rooks() & region;
        if p.any() {
            return p.first();
        }
        let promo_pawns = (self.pawns() & region) - non_promo_pawns;
        if promo_pawns.any() {
            return promo_pawns.first();
        }
        let p = self.queens() & region;
        if p.any() {
            return p.first();
        }
        let p = self.kings() & region;
        if p.any() {
            return p.first();
        }

        Bitboard::EMPTY
    }

    #[inline]
    pub fn most_valuable_piece_except_king(&self, region: Bitboard) -> Option<(Piece, Square)> {
        // we dont count the king here
        for &p in Piece::ALL_BAR_KING.iter().rev() {
            if self.pieces(p).intersects(region) {
                return Some((p, (self.pieces(p) & region).first_square()));
            }
        }
        None
    }

    // https://www.chessprogramming.org/Color_Flipping
    pub fn color_flip(&self) -> Board {
        let mut b = self.clone();
        b.colors = [
            self.colors[1].flip_vertical(),
            self.colors[0].flip_vertical(),
        ];
        b.pieces.iter_mut().for_each(|bb| *bb = bb.flip_vertical());
        b.turn = self.turn.opposite();
        b.en_passant = self.en_passant().flip_vertical();
        b.castling = self.castling.color_flip();
        b.calculate_internals();
        debug_assert!(b.validate().is_ok());
        b
    }

    pub fn to_fen(&self) -> String {
        let b = self.clone();

        let mut fen = Bitboard::RANKS
            .iter()
            .rev()
            .map(|&r| b.get(r))
            .collect::<Vec<String>>()
            .join("/");

        // replace continguous empties by a count
        for i in (1..=8).rev() {
            fen = fen.replace(".".repeat(i).as_str(), i.to_string().as_str());
        }
        format!(
            "{fen} {turn} {castle} {ep} {fifty} {count}",
            fen = fen,
            turn = self.color_us(),
            castle = self.castling(),
            ep = if self.en_passant().is_empty() {
                "-".to_string()
            } else {
                self.en_passant().uci()
            },
            fifty = self.fifty_halfmove_clock(),
            count = self.fullmove_number()
        )
    }
}

// thread_local! {
//     static CACHE: [SimpleCache<Bitboard>;2] = Default::default();
// }
// #[derive(Default)]
// struct CacheX([ArrayCache<Bitboard, LEN_PLY >;2]);

// unsafe impl Sync for CacheX {}

// use static_init::dynamic;
// #[dynamic]
// static CACHE: CacheX = Default::default();

impl Board {
    // all pieces of either color attacking a region
    #[inline]
    pub fn attacked_by(&self, targets: Bitboard) -> Bitboard {
        BoardCalcs::attacked_by(targets, self.occupied(), self)
    }

    #[inline]
    pub fn pinned(&self, king_color: Color) -> Bitboard {
        let mut pi = self.pinned[king_color].get();
        if pi == Bitboard::niche() {
            let pd = BoardCalcs::pinned_and_discoverers(self, king_color);
            self.pinned[king_color].set(pd.0);
            self.discoverer[king_color].set(pd.1);
            pi = pd.0;
        }
        pi
    }

    #[inline]
    pub fn discoverer(&self, king_color: Color) -> Bitboard {
        let mut di = self.discoverer[king_color].get();
        if di == Bitboard::niche() {
            let pd = BoardCalcs::pinned_and_discoverers(self, king_color);
            self.pinned[king_color].set(pd.0);
            self.discoverer[king_color].set(pd.1);
            di = pd.1;
        }
        di
    }

    #[inline]
    pub fn maybe_gives_discovered_check(&self, mv: Move) -> bool {
        debug_assert!(self.is_legal_move(&mv));
        let their_king_color = self.color_them();
        mv.from().is_in(self.discoverer(their_king_color))
    }

    pub fn gives_check(&self, mv: &Move) -> bool {
        debug_assert!(self.is_legal_move(mv));
        let their_king_color = self.color_them();
        self.make_move(mv).is_in_check(their_king_color)
    }

    #[inline]
    pub fn checkers_of(&self, king_color: Color) -> Bitboard {
        let mut ch = self.checkers_of[king_color].get();
        if ch == Bitboard::niche() {
            ch = BoardCalcs::checkers_of(self, king_color);
            self.checkers_of[king_color].set(ch);
        }
        ch
    }

    // #[inline]
    // pub fn checkers_of(&self, king_color: Color) -> Bitboard {
    //     // CACHE.with(|c| {
    //         let checkers = CACHE.0[king_color.index()].probe(self.ply(), self.hash());
    //         if let Some(checkers) = checkers {
    //             checkers
    //         } else {
    //             let ch = BoardCalcs::checkers_of(self, king_color);
    //             CACHE.0[king_color.index()].store(self.ply(), self.hash(), ch);
    //             ch
    //         }
    //     // })
    // }

    #[inline]
    pub fn all_attacks_on(&self, defender: Color) -> Bitboard {
        let mut th = self.threats_to[defender].get();
        if th == Bitboard::niche() {
            th = BoardCalcs::all_attacks_on(self, defender, self.occupied());
            self.threats_to[defender].set(th);
        }
        th
    }

    pub fn has_legal_moves(&self) -> bool {
        !self.legal_moves().is_empty()
    }

    /// called with is_in_check( board.turn() ) to see if currently in check
    pub fn is_in_check(&self, king_color: Color) -> bool {
        let them = self.color(king_color.opposite());
        self.checkers_of(king_color).intersects(them)
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char('\n')?;
        let b = self.clone();
        for &r in Bitboard::RANKS.iter().rev() {
            f.write_str(&b.get(r))?;
            f.write_char('\n')?;
        }
        write!(f, "\nfen: {} \n", self.to_fen())?;
        // write!(fmt, "Moves: {}", self.moves)?;
        if f.alternate() {
            writeln!(f, "Hash: {:x}", self.hash())?;
            writeln!(f, "Rep count: {:x}", self.repetition_count().total)?;
            writeln!(f, "White:\n{}\nBlack:\n{}\n", self.white(), self.black())?;
            for &p in Piece::ALL.iter() {
                writeln!(
                    f,
                    "Pieces: {}{}\n{}\n",
                    p.to_upper_char(),
                    p.to_lower_char(),
                    self.pieces(p)
                )?;
            }
            writeln!(
                f,
                "Pinned on white king:\n{}\n",
                self.pinned[Color::White].get()
            )?;
            writeln!(
                f,
                "Pinned on black king:\n{}\n",
                self.pinned[Color::Black].get()
            )?;
            writeln!(
                f,
                "Checkers of white:\n{}\n",
                self.checkers_of[Color::White].get()
            )?;
            writeln!(
                f,
                "Checkers of black:\n{}\n",
                self.checkers_of[Color::Black].get()
            )?;
            writeln!(
                f,
                "Threats to white:\n{}\n",
                self.threats_to[Color::White].get()
            )?;
            writeln!(
                f,
                "Threats to black:\n{}\n",
                self.threats_to[Color::Black].get()
            )?;
        }

        Ok(())
    }
}

impl Default for Board {
    #[inline]
    fn default() -> Self {
        Board {
            pieces: Default::default(),
            colors: Default::default(),
            castling: CastlingRights::NONE,
            en_passant: Default::default(),
            turn: Default::default(),
            ply: 0,
            fifty_clock: Default::default(),
            fullmove_number: 1,
            repetition_count: Cell::<_>::new(Repeats::default()),
            threats_to: [
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
            ],
            checkers_of: [
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
            ],
            pinned: [
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
            ],
            discoverer: [
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
            ],
            // material: Cell::<_>::new(Material::niche()),
            hash: 0,
            // moves: MoveList,
        }
        // b.hash = Hasher::default().hash_board(&b);
    }
}

impl Board {
    // pub fn new_empty() -> BoardBuf {
    //     BoardBuf { board: Board::new_empty() }
    // }

    #[inline]
    pub fn set_turn(&mut self, c: Color) {
        self.turn = c;
        self.calculate_internals();
    }

    #[inline]
    pub fn set_castling(&mut self, cr: CastlingRights) {
        self.castling = cr;
        self.calculate_internals();
    }

    #[inline]
    pub fn set_en_passant(&mut self, sq: Bitboard) {
        self.en_passant = sq;
        self.calculate_internals();
    }

    #[inline]
    pub fn set_fifty_halfmove_clock(&mut self, hmvc: i32) {
        self.fifty_clock = hmvc as u16;
        self.calculate_internals();
    }

    #[inline]
    pub fn set_fullmove_number(&mut self, fmvc: i32) {
        self.fullmove_number = fmvc as u16;
        self.calculate_internals();
    }

    // #[inline]
    // fn color_at(&self, at: Bitboard) -> Option<Color> {
    //     if self.color(Color::White).contains(at) {
    //         return Some(Color::White);
    //     } else if self.color(Color::Black).contains(at) {
    //         return Some(Color::Black);
    //     }
    //     None
    // }

    #[inline]
    fn color_of(&self, sq: Square) -> Option<Color> {
        if sq.is_in(self.color(Color::White)) {
            return Some(Color::White);
        } else if sq.is_in(self.color(Color::Black)) {
            return Some(Color::Black);
        }
        None
    }

    #[inline]
    fn color_of_unchecked(&self, sq: Square) -> Color {
        self.color_of(sq)
            .unwrap_or_else(|| panic!("No coloured piece at {} of {}", sq, self.to_fen()))
    }

    pub fn get(&self, bb: Bitboard) -> String {
        let mut res = String::new();
        for sq in bb.squares() {
            let p = self.piece(sq);
            let ch = match p {
                // avoid calling unchecked that can recursively call to_fen
                Some(p) => p.to_char(self.color_of(sq).unwrap()),
                None => '.',
            };
            res.push(ch);
        }
        res
    }

    pub fn set(&mut self, bb: Bitboard, pieces: &str) -> Result<&mut Self> {
        if bb.popcount() != pieces.chars().count() as i32 {
            bail!(
                "Bitboard {} and pieces {} have different counts",
                bb,
                pieces
            );
        }
        for (sq, ch) in bb.squares().zip(pieces.chars()) {
            match ch {
                '.' | ' ' => {
                    self.set_piece_at(sq, None);
                    self.set_color_at(sq.as_bb(), None);
                }
                _ => {
                    let p = Piece::from_char(ch)?;
                    self.set_piece_at(sq, Some(p));
                    let c = Color::from_piece_char(ch)?;
                    self.set_color_at(sq.as_bb(), Some(c));
                }
            }
        }
        self.calculate_internals();
        Ok(self)
    }

    pub fn as_board(&self) -> Board {
        self.clone()
    }

    pub fn validate(&self) -> Result<()> {
        if self.black().intersects(self.white()) {
            bail!(
                "White\n{}\n and black\n{}\n are not disjoint",
                self.white(),
                self.black()
            );
        }
        let mut bb = Bitboard::all();
        for &p in Piece::ALL.iter() {
            bb &= self.pieces(p);
        }
        if !bb.is_empty() {
            bail!("Piece bitboards are not disjoint");
        }

        // if self.fullmove_counter() < self.fifty_halfmove_clock() * 2 {
        //     bail!("Fullmove number (fmvn: {}) < twice half move clock (hmvc: {})", self.fullmove_counter(), self.fifty_halfmove_clock() );
        // }
        let ep = self.en_passant();
        if !ep.is_empty() {
            if !ep.intersects(Bitboard::RANK_3 | Bitboard::RANK_6) {
                bail!(
                    "En passant square must be rank 3 or 6 not {}",
                    ep.sq_as_uci()
                );
            }
            let capture_square = ep.shift(self.color_them().forward());
            if !(self.pawns() & self.them()).contains(capture_square) {
                bail!(
                    "En passant square of {} entails a pawn on square {}",
                    ep.sq_as_uci(),
                    capture_square.sq_as_uci()
                );
            }
        }
        if self.hash() != Hasher::default().hash_board(self) {
            bail!("Hash is incorrect");
        }
        Ok(())
    }

    /// Parses a FEN string to create a board. FEN format is detailed at https://en.wikipedia.org/wiki/Forsyth–Edwards_Notation
    /// terminology of "piece placement data" from http://kirill-kryukov.com/chess/doc/fen.html
    pub fn parse_piece_placement(fen: &str) -> Result<Self> {
        let mut bb = Board::new_empty();
        let mut pos = String::from(fen);
        for i in 1..=8 {
            pos = pos.replace(i.to_string().as_str(), " ".repeat(i).as_str());
        }
        // pos.retain(|ch| "pPRrNnBbQqKk ".contains(ch));
        let r: Vec<&str> = pos.rsplit('/').collect();
        if r.iter().any(|r| r.chars().count() != 8) || r.len() != 8 {
            bail!("Expected 8 ranks of 8 pieces in fen {}", fen);
        }
        bb.set(Bitboard::all(), &r.concat())?;
        bb.calculate_internals();
        Ok(bb)
    }

    /// 0. Piece placement
    /// 1. Active color
    /// 2. Castling rights
    /// 3. E/P square
    /// 4. Half move clock
    /// 5. Full move counter
    pub fn parse_fen(fen: &str) -> Result<Self> {
        let words = fen.split_whitespace().collect::<Vec<_>>();
        if words.len() < 6 {
            bail!("Must specify at least 6 parts in epd/fen '{}'", fen);
        }
        let mut bb = Self::parse_piece_placement(words[0])?;
        bb.turn = Color::parse(words[1])?;
        bb.castling = CastlingRights::parse(words[2])?;
        bb.en_passant = if words[3] == "-" {
            Bitboard::EMPTY
        } else {
            Bitboard::parse_square(words[3])?.as_bb()
        };
        bb.fifty_clock = words[4]
            .parse()
            .context(format!("Invalid halfmove clock '{}'", words[4]))?;
        bb.fullmove_number = words[5]
            .parse()
            .context(format!("Invalid fullmove count '{}'", words[5]))?;
        bb.calculate_internals();
        bb.validate()?;
        Ok(bb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::*;
    use crate::globals::constants::*;
    use crate::infra::black_box;
    use crate::infra::profiler::Profiler;

    #[test]
    fn test_serde() {
        let board1 = Board::parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .unwrap()
            .as_board();
        assert_eq!(
            serde_json::to_string(&board1).unwrap(),
            "\"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1\""
        );
        assert_eq!(
            serde_json::from_str::<Board>(
                "\"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1\""
            )
            .unwrap(),
            board1
        );
    }

    #[test]
    fn test_color_flip() {
        let board1 = Board::parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .unwrap()
            .as_board();
        let board2 = Board::parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1")
            .unwrap()
            .as_board();
        assert_eq!(
            board1.color_flip().to_fen(),
            board2.to_fen(),
            "{:#}\n{:#}",
            board1.color_flip(),
            board2
        );
        assert_eq!(board2.color_flip().to_fen(), board1.to_fen());

        let board1 =
            Board::parse_fen("rnb1k2r/pp3ppp/4p3/3pB3/2pPn3/2P1PN2/q1P1QPPP/2KR1B1R b kq - 1 11")
                .unwrap();
        let board2 =
            Board::parse_fen("2kr1b1r/Q1p1qppp/2p1pn2/2PpN3/3Pb3/4P3/PP3PPP/RNB1K2R w KQ - 1 11")
                .unwrap();
        assert_eq!(
            board1.color_flip().to_fen(),
            board2.to_fen(),
            "{:#}\n{:#}",
            board1.color_flip(),
            board2
        );
        assert_eq!(board2.color_flip().to_fen(), board1.to_fen());
    }

    #[test]
    fn to_fen() {
        for &fen in &[
            "7k/8/8/8/8/8/8/7K b KQkq - 45 100",
            Catalog::STARTING_POSITION_FEN,
            "8/8/8/8/8/8/8/B7 w - - 0 0",
        ] {
            let b = Board::parse_fen(fen).unwrap().as_board();
            assert_eq!(fen, b.to_fen());
            println!("{:#}", b);
        }
    }

    #[test]
    fn board_bitboards() -> Result<(), String> {
        let board = Board::parse_piece_placement("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR")
            .unwrap()
            .as_board();
        assert_eq!(board.color_us(), Color::White);
        assert_eq!(board.color_them(), Color::Black);
        // assert_eq!(board.en_passant(), Bitboard::empty());
        // assert_eq!(board.move_count(), 0);
        assert_eq!(board.pawns() & board.us(), Bitboard::RANK_2);
        assert_eq!(board.rooks() & board.them(), a8 | h8);
        assert_eq!(board.bishops() & board.us(), c1 | f1);
        assert_eq!(board.them(), Bitboard::RANK_7 | Bitboard::RANK_8);
        Ok(())
    }

    //
    // interface designs....
    //
    // let b = hashmap!{ a1+h1 => "R", b1+g1 => "N" };
    // let b = BoardBuf::new().rooks(a1|h1).knights(b1|g1).pawns(rank_2).set("RNBQKBNR", rank_1);
    // let b = BoardBuf::new("rnbqkbnr/
    //     pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR");
    // let b = BoardBuf::new().k(a1).K(h8).r(a2).R(c3);
    // let b = BoardBuf::new().set(a1=k, rank_2=p, );
    // todo!()

    #[test]
    fn boardbuf_sets() -> Result<()> {
        let board = Board::new_empty();
        assert_eq!(board.kings(), Bitboard::EMPTY);
        assert_eq!(board.us(), Bitboard::EMPTY);
        assert_eq!(board.color_us(), Color::White);

        // assert_eq!(board[a1], 'R');
        let mut board1 = Board::new_empty();
        board1 = board1
            .set(Bitboard::RANK_2, "PPPPPPPP")?
            .set(a1 | h1, "RR")?
            .set(b1 | g1, "NN")?
            .set(c1 | d1 | e1 | f1, "BQKB")?
            .as_board();
        board1
            .set(Bitboard::RANK_7, "pppppppp")?
            .set(Bitboard::RANK_8, "rnbqkbnr")?
            .as_board();
        assert_eq!(board1.get(a1), "R");
        let str1 = board1.to_string();
        let mut board2 = board1;
        let board2 = board2
            .set(Bitboard::RANK_7, "pppppppp")?
            .set(Bitboard::RANK_8, "rnbqkbnr")?
            .as_board();
        assert_eq!(str1, board2.to_string());
        println!("{}", board2.as_board());
        Ok(())
    }

    #[test]
    fn parse_piece() -> Result<()> {
        let fen1 = "1/1/7/8/8/8/PPPPPPPP/RNBQKBNR";
        assert_eq!(
            Board::parse_piece_placement(fen1).unwrap_err().to_string(),
            "Expected 8 ranks of 8 pieces in fen 1/1/7/8/8/8/PPPPPPPP/RNBQKBNR"
        );
        assert!(Board::parse_piece_placement("8")
            .unwrap_err()
            .to_string()
            .starts_with("Expected 8"));
        assert!(Board::parse_piece_placement("8/8")
            .unwrap_err()
            .to_string()
            .starts_with("Expected 8"));
        assert_eq!(
            Board::parse_piece_placement("X7/8/8/8/8/8/8/8")
                .unwrap_err()
                .to_string(),
            "Unknown piece 'X'"
        );
        let buf =
            Board::parse_piece_placement("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR").unwrap();
        assert_eq!(buf.get(a1), "R");
        assert_eq!(buf.get(Bitboard::FILE_H), "RP....pr");
        Ok(())
    }

    #[test]
    fn parse_fen() -> Result<()> {
        let b = Board::parse_fen("7k/8/8/8/8/8/8/7K b KQkq - 45 100")?.as_board();
        assert_eq!(b.color_us(), Color::Black);
        assert_eq!(b.fullmove_number(), 100);
        assert_eq!(b.fifty_halfmove_clock(), 45);
        assert_eq!(b.castling(), CastlingRights::all());
        Ok(())
    }
    #[test]
    fn parse_invalid_fen() -> Result<()> {
        assert_eq!(
            Board::parse_fen("7k/8/8/8/8/8/8/7K B Qkq - 45 100")
                .unwrap_err()
                .to_string(),
            "Invalid color: 'B'".to_string()
        );
        assert_eq!(
            Board::parse_fen("7k/8/8/8/8/8/8/7K b XQkq - 45 100")
                .unwrap_err()
                .to_string(),
            "Invalid character 'X' in castling rights 'XQkq'".to_string()
        );
        assert_eq!(
            Board::parse_fen("7k/8/8/8/8/8/8/7K b - - fifty 100")
                .unwrap_err()
                .to_string(),
            "Invalid halfmove clock 'fifty'".to_string()
        );
        assert_eq!(
            Board::parse_fen("7k/8/8/8/8/8/8/7K b - - 50 full")
                .unwrap_err()
                .to_string(),
            "Invalid fullmove count 'full'".to_string()
        );
        Ok(())
    }

    #[test]
    fn bench_board() {
        let bd =
            Board::parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();

        let mut prof1 = Profiler::new("board.piece".into());
        for _ in 0..100 {
            for sq in bd.occupied().squares() {
                prof1.benchmark(|| bd.piece(black_box(sq)));
            }
        }
    }
}
