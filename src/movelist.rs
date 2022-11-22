use crate::board::Board;
use crate::eval::score::{Score, ToScore};
use crate::infra::utils::Displayable;
use crate::mv::{BareMove, Move};
use crate::parse::Parse;
use crate::piece::MAX_LEGAL_MOVES;
use crate::piece::{Color, Piece};
use crate::tags::Tags;
use crate::variation::Variation;
use anyhow::{anyhow, Result};
use arrayvec::ArrayVec;
use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;
use std::fmt;

// // moves: ArrayVec<Move,128>,
// // moves: ArrayVec::new(),
// #[derive(Debug, PartialEq, Eq)]
// pub struct MoveList {
//     moves: [Move; MAX_LEGAL_MOVES],
//     size: usize,
// }

// // pub struct MoveList(ArrayVec::<[Move; 384]>);
// // impl Default for MoveList {
// //     fn default() -> MoveList { MoveList::new() }
// // }

// impl Default for MoveList {
//     #[inline]
//     fn default() -> Self {
//         Self {
//             moves: unsafe { std::mem::MaybeUninit::uninit().assume_init() },
//             size: 0,
//         }
//     }
// }

// impl Clone for MoveList {

//     #[inline]
//     fn clone(&self) -> Self {
//         let mut cl = MoveList::default();
//         for &mv in self.iter() {
//             cl.push(mv);
//         }
//         cl
//     }
// }

// impl std::iter::FromIterator<Move> for MoveList {
//     #[inline]
//     fn from_iter<I: IntoIterator<Item = Move>>(iter: I) -> Self {
//         let mut ml = MoveList::new();
//         for mv in iter {
//             ml.push(mv);
//         }
//         ml
//     }
// }

// impl MoveList {
//     #[inline]
//     pub fn new() -> Self {
//         Self::default()
//     }

//     #[inline]
//     pub fn sort(&mut self) -> &mut Self {
//         self.moves[..self.size].sort_by_key(|m| m.to_string());
//         self
//     }

//     #[inline]
//     pub fn contains(&self, m: &Move) -> bool {
//         self.moves[..self.size].contains(m)
//     }

//     #[inline]
//     pub fn iter(&self) -> impl Iterator<Item = &Move> + '_ {
//         //    pub fn iter(&self) -> impl Iterator<Item = &Move> {
//         (self.moves[..self.size]).iter()
//     }

//     #[inline]
//     pub fn push(&mut self, mv: Move) {
//         debug_assert!(self.size < MAX_LEGAL_MOVES);
//         unsafe {
//             *self.moves.get_unchecked_mut(self.size) = mv;
//         }
//         self.size += 1;
//     }

//     #[inline]
//     pub fn clear(&mut self) {
//         // self.moves.clear();
//         self.size = 0;
//     }

//     #[inline]
//     pub fn swap(&mut self, i: usize, j: usize) {
//         self.moves[..self.size].swap(i, j);
//     }

//     #[inline]
//     pub fn retain<F>(&mut self, f: F)
//     where
//         F: FnMut(&Move) -> bool,
//     {
//         let mut v = Vec::<Move>::new();
//         v.extend(self.iter());
//         v.retain(f);
//         for i in 0..v.len() {
//             self.moves[i] = v[i];
//         }
//         self.size = v.len();
//     }

//     #[inline]
//     pub fn sort_unstable_by_key<K, F>(&mut self, f: F)
//     where
//         F: FnMut(&Move) -> K,
//         K: Ord,
//     {
//         self.moves[..self.size].sort_unstable_by_key(f)
//     }

//     #[inline]
//     pub fn reverse(&mut self) {
//         self.moves[..self.size].reverse();
//     }

//     #[inline]
//     pub fn extend<T: IntoIterator<Item = Move>>(&mut self, iter: T) {
//         for m in iter {
//             self.push(m);
//         }
//     }

//     #[inline]
//     pub fn len(&self) -> usize {
//         self.size
//     }

//     #[inline]
//     pub fn is_empty(&self) -> bool {
//         self.len() == 0
//     }

//     pub fn uci(&self) -> String {
//         self.iter().map(|mv| mv.uci()).collect::<Vec<String>>().join(" ")
//     }
// }

// impl std::ops::Index<usize> for MoveList {
//     type Output = Move;

//     #[inline]
//     fn index(&self, i: usize) -> &Self::Output {
//         debug_assert!(i < self.size);
//         &(self.moves[..self.size])[i]
//     }
// }

// impl fmt::Display for MoveList {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         if f.alternate() {
//             for mv in self.iter() {
//                 writeln!(f, "{:#}", mv)?;
//             }
//         } else {
//             let strings: Vec<String> = self.iter().map(Move::to_string).collect();
//             f.write_str(&strings.join(", "))?
//         }
//         Ok(())
//     }
// }

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct ScoredMoveList {
    moves: Vec<(BareMove, Score)>,
}

impl ScoredMoveList {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn push(&mut self, smv: (BareMove, Score)) {
        self.moves.push(smv);
    }

    fn fmt_san(&self, f: &mut fmt::Formatter, b: &Board) -> fmt::Result {
        write!(
            f,
            "{}",
            self.moves
                .iter()
                .map(|(mv, s)| { format!("{mv}:{score}", mv = mv.to_san(b), score = s.to_pgn()) })
                .join(" ")
        )?;
        Ok(())
    }

    pub fn iter(&self) -> impl Iterator<Item = (BareMove, Score)> + '_ {
        self.moves.iter().cloned()
    }

    pub fn display_san<'a>(&'a self, b: &'a Board) -> impl fmt::Display + 'a {
        Displayable(|f| self.fmt_san(f, b))
    }

    pub fn to_san(&self, b: &Board) -> String {
        format!("{}", self.display_san(b))
    }

    pub fn parse_san(s: &str, b: &Board) -> anyhow::Result<Self> {
        let mut moves = Self::new();
        let s = s.replace(',', " ");
        for smv in s.split_ascii_whitespace() {
            if let Some((before, after)) = smv.split_once(":") {
                let mv = b.parse_san_move(before)?.to_inner();
                let score = Score::parse_pgn(after)?;
                moves.push((mv, score));
            } else {
                anyhow::bail!("Unable to parse scored move '{smv}' in '{s}'");
            }
        }
        Ok(moves)
    }

    pub fn best_score(&self) -> Option<Score> {
        self.iter().nth(0).map(|(_mv, s)| s)
    }

    pub fn centipawn_loss(&self, actual: BareMove) -> Option<Score> {
        let best = self.best_score().unwrap_or_default();
        let worst = self.iter().last().map(|(_mv, s)| s).unwrap_or_default();
        let matching_score = self.iter().find(|&(mv, _s)| mv == actual).map(|(_mv, s)| s);
        match matching_score {
            Some(ms) if ms.is_numeric() && best.is_numeric() => Some(ms - best),
            // no match so return one less than worst-best
            None if best.is_numeric() && worst.is_numeric() => Some(worst - best - 1.cp()),
            // one is a mate score, so no numerics to return
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests_smv {
    use super::*;
    use crate::catalog::Catalog;

    #[test]
    fn test_scoredmovelist() {
        let b = Catalog::starting_board();
        let moves = ScoredMoveList::parse_san("a3:+0.34 h3:+M5 e2e4:0.90", &b).unwrap();
        assert_eq!(moves.iter().nth(0).unwrap().1.as_i16(), 34);
        assert_eq!(moves.to_san(&b), "a3:+0.34 h3:+M5 e4:+0.90");
        let moves = ScoredMoveList::parse_san("a3:+0.34 h3:-0.45 e2e4:0.90", &b).unwrap();
        assert_eq!(moves.to_san(&b), "a3:+0.34 h3:-0.45 e4:+0.90");
    }
}

// moves: ArrayVec<Move,128>,
// moves: ArrayVec::new(),
#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub struct MoveList {
    moves: ArrayVec<Move, MAX_LEGAL_MOVES>,
}

// impl Default for MoveList {
//     #[inline]
//     fn default() -> Self {
//         Self {
//             moves: ArrayVec::new(),
//         }
//     }
// }

// impl Clone for MoveList {
//     #[inline]
//     fn clone(&self) -> Self {
//         MoveList {
//             moves: self.moves.clone(),
//         }
//     }
// }
// impl Clone for MoveList {
//     #[inline]
//     fn clone(&self) -> Self {
//         let mut other = MoveList {
//             moves: ArrayVec::new(),
//         };
//         unsafe {
//             other.moves.set_len(self.len());
//         }
//         other.moves.copy_from_slice(&self.moves);
//         // for &mv in self.iter() {
//         //     other.push(mv);
//         // }
//         other
//     }
// }

impl std::iter::FromIterator<Move> for MoveList {
    #[inline]
    fn from_iter<I: IntoIterator<Item = Move>>(iter: I) -> Self {
        let mut ml = MoveList::new();
        for mv in iter {
            ml.push(mv);
        }
        ml
    }
}

impl MoveList {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn sort(&mut self) -> &mut Self {
        self.moves.sort_by_key(|m| m.to_string());
        self
    }

    #[inline]
    pub fn contains(&self, m: &Move) -> bool {
        self.moves.contains(m)
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Move> + '_ {
        self.moves.iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Move> + '_ {
        self.moves.iter_mut()
    }

    #[inline]
    pub fn push(&mut self, mv: Move) {
        debug_assert!(self.len() < MAX_LEGAL_MOVES);
        #[cfg(feature = "unchecked_indexing")]
        unsafe {
            self.moves.push_unchecked(mv);
        }
        #[cfg(not(feature = "unchecked_indexing"))]
        {
            self.moves.push(mv);
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.moves.clear();
    }

    #[inline]
    pub fn swap(&mut self, i: usize, j: usize) {
        self.moves.swap(i, j);
    }

    #[inline]
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&mut Move) -> bool,
    {
        self.moves.retain(f);
    }

    #[inline]
    pub fn sort_unstable_by_key<K, F>(&mut self, f: F)
    where
        F: FnMut(&Move) -> K,
        K: Ord,
    {
        self.moves.sort_unstable_by_key(f)
    }

    #[inline]
    pub fn sort_by_cached_key<K, F>(&mut self, f: F)
    where
        F: FnMut(&Move) -> K,
        K: Ord,
    {
        self.moves.sort_by_cached_key(f)
    }

    #[inline]
    pub fn reverse(&mut self) {
        self.moves.reverse();
    }

    #[inline]
    pub fn extend<T: IntoIterator<Item = Move>>(&mut self, iter: T) {
        self.moves.extend(iter);
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.moves.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }

    pub fn uci(&self) -> String {
        self.iter()
            .map(|mv| mv.to_uci())
            .collect::<Vec<String>>()
            .join(" ")
    }
}

impl std::ops::Index<usize> for MoveList {
    type Output = Move;

    #[inline]
    fn index(&self, i: usize) -> &Self::Output {
        &self.moves[i]
    }
}

impl fmt::Display for MoveList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            for mv in self.iter() {
                writeln!(f, "{:#}", mv)?;
            }
        } else {
            let strings: Vec<String> = self.iter().map(Move::to_string).collect();
            f.write_str(&strings.join(", "))?
        }
        Ok(())
    }
}

// pub trait MoveValidator {
//     fn parse_uci_move(&self, mv: &str) -> Result<Move, String>;
//     fn parse_uci_choices(&self, moves: &str) -> Result<MoveList, String>;
//     fn parse_uci_moves(&self, moves: &str) -> Result<Variation, String>;

//     fn parse_san_move(&self, mv: &str) -> Result<Move, String>;
//     fn parse_san_choices(&self, moves: &str) -> Result<MoveList, String>;
//     fn parse_san_moves(&self, moves: &str) -> Result<Variation, String>;

//     fn to_san(&self, mv: &Move) -> String;
//     fn to_san_moves(&self, moves: &Variation, vec_tags: Option<&Vec<Tags>>) -> String;
// }

impl Board {
    pub fn parse_uci_move(&self, mv: &str) -> Result<Move> {
        let moves = self.legal_moves();
        for &m in moves.iter() {
            if m.to_uci() == mv {
                return Ok(m);
            }
        }
        Err(anyhow!(
            "Move {} is not legal for board {}",
            mv,
            self.to_fen()
        ))
    }

    pub fn parse_uci_movelist(&self, s: &str) -> Result<MoveList> {
        let mut moves = MoveList::new();
        let s = s.replace(',', " ");
        let s = strip_move_numbers(&s);
        for mv in s.split_ascii_whitespace() {
            moves.push(self.parse_uci_move(mv)?);
        }
        Ok(moves)
    }

    pub fn parse_uci_variation(&self, s: &str) -> Result<Variation> {
        let mut board = self.clone();
        let mut moves = Variation::new();
        let s = s.replace(',', " ");
        let s = strip_move_numbers(&s);
        for mv in s.split_ascii_whitespace() {
            let mv = board.parse_uci_move(mv)?;
            moves.push(mv);
            board = board.make_move(&mv);
        }
        Ok(moves)
    }

    pub fn parse_san_move(&self, mv: &str) -> Result<Move> {
        Parse::move_san(mv, self)
    }

    pub fn parse_san_movelist(&self, s: &str) -> Result<MoveList> {
        let mut moves = MoveList::new();
        let s = s.replace(',', " ");
        let s = strip_move_numbers(&s);
        for mv in s.split_ascii_whitespace() {
            moves.push(self.parse_san_move(mv)?);
        }
        Ok(moves)
    }

    pub fn parse_san_variation(&self, s: &str) -> Result<Variation> {
        let mut board = self.clone();
        let mut moves = Variation::new();
        let s = s.replace(',', " ");
        let s = strip_move_numbers(&s);
        for mv in s.split_ascii_whitespace() {
            let mv = board.parse_san_move(mv)?;
            moves.push(mv);
            board = board.make_move(&mv);
        }
        Ok(moves)
    }

    pub fn to_san(&self, mv: &Move) -> String {
        if mv.is_null() {
            return "--".to_string();
        }

        if mv.is_castle() {
            if mv.castling_side().is_king_side() {
                return String::from("O-O");
            } else {
                return String::from("O-O-O");
            }
        }

        let mut s = String::new();
        if mv.mover_piece() != Piece::Pawn {
            s += &mv.mover_piece().to_upper_char().to_string();
        }
        // ambiguity resolution
        let mut pieces = 0;
        let mut file_pieces = 0;
        let mut rank_pieces = 0;
        for lm in self.legal_moves().iter() {
            if lm.to() == mv.to() && lm.mover_piece() == mv.mover_piece() {
                pieces += 1;
                if lm.from().file_char() == mv.from().file_char() {
                    file_pieces += 1;
                }
                if lm.from().rank_char() == mv.from().rank_char() {
                    rank_pieces += 1;
                }
            }
        }
        if pieces > 1 || (mv.mover_piece() == Piece::Pawn && mv.is_capture()) {
            // need to resolve ambiguity
            if file_pieces == 1 {
                s.push(mv.from().file_char());
            } else if rank_pieces == 1 {
                s.push(mv.from().rank_char());
            } else {
                s += mv.from().uci();
            }
        }

        if mv.is_capture() {
            s.push('x');
        }
        s += mv.to().uci();
        // if mv.is_ep_capture() {
        //     s += " e.p.";
        // }
        if let Some(promo) = mv.promo() {
            s.push('=');
            s.push(promo.to_upper_char());
        }
        if self.gives_check(mv) {
            s.push('+');
        }
        s
    }

    pub fn to_san_movelist(&self, moves: &MoveList) -> String {
        let mut v = Vec::new();
        for mv in moves.iter() {
            debug_assert!(
                self.is_legal_move(mv),
                "mv {} is illegal for board {}",
                mv,
                self.to_fen()
            );
            v.push(self.to_san(mv));
        }
        v.join(" ")
    }

    pub fn to_san_variation(&self, moves: &Variation, _vec_tags: Option<&Vec<Tags>>) -> String {
        let mut s = String::new();
        let mut board = self.clone();
        for (i, mv) in moves.iter().enumerate() {
            debug_assert!(
                board.is_legal_move(mv),
                "mv {} is illegal for board {}",
                mv,
                board.to_fen()
            );
            if i % 2 == 0 {
                if i != 0 {
                    s += "\n";
                }
                s += &board.fullmove_number().to_string();
                s += ".";
            }
            if i == 0 && board.color_us() == Color::Black {
                s += "..";
            }
            s += " ";
            s += &board.to_san(mv);
            // if let Some(vec) = vec_tags {
            //     let tags = &vec[i];
            //     s += &tags.to_pgn();
            // }

            board = board.make_move(mv);
        }
        s
    }
}

static REGEX_MOVE_NUMBERS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?x)         # x flag to allow whitespace and comments
    ((\d)+\.(\s)*(\.\.)\s+|(\d)+\.\s+|(\d)+\.$)?      # digits a '.' and then whitespace and optionally ".."
    "#,
    )
    .unwrap()
});

pub fn strip_move_numbers(s: &str) -> String {
    REGEX_MOVE_NUMBERS.replace_all(s, "").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::globals::constants::*;

    #[test]
    fn test_movelist() -> Result<()> {
        let move_a1b2 = Move::new_quiet(Piece::Bishop, a1.square(), b2.square());
        let promo_a7a8 = Move::new_promo(a7.square(), a8.square(), Piece::Queen);

        let mut moves = MoveList::new();
        assert_eq!(moves.iter().count(), 0);
        moves.push(move_a1b2);
        assert_eq!(moves.contains(&promo_a7a8), false);
        moves.reverse();
        assert_eq!(moves.iter().count(), 1);

        moves.push(promo_a7a8);
        assert_eq!(moves.contains(&move_a1b2), true);

        assert_eq!(moves.to_string(), "a1b2, a7a8q");

        let mut moves = Variation::new();
        moves.set_last_move(1, &move_a1b2);
        assert_eq!(moves.to_string(), "a1b2");
        moves.set_last_move(1, &promo_a7a8);
        assert_eq!(moves.to_string(), "a7a8q");

        moves.set_last_move(0, &promo_a7a8);
        assert_eq!(moves.to_string(), "");

        moves.set_last_move(1, &move_a1b2);
        moves.set_last_move(2, &promo_a7a8);
        assert_eq!(moves.to_string(), "a1b2, a7a8q");

        moves.set_last_move(0, &promo_a7a8);
        moves.set_last_move(2, &move_a1b2);
        assert_eq!(moves.to_string(), "a1b2, a1b2");

        let s = strip_move_numbers("1. .. c4c5 2. c6c7 3.");
        assert_eq!(s, "c4c5 c6c7 ");

        let s = strip_move_numbers("1... c4c5 2. c6c7 3.");
        assert_eq!(s, "c4c5 c6c7 ");

        let s = strip_move_numbers("1. c1c2 c4c5 2. c6c7 3.");
        assert_eq!(s, "c1c2 c4c5 c6c7 ");

        let board = Catalog::starting_board();

        let list = board.parse_uci_movelist("a2a3, b2b3  c2c4  ")?;
        assert_eq!(list.to_string(), "a2a3, b2b3, c2c4");

        let list = board.parse_uci_movelist("1. a2a3, 2. b2b3  c2c4  ")?;
        assert_eq!(list.to_string(), "a2a3, b2b3, c2c4");

        let list = board.parse_uci_variation("1. a2a3 h7h6 2. b2b3 h6h5")?;
        assert_eq!(list.to_string(), "a2a3, h7h6, b2b3, h6h5");

        let mv = board.parse_uci_move("a2a3")?;
        let board2 = board.make_move(&mv);
        let list = board2.parse_uci_variation("1. .. h7h6 2. b2b3 h6h5")?;

        assert_eq!(list.to_string(), "h7h6, b2b3, h6h5");

        let list = board.parse_san_movelist("Nc3, c3  Pc2c3")?;
        assert_eq!(list.to_string(), "b1c3, c2c3, c2c3");

        let san = r"
            1. d4 c6 2. Bf4 d6 3. Nd2 h6 
            4. Ngf3 g5 5. Bg3 Qb6 6. Nc4 Qb4+ 

            7. Nfd2 Be6 8. c3 Qb5 9. e3 Bxc4 
            10. Nxc4 Qd5 11. Qf3 Qxf3 12. gxf3 Nd7 

            13. h4 Bg7 14. e4 Ngf6 15. Bd3 Nh5 
            16. hxg5 Nxg3 17. fxg3 hxg5 18. Rxh8+ Bxh8 

            19. Kd2 O-O-O 20. Ne3 e6 21. Rh1 b5";

        let mut s = String::new();
        s += "d2d4, c7c6, c1f4, d7d6, b1d2, h7h6, ";
        s += "g1f3, g7g5, f4g3, d8b6, d2c4, b6b4, ";

        s += "f3d2, c8e6, c2c3, b4b5, e2e3, e6c4, ";
        s += "d2c4, b5d5, d1f3, d5f3, g2f3, b8d7, ";

        s += "h2h4, f8g7, e3e4, g8f6, f1d3, f6h5, ";
        s += "h4g5, h5g3, f2g3, h6g5, h1h8, g7h8, ";

        s += "e1d2, e8c8, c4e3, e7e6, a1h1, b7b5";
        assert_eq!(board.parse_san_variation(san)?.to_string(), s);
        let s1: String = board
            .to_san_variation(&board.parse_san_variation(san)?, None)
            .split_whitespace()
            .collect();
        let s2: String = san.split_whitespace().collect();
        assert_eq!(s1, s2);

        let board =
            Board::parse_fen("rnbqkbnr/pp2ppp1/2pp3p/8/3P1B2/8/PPPNPPPP/R2QKBNR w KQkq - 0 4")
                .unwrap();
        println!("{}", board.legal_moves());
        let mv = board.parse_uci_move("g1f3")?;
        assert_eq!(board.to_san(&mv), "Ngf3");
        Ok(())
    }
}
