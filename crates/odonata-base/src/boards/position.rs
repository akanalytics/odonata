use crate::{
    other::outcome::Outcome,
    piece::{FlipSide, Ply, Repeats},
    prelude::{Board, Hash, Move, Result, Variation},
    Epd,
};
use std::fmt::{Debug, Display};

/// initial board >---(starting-moves)--> root_board >---(search-variation)--> board
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Position {
    pub board: Board,
    hash:      Hash,
    history:   Vec<(Board, Hash, Move)>, // preboard
    ply:       usize,
    sel_ply:   usize,
}

impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            let setup = &self.setup_board();
            let root = &self.root_board();
            let setup_var = self.played_moves().to_san(setup);
            let search_var = self.search_variation().to_san(root);
            writeln!(f)?;
            writeln!(f, "setup  {setup}")?;
            writeln!(f, "setup  {setup_var}")?;
            writeln!(f, "root   {root}")?;
            writeln!(f, "search {search_var}")?;
            Ok(())
        } else {
            let b = self.setup_board();
            write!(
                f,
                "{fen} moves {mvs}; pv {pv}; rc {rc}",
                fen = b.to_fen(),
                mvs = self.played_moves().to_san(&b),
                pv = self.search_variation().to_san(&self.root_board()),
                rc = self.played_reps(),
            )
        }
    }
}

impl Debug for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            writeln!(f, "{self:#}")?;
            let width = self
                .history
                .iter()
                .map(|(b, ..)| b.to_fen().len())
                .max()
                .unwrap_or_default()
                .max(self.board().to_fen().len());
            writeln!(
                f,
                "{n:>3} {ply:>3} {fen:<width$} {h:x}",
                n = "cur",
                ply = self.ply,
                h = self.board().hash(),
                fen = self.board().to_fen(),
            )?;

            for (n, (b, h, m)) in self.history.iter().enumerate().rev() {
                writeln!(
                    f,
                    "{n:>3} {ply:>3} {fen:<width$} {h:x} {mv}",
                    ply = n.saturating_sub(self.history.len() - self.ply),
                    fen = b.to_fen(),
                    mv = m.to_san(b)
                )?;
            }
            Ok(())
        } else {
            f.debug_struct("Position")
                .field("board", &self.board)
                .field("history", &self.history)
                .field("hash", &self.hash)
                .field("ply", &self.ply)
                .field("sel_ply", &self.sel_ply)
                .finish()
        }
    }
}

impl Position {
    pub fn starting_pos() -> Self {
        Self::from_board(Board::starting_pos())
    }

    pub fn from_board(board: Board) -> Self {
        Self {
            board,
            ..Position::default()
        }
    }

    pub fn from_played_moves(starting_board: Board, setup_moves: Variation) -> Self {
        let mut pos = Position::from_board(starting_board);
        pos.push_moves(setup_moves);
        pos.ply = 0;
        pos
    }

    pub fn has_null_move(&self) -> bool {
        self.search_history().iter().any(|&(_, _, mv)| mv.is_null())
    }

    #[inline]
    pub fn board(&self) -> &Board {
        &self.board
    }

    #[inline]
    pub fn hash(&self) -> Hash {
        self.hash
    }

    pub fn root_board(&self) -> Board {
        self.get_board(self.played_history().len())
    }

    fn get_board(&self, index: usize) -> Board {
        self.history
            .get(index)
            .map(|(b, _h, _mv)| b)
            .unwrap_or(&self.board)
            .clone()
    }

    pub(super) fn played_history(&self) -> &[(Board, Hash, Move)] {
        let played_history_len = self.history.len() - self.ply;
        &self.history[0..played_history_len]
    }

    fn ply(&self) -> Ply {
        self.ply as Ply
    }

    pub(super) fn search_history(&self) -> &[(Board, Hash, Move)] {
        let played_history_len = self.history.len() - self.ply;
        &self.history[played_history_len..]
    }

    pub fn setup_board(&self) -> Board {
        self.get_board(0)
    }

    pub fn played_moves(&self) -> Variation {
        self.played_history().iter().map(|(_b, _h, m)| *m).collect()
    }

    // excludes starting moves
    pub fn search_variation(&self) -> Variation {
        self.search_history().iter().map(|(_b, _h, m)| *m).collect()
    }

    pub fn push_moves_str(&mut self, var: &str) -> Result<()> {
        let var = self.board.parse_san_variation(var)?;
        self.push_moves(var);
        Ok(())
    }

    pub fn push_moves(&mut self, moves: impl IntoIterator<Item = Move>) {
        moves.into_iter().for_each(|mv| self.push_move(mv));
    }

    #[inline]
    pub fn push_move(&mut self, mv: Move) {
        self.history.push((self.board.make_move(mv), self.hash, mv));
        let (old_mut, ..) = self.history.last_mut().unwrap();
        std::mem::swap(&mut self.board, old_mut);
        self.hash = self.board.hash();
        self.ply += 1;
    }

    pub fn prior_move(&self) -> Option<Move> {
        self.history.last().map(|(_b, _h, mv)| *mv)
    }

    pub fn prior_board(&self) -> Option<&Board> {
        self.history.last().map(|(b, _h, _mv)| b)
    }

    #[inline]
    pub fn pop_move(&mut self) {
        debug_assert!(self.ply > 0);
        (self.board, self.hash, _) = self.history.pop().unwrap();
        self.ply -= 1;
    }

    /// current position counts as 1
    pub fn played_reps(&self) -> usize {
        let hash = self.board().hash();
        self.played_history()
            .iter()
            .skip_while(|&(b, ..)| b.turn() != self.board().turn())
            .step_by(2)
            .filter(|&(_, h, _)| *h == hash)
            .count()
            + 1
    }

    pub fn repetition_counts(&self) -> Repeats {
        Repeats {
            in_search: self.search_reps() as u16,
            in_played: self.played_reps() as u16,
        }
    }

    /// current position counts as 1,
    /// so search reps = 2 => a repeat has occurred in the search
    pub fn search_reps(&self) -> usize {
        let include_prior_to_null_moves = true;
        let hash = self.board().hash();
        self.search_history()
            .iter()
            .rev()
            .take_while(|&(_, _, mv)| include_prior_to_null_moves || !mv.is_null())
            .skip(1)
            .step_by(2)
            .filter(|&(_, h, _)| *h == hash)
            .count()
            + 1
    }
}

impl Position {
    pub fn outcome(&self) -> Outcome {
        if let Some(outcome) = self.draw_outcome() {
            return outcome;
        }
        let color_to_play = self.board().turn();
        let can_move = self.board().has_legal_moves();
        if self.board().is_in_check(color_to_play) && !can_move {
            // white to play and in check with no moves => black win
            return Outcome::WinByCheckmate(color_to_play.flip_side());
        }
        if !can_move {
            return Outcome::DrawStalemate;
        }
        Outcome::Unterminated
    }

    pub fn is_draw_insufficient_material(&self) -> bool {
        self.board().is_draw_insufficient_material()
    }

    pub fn is_draw_rule_fifty(&self) -> bool {
        self.board().halfmove_clock() >= 2 * 50
    }

    pub fn draw_outcome(&self) -> Option<Outcome> {
        // note: 2 repeats of current move makes the current move the 3rd repeat
        match () {
            _ if self.is_draw_rule_fifty() => Some(Outcome::DrawRule50),
            _ if self.played_reps() >= 5 => Some(Outcome::DrawRepetition5),
            _ if self.played_reps() >= 3 => Some(Outcome::DrawRepetition3),
            _ if self.is_draw_insufficient_material() => Some(Outcome::DrawInsufficientMaterial),
            _ => None,
        }
    }
}

impl Position {
    pub fn parse_uci(uci: &str) -> Result<Position> {
        let uci = uci
            .trim_start()
            .strip_prefix("position")
            .unwrap_or(uci)
            .trim_start();
        let (fen, moves) = if let Some((fen, moves)) = uci.split_once("moves") {
            (fen.trim(), Some(moves.trim()))
        } else {
            (uci, None)
        };

        let board = match fen {
            "startpos" => Board::starting_pos(),
            fen => Board::parse_fen(fen)?,
        };

        let var = match moves {
            None => Variation::new(),
            Some(moves) => board.parse_uci_variation(moves)?,
        };
        Ok(Position::from_played_moves(board, var))
    }

    pub fn from_epd(epd: Epd) -> Position {
        let mut pos = Self::from_played_moves(epd.setup_board(), epd.played().clone());
        let pv = epd.var("pv").unwrap_or_default();
        pos.push_moves(pv);
        pos
    }

    pub fn to_epd(&self) -> Epd {
        Epd::from_var(self.setup_board(), self.played_moves())
    }
}

#[cfg(test)]
mod tests {
    use std::{hint::black_box, mem::size_of};

    use crate::{catalog::Catalog, infra::profiler::PerfProfiler, Color};

    use super::*;
    use test_log::test;

    #[test]
    fn test_position_basics() {
        let sb = Board::starting_pos();
        let mut start = Position::from_board(sb.clone());
        let mv1 = sb.parse_san_move("e4").unwrap();
        let mv2 = sb.make_move(mv1).parse_san_move("e5").unwrap();
        let clone = start.clone();
        assert_eq!(start, clone);
        start.push_move(mv1);
        start.push_move(mv2);
        start.pop_move();
        start.pop_move();
        assert_eq!(start, clone);
    }

    #[test]
    fn test_position_parse_uci() {
        let start = Position::from_board(Board::starting_pos());
        assert_eq!(start, Position::parse_uci("position startpos").unwrap());
        assert_eq!(
            start,
            Position::parse_uci("position startpos moves").unwrap()
        );
        assert_eq!(start, Position::parse_uci("startpos").unwrap());
        assert_eq!(start, Position::parse_uci(" startpos  moves ").unwrap());
        assert_eq!(
            Position::parse_uci("").unwrap_err().to_string(),
            "must specify at least 6 parts in epd/fen ''"
        );
        assert_eq!(
            Position::parse_uci("position x").unwrap_err().to_string(),
            "must specify at least 6 parts in epd/fen 'x'"
        );
        assert_eq!(
            Position::parse_uci("position startpos moves e2e4").unwrap(),
            Position::from_epd(
                Epd::parse_epd(&(Board::starting_pos().to_fen() + " moves e4;")).unwrap()
            )
        );
        let mut pos = Position::parse_uci("position startpos moves e2e4 e7e5").unwrap();
        assert_eq!(
            pos,
            Position::from_epd(
                Epd::parse_epd(&(Board::starting_pos().to_fen() + " moves e4 e5;")).unwrap()
            )
        );
        pos.push_moves_str("a2a4 a7a6").unwrap();
        assert_eq!(pos.search_variation().len(), 2);
        assert_eq!(pos.search_variation().to_uci(), "a2a4 a7a6");
        pos.pop_move();
        assert_eq!(pos.search_variation().to_uci(), "a2a4", "{pos:#?}");
        pos.pop_move();
        assert_eq!(pos.search_variation().to_uci(), "", "{pos:#?}");
    }

    #[test]
    fn test_position_parse_epd() -> Result<()> {
        let epd = "8/1k6/8/8/8/2K4R/8/8 w - - 0 1 moves Rh1 Ka8 Rh2 Ka7 Rh1 Ka8 Rh2 Ka7; id 'KRK'; pv Rh1 Ka8 Rh2 Ka7 Rh1 Ka8;";
        let p = Position::from_epd(Epd::parse_epd(epd)?);
        println!("{p:#?}");
        let setup = &p.setup_board();
        let root = &p.root_board();
        assert_eq!(
            p.played_moves().to_san(setup),
            "Rh1 Ka8 Rh2 Ka7 Rh1 Ka8 Rh2 Ka7"
        );
        assert_eq!(p.search_variation().to_san(root), "Rh1 Ka8 Rh2 Ka7 Rh1 Ka8");
        assert_eq!(p.played_reps(), 3);
        assert_eq!(p.search_reps(), 2);
        Ok(())
    }

    #[test]
    fn test_position_checkmate() {
        assert_eq!(
            Position::from_epd(Catalog::checkmates()[0].clone()).outcome(),
            Outcome::WinByCheckmate(Color::White)
        );
        assert_eq!(
            Position::from_epd(Catalog::checkmates()[1].clone()).outcome(),
            Outcome::WinByCheckmate(Color::Black)
        );
    }

    #[test]
    fn test_position_size() {
        assert_eq!(size_of::<Position>(), 200);
    }

    #[test]
    fn test_position_repetitions() {
        for epd in Catalog::repetitions() {
            let played_reps = epd.int("rc").unwrap() as u16;
            let search_reps: u16 = epd
                .tag("c0")
                .as_ref()
                .expect("c0 missing")
                .strip_prefix("search_reps:")
                .expect("search_reps: missing")
                .parse()
                .unwrap();
            let pos = Position::from_epd(epd);
            assert_eq!(pos.repetition_counts().in_played, played_reps, "{pos}");
            assert_eq!(pos.repetition_counts().in_search, search_reps, "{pos}");
        }
    }

    #[test]
    fn test_position_stalemate() {
        assert_eq!(
            Position::from_epd(Catalog::stalemates()[0].clone()).outcome(),
            Outcome::DrawStalemate
        );
        assert_eq!(
            Position::from_epd(Catalog::stalemates()[1].clone()).outcome(),
            Outcome::DrawStalemate
        );
    }

    #[test]
    fn bench_position() -> Result<()> {
        let mut pos = Position::starting_pos();
        pos.push_moves_str("e4 e5 a3 a6 h4 h5 d4 d5 Nc3 Nc6 Nf3 Nf6")?;
        let mv = pos.board().parse_san_move("b2b3")?;
        assert_eq!(pos.ply(), 12);
        assert_eq!(pos.prior_move().unwrap().to_uci(), "g8f6");
        assert_eq!(pos.setup_board().to_fen(), Board::starting_pos().to_fen());

        PerfProfiler::new("pos.clone").bench(|| black_box(&pos).clone());
        PerfProfiler::new("pos.push_move").bench(|| black_box(&mut pos).push_move(mv));
        PerfProfiler::new("pos.pop_move").bench(|| black_box(&mut pos).pop_move());
        PerfProfiler::new("pos.prior_board").bench(|| black_box(&pos).prior_board());
        PerfProfiler::new("pos.played_moves").bench(|| black_box(&pos).played_moves());
        PerfProfiler::new("pos.outcome").bench(|| black_box(&pos).outcome());
        Ok(())
    }
}
