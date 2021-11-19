use crate::board::boardbuf::BoardBuf;
use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::catalog::Catalog;
// use crate::movelist::MoveValidator;
use crate::mv::{Move};
use crate::variation::Variation;
use crate::outcome::Outcome;
use crate::search::algo::Algo;
use crate::position::Position;
use crate::eval::score::Score;
use crate::tags::Tags;
use crate::types::Color;
use std::fmt;
use std::rc::Rc;

#[derive(Clone, Default)]
pub struct Game {
    starting_pos: Position,
    board: Board,
    tags: Tags,
    moves: Variation,
    annotations: Vec<Tags>,
    event: String,
    site: String,
    date: String,
    round: String,
    name_w: String,
    name_b: String,
    outcome: Outcome,
    callback: Option<Rc<dyn Fn(&Game, &Move, &Tags)>>,
}

impl Game {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_starting_pos(&mut self, pos: Position) -> &mut Self {
        self.board = pos.board().clone();
        self.starting_pos = pos;
        self
    }

    pub fn set_callback(&mut self, callback: impl Fn(&Game, &Move, &Tags) + 'static) -> &mut Self {
        self.callback = Some(Rc::new(callback));
        self
    }

    pub fn outcome(&self) -> Outcome {
        self.outcome
    }


    pub fn play(&mut self, white: &mut Algo, black: &mut Algo) {
        while !self.board.outcome().is_game_over() {
            let mv = self.choose_move(white, black);
            white.repetition.push_move(&mv, &self.board);
            black.repetition.push_move(&mv, &self.board);
        }
    }

    pub fn choose_move(&mut self, white: &mut Algo, black: &mut Algo) -> Move {
        if !self.board.outcome().is_game_over() {
            if let Err(e) = self.board.validate() {
                panic!("Error on board {} {:#}", e, self.board);
            };

            let player = if self.board.color_us() == Color::White {
                white
            } else {
                black
            };
            player.set_position(Position::from_board(self.board.clone())).search();
            let mv = player.results.bm();
            let tags = player.results_as_position().tags().clone();
            if player.score() == -Score::INFINITY {
                print!("{}", player);
            }
            self.record_move(&mv, &tags);
            if let Some(callback) = &self.callback {
                callback(self, &mv, &tags);
            }
            return mv;
        }
        Move::new_null()
    }

    pub fn record_move(&mut self, mv: &Move, tags: &Tags) {
        self.print_move(&mv, &tags);
        self.moves.push(mv.clone());
        self.annotations.push(tags.clone());
        self.board = self.board.make_move(mv);
        self.outcome = self.board.outcome();
    }

    pub fn print_move(&self, mv: &Move, tags: &Tags) {
        println!(
            "{:>2}.{:<8}  {}  {}",
            self.board.fullmove_number(),
            if self.board.color_us() == Color::Black {
                ".. ".to_string()
            } else {
                "".to_string()
            } + &self.board.to_san(&mv),
            self.board.to_fen(),
            tags,
        );
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
            writeln!(f, "[FEN \"{}\"]", self.starting_pos.board().to_fen())?;
            writeln!(f, "[SetUp \"1\"]")?;
        }
        let mut moves = self
            .starting_pos.board()
            .to_san_variation(&self.moves, Some(&self.annotations));
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
    use crate::infra::component::Component;


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

        let pos = Catalog::starting_position();
        let mut game = Game::new();
        game.set_starting_pos(pos);
        let callback = |gm: &Game, mv: &Move, tags: &Tags| gm.print_move(mv, tags);
        game.set_callback(callback);
        game.play(&mut white, &mut black);
        println!("{}", game);
        println!("{}", white);
        assert_eq!(game.outcome().winning_color(), Some(Color::Black));
        // assert_eq!(game.moves.len(), 192);
    }

    // use crate::comms::uci::Uci;

    #[test]
    #[ignore]
    fn test_game() {
        let tc = TimeControl::NodeCount(5_000);
        // let tc = TimeControl::from_remaining_time(Duration::from_millis(3000));
        // let tc = TimeControl::Depth(3);
        // let tc = TimeControl::from_move_time_millis(200);
        let mut new = Algo::new().set_timing_method(tc).build();
        let mut old = Algo::new().set_timing_method(tc).build();
        // new.set_callback(Uci::uci_info);


        // new.eval.rook_open_file = 20;
        // new.eval.cache_eval = true;
        // // new.eval.cache.capacity = 1000;
        // // new.eval.qcache.capacity = 1000;
        // new.eval.cache_eval = true;
        // new.eval.cache_qeval = true;
        // new.mte.branching_factor = 5;  // cause more failed plys
        // new.ids.part_ply = true;
        // new.tt.aging = true;
        // old.eval.undefended_sq = 3;  // was 3
        // new.tt.use_tt_for_pv = false;
        // new.eval.safety = true;
        // new.repetition.avoid_tt_on_repeats = false;

        // old.repetition.avoid_tt_on_repeats = true;
        // old.tt.use_tt_for_pv = true;
        // old.eval.safety = true;
        // old.eval.undefended_sq = 0;
        // old.eval.undefended_piece = 0;
        // old.mte.branching_factor = 5;  // cause more failed plys
        // old.ids.part_ply = false;
        // old.eval.cache_eval = false;
        // old.eval.cache_qeval = false;
        // //old.eval.rook_open_file = 0;
        // old.tt.aging = true;

        let wdl = tournament(&mut new, &mut old);
        println!(
            "\nscore as new {}\nELO difference {:.02}",
            wdl,
            wdl.elo(),
        );
    }

    fn tournament(new: &mut Algo, old: &mut Algo) -> ScoreWdl {
        let mut wdl = ScoreWdl::default();
        const N: u32 = 960;
        for id in 0..N {
            let mut pos = Catalog::chess960(id);
            pos.board_mut().set_castling(CastlingRights::NONE);

            new.new_game();
            old.new_game();
            let mut gm1 = Game::new();
            gm1.round = pos.id().unwrap().to_string() + " W";
            gm1.set_starting_pos(pos.clone());
            gm1.play(new, old);
            eprintln!("{}\n", gm1);
            if id == N - 1 {
                println!("Last white play for **NEW**\n{}", new);
            }

            new.new_game();
            old.new_game();
            let mut gm2 = Game::new();
            gm2.round = pos.id().unwrap().to_string() + " B";
            gm2.set_starting_pos(pos.clone());
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
                println!("score as new {}\nELO difference {:.02}", wdl, wdl.elo());
            }
        }
        wdl
    }

    #[test]
    fn test_bug1() {
        let pos = Position::parse_epd("1rk2qRr/8/B3P3/B4QN1/P4p2/2K1PP1P/P7/R2N4 b - - 0 38").unwrap();
        let tc = TimeControl::SearchTime(Duration::from_millis(100));
        let mut white = Algo::new().set_timing_method(tc).build();
        let mut black = Algo::new().set_timing_method(tc).build();
        white.move_orderer.mvv_lva = true;
        black.move_orderer.mvv_lva = false;
        black.set_position(pos).search();
    }

    #[test]
    fn test_bug2() {
        let pos1 = Position::parse_epd("1r3rbQ/p1p1kp2/4pn2/2Pp4/2n3p1/1P1N4/2P1PPPP/q2K1RBB w - - 0 23").unwrap();
        // let tc = TimeControl::SearchTime(Duration::from_millis(140));
        let tc = TimeControl::from_remaining_time(Duration::from_millis(1750));
        let mut white = Algo::new();
        let _black = Algo::new();
        white.set_position(pos1.clone()).set_timing_method(tc).search();
        white.set_position(pos1.clone()).set_timing_method(tc).search();
        white.set_position(pos1).set_timing_method(tc).search();
        // let b2 = b1.make_move(&black.bm());
        //white.search(&b2);
        println!("{}", white);
        assert_eq!(white.results.bm().uci(), "d3c1");
    }
}
