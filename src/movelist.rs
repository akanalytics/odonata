use crate::bitboard::Bitboard;
use crate::board::makemove::MoveMaker;
use crate::board::movegen::MoveGen;
use crate::board::Board;
use crate::parse::Parse;
use crate::types::{CastlingRights, Color, Piece};
use crate::utils::StringUtils;
use regex::Regex;
use std::fmt;
use std::ops::{Deref, DerefMut};

// FIXME: public methods
#[derive(Debug, Default, Copy, Clone)]
pub struct Move {
    pub from: Bitboard,
    pub to: Bitboard,
    pub ep: Bitboard,
    pub promo: Piece,
    pub capture: Piece,
    pub mover: Piece,

    pub castle_side: CastlingRights,
    pub is_null: bool,
    pub is_drop: bool, // used for board setup not variant play
}

impl Move {
    #[inline]
    pub fn new_null() -> Move {
        Move { is_null: true, ..Default::default() }
    }

    #[inline]
    pub fn to(&self) -> Bitboard {
        self.to
    }

    #[inline]
    pub fn from(&self) -> Bitboard {
        self.from
    }

    #[inline]
    pub fn ep(&self) -> Bitboard {
        self.ep
    }

    #[inline]
    pub fn is_null(&self) -> bool {
        self.is_null
    }

    #[inline]
    pub fn is_promo(&self) -> bool {
        self.promo != Piece::None
    }

    #[inline]
    pub fn promo_piece(&self) -> Piece {
        self.promo
    }

    #[inline]
    pub fn capture_piece(&self) -> Piece {
        self.capture
    }

    #[inline]
    pub fn mover_piece(&self) -> Piece {
        self.mover
    }

    #[inline]
    pub fn is_capture(&self) -> bool {
        self.capture != Piece::None
    }

    #[inline]
    pub fn is_castle(&self) -> bool {
        !self.castle_side.is_empty()
    }

    #[inline]
    pub fn castling_side(&self) -> CastlingRights {
        self.castle_side
    }

    #[inline]
    pub fn is_drop(&self) -> bool {
        self.is_drop
    }

    #[inline]
    pub fn is_ep_capture(&self) -> bool {
        !self.ep.is_empty() && self.is_capture()
    }

    #[inline]
    pub fn is_pawn_double_push(&self) -> bool {
        !self.ep.is_empty() && !self.is_capture()
    }

    pub fn uci(&self) -> String {
        if self.is_null() {
            return String::from('-');
        }
        let mut res = String::new();
        res.push_str(&self.from.uci());
        res.push_str(&self.to.uci());
        if self.is_promo() {
            res.push(self.promo.to_char(Some(Color::Black)));
        }
        res
    }

    pub fn parse_uci(s: &str) -> Result<Move, String> {
        let from = Bitboard::parse_square(s.take_slice(0..2))?;
        let to = Bitboard::parse_square(s.take_slice(2..4))?;
        let promo;
        if let Some(ch) = s.take_char_at(4) {
            promo = Piece::from_char(ch)?;
        } else {
            promo = Piece::None;
        }
        Ok(Move { to, from, promo, ..Default::default() })
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.uci())
    }
}

#[derive(Debug, Clone, Default)]
pub struct MoveList(Vec<Move>);

// pub struct MoveList(ArrayVec::<[Move; 384]>);
// impl Default for MoveList {
//     fn default() -> MoveList { MoveList::new() }
// }

impl MoveList {
    pub fn new() -> Self {
        Self(Vec::with_capacity(250)) // TODO: capacity??
    }

    pub fn sort(&mut self) -> &mut Self {
        self.0.sort_by_key(|m| m.to_string());
        self
    }
}

impl Deref for MoveList {
    type Target = Vec<Move>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MoveList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl fmt::Display for MoveList {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let strings: Vec<String> = self.0.iter().map(Move::to_string).collect();
        fmt.write_str(&strings.join(", "))
    }
}

pub trait MoveValidator {
    fn parse_uci_move(&self, mv: &str) -> Result<Move, String>;
    fn parse_uci_choices(&self, moves: &str) -> Result<MoveList, String>;
    fn parse_uci_moves(&self, moves: &str) -> Result<MoveList, String>;

    fn parse_san_move(&self, mv: &str) -> Result<Move, String>;
    fn parse_san_choices(&self, moves: &str) -> Result<MoveList, String>;
    fn parse_san_moves(&self, moves: &str) -> Result<MoveList, String>;

    fn to_san(&self, mv: &Move) -> String;
    fn to_san_moves(&self, moves: &MoveList) -> String;
}

impl MoveValidator for Board {
    fn parse_uci_move(&self, mv: &str) -> Result<Move, String> {
        let moves = self.legal_moves();
        if let Some(pos) = moves.iter().position(|m| m.uci() == mv) {
            return Ok(moves[pos]);
        }
        Err(format!("Move {} is not legal for board {}", mv, self.to_fen()))
    }

    fn parse_uci_choices(&self, s: &str) -> Result<MoveList, String> {
        let mut moves = MoveList::new();
        let s = s.replace(",", " ");
        let s = strip_move_numbers(&s);
        for mv in s.split_ascii_whitespace() {
            moves.push(self.parse_uci_move(mv)?);
        }
        Ok(moves)
    }

    fn parse_uci_moves(&self, s: &str) -> Result<MoveList, String> {
        let mut board = self.clone();
        let mut moves = MoveList::new();
        let s = s.replace(",", " ");
        let s = strip_move_numbers(&s);
        for mv in s.split_ascii_whitespace() {
            let mv = board.parse_uci_move(mv)?;
            moves.push(mv);
            board = board.make_move(&mv);
        }
        Ok(moves)
    }

    fn parse_san_move(&self, mv: &str) -> Result<Move, String> {
        Parse::move_san(mv, self)
    }

    fn parse_san_choices(&self, s: &str) -> Result<MoveList, String> {
        let mut moves = MoveList::new();
        let s = s.replace(",", " ");
        let s = strip_move_numbers(&s);
        for mv in s.split_ascii_whitespace() {
            moves.push(self.parse_san_move(mv)?);
        }
        Ok(moves)
    }

    fn parse_san_moves(&self, s: &str) -> Result<MoveList, String> {
        let mut board = self.clone();
        let mut moves = MoveList::new();
        let s = s.replace(",", " ");
        let s = strip_move_numbers(&s);
        for mv in s.split_ascii_whitespace() {
            let mv = board.parse_san_move(mv)?;
            moves.push(mv);
            board = board.make_move(&mv);
        }
        Ok(moves)
    }

    fn to_san(&self, mv: &Move) -> String {
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
                if lm.from().files() == mv.from().files() {
                    file_pieces += 1;
                }
                if lm.from().ranks() == mv.from().ranks() {
                    rank_pieces += 1;
                }
            }
        }
        if pieces > 1 || (mv.mover_piece() == Piece::Pawn && mv.is_capture()) {
            // need to resolve ambiguity
            if file_pieces == 1 {
                s += &mv.from().files();
            } else if rank_pieces == 1 {
                s += &mv.from().ranks();
            } else {
                s += &mv.from().sq_as_uci();
            }
        }

        if mv.is_capture() {
            s.push('x');
        }
        s += &mv.to().sq_as_uci();
        if mv.is_ep_capture() {
            s += " e.p.";
        }
        if mv.is_promo() {
            s.push('=');
            s.push(mv.promo_piece().to_upper_char());
        }
        if self.will_check(mv) {
            s.push('+');
        }
        s
    }

    fn to_san_moves(&self, moves: &MoveList) -> String {
        let mut s = String::new();
        let mut board = self.clone();
        for (i, mv) in moves.iter().enumerate() {
            debug_assert!(board.is_legal_move(mv));
            if i % 2 == 0 {
                if i != 0 {
                    s += "\n";
                }
                s += &board.fullmove_counter().to_string();
                s += ".";
            }
            if i == 0 && board.color_us() == Color::Black {
                s += "..";
            }
            s += " ";
            s += &board.to_san(mv);

            board = board.make_move(mv);
        }
        s
    }
}

fn strip_move_numbers(s: &str) -> String {
    let re = Regex::new(
        r#"(?x)         # x flag to allow whitespace and comments
        (\d)+\.\s?      # digits a '.' and then whitespace
        "#,
    )
    .unwrap();

    re.replace_all(&s, "").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::*;
    use crate::board::movegen::*;
    use crate::catalog::Catalog;
    use crate::globals::constants::*;

    #[test]
    fn test_move() {
        assert_eq!(Move::new_null().to_string(), "-");

        let move_a1b2 = Move { from: a1, to: b2, ..Default::default() };
        let promo_a7a8 = Move { from: a7, to: a8, promo: Piece::Queen, ..Default::default() };
        assert_eq!(move_a1b2.to_string(), "a1b2");
        assert_eq!(promo_a7a8.to_string(), "a7a8q");

        let move_e2e4 = Move::parse_uci("e2e4").unwrap();
        assert_eq!(move_e2e4.to_string(), "e2e4");

        let move_e7e8 = Move::parse_uci("e7e8p").unwrap();
        assert_eq!(move_e7e8.to_string(), "e7e8p");

        let board = Catalog::starting_position();
        assert_eq!(board.parse_san_move("Nc3").unwrap().to_string(), "b1c3");
        assert_eq!(board.parse_san_move("c3").unwrap().to_string(), "c2c3");
        assert_eq!(board.parse_san_move("c2c4").unwrap().to_string(), "c2c4");
        assert_eq!(board.parse_san_move("c2-c4").unwrap().to_string(), "c2c4");
        assert_eq!(board.parse_san_move("Pc4").unwrap().to_string(), "c2c4");
        assert_eq!(board.parse_san_move("Pc2c4").unwrap().to_string(), "c2c4");
    }

    #[test]
    fn test_movelist() -> Result<(), String> {
        let move_a1b2 = Move { from: a1, to: b2, ..Default::default() };
        let promo_a7a8 = Move { from: a7, to: a8, promo: Piece::Queen, ..Default::default() };

        let mut moves = MoveList::new();
        moves.push(move_a1b2);
        moves.push(promo_a7a8);
        assert_eq!(moves.to_string(), "a1b2, a7a8q");

        let s = strip_move_numbers("1. c1c2 c4c5 2. c6c7 3.");
        assert_eq!(s, "c1c2 c4c5 c6c7 ");

        let board = Catalog::starting_position();

        let list = board.parse_uci_choices("a2a3, b2b3  c2c4  ")?;
        assert_eq!(list.to_string(), "a2a3, b2b3, c2c4");

        let list = board.parse_uci_choices("1. a2a3, 2. b2b3  c2c4  ")?;
        assert_eq!(list.to_string(), "a2a3, b2b3, c2c4");

        let list = board.parse_uci_moves("1. a2a3 h7h6 2. b2b3 h6h5")?;
        assert_eq!(list.to_string(), "a2a3, h7h6, b2b3, h6h5");

        let list = board.parse_san_choices("Nc3, c3  Pc2c3")?;
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
        assert_eq!(board.parse_san_moves(san)?.to_string(), s);
        let s1: String = board.to_san_moves(&board.parse_san_moves(san)?).split_whitespace().collect();
        let s2: String = san.split_whitespace().collect();
        assert_eq!(s1, s2);

        let board =
            Board::parse_fen("rnbqkbnr/pp2ppp1/2pp3p/8/3P1B2/8/PPPNPPPP/R2QKBNR w KQkq - 0 4").unwrap();
        println!("{}", board.legal_moves());
        let mv = board.parse_uci_move("g1f3")?;
        assert_eq!(board.to_san(&mv), "Ngf3");
        Ok(())
    }

    #[test]
    fn test_to_san() {
        let mut board = Catalog::starting_position();
        let a2a3 = board.parse_uci_move("a2a3").unwrap();
        let b1c3 = board.parse_uci_move("b1c3").unwrap();
        assert_eq!(board.to_san(&a2a3), "a3");
        assert_eq!(board.to_san(&b1c3), "Nc3");

        let board = board.set(d3, "p").unwrap();
        let board = board.set(f3, "p").unwrap();

        let c2d3 = board.parse_uci_move("c2d3").unwrap();
        assert_eq!(board.to_san(&c2d3), "cxd3");

        let e2d3 = board.parse_uci_move("e2d3").unwrap();
        assert_eq!(board.to_san(&e2d3), "exd3");

        let g1f3 = board.parse_uci_move("g1f3").unwrap();
        assert_eq!(board.to_san(&g1f3), "Nxf3");

        // knight ambiguity
        let board = board.set(g5, "N").unwrap();
        let g1f3 = board.parse_uci_move("g1f3").unwrap();
        assert_eq!(board.to_san(&g1f3), "N1xf3");

        // two knights same rank and file as g5
        let board = board.set(e5, "N").unwrap();
        let g1f3 = board.parse_uci_move("g5f3").unwrap();
        assert_eq!(board.to_san(&g1f3), "Ng5xf3");

        // remove some minor pieces to allow castling
        let board = board.set(Bitboard::RANK_8, "r...k..r").unwrap();
        board.set_turn(Color::Black);
        let castle_k = board.parse_uci_move("e8g8").unwrap();
        assert_eq!(board.to_san(&castle_k), "O-O");
        let castle_q = board.parse_uci_move("e8c8").unwrap();
        assert_eq!(board.to_san(&castle_q), "O-O-O");
    }
}