use crate::board::boardbuf::BoardBuf;
use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::catalog::Catalog;
use crate::movelist::MoveValidator;
use crate::movelist::{Move, MoveList};
use crate::outcome::GameEnd;
use crate::outcome::Outcome;
use crate::search::algo::Algo;
use crate::tags::Tags;
use crate::types::Color;
use std::fmt;

#[derive(Clone, Debug, Default)]
pub struct Game {
    starting_pos: Board,
    board: Board,
    tags: Tags,
    moves: MoveList,
    annotations: Vec<Tags>,
    event: String,
    site: String,
    date: String,
    round: String,
    name_w: String,
    name_b: String,
    outcome: Outcome,
}

impl Game {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_starting_pos(&mut self, board: &Board) -> &mut Self {
        self.starting_pos = board.clone();
        self.board = board.clone();
        self
    }

    pub fn outcome(&self) -> Outcome {
        self.outcome
    }

    pub fn record_move(&mut self, mv: Move, tags: Tags) {
        self.moves.push(mv);
        self.annotations.push(tags);
        self.board = self.board.make_move(&mv);
        self.outcome = self.board.outcome();
    }

    pub fn play(&mut self, white: &mut Algo, black: &mut Algo) {
        while !self.board.outcome().is_game_over() {
            let mv = self.choose_move(white, black);
            white.repetition.push(&mv, &self.board);
            black.repetition.push(&mv, &self.board);
        }
    }

    pub fn choose_move(&mut self, white: &mut Algo, black: &mut Algo) -> Move {
        if !self.board.outcome().is_game_over() {
            if let Err(e) = self.board.validate() {
                panic!("Error on board {}", e);
            };

            let player = self.board.color_us().chooser_wb(white, black);
            player.search(&self.board);
            let m = player.bm();
            if m.is_null {
                println!("{}", player);
            }
            // if self.board.fullmove_number() == 50 {
            //     println!("{}", player);
            // }
            let tags = player.results().tags().clone();
            self.record_move(m, tags);
            // FIXME
            if 1 == 0 {
                println!(
                    "{}.{} {}   {}",
                    self.board.fullmove_number(),
                    if self.board.color_us() == Color::Black { ".. " } else { "" },
                    self.board.to_san(&m),
                    self.board.to_fen()
                );
            }
            return m;
        }
        Move::new_null()
    }

    // pub const MOVE_SORTS: &'static [&'static str] = &["Natural", "PV from Prior Iteration", "MVV/LVA"];
}

// [Event "GRENKE Chess Classic 2019"]
// [Site "Karlsruhe/Baden Baden GER"]
// [Date "2019.04.20"]
// [Round "1.2"]
// [White "Svidler, Peter"]
// [Black "Caruana, Fabiano"]
// [Result "1/2-1/2"]
// [WhiteTitle "GM"]
// [BlackTitle "GM"]
// [WhiteElo "2735"]
// [BlackElo "2819"]
// [ECO "B33"]
// [Opening "Sicilian"]
// [Variation "Pelikan (Lasker/Sveshnikov) variation"]
// [WhiteFideId "4102142"]
// [BlackFideId "2020009"]
// [EventDate "2019.04.20"]
// [WhiteACPL "252"]
// [BlackACPL "141"]
impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // seven tag roster
        writeln!(f, "[Event \"{}\"]", self.event)?;
        writeln!(f, "[Site \"{}\"]", self.site)?;
        writeln!(f, "[Date \"{}\"]", self.date)?;
        writeln!(f, "[Round \"{}\"]", self.round)?;
        writeln!(f, "[White \"{}\"]", self.name_w)?;
        writeln!(f, "[Black \"{}\"]", self.name_b)?;
        writeln!(f, "[Result \"{}\"]", self.outcome.as_pgn())?;

        // // optional tag pairs
        // writeln!(f, "[Time \"{}\"]", self.white.name())?;
        // writeln!(f, "[Annotator \"{}\"]", self.outcome().as_pgn())?;
        // writeln!(f, "[PlyCount \"{}\"]", self.outcome().as_pgn())?;
        // writeln!(f, "[Time \"{}\"]", self.white.name())?;
        // writeln!(f, "[Termination \"{}\"]", self.white.name())?;
        // writeln!(f, "[Mode \"{}\"]", self.white.name())?;
        if self.starting_pos != Catalog::starting_position() {
            writeln!(f, "[FEN \"{}\"]", self.starting_pos.to_fen())?;
            writeln!(f, "[SetUp \"1\"]")?;
        }
        let mut moves = self.starting_pos.to_san_moves(&self.moves, Some(&self.annotations));
        if !f.alternate() {
            moves = moves.replace("\n", " ");
        }
        writeln!(
            f,
            "{} {}",
            moves,
            self.outcome.as_pgn()
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::eval::*;
    use crate::search::timecontrol::*;
    use crate::types::{CastlingRights, ScoreWdl};
    use std::time::Duration;

    #[test]
    fn test_game() {
        let eval_w = SimpleScorer::new().set_position(true);
        let mut white = Algo::new().set_timing_method(TimeControl::Depth(2)).set_eval(eval_w).build();

        let eval_b = SimpleScorer::new().set_position(false);
        let mut black = Algo::new().set_timing_method(TimeControl::Depth(2)).set_eval(eval_b).build();

        let board = Catalog::starting_position();
        let mut game = Game::new();
        game.set_starting_pos(&board);
        game.play(&mut white, &mut black);
        println!("{}", game);
        assert_eq!(game.outcome().winning_color(), Some(Color::White));
    }

    // use crate::comms::uci::Uci;

    #[test]
    #[ignore]
    fn competition() {
        //let tc = TimeControl::NodeCount(1_000);
        let tc = TimeControl::from_remaining_time(Duration::from_millis(1000));
        //let tc = TimeControl::Depth(3);
        let mut white = Algo::new().set_timing_method(tc).build();
        let mut black = Algo::new().set_timing_method(tc).build();
        // white.set_callback(Uci::uci_info);

        white.mte.deterministic = true;
        white.tt.enabled = true;
        white.eval.mobility = true;

        black.mte.deterministic = true;
        black.tt.enabled = true;
        black.eval.mobility = false;

        let wdl = tournament(&mut white, &mut black);
        println!("score as white {}\nELO difference {:.02}", wdl, wdl.elo_differnce());
    }

    fn tournament(white: &mut Algo, black: &mut Algo) -> ScoreWdl {
        let mut wdl = ScoreWdl::default();
        for id in 0..960 {
            let pos = Catalog::chess960(id);
            let mut board = pos.board().clone();
            board.set_castling(CastlingRights::NONE);

            white.reset();
            black.reset();
            let mut gm1 = Game::new();
            gm1.round = pos.id().unwrap().to_string() + " W";
            gm1.set_starting_pos(&board);
            gm1.play(white, black);
            eprintln!("{}\n", gm1);

            white.tt.clear();
            black.tt.clear();
            let mut gm2 = Game::new();
            gm2.round = pos.id().unwrap().to_string() + " B";
            gm2.set_starting_pos(&board);
            gm2.play(black, white);
            eprintln!("{}\n", gm2);

            wdl += gm1.outcome().as_wdl() - gm2.outcome().as_wdl();

            print!("pos: {} score {}   {:<15} {:<15} ", pos.id().unwrap(), wdl, gm1.outcome(), gm2.outcome());
            if gm1.outcome() == Outcome::DrawRule75 || gm2.outcome() == Outcome::DrawRule75 {
                print!("mat.score:{:>4} mat:{}  ", gm1.board.material().centipawns(), gm1.board.material());
                print!("mat.score:{:>4} mat:{}  ", gm2.board.material().centipawns(), gm2.board.material());
            }
            println!();
            if (id+1) % 10 == 0 {
                println!("score as white {}\nELO difference {:.02}", wdl, wdl.elo_differnce());
            }
        }
        wdl
    }

    #[test]
    fn test_bug1() {
        let b = Board::parse_fen("1rk2qRr/8/B3P3/B4QN1/P4p2/2K1PP1P/P7/R2N4 b - - 0 38").unwrap();
        let tc = TimeControl::MoveTime(Duration::from_millis(100));
        let mut white = Algo::new().set_timing_method(tc).build();
        let mut black = Algo::new().set_timing_method(tc).build();
        white.move_orderer.mvv_lva = true;
        black.move_orderer.mvv_lva = false;
        black.search(&b);
    }

    #[test]
    fn test_bug2() {
        let b1 = Board::parse_fen("1r3rbQ/p1p1kp2/4pn2/2Pp4/2n3p1/1P1N4/2P1PPPP/q2K1RBB w - - 0 23").unwrap();
        // let tc = TimeControl::MoveTime(Duration::from_millis(140));
        let tc = TimeControl::from_remaining_time(Duration::from_millis(1750));
        let mut white = Algo::new().set_timing_method(tc).build();
        let _black = Algo::new().set_timing_method(tc).build();
        white.search(&b1);
        white.search(&b1);
        white.search(&b1);
        // let b2 = b1.make_move(&black.bm());
        //white.search(&b2);
        println!("{}", white);
        assert_eq!(white.bm().uci(), "d3c1");
    }
}
