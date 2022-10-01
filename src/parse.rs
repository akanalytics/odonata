use crate::bits::bitboard::Bitboard;
use crate::board::Board;
use crate::movelist::MoveList;
use crate::mv::Move;
use crate::piece::Color;
use crate::infra::utils::StringUtils;
use once_cell::sync::Lazy;
use regex::Regex;
pub struct Parse;
use anyhow::{anyhow, bail, Result};

// regex from https://stackoverflow.com/questions/40007937/regex-help-for-chess-moves-san
// /^([NBRQK])?([a-h])?([1-8])?(x)?([a-h][1-8])(=[NBRQK])?(\+|#)?$|^O-O(-O)?$/
// which claims... 'This was unit tested against 2599 cases'
//
// change
//   convert python  : $ to \Z
//   allow "-"       : (\-|x)
//   allow lc promos : [nbrqkNBRQK]
//
// r"^([NBRQK])?([a-h])?([1-8])?(\-|x)?([a-h][1-8])(=[NBRQ])?(\+|#)?\Z|^O-O(-O)?\Z"
//
static REGEX_SAN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?x)    # x flag to allow whitespace and comments
    ^
    ([PNBRQK])?     # piece - grp(1)  Fix:18/3/21 allow P
    ([a-h])?        # src square file grp(2)
    ([1-8])?        # src square rank grp(3)
    (\-|x)?         # move or capture grp(4)
    ([a-h][1-8])?   # square - both rank and file grp(5)
    (=[NBRQ])?      # promo grp(6) 
    (\+|\#)?        # check or checkmate grp(7)
    \z
    |               # OR
    ^O-O(-O)?\z     #   or castling king (or queens) side and eol
    |
    ^([a-h][1-8][a-h][1-8][nbrq])\z  # uci promo grp(9)
    "#,
    )
    .unwrap()
});

impl Parse {
    pub fn move_san(s: &str, board: &Board) -> Result<Move> {
        let orig = s; // save original string
                      //  convert 0's to O's
                      //  Wikipedia:
                      //    Castling is indicated by the special notations 0-0 (for kingside castling) and 0-0-0 (queenside castling).
                      //    While the FIDE standard [5] is to use the digit zero (0-0 and 0-0-0), PGN uses the uppercase letter O (O-O and O-O-O).[6]
        let mut s = orig.replace('0', "O");

        // Checkmate at the completion of moves is represented by the symbol "#" in standard FIDE notation and PGN.
        // The word mate is commonly used instead; occasionally a double dagger (‡)
        s = s.replace("mate", "#");
        s = s.replace('‡', "#");
        s = s.replace('?', "");
        s = s.replace('!', "");

        // strip whitespace
        s = s.replace(" ", "");

        let caps = REGEX_SAN
            .captures(&s)
            .ok_or_else(|| anyhow!("Unable to parse '{}' as an algebraic move", s))?;
        // if not match:
        //     raise ValueError(f"Move {orig} is invalid - wrong format")

        // parse checkmates
        let _checkmate = s.contains('#');

        let legal_moves = board.legal_moves();
        // caps.get(0).unwrap().as_str();
        let mut piece = caps.get(1).map_or("", |m| m.as_str());
        let mut src_file = caps.get(2).map_or("", |m| m.as_str());
        let mut src_rank = caps.get(3).map_or("", |m| m.as_str());
        // move or capture = grp 4
        let mut dst_square = caps.get(5).map_or("", |m| m.as_str()).to_string();
        let promo = caps.get(6).map_or("", |m| m.as_str());
        let _checks = caps.get(7).map_or("", |m| m.as_str());
        let _q_side_castle = caps.get(8).map_or("", |m| m.as_str());
        let _uci_promo = caps.get(9).map_or("", |m| m.as_str());
        // println!("Parsed p={} f={} r={} to={}", piece, src_file, src_rank, dst_square);

        // if one square is given, its the destination not the source
        if dst_square.is_empty() && !src_rank.is_empty() && !src_rank.is_empty() {
            dst_square = src_file.to_owned() + src_rank;
            src_rank = "";
            src_file = "";
        }

        // pawn prefixs are omiited
        if piece.is_empty() && !dst_square.is_empty() {
            piece = "P";
        }

        // strategy: search through legal moves looking for a move that fits the criteria
        // this is slow but easy to understand. Castling has already been dealt with
        let mut matching_moves = MoveList::new();
        for lm in legal_moves.iter() {
            // allow UCI moves as well as SAN
            if lm.uci() == s {
                matching_moves.clear();
                matching_moves.push(*lm);
                break;
            }

            if !dst_square.is_empty() && lm.to() != Bitboard::parse_square(&dst_square)? {
                continue;
            }
            if !src_file.is_empty() && lm.from().uci().take_substring(0, 1) != src_file {
                continue;
            }
            if !src_rank.is_empty() && lm.from().uci().take_substring(1, 1) != src_rank {
                continue;
            }
            if !piece.is_empty() {
                match board.piece(lm.from()) {
                    None => continue,
                    Some(p) if p.to_upper_char().to_string() != piece => continue,
                    _ => {}
                }
            }
            // SAN promos are upper case eg "=Q" "=B"
            if let Some(pp) = lm.promo() {
                if !promo.is_empty()
                    && "=".to_string() + &pp.to_char(Color::White).to_string() != promo
                {
                    continue;
                }
            }
            // lm is castle but s isnt
            if lm.is_castle() && lm.castling_side().is_king_side() && s != "O-O"
                || lm.is_castle() && lm.castling_side().is_queen_side() && s != "O-O-O"
            {
                continue;
            }
            // s is castle but lm isnt
            if (s == "O-O" || s == "O-O-O") && !lm.is_castle() {
                continue;
            }
            matching_moves.push(*lm);
        }
        if matching_moves.is_empty() {
            bail!(
                "Move {} is invalid - not a legal move for board {}",
                orig,
                board.to_fen()
            );
        }
        if matching_moves.len() > 1 {
            bail!(
                "Move {} is ambiguous - moves {} match. For board {}",
                orig,
                matching_moves,
                board.to_fen()
            );
        }

        // FIXME: warnings on non-captures, non-checkmates etc
        Ok(matching_moves[0])
        // matching_moves.iter().next().cloned().ok_or_else(|| "matching moves empty!".to_string())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use crate::catalog::Catalog;

    #[test]
    fn test_parse_move() {
        let bd = Catalog::starting_board();
        let bd = do_test_and_make_move(&bd, "d4", "d2d4");
        let bd = do_test_and_make_move(&bd, "c6", "c7c6");
        let bd = do_test_and_make_move(&bd, "Bf4", "c1f4");
        let bd = do_test_and_make_move(&bd, "d6", "d7d6");
        let bd = do_test_and_make_move(&bd, "Nd2", "b1d2");
        let bd = do_test_and_make_move(&bd, "h6", "h7h6");
        let bd = do_test_and_make_move(&bd, "Ngf3", "g1f3");
        let bd = do_test_and_make_move(&bd, "g5", "g7g5");
        let bd = do_test_and_make_move(&bd, "Bg3", "f4g3");
        let bd = do_test_and_make_move(&bd, "Qb6", "d8b6");
        let bd = do_test_and_make_move(&bd, "Nc4", "d2c4");
        let _bd = do_test_and_make_move(&bd, "Qb4+", "b6b4");
    }

    #[test]
    fn test_parse_move2() {
        let bd = Board::parse_fen("4k1n1/1p1p3P/8/8/pPp1p3/3P1P2/P1P1P1P1/R3K3 w Q - 0 1").unwrap();
        // promo
        do_test_and_make_move(&bd, "h8=Q", "h7h8q");
        do_test_and_make_move(&bd, "h7h8q", "h7h8q");
        do_test_and_make_move(&bd, "h7h8r", "h7h8r");
        do_test_and_make_move(&bd, "h7h8b", "h7h8b");
        do_test_and_make_move(&bd, "h7h8n", "h7h8n");
        do_test_and_make_move(&bd, "h8=R", "h7h8r");
        do_test_and_make_move(&bd, "h8=B", "h7h8b");
        do_test_and_make_move(&bd, "h8=N", "h7h8n");

        do_test_and_make_move(&bd, "g8=Q", "h7g8q");
        do_test_and_make_move(&bd, "g8=R", "h7g8r");
        do_test_and_make_move(&bd, "hxg8=Q", "h7g8q");
        do_test_and_make_move(&bd, "h7xg8=Q", "h7g8q");

        do_test_and_make_move(&bd, "a2a3", "a2a3");
        do_test_and_make_move(&bd, "a1b1", "a1b1");
    }

    fn do_test_and_make_move(bd: &Board, san: &str, uci: &str) -> Board {
        let mv = Parse::move_san(san, &bd);
        assert!(
            mv.is_ok(),
            "\nmove : {}\nerror: {}\nmoves: {}\nboard: {}\n",
            san,
            mv.unwrap_err(),
            bd.legal_moves(),
            bd
        );
        let mv = mv.unwrap();
        assert_eq!(mv.to_string(), uci);
        bd.clone().make_move(&mv)
    }

    #[test]
    fn test_parse_pgn() {
        // [Event "Let\\'s Play!"]
        // [Site "Chess.com"]
        // [Date "2020.11.30"]
        // [Round "?"]
        // [White "akanalytics"]
        // [Black "raistrma"]
        // [Result "*"]
        // [ECO "A40"]
        // [WhiteElo "1169"]
        // [BlackElo "1118"]
        // [TimeControl "1/604800"]
        //      let moves = "1. d4 c6 2. Bf4 d6 3. Nd2 h6 4. Ngf3 g5 5. Bg3 Qb6 6. Nc4 Qb4+ 7. Nfd2 Be6 8. c3
        // Qb5 9. e3 Bxc4 10. Nxc4 Qd5 11. Qf3 Qxf3 12. gxf3 Nd7 13. h4 Bg7 14. e4 Ngf6 15.
        // Bd3 Nh5 16. hxg5 Nxg3 17. fxg3 hxg5 18. Rxh8+ Bxh8 19. Kd2 O-O-O 20. Ne3 e6
        // 21. Rh1 b5 *"

        //     // let fen = "2kr3b/p2n1p2/2ppp3/1p4p1/3PP3/2PBNPP1/PP1K4/7R w - b6 0 22"
        // self.do_parse_fen_or_pgn(pgn, fen, depth=2)
    }
}
