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
use std::rc::Rc;

#[derive(Clone, Default)]
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
    callback: Option<Rc<dyn Fn(&Game, &Move)>>,
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

    pub fn set_callback(&mut self, callback: impl Fn(&Game, &Move) + 'static) -> &mut Self {
        self.callback = Some(Rc::new(callback));
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

    pub fn print_move(&self, mv: &Move) {
        println!(
            "{:>2}.{:<8}  {}",
            self.board.fullmove_number(),
            if self.board.color_us() == Color::Black {
                ".. ".to_string()
            } else {
                "".to_string()
            } + &self.board.to_san(&mv),
            self.board.to_fen()
        );
    }

    pub fn choose_move(&mut self, white: &mut Algo, black: &mut Algo) -> Move {
        if !self.board.outcome().is_game_over() {
            if let Err(e) = self.board.validate() {
                panic!("Error on board {}", e);
            };

            let player = if self.board.color_us() == Color::White {
                white
            } else {
                black
            };
            player.search(&self.board);
            let m = player.bm();
            let tags = player.results().tags().clone();
            self.record_move(m, tags);
            if let Some(cb) = &self.callback {
                cb(self, &m);
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
        let mut moves = self
            .starting_pos
            .to_san_moves(&self.moves, Some(&self.annotations));
        if !f.alternate() {
            moves = moves.replace("\n", " ");
        }
        writeln!(f, "{} {}", moves, self.outcome.as_pgn())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::eval::*;
    use crate::search::timecontrol::*;
    use crate::types::{ScoreWdl};
    use std::time::Duration;
    use crate::bitboard::castling::CastlingRights;

    #[test]
    #[ignore]
    fn single_game() {
        let eval_w = SimpleScorer::new();
        let mut white = Algo::new()
            .set_timing_method(TimeControl::Depth(6))
            .set_eval(eval_w)
            .build();

        let mut eval_b = SimpleScorer::new();
        eval_b.mobility = false;
        let mut black = Algo::new()
            .set_timing_method(TimeControl::Depth(6))
            .set_eval(eval_b)
            .build();

        let board = Catalog::starting_position();
        let mut game = Game::new();
        game.set_starting_pos(&board);
        let callback = |gm: &Game, mv: &Move| gm.print_move(mv);
        game.set_callback(callback);
        game.play(&mut white, &mut black);
        println!("{}", game);
        println!("{}", white);
        assert_eq!(game.outcome().winning_color(), Some(Color::White));
        assert_eq!(game.moves.len(), 247);
    }

    // use crate::comms::uci::Uci;

    #[test]
    #[ignore]
    fn games() {
        //let tc = TimeControl::NodeCount(1_000);
        let tc = TimeControl::from_remaining_time(Duration::from_millis(3000));
        // let tc = TimeControl::Depth(3);
        let mut new = Algo::new().set_timing_method(tc).build();
        let mut old = Algo::new().set_timing_method(tc).build();
        // new.set_callback(Uci::uci_info);

        new.mte.deterministic = false;
        // new.eval.rook_open_file = 20;
        new.eval.cache_eval = true;
        // new.eval.cache.capacity = 1000;
        // new.eval.qcache.capacity = 1000;
        new.eval.cache_eval = true;
        new.eval.cache_qeval = true;
        new.tt.aging = true;

        old.mte.deterministic = false;
        old.eval.cache_eval = false;
        old.eval.cache_qeval = false;
        //old.eval.rook_open_file = 0;
        old.tt.aging = true;

        let wdl = tournament(&mut new, &mut old);
        println!(
            "\nscore as new {}\nELO difference {:.02}",
            wdl,
            wdl.elo_differnce(),
        );
    }

    fn tournament(new: &mut Algo, old: &mut Algo) -> ScoreWdl {
        let mut wdl = ScoreWdl::default();
        const N: u32 = 960;
        for id in 0..N {
            let pos = Catalog::chess960(id);
            let mut board = pos.board().clone();
            board.set_castling(CastlingRights::NONE);

            new.new_game();
            old.new_game();
            let mut gm1 = Game::new();
            gm1.round = pos.id().unwrap().to_string() + " W";
            gm1.set_starting_pos(&board);
            gm1.play(new, old);
            eprintln!("{}\n", gm1);
            if id == N - 1 {
                println!("Last white play for **NEW**\n{}", new);
            }

            new.new_game();
            old.new_game();
            let mut gm2 = Game::new();
            gm2.round = pos.id().unwrap().to_string() + " B";
            gm2.set_starting_pos(&board);
            gm2.play(old, new);
            eprintln!("{}\n", gm2);
            if id == N - 1 {
                println!("Last white play for **OLD**\n{}", old);
            }

            wdl += gm1.outcome().as_wdl() - gm2.outcome().as_wdl();

            print!(
                "pos: {} score {}   {:<15} {:<15} ",
                pos.id().unwrap(),
                wdl,
                gm1.outcome(),
                gm2.outcome()
            );
            if gm1.outcome() == Outcome::DrawRule75 || gm2.outcome() == Outcome::DrawRule75 {
                print!(
                    "mat.score:{:>4} mat:{}  ",
                    gm1.board.material().centipawns(),
                    gm1.board.material()
                );
                print!(
                    "mat.score:{:>4} mat:{}  ",
                    gm2.board.material().centipawns(),
                    gm2.board.material()
                );
            }
            println!();
            if (id + 1) % 10 == 0 {
                println!("score as new {}\nELO difference {:.02}", wdl, wdl.elo_differnce());
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
