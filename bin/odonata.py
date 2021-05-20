"""
    Based upon https://github.com/zhelyabuzhsky/odonata (MIT license)
"""


"""
    This module implements the Odonata class.
    :copyright: (c) 2016-2021 by Ilya Zhelyabuzhsky.
    :license: MIT, see LICENSE for more details.
"""


import subprocess
from typing import Any, List, Optional


DEFAULT_ODONATA_PARAMS = {
    "Hash": 16,
}


DEFAULT_STOCKFISH_PARAMS = {
    "Write Debug Log": "false",
    "Contempt": 0,
    "Min Split Depth": 0,
    "Threads": 1,
    "Ponder": "false",
    "Hash": 16,
    "MultiPV": 1,
    "Skill Level": 20,
    "Move Overhead": 30,
    "Minimum Thinking Time": 20,
    "Slow Mover": 80,
    "UCI_Chess960": "false",
    "UCI_LimitStrength": "false",
    "UCI_Elo": 1350,
}


class Odonata:
    """Integrates the Odonata chess engine with Python."""

    def __init__(
        self, 
        path: str = "odonata", 
        debug: bool = False,
        depth: int = 2, 
        parameters: dict = None
    ) -> None:
        self.debug: bool = debug
        self.odonata = subprocess.Popen(
            path, universal_newlines=True, stdin=subprocess.PIPE, stdout=subprocess.PIPE
        )

        # self._odonata_major_version: int = int(self._read_line().split(" ")[1])

        self._put("uci")

        self.depth = str(depth)
        self.info: str = ""

        if parameters is None:
            parameters = {}
        self._parameters = DEFAULT_ODONATA_PARAMS
        self._parameters.update(parameters)
        for name, value in list(self._parameters.items()):
            self._set_option(name, value)

        self._start_new_game()

    def get_parameters(self) -> dict:
        """Returns current board position.
        Returns:
            Dictionary of current Odonata engine's parameters.
        """
        return self._parameters

    def _start_new_game(self) -> None:
        self._put("ucinewgame")
        self._is_ready()
        self.info = ""

    def _put(self, command: str) -> None:
        if not self.odonata.stdin:
            raise BrokenPipeError()
        if self.debug: 
            print("  >", command)
        self.odonata.stdin.write(f"{command}\n")
        self.odonata.stdin.flush()

    def _read_line(self) -> str:
        if not self.odonata.stdout:
            raise BrokenPipeError()
        text = self.odonata.stdout.readline().strip()
        if self.debug: 
            print("<", text)
        return text

    def _set_option(self, name: str, value: Any) -> None:
        self._put(f"setoption name {name} value {value}")
        self._is_ready()

    def _is_ready(self) -> None:
        self._put("isready")
        while True:
            if self._read_line() == "readyok":
                return

    def _go(self) -> None:
        self._put(f"go depth {self.depth}")

    def _go_time(self, time: int) -> None:
        self._put(f"go movetime {time}")

    @staticmethod
    def _convert_move_list_to_str(moves: List[str]) -> str:
        result = ""
        for move in moves:
            result += f"{move} "
        return result.strip()

    def set_position(self, moves: List[str] = None) -> None:
        """Sets current board position.
        Args:
            moves:
              A list of moves to set this position on the board.
              Must be in full algebraic notation.
              example: ['e2e4', 'e7e5']
        """
        self._start_new_game()
        if moves is None:
            moves = []
        self._put(
            f"position startpos moves {self._convert_move_list_to_str(moves)}")

    def get_board_visual(self) -> str:
        """Returns a visual representation of the current board position.
        Returns:
            String of visual representation of the chessboard with its pieces in current position.
        """
        self._put("b")
        board_rep = ""
        count_lines = 0
        while count_lines < 15:
            board_str = self._read_line()
            if "info string" in board_str or "+" in board_str or "|" in board_str:
                count_lines += 1
                text = board_str.split("info string",1)[1]
                board_rep += f"{text}\n"
        return board_rep

    def get_fen_position(self) -> str:
        """Returns current board position in Forsyth–Edwards notation (FEN).
        Returns:
            String with current position in Forsyth–Edwards notation (FEN)
        """
        self._put("d")
        while True:
            text = self._read_line()
            splitted_text = text.split(" ")
            if splitted_text[0] == "Fen:":
                return " ".join(splitted_text[1:])

    def set_skill_level(self, skill_level: int = 20) -> None:
        """Sets current skill level of odonata engine.
        Args:
            skill_level:
              Skill Level option between 0 (weakest level) and 20 (full strength)
        Returns:
            None
        """
        self._set_option("UCI_LimitStrength", "false")
        self._set_option("Skill Level", skill_level)
        self._parameters.update({"Skill Level": skill_level})

    def set_elo_rating(self, elo_rating: int = 1350) -> None:
        """Sets current elo rating of odonata engine, ignoring skill level.
        Args:
            elo_rating: Aim for an engine strength of the given Elo
        Returns:
            None
        """
        self._set_option("UCI_LimitStrength", "true")
        self._set_option("UCI_Elo", elo_rating)
        self._parameters.update({"UCI_Elo": elo_rating})

    def set_fen_position(self, fen_position: str) -> None:
        """Sets current board position in Forsyth–Edwards notation (FEN).
        Args:
            fen_position:
              FEN string of board position.
        Returns:
            None
        """
        self._start_new_game()
        self._put(f"position fen {fen_position}")

    def get_best_move(self) -> Optional[str]:
        """Returns best move with current position on the board.
        Returns:
            A string of move in algebraic notation or None, if it's a mate now.
        """
        self._go()
        last_text: str = ""
        while True:
            text = self._read_line()
            splitted_text = text.split(" ")
            if splitted_text[0] == "bestmove":
                if splitted_text[1] == "(none)":
                    return None
                self.info = last_text
                return splitted_text[1]
            last_text = text

    def get_best_move_time(self, time: int = 1000) -> Optional[str]:
        """Returns best move with current position on the board after a determined time
        Args:
            time:
              Time for odonata to determine best move in milliseconds (int)
        Returns:
            A string of move in algebraic notation or None, if it's a mate now.
        """
        self._go_time(time)
        last_text: str = ""
        while True:
            text = self._read_line()
            splitted_text = text.split(" ")
            if splitted_text[0] == "bestmove":
                if splitted_text[1] == "(none)":
                    return None
                self.info = last_text
                return splitted_text[1]
            last_text = text

    def is_move_correct(self, move_value: str) -> bool:
        """Checks new move.
        Args:
            move_value:
              New move value in algebraic notation.
        Returns:
            True, if new move is correct, else False.
        """
        self._put(f"go depth 1 searchmoves {move_value}")
        while True:
            text = self._read_line()
            splitted_text = text.split(" ")
            if splitted_text[0] == "bestmove":
                if splitted_text[1] == "(none)":
                    return False
                else:
                    return True

    def get_evaluation(self) -> dict:
        """Evaluates current position
        Returns:
            A dictionary of the current advantage with "type" as "cp" (centipawns) or "mate" (checkmate in)
        """

        evaluation = dict()
        fen_position = self.get_fen_position()
        if "w" in fen_position:  # w can only be in FEN if it is whites move
            compare = 1
        else:  # odonata shows advantage relative to current player, convention is to do white positive
            compare = -1
        self._put(f"position {fen_position}")
        self._go()
        while True:
            text = self._read_line()
            splitted_text = text.split(" ")
            if splitted_text[0] == "info":
                for n in range(len(splitted_text)):
                    if splitted_text[n] == "score":
                        evaluation = {
                            "type": splitted_text[n + 1],
                            "value": int(splitted_text[n + 2]) * compare,
                        }
            elif splitted_text[0] == "bestmove":
                return evaluation

    def set_depth(self, depth_value: int = 2) -> None:
        """Sets current depth of odonata engine.
        Args:
            depth_value: Depth option higher than 1
        """
        self.depth = str(depth_value)


    def get_odonata_major_version(self):
        """Returns Odonata engine major version.
        Returns:
            Current odonata major version
        """

        return self._odonata_major_version

    def __del__(self) -> None:
        self._put("quit")
        self.odonata.kill()


def main():
    print("Starting...\n")
    odo = Odonata("odonata-0.3.12.exe")
    odo.set_fen_position("kr6/p7/8/8/8/8/P7/5BRK w - - 0 31")
    print(odo.get_board_visual())
    
    print("best move", odo.get_best_move())


if __name__ == "__main__":
    main()
