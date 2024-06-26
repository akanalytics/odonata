use std::fmt::{self, Write};
use std::iter::*;
use std::str::FromStr;

use anyhow::bail;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::hasher::Hasher;
use super::BoardCalcs;
use crate::bits::bitboard::LazyBitboard;
use crate::bits::castling::CastlingRights;
use crate::catalog::Catalog;
use crate::domain::Material;
use crate::infra::utils::ToStringOr;
use crate::prelude::*;

pub struct Var {
    moves:  Vec<Move>,
    boards: Vec<Board>,
    ply:    usize,
}

impl Var {
    pub fn new(b: Board) -> Self {
        let mut me = Self {
            boards: Default::default(),
            moves:  Vec::new(),
            ply:    0,
        };
        me.boards.resize(128, Board::default());
        me.boards[0] = b;
        me
    }

    #[inline]
    pub fn board(&self) -> &Board {
        &self.boards[self.ply]
    }

    #[inline]
    pub fn ply(&self) -> usize {
        self.ply
        // self.moves.len()
    }

    pub fn board_mut(&mut self) -> &mut Board {
        let i = self.ply();
        &mut self.boards[i]
    }

    #[inline]
    pub fn push_move(&mut self, mv: Move) {
        let i = self.ply();
        self.ply += 1;
        self.moves.push(mv);
        // mem::swap(&mut self.current, &mut self.boards[i]); // board in [i]
        // self.current.copy_from(&self.boards[i]);
        // self.current.apply_move(mv);

        let (start, end) = self.boards.split_at_mut(i + 1);
        end[0].copy_from(&start[i]);
        end[0].apply_move(mv);
    }

    #[inline]
    pub fn pop_move(&mut self) {
        self.ply -= 1;
        // let i = self.ply();
        // mem::swap(&mut self.current, &mut self.boards[i]); // board in [i]
        self.moves.pop();
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Board {
    pub(super) pieces:          [Bitboard; Piece::len()],
    pub(super) colors:          [Bitboard; Color::len()],
    pub(super) fullmove_number: u16,
    pub(super) turn:            Color,
    pub(super) hash:            Hash,
    pub(super) ply:             Ply,
    pub(super) castling:        CastlingRights,
    pub(super) en_passant:      Option<Square>,
    pub(super) halfmove_clock:  u16,
    pub(super) threats_to:      [LazyBitboard<{ Bitboard::ALL.bits() }>; Color::len()],
    pub(super) checkers_of:     [LazyBitboard<{ Bitboard::ALL.bits() }>; Color::len()],
    pub(super) pinned:          [LazyBitboard<{ Bitboard::ALL.bits() }>; Color::len()],
    pub(super) discoverer:      [LazyBitboard<{ Bitboard::ALL.bits() }>; Color::len()],
}

#[derive(Clone, Debug, Default)]
pub struct BoardBuilder(Board);

// const ASSERT2: () = assert!(std::mem::size_of::<Board>() == 152);

impl PartialEq for Board {
    fn eq(&self, other: &Self) -> bool {
        self.pieces == other.pieces
            && self.colors == other.colors
            && self.fullmove_number == other.fullmove_number
            && self.turn == other.turn
            // && self.hash == other.hash
            && self.ply == other.ply
            && self.castling == other.castling
            && self.en_passant == other.en_passant
            && self.halfmove_clock == other.halfmove_clock
        // && self.threats_to == other.threats_to
        // && self.checkers_of == other.checkers_of
        // && self.pinned == other.pinned
        // && self.discoverer == other.discoverer
    }
}

impl Eq for Board {}

// impl Serialize for Board {
//     fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
//         serializer.serialize_str(&self.to_fen())
//     }
// }

impl Default for Board {
    #[inline]
    fn default() -> Self {
        Board {
            pieces:          Default::default(),
            colors:          Default::default(),
            castling:        CastlingRights::NONE,
            en_passant:      None,
            turn:            Default::default(),
            ply:             0,
            halfmove_clock:  0,
            fullmove_number: 1,
            threats_to:      Default::default(),
            checkers_of:     Default::default(),
            pinned:          Default::default(),
            discoverer:      Default::default(),
            hash:            0,
            // moves: MoveList,
        }
        // b.hash = Hasher::default().hash_board(&b);
    }
}

impl fmt::Debug for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Board").field("fen", &self.to_fen()).finish()
    }
}

impl FromStr for Board {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        Board::parse_fen(s)
    }
}

impl BoardBuilder {
    pub fn try_build(mut self) -> Result<Board> {
        self.0.calculate_internals();
        self.0.validate()?;
        Ok(self.0)
    }

    /// may panic or crate an invalid board - use try_build if unsure
    #[inline]
    pub fn build(mut self) -> Board {
        self.0.calculate_internals();
        self.0
    }

    pub fn set_castling(&mut self, castling: CastlingRights) {
        self.0.castling = castling;
    }

    pub fn set_ep_square(&mut self, ep: Option<Square>) {
        self.0.en_passant = ep;
    }

    pub fn set_turn(&mut self, turn: Color) {
        self.0.turn = turn;
    }

    pub fn set_fullmove_number(&mut self, fmvn: u16) {
        self.0.fullmove_number = fmvn;
    }

    pub fn set_halfmove_clock(&mut self, hmvc: u16) {
        self.0.halfmove_clock = hmvc;
    }

    /// clears and adds a a piece
    pub fn set_piece(&mut self, sq: Square, pc: Option<(Piece, Color)>) {
        let bb = sq.as_bb();
        self.0.colors[Color::White].remove(bb);
        self.0.colors[Color::Black].remove(bb);
        self.0.pieces[Piece::Pawn].remove(bb);
        self.0.pieces[Piece::Knight].remove(bb);
        self.0.pieces[Piece::Bishop].remove(bb);
        self.0.pieces[Piece::Rook].remove(bb);
        self.0.pieces[Piece::Queen].remove(bb);
        self.0.pieces[Piece::King].remove(bb);
        if let Some((p, c)) = pc {
            self.add_piece(sq, p, c);
        }
    }

    /// will not clear other pieces on this square. explicitly remove them first if required
    #[inline]
    pub fn add_piece(&mut self, sq: Square, p: Piece, c: Color) {
        let bb = sq.as_bb();
        self.0.colors[c].insert(bb);
        self.0.pieces[p].insert(bb);
    }

    /// Parses a FEN string to create a board. FEN format is detailed at https://en.wikipedia.org/wiki/Forsyth–Edwards_Notation
    /// terminology of "piece placement data" from http://kirill-kryukov.com/chess/doc/fen.html
    pub fn parse_piece_placement(fen: &str) -> Result<Self> {
        let mut pos = String::from(fen);
        for i in 1..=8 {
            pos = pos.replace(i.to_string().as_str(), " ".repeat(i).as_str());
        }
        // pos.retain(|ch| "pPRrNnBbQqKk ".contains(ch));
        let r: Vec<&str> = pos.rsplit('/').collect();
        if r.iter().any(|r| r.chars().count() != 8) || r.len() != 8 {
            bail!("expected 8 ranks of 8 pieces in fen {}", fen);
        }
        let mut bb = Board::builder();
        bb.set(Bitboard::all(), &r.concat())?;
        Ok(bb)
    }

    pub fn clear(&mut self, bb: Bitboard) {
        for sq in (self.0.occupied() & bb).squares() {
            self.set_piece(sq, None);
        }
    }

    pub fn set(&mut self, bb: Bitboard, pieces: &str) -> Result<()> {
        if bb.popcount() != pieces.chars().count() as i32 {
            bail!("Bitboard {} and pieces {} have different counts", bb, pieces);
        }
        for (sq, ch) in bb.squares().zip(pieces.chars()) {
            match ch {
                '.' | ' ' => {
                    self.set_piece(sq, None);
                }
                _ => {
                    let p = Piece::from_char(ch)?;
                    let c = Color::from_piece_char(ch)?;
                    self.set_piece(sq, Some((p, c)));
                }
            }
        }
        Ok(())
    }
}

impl Board {
    fn clone_from2(&mut self, src: &Board) {
        *self = Self {
            threats_to: src.threats_to.clone(),
            checkers_of: src.checkers_of.clone(),
            pinned: src.pinned.clone(),
            discoverer: src.discoverer.clone(),
            ..*src
        }
    }
}

impl Board {
    /// white to move, no castling rights or en passant
    pub fn new_empty() -> Board {
        Default::default()
    }

    pub fn builder() -> BoardBuilder {
        BoardBuilder(Self::default())
    }

    pub fn into_builder(self) -> BoardBuilder {
        BoardBuilder(self)
    }

    pub fn starting_pos() -> Self {
        Catalog::starting_board()
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
    pub fn bishops_or_queens(&self) -> Bitboard {
        self.bishops() | self.queens()
    }

    #[inline]
    pub fn king(&self, king_color: Color) -> Square {
        (self.pieces(Piece::King) & self.color(king_color))
            .find_first_square()
            .expect("no king found")
    }

    #[inline]
    pub fn our_king(&self) -> Square {
        (self.pieces(Piece::King) & self.us())
            .find_first_square()
            .expect("no king found")
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
    pub fn is_occupied_by(&self, sq: Square, p: Piece) -> bool {
        sq.is_in(self.pieces(p))
    }

    #[inline]
    pub fn piece_unchecked(&self, sq: Square) -> Piece {
        match sq {
            _ if sq.is_in(self.pawns()) => Piece::Pawn,
            _ if sq.is_in(self.knights()) => Piece::Knight,
            _ if sq.is_in(self.bishops()) => Piece::Bishop,
            _ if sq.is_in(self.rooks()) => Piece::Rook,
            _ if sq.is_in(self.queens()) => Piece::Queen,
            _ if sq.is_in(self.kings()) => Piece::King,
            _ => panic!("No piece found for sq {sq} on {self}"),
        }
    }

    pub fn toggle_piece(&mut self, sq: Bitboard, p: Piece, c: Color) {
        self.pieces[p] ^= sq;
        self.colors[c] ^= sq;
    }

    pub fn move_piece(&mut self, from_sq: Bitboard, to_sq: Bitboard, p: Piece, c: Color) {
        self.pieces[p] ^= from_sq | to_sq;
        self.colors[c] ^= from_sq | to_sq;
    }

    pub fn change_piece(&mut self, sq: Bitboard, from: Piece, to: Piece) {
        self.pieces[from] ^= sq;
        self.pieces[to] ^= sq;
    }

    // pub fn toggle_piece(&mut self, sq: Square, p: Piece, c: Color) {
    //     let bb = sq.as_bb();
    //     self.pieces[p] ^= bb;
    //     self.colors[c] ^= bb;
    // }

    // pub fn move_piece(&mut self, from_sq: Square, to_sq: Square, p: Piece, c: Color) {
    //     let bb = from_sq.as_bb()| to_sq.as_bb();
    //     self.pieces[p] ^= bb;
    //     self.colors[c] ^= bb;
    // }

    // pub fn change_piece(&mut self, sq: Square, from: Piece, to: Piece) {
    //     let bb = sq.as_bb();
    //     self.pieces[from] ^= bb;
    //     self.pieces[to] ^= bb;
    // }

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

    pub fn set_color_at(&mut self, sq: Square, c: Option<Color>) {
        let bb = sq.as_bb();
        if let Some(c) = c {
            self.colors[c.flip_side()].remove(bb);
            self.colors[c].insert(bb);
        } else {
            self.colors[Color::White].remove(bb);
            self.colors[Color::Black].remove(bb);
        }
        self.calculate_internals();
    }
}

impl Board {
    pub fn is_draw_insufficient_material(&self) -> bool {
        self.material().is_insufficient()
    }

    pub fn is_draw_rule_fifty(&self) -> bool {
        self.halfmove_clock() >= 2 * 50
    }

    #[inline]
    pub fn calculate_internals(&mut self) {
        self.hash = Hasher::instance().hash_board(self);
        // self.material.set(Material::niche());
        self.pinned = Default::default();
        self.discoverer = Default::default();
        self.threats_to = Default::default();
        self.checkers_of = Default::default();
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
    pub fn turn(&self) -> Color {
        self.turn
    }

    #[inline]
    pub fn color_them(&self) -> Color {
        self.turn.flip_side()
    }

    #[inline]
    pub fn them(&self) -> Bitboard {
        self.color(self.turn.flip_side())
    }

    #[inline]
    pub fn us(&self) -> Bitboard {
        self.color(self.turn)
    }

    #[inline]
    pub fn is_en_passant_square(&self, sq: Square) -> bool {
        Some(sq) == self.en_passant
    }

    #[inline]
    pub fn en_passant_square(&self) -> Option<Square> {
        self.en_passant
    }

    #[inline]
    pub fn halfmove_clock(&self) -> i32 {
        self.halfmove_clock.into()
    }

    #[inline]
    pub fn fullmove_number(&self) -> i32 {
        self.fullmove_number as i32
    }

    #[inline]
    pub fn total_halfmove_ply(&self) -> Ply {
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
    pub fn least_valuable_piece(&self, region: Bitboard) -> Option<Square> {
        // cannot use b.turn as this flips during see!
        // the king is an attacker too!
        let non_promo_pawns = (self.pawns() & self.white() & region & (Bitboard::all().xor(Bitboard::RANK_7)))
            | (self.pawns() & self.black() & region & (Bitboard::all().xor(Bitboard::RANK_2)));
        if non_promo_pawns.any() {
            return non_promo_pawns.find_first_square();
        }
        let p = self.knights() & region;
        if p.any() {
            return p.find_first_square();
        }
        let p = self.bishops() & region;
        if p.any() {
            return p.find_first_square();
        }
        let p = self.rooks() & region;
        if p.any() {
            return p.find_first_square();
        }
        let promo_pawns = (self.pawns() & region) - non_promo_pawns;
        if promo_pawns.any() {
            return promo_pawns.find_first_square();
        }
        let p = self.queens() & region;
        if p.any() {
            return p.find_first_square();
        }
        let p = self.kings() & region;
        if p.any() {
            return p.find_first_square();
        }

        None
    }

    #[inline]
    pub fn most_valuable_piece_except_king(&self, region: Bitboard) -> Option<(Piece, Square)> {
        // we dont count the king here
        for &p in Piece::ALL_BAR_KING.iter().rev() {
            if let Some(square) = (self.pieces(p) & region).find_first_square() {
                return Some((p, square));
            }
        }
        None
    }

    // https://www.chessprogramming.org/Color_Flipping
    /// flips vertical and changes color
    pub fn color_flip(&self) -> Board {
        let mut b = self.clone();
        b.colors = [self.colors[1].flip_vertical(), self.colors[0].flip_vertical()];
        b.pieces.iter_mut().for_each(|bb| *bb = bb.flip_vertical());
        b.turn = self.turn.flip_side();
        if let Some(sq) = b.en_passant {
            b.en_passant = Some(sq.flip_vertical());
        }
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
            ep = if let Some(sq) = self.en_passant_square() {
                sq.uci().to_string()
            } else {
                "-".to_string()
            },
            fifty = self.halfmove_clock(),
            count = self.fullmove_number()
        )
    }
}

impl Board {
    // all pieces of either color attacking a region
    #[inline]
    pub fn attacked_by(&self, targets: Bitboard) -> Bitboard {
        BoardCalcs::attacked_by(targets, self.occupied(), self)
    }

    #[inline]
    pub fn pinned(&self, king_color: Color) -> Bitboard {
        let mut pi = self.pinned[king_color].get();
        if pi == self.pinned[Color::White].niche() {
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
        if di == self.discoverer[Color::White].niche() {
            let pd = BoardCalcs::pinned_and_discoverers(self, king_color);
            self.pinned[king_color].set(pd.0);
            self.discoverer[king_color].set(pd.1);
            di = pd.1;
        }
        di
    }

    #[inline]
    pub fn maybe_gives_discovered_check(&self, mv: Move) -> bool {
        debug_assert!(mv.is_valid(self));
        let their_king_color = self.color_them();
        mv.from().is_in(self.discoverer(their_king_color))
    }

    pub fn gives_check(&self, mv: Move) -> bool {
        debug_assert!(mv.is_valid(self));
        let their_king_color = self.color_them();
        self.make_move(mv).is_in_check(their_king_color)
    }

    #[inline]
    pub fn checkers_of(&self, king_color: Color) -> Bitboard {
        self.checkers_of[king_color].get_or_init(|| BoardCalcs::checkers_of(self, king_color))
    }

    #[inline]
    pub fn all_attacks_on(&self, defender: Color) -> Bitboard {
        self.threats_to[defender].get_or_init(|| BoardCalcs::all_attacks_on(self, defender, self.occupied()))
    }
    pub fn has_legal_moves(&self) -> bool {
        let mut has_moves = false;
        self.legal_moves_with(|_mv| has_moves = true);
        has_moves
    }

    // pub fn count_legal_moves(&self) -> usize {
    //     let mut count_moves = 0;
    //     self.legal_moves_general(Bitboard::all(), |_mk, _p, _sq, bb| {
    //         count_moves += bb.popcount()
    //     });
    //     count_moves as usize
    // }

    /// called with is_in_check( board.turn() ) to see if currently in check
    pub fn is_in_check(&self, king_color: Color) -> bool {
        let them = self.color(king_color.flip_side());
        self.checkers_of(king_color).intersects(them)
    }

    pub fn to_diagram(&self) -> String {
        let mut f = String::new();
        for &r in Bitboard::RANKS.iter().rev() {
            f += &self.get(r);
            f += "\n";
        }
        f
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.fill() == 'L' {
            let url = format!("https://lichess.org/editor/{}", self.to_fen()).replace(' ', "_");
            write!(f, "{url}")?;
        } else {
            write!(f, "{}", self.to_fen())?;
        }

        if f.alternate() {
            f.write_char('\n')?;
            f.write_str(&self.to_diagram())?;
            write!(f, "\nfen: {} \n", self.to_fen())?;
            // write!(fmt, "Moves: {}", self.moves)?;
            writeln!(f, "Hash: {:x}", self.hash())?;
            writeln!(f, "Ply: {}", self.ply())?;
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
            writeln!(f, "En passant: {}\n", self.en_passant.to_string_or("-"))?;
            writeln!(f, "Pinned on white king:\n{}\n", self.pinned[Color::White].get())?;
            writeln!(f, "Pinned on black king:\n{}\n", self.pinned[Color::Black].get())?;
            writeln!(f, "Checkers of white:\n{}\n", self.checkers_of[Color::White].get())?;
            writeln!(f, "Checkers of black:\n{}\n", self.checkers_of[Color::Black].get())?;
            writeln!(f, "Threats to white:\n{}\n", self.threats_to[Color::White].get())?;
            writeln!(f, "Threats to black:\n{}\n", self.threats_to[Color::Black].get())?;
        }

        Ok(())
    }
}

impl Board {
    // pub fn new_empty() -> BoardBuf {
    //     BoardBuf { board: Board::new_empty() }
    // }

    pub fn set_turn(&mut self, c: Color) {
        self.turn = c;
        self.calculate_internals();
    }

    pub fn set_castling(&mut self, cr: CastlingRights) {
        self.castling = cr;
        self.calculate_internals();
    }

    #[inline]
    pub fn set_en_passant(&mut self, sq: Option<Square>) {
        self.en_passant = sq;
        self.calculate_internals();
    }

    pub fn set_halfmove_clock(&mut self, hmvc: i32) {
        self.halfmove_clock = hmvc as u16;
        self.calculate_internals();
    }

    pub fn set_ply(&mut self, ply: i32) {
        self.ply = ply;
        self.calculate_internals();
    }

    pub fn set_fullmove_number(&mut self, fmvc: i32) {
        self.fullmove_number = fmvc as u16;
        self.calculate_internals();
    }

    #[inline]
    pub fn color_of(&self, sq: Square) -> Option<Color> {
        if sq.is_in(self.color(Color::White)) {
            return Some(Color::White);
        } else if sq.is_in(self.color(Color::Black)) {
            return Some(Color::Black);
        }
        None
    }

    // #[inline]
    // fn color_of_unchecked(&self, sq: Square) -> Color {
    //     self.color_of(sq)
    //         .unwrap_or_else(|| panic!("No coloured piece at {} of {}", sq,
    // self.to_fen())) }

    pub fn get(&self, bb: Bitboard) -> String {
        let mut res = String::new();
        for sq in bb.squares() {
            let p = self.piece(sq);
            let ch = match p {
                // avoid calling unchecked that can recursively call to_fen
                Some(p) => p.to_char(self.color_of(sq).unwrap_or_default()),
                None => '.',
            };
            res.push(ch);
        }
        res
    }

    // pub fn as_board(&self) -> Board {
    //     self.clone()
    // }

    pub fn validate(&self) -> Result<()> {
        if self.black().intersects(self.white()) {
            bail!(
                "White\n{}\n and black\n{}\n are not disjoint",
                self.white(),
                self.black()
            );
        }

        let mut bb = Bitboard::empty();
        for &p in Piece::ALL.iter() {
            let pieces = self.pieces(p);
            if pieces.intersects(bb) {
                bail!("Piece bitboard for {p} intersects other pieces {self:#}");
            }
            if !self.occupied().contains(pieces) {
                bail!("Piece bitboard for {p} not contained in black/white {self:#}");
            }
            bb |= pieces;
        }
        if bb != self.occupied() {
            bail!("Piece bitboards and occupied squares do not match {self:#}");
        }
        // if self.fullmove_counter() < self.fifty_halfmove_clock() * 2 {
        //     bail!("Fullmove number (fmvn: {}) < twice half move clock (hmvc: {})",
        // self.fullmove_counter(), self.fifty_halfmove_clock() ); }
        if let Some(ep) = self.en_passant_square() {
            if !ep.is_in(Bitboard::RANK_3 | Bitboard::RANK_6) {
                bail!("en passant square must be rank 3 or 6 not {}", ep.uci());
            }
            let capture_square = ep.shift(self.color_them().forward());
            if !capture_square.is_in(self.pawns() & self.them()) {
                bail!("en passant square of {ep} entails a pawn on square {capture_square}",);
            }
        }
        if self.hash() != Hasher::instance().hash_board(self) {
            bail!("Hash is incorrect");
        }
        Ok(())
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
            bail!("must specify at least 6 parts in epd/fen '{}'", fen);
        }
        let mut bb = BoardBuilder::parse_piece_placement(words[0])?;
        bb.set_turn(Color::parse(words[1])?);
        bb.set_castling(CastlingRights::parse(words[2])?);
        bb.set_ep_square(if words[3] == "-" {
            None
        } else {
            Some(Square::parse(words[3])?)
        });
        bb.set_halfmove_clock(
            words[4]
                .parse()
                .with_context(|| format!("invalid halfmove clock '{}'", words[4]))?,
        );
        bb.set_fullmove_number(
            words[5]
                .parse()
                .with_context(|| format!("invalid fullmove count '{}'", words[5]))?,
        );
        bb.try_build()
    }

    pub fn parse_diagram(s: &str) -> Result<Self> {
        static REGEX_CR_PLUS_WS: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s*\n\s*").unwrap());
        let s = s.trim_start();
        let fen = REGEX_CR_PLUS_WS.replace_all(s, "/");
        Board::parse_fen(&fen)
    }
}

#[cfg(test)]
mod tests {
    use std::hint::black_box;

    use test_log::test;

    use super::*;
    use crate::infra::profiler::PerfProfiler;
    use crate::other::Perft;

    #[test]
    fn test_serde() {
        let board1 = Board::parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let s = serde_json::to_string(&board1).unwrap();
        let board2 = serde_json::from_str::<Board>(&s).unwrap();
        assert_eq!(board1, board2);
    }

    #[test]
    fn test_color_flip() {
        let board1 = Board::parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let board2 = Board::parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1").unwrap();
        assert_eq!(
            board1.color_flip().to_fen(),
            board2.to_fen(),
            "{:#}\n{:#}",
            board1.color_flip(),
            board2
        );
        assert_eq!(board2.color_flip().to_fen(), board1.to_fen());

        let board1 = Board::parse_fen("rnb1k2r/pp3ppp/4p3/3pB3/2pPn3/2P1PN2/q1P1QPPP/2KR1B1R b kq - 1 11").unwrap();
        let board2 = Board::parse_fen("2kr1b1r/Q1p1qppp/2p1pn2/2PpN3/3Pb3/4P3/PP3PPP/RNB1K2R w KQ - 1 11").unwrap();
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
            let b = Board::parse_fen(fen).unwrap();
            assert_eq!(fen, b.to_fen());
            println!("{:#}", b);
            println!("{}", b);
            println!("{:L>}", b);
        }
    }

    #[test]
    fn board_bitboards() -> Result<()> {
        use Square::*;
        let board = BoardBuilder::parse_piece_placement("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR")?.build();
        assert_eq!(board.color_us(), Color::White);
        assert_eq!(board.color_them(), Color::Black);
        // assert_eq!(board.en_passant(), Bitboard::empty());
        // assert_eq!(board.move_count(), 0);
        assert_eq!(board.pawns() & board.us(), Bitboard::RANK_2);
        assert_eq!(board.rooks() & board.them(), A8 | H8);
        assert_eq!(board.bishops() & board.us(), C1 | F1);
        assert_eq!(board.them(), Bitboard::RANK_7 | Bitboard::RANK_8);
        Ok(())
    }

    #[test]
    fn parse_piece() -> Result<()> {
        let fen1 = "1/1/7/8/8/8/PPPPPPPP/RNBQKBNR";
        assert_eq!(
            BoardBuilder::parse_piece_placement(fen1).unwrap_err().to_string(),
            "expected 8 ranks of 8 pieces in fen 1/1/7/8/8/8/PPPPPPPP/RNBQKBNR"
        );
        assert!(BoardBuilder::parse_piece_placement("8")
            .unwrap_err()
            .to_string()
            .starts_with("expected 8"));
        assert!(BoardBuilder::parse_piece_placement("8/8")
            .unwrap_err()
            .to_string()
            .starts_with("expected 8"));
        assert_eq!(
            BoardBuilder::parse_piece_placement("X7/8/8/8/8/8/8/8")
                .unwrap_err()
                .to_string(),
            "Unknown piece 'X'"
        );
        let buf = BoardBuilder::parse_piece_placement("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR")?.build();
        assert_eq!(buf.get(Bitboard::A1), "R");
        assert_eq!(buf.get(Bitboard::FILE_H), "RP....pr");
        Ok(())
    }

    #[test]
    fn parse_fen() -> Result<()> {
        let b = Board::parse_fen("7k/8/8/8/8/8/8/7K b KQkq - 45 100")?;
        assert_eq!(b.color_us(), Color::Black);
        assert_eq!(b.fullmove_number(), 100);
        assert_eq!(b.halfmove_clock(), 45);
        assert_eq!(b.castling(), CastlingRights::all());
        Ok(())
    }

    #[test]
    fn test_parse_diagram() -> Result<()> {
        let s = r"
        K....... 
        PPP.....
        ........
        ........
        ........
        ........
        ppppp...
        rnbqk... w KQkq - 0 1";
        let board = Board::parse_diagram(s)?;
        assert_eq!(board.to_fen(), "K7/PPP5/8/8/8/8/ppppp3/rnbqk3 w KQkq - 0 1");
        Ok(())
    }

    #[test]
    fn parse_invalid_fen() -> Result<()> {
        assert_eq!(
            Board::parse_fen("7k/8/8/8/8/8/8/7K B Qkq - 45 100")
                .unwrap_err()
                .to_string(),
            "invalid color: 'B'".to_string()
        );
        assert_eq!(
            Board::parse_fen("7k/8/8/8/8/8/8/7K b XQkq - 45 100")
                .unwrap_err()
                .to_string(),
            "invalid character 'X' in castling rights 'XQkq'".to_string()
        );
        assert_eq!(
            Board::parse_fen("7k/8/8/8/8/8/8/7K b - - fifty 100")
                .unwrap_err()
                .to_string(),
            "invalid halfmove clock 'fifty'".to_string()
        );
        assert_eq!(
            Board::parse_fen("7k/8/8/8/8/8/8/7K b - - 50 full")
                .unwrap_err()
                .to_string(),
            "invalid fullmove count 'full'".to_string()
        );
        Ok(())
    }

    #[test]
    fn bench_board() {
        let mut starting_pos = Board::starting_pos();

        let mut clone = PerfProfiler::new("board.clone");
        let mut clone_from = PerfProfiler::new("board: clone_from");
        let mut mem_swap = PerfProfiler::new("board: mem_swap");
        let mut copy_from = PerfProfiler::new("makemove: copy_from");
        let mut apply_move = PerfProfiler::new("makemove: apply_move");
        let mut make_move = PerfProfiler::new("makemove: perft_make_move");
        let mut var_push_move = PerfProfiler::new("makemove: perft_var_push_move");
        let mut var_pop_move = PerfProfiler::new("makemove: perft_var_pop_move");
        let mut is_b_or_n = PerfProfiler::new("board: is_b_or_n");
        let mut is_pawn = PerfProfiler::new("board: is_pawn");
        let mut is_pawn_fast = PerfProfiler::new("board: is_pawn.fast");
        let mut piece_is = PerfProfiler::new("board: is_occupied_by");
        let mut piece_at = PerfProfiler::new("board: piece_at");
        let mut piece_unchecked = PerfProfiler::new("board: piece_unchecked");
        let mut mover_piece = PerfProfiler::new("move: mover_piece (board)");
        let mut perft = PerfProfiler::new("perft: perft");

        let mut dest = Board::starting_pos();
        // let mut boards = [dest.clone(), dest.clone()];
        let mut func = |bd: &Board, mv: Move| {
            let mut dest2 = bd.clone();
            mem_swap.bench(|| std::mem::swap(black_box(&mut dest), black_box(&mut dest2)));
            clone.bench(|| black_box(bd).clone());
            clone_from.bench(|| dest.clone_from(black_box(bd)));
            // copy_from.benchmark(|| Board::copy_from(black_box(&mut boards), 1, 0));
            copy_from.bench(|| black_box(&mut dest).copy_from(black_box(bd)));
            make_move.bench(|| black_box(bd).make_move(mv));
            let mut bd2 = bd.clone();
            apply_move.bench(|| black_box(&mut bd2).apply_move(mv));
            let mut var = Var::new(bd.clone());
            var_push_move.bench(|| {
                black_box(black_box(&mut var)).push_move(black_box(mv));
            });
            var_pop_move.bench(|| {
                black_box(black_box(&mut var)).pop_move();
            });
            is_pawn.bench(|| black_box(bd).piece(mv.from()) == Some(Piece::Pawn));
            is_pawn_fast.bench(|| mv.from().is_in(black_box(bd).pawns()));
            piece_unchecked.bench(|| black_box(bd).piece_unchecked(mv.from()));
            piece_at.bench(|| black_box(bd).piece(mv.from()));
            piece_is.bench(|| black_box(bd).is_occupied_by(black_box(mv).from(), Piece::Knight));
            mover_piece.bench(|| black_box(mv).mover_piece(black_box(bd)));
            is_b_or_n.bench(|| {
                black_box(bd).piece(Square::A3) == Some(Piece::Bishop)
                    || black_box(bd).piece(Square::A3) == Some(Piece::Knight)
            });
        };
        const BULK_COUNT: bool = true;
        Perft::<BULK_COUNT>::perft_with(&mut starting_pos, 2, &mut func);
        perft.bench(|| Perft::<BULK_COUNT>::count(black_box(&starting_pos), 5));
    }
}
