use crate::board::makemove::MoveMaker;
use crate::board::movegen::MoveGen;
use crate::board::Board;
use crate::pvtable::PvTable;
use crate::eval::{Scorable, Score};
use crate::movelist::{Move, MoveList};
use crate::types::Color;





struct Game {
    Board
    moves
    outcome
    
}




struct Match {
    players
    board

}





class Match:

    def __init__( self, player_w: ChessPlayer, player_b: ChessPlayer, mg: MoveGenerator, board: Board ) -> None:
        self.player_w = player_w
        self.player_b = player_b
        self.game = Game( board, mg )

    @Profiler
    def moves(self) -> Iterable[Board]:
        while not self.game.is_over():
            player = self.player_w if self.game.board().turn == 'w' else self.player_b
            player.analyse( self.game.board() )
            self.game.record_move( self.game.board().state.best_move )
            yield self.game.board()

    def __str__(self) -> str:
        return f"{self.player_w} vs {self.player_b}"