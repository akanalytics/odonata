#!/usr/bin/env python3

from __future__ import annotations
from typing import Any, MutableSet, Iterator, Optional
from typing import List, Iterable
import logging
from textwrap import wrap
import subprocess
import os


logger = logging.getLogger()


class Bitwise:

    mask = (1 << 64) - 1

    # https://www.chessprogramming.org/General_Setwise_Operations#Least_Significant_One
    @staticmethod
    def lsb_isolate(x: int) -> int:
        """
        least significant bit isolation
        """
        return x & -x

    # https://www.chessprogramming.org/General_Setwise_Operations#Least_Significant_One
    @staticmethod
    def lsb_reset(x: int) -> int:
        """
        clears the least significant bit
        """
        return x & (x - 1)

    # https://www.chessprogramming.org/BitScan
    # https://docs.python.org/3/library/stdtypes.html len('100101') --> 6
    @staticmethod
    def bit_scan_forward(x: int) -> int:
        """
        position of the least significant bit (zero based position)
        """
        return x.bit_length() - 1

    @staticmethod
    def pop_count(x: int) -> int:
        """
        how many bits set
        """
        return bin(x).count('1')

    @staticmethod
    def flip_bits(x: int) -> int:
        """
        ones complement  ~x
        """
        return Bitwise.mask ^ x


Color = str  # Literal["w", "b"]


class Sides:
    """
    using CPW convention of white = 0, black = 1
    """
    WHITE = 0
    BLACK = 1

    @staticmethod
    def bitwise_direction_forward(c: Color) -> int:
        """
        8 if playing up the board (white, -8 if playing down (black)
        """
        return 8 if c == "w" else -8

    @staticmethod
    def opposite(c: Color) -> Color:
        return "b" if c == "w" else "w"

    @staticmethod
    def to_index(c: Color) -> int:
        return c == "b"


Piece = str

# we follow pythons collections, builtin, ctypes and functools and
# use a plural for these 'utility' classes
#


class Pieces:

    PIECES_BLACK = "pnbrqk"
    PIECES_WHITE = "PNBRQK"

    PIECES = [PAWN, KNIGHT, BISHOP, ROOK, QUEEN, KING] = range(6)

    EMPTY = " "
    PIECES_SLIDERS = "BRQbrq"
    BOTH_SIDES = [PIECES_WHITE, PIECES_BLACK]

    @staticmethod
    def colour(piece: Piece) -> Color:
        return 'b' if piece.islower() else 'w'

    @staticmethod
    def to_index(p: Piece) -> int:
        return Pieces.PIECES_BLACK.index(p)

    @staticmethod
    def to_colour(pieces: str, c: Color) -> str:
        """
        Convert a string of pieces (can be empty) such as 'kKR' 'Q' 'pPRq' etc to the given colour, 'b' or 'w'
        """
        return pieces.lower() if c == "b" else pieces.upper()

    @staticmethod
    def all_of(c: Color) -> str:
        """
        :param colour: either "b" or "w"
        :return: PNBRQK or pnbrqk
        """
        return Pieces.PIECES_WHITE if c == "w" else Pieces.PIECES_BLACK

    @staticmethod
    def is_valid(piece: Piece) -> bool:
        return piece in Pieces.PIECES_WHITE or piece in Pieces.PIECES_BLACK


# CPW calls this LSF.
# squareIndex (8*rankIndex + fileIndex)
#
# CPW calls the bit: little endian rank-file
# Rank 1 .. Rank 8 -> 0..7
# A-File .. H-File -> 0..7
#
# Compass rose
#
#   noWe         nort         noEa
#           +7    +8    +9
#               \  |  /
#   west    -1 <-  0 -> +1    east
#               /  |  \
#           -9    -8    -7
#   soWe         sout         soEa
#
class Square:

    ALL: List[Square]

    def __init__(self, i: int):
        self.index_x = i % 8
        self.index_y = i // 8
        self.index_i = i
        self.bitmap = 1 << i

    @staticmethod
    def parse(sq: str) -> Square:
        if len(sq) != 2 or sq[0].lower() not in 'abcdefgh' or sq[1] not in '12345678':
            raise ValueError(f'The square "{sq}" is not valid')
        return Square(int(sq[1]) * 8 - 8 + ord(sq[0].lower()) - ord('a'))

    @staticmethod
    def from_xy(file_x: int, rank_y: int) -> Square:
        return Square(file_x + rank_y * 8)

    @staticmethod
    def of(sq: int) -> Square:
        return Square.ALL[sq]

    def rank(self) -> str:
        return "12345678"[self.index_y]

    def file(self) -> str:
        return "abcdefgh"[self.index_x]

    @staticmethod
    def is_off_board(sq: int, delta: int) -> bool:
        if sq + delta < 0 or sq + delta > 63:
            return True
        # big jump in rank means sq+delta wrapper around the sides of board
        return abs((sq % 8) - (sq+delta) % 8) > 5

    def bit(self) -> int:
        return self.bitmap

    def __ilshift__(self, i: int) -> Square:
        self.index_i -= i
        self.index_x = self.index_i % 8
        self.index_y = self.index_i // 8
        return self

    def __irshift__(self, i: int) -> Square:
        self.index_i += i
        self.index_x = self.index_i % 8
        self.index_y = self.index_i // 8
        return self

    def index(self) -> int:
        return self.index_i

    def x(self) -> int:
        """0 to 7"""
        return self.index_x

    def y(self) -> int:
        """0 to 7"""
        return self.index_y

    def name(self) -> str:
        """a1 to h8"""
        return self.file() + self.rank()

    def __str__(self) -> str:
        """a1 to h8"""
        return self.name()

    def __repr__(self) -> str:
        """a1 to h8"""
        return self.name()

    def __hash__(self) -> int:
        return self.index_y * 8 + self.index_x

    def __eq__(self, other) -> bool:
        if other is None:
            return False
        elif type(other) is type(self):
            return self.index_x == other.index_x and self.index_y == other.index_y
        else:
            raise ValueError(f"Cannot compare a square to {other.__class__}")


Square.ALL = [Square(sq) for sq in range(64)]


Move = str


class Moves:

    @staticmethod
    def validate(m: str) -> bool:
        assert len(m) in [4, 5]
        if m == "Null":
            return True
        src = Square.parse(m[:2])
        dst = Square.parse(m[2:4])
        promo = m[4:]
        assert promo in 'nbrq', "promotion should be one of 'n, b, r or q'"
        return True


# Inherited Set methods and clear, pop, remove, __ior__, __iand__, __ixor__, and __isub__
# class Bitboard(MutableSet[Square]):

class Bitboard(MutableSet[Square]):

    def __init__(self, squares: Optional[Iterable[Square]] = None, bits=0) -> None:
        self.bits = 0
        if squares is not None:
            for s in squares:
                self.bits |= s.bit()
        if bits:
            self.bits = bits

    @staticmethod
    def file(s: Square) -> Bitboard:
        return Bitboard([Square.of(r * 8 + s.x()) for r in range(8)])

    @staticmethod
    def rank(s: Square) -> Bitboard:
        return Bitboard([Square.of(f + s.y() * 8) for f in range(8)])

    def __contains__(self, s: Square) -> bool:
        return bool(s.bit() & self.bits)

    # https://stackoverflow.com/questions/6632188/explicitly-select-items-from-a-list-or-tuple
    # 19.7 usec: [ myBigList[i] for i in [87, 342, 217, 998, 500] ]
    # 20.6 usec: map(myBigList.__getitem__, (87, 342, 217, 998, 500))
    # 22.7 usec: itemgetter(87, 342, 217, 998, 500)(myBigList)
    # 24.6 usec: list( myBigList[i] for i in [87, 342, 217, 998, 500] )
    #
    # alternatives
    # newList = [item for i, item in enumerate(s) if b[i]]
    # [ item for item, flag in zip( s, b ) if flag == 1 ]
    #

    def __iter__(self) -> Iterator[Square]:
        # return [s for s in Square.ALL if s.bit()]
        return (s for s in Square.ALL if s.bit() & self.bits)

    def __len__(self) -> int:
        return bin(self.bits).count("1")

    def add(self, s: Square) -> None:
        self.bits |= s.bit()

    def is_disjoint(self, other: Bitboard) -> bool:
        return self.bits & other.bits == 0

    def discard(self, s: Square) -> None:
        self.bits &= (((1 << 64) - 1) ^ s.bit())

    def remove(self, s: Square) -> None:
        self.discard(s)

    def clone(self) -> Bitboard:
        return Bitboard(self)

    def clear(self) -> None:
        self.bits = 0

    def __ilshift__(self, i: int) -> Bitboard:
        self.bits <<= i
        return self

    def __irshift__(self, i: int) -> Bitboard:
        self.bits >>= i
        return self

    def __ior__(self, other: Bitboard) -> Bitboard:
        self.bits |= other.bits
        return self

    def __ixor__(self, other: Bitboard) -> Bitboard:
        self.bits ^= other.bits
        return self

    def __iand__(self, other: Bitboard) -> Bitboard:
        self.bits &= other.bits
        return self

    def __invert__(self) -> Bitboard:
        return Bitboard(bits=self.bits ^ ((1 << 64) - 1))

    def __and__(self, other: Bitboard) -> Bitboard:
        return Bitboard(bits=self.bits & other.bits)

    def __add__(self, other: Bitboard) -> Bitboard:
        return Bitboard(bits=self.bits | other.bits)

    def __sub__(self, other: Bitboard) -> Bitboard:
        return Bitboard(bits=self.bits & ~other.bits)

    def __or__(self, other: Bitboard) -> Bitboard:
        return Bitboard(bits=self.bits | other.bits)

    def __xor__(self, other: Bitboard) -> Bitboard:
        return Bitboard(bits=self.bits ^ other.bits)

    def __str__(self) -> str:
        return f"{sorted(self, key=lambda x: self.bits)}"

    def __repr__(self) -> str:
        return f"{self.__class__.__name__}({set(self)})"

    def __hash__(self) -> int:
        return self.bits

    @property
    def grid(self) -> str:
        s = f"{self.bits:064b}".replace('0', '.')
        # return "\n".join(list(map(''.join, zip(*[iter(s)]*8)))[::-1]) # sorry: too perl-like
        # return "\n".join([ s[r*8:r*8+8][::-1] for r in range(8) ]) #  not much better!
        return '\n'.join(wrap(s, 8))  # finally!


class B:

    # the below is "upside down". The board has a1 = sq[0] = bit 1.
    SQUARES = [
        a1, b1, c1, d1, e1, f1, g1, h1,
        a2, b2, c2, d2, e2, f2, g2, h2,
        a3, b3, c3, d3, e3, f3, g3, h3,
        a4, b4, c4, d4, e4, f4, g4, h4,
        a5, b5, c5, d5, e5, f5, g5, h5,
        a6, b6, c6, d6, e6, f6, g6, h6,
        a7, b7, c7, d7, e7, f7, g7, h7,
        a8, b8, c8, d8, e8, f8, g8, h8
    ] = [Bitboard([s]) for s in Square.ALL]

    RANKS = [RANK_1, RANK_2, RANK_3, RANK_4, RANK_5, RANK_6, RANK_7,
             RANK_8] = [Bitboard(bits=255 << 8*s) for s in range(8)]
    FILES = [FILE_A, FILE_B, FILE_C, FILE_D, FILE_E, FILE_F, FILE_G, FILE_H] = [
        Bitboard(bits=sum([1 << s*8 for s in range(8)]) << f) for f in range(8)]

    # andy got caught out by operator precedence
    ALL = Bitboard(bits=(1 << 64) - 1)


class S:

    # the below is "upside down". The board has a1 = sq[0] = bit 1.
    SQUARES = [
        a1, b1, c1, d1, e1, f1, g1, h1,
        a2, b2, c2, d2, e2, f2, g2, h2,
        a3, b3, c3, d3, e3, f3, g3, h3,
        a4, b4, c4, d4, e4, f4, g4, h4,
        a5, b5, c5, d5, e5, f5, g5, h5,
        a6, b6, c6, d6, e6, f6, g6, h6,
        a7, b7, c7, d7, e7, f7, g7, h7,
        a8, b8, c8, d8, e8, f8, g8, h8
    ] = Square.ALL


class Board():

    def __init__(self) -> None:
        self._pieces: List[int]
        self._colors: List[int]
        self._turn: Color
        self._castling: str
        self._en_passant: Optional[Square]
        self._fifty_count: int
        self._move_count: int
        self._hash: int
        # self._state: BoardState
        Board._init(self)

    def _init(self,
              grid: str = None,
              turn: Color = 'w',
              castling: str = 'KQkq',
              en_passant: Optional[Square] = None,
              fifty_halfmove_count: int = 0,
              move_number: int = 1) -> None:

        self._pieces = [0, 0, 0, 0, 0, 0]
        self._colors = [0, 0]
        if grid is None:
            r8 = "rnbqkbnr"
            r7 = 'pppppppp'
            grid = '\n'.join([r8, r7] + ['.' * 8] * 4 +
                             [r7.upper(), r8.upper()])
        if grid:
            list = grid.replace('.', ' ').split('\n')
            for s in Square.ALL:
                self.place(s, list[7-s.y()][s.x()])
        self._turn = turn
        self._castling = castling
        self._en_passant = en_passant
        self._fifty_count = fifty_halfmove_count
        self._move_count = move_number
        self._hash = 0
        # self._state = BoardState()

    def clone(self) -> Board:
        b = Board()
        b._init(self.grid, self.turn, self.castling_rights,
                self.en_passant, self.fifty_halfmove_count, self.move_number)
        return b

    def pieces(self, p: Piece) -> Bitboard:
        return Bitboard(bits=self._pieces[Pieces.to_index(p)])

    def piece_on(self, s: Square) -> Piece:
        for bb, p in zip(self._pieces, Pieces.PIECES_BLACK):
            if s.bit() & bb:
                return p if s.bit() & self._colors[1] else p.upper()
        return ' '

    def remove(self, s: Square) -> None:
        for p in Pieces.PIECES:
            self._pieces[p] &= (((1 << 64) - 1) ^ s.bit())
        self._colors[0] &= (((1 << 64) - 1) ^ s.bit())
        self._colors[1] &= (((1 << 64) - 1) ^ s.bit())

    def place(self, s: Square, p: Piece) -> None:
        self.remove(s)
        if p != ' ':
            self._pieces[Pieces.to_index(p.lower())] |= s.bit()
            self._colors[Sides.to_index(Pieces.colour(p))] |= s.bit()

    def __repr__(self) -> str:
        return f"Board({self._pieces}, {self._colors})"

    def __eq__(self, other: object) -> bool:
        return isinstance(other, Board) and self.to_fen() == other.to_fen()

    @property
    def w(self) -> Bitboard:
        return Bitboard(bits=self._colors[Sides.to_index("w")])

    @property
    def b(self) -> Bitboard:
        return Bitboard(bits=self._colors[Sides.to_index("b")])

    @property
    def turn(self) -> Color:
        return self._turn

    @property
    def en_passant(self) -> Optional[Square]:
        return self._en_passant

    # @turn.setter
    # def turn(self, c: Color) -> None:...

    def __getitem__(self, region: Bitboard) -> str:
        pieces = []
        for c in range(2):
            for i in range(len(Pieces.PIECES)):
                piece = Pieces.BOTH_SIDES[c][i]
                pieces.append(
                    piece * Bitwise.pop_count(self._pieces[i] & region.bits & self._colors[c]))
        return ''.join(pieces)

    # def set_piece_on(self, single_square: Bitboard, p: Piece) -> None:...

    @property
    def kings(self) -> Bitboard:
        return Bitboard(bits=self._pieces[Pieces.KING])

    @property
    def queens(self) -> Bitboard:
        return Bitboard(bits=self._pieces[Pieces.QUEEN])

    @property
    def rooks(self) -> Bitboard:
        return Bitboard(bits=self._pieces[Pieces.ROOK])

    @property
    def bishops(self) -> Bitboard:
        return Bitboard(bits=self._pieces[Pieces.BISHOP])

    @property
    def knights(self) -> Bitboard:
        return Bitboard(bits=self._pieces[Pieces.KNIGHT])

    @property
    def pawns(self) -> Bitboard:
        return Bitboard(bits=self._pieces[Pieces.PAWN])

    def moves(self) -> List[Move]:
        return []  # MoveGenBB().legal_moves(self)

    def pseudo_legal_moves(self) -> List[Move]:
        return []  # MoveGenBB().pseudo_legal_moves(self)

    def validate_move(self, m: Move) -> str:
        return ""  # MoveGenBB().illegal_move_reason(self, m)

    @property
    def castling_rights(self) -> str:
        return self._castling

    # @castling_rights.setter
    # def castling_rights(self, castling: str) -> None:...

    @property
    def fifty_halfmove_count(self) -> int:
        return self._fifty_count

    @property
    def move_number(self) -> int:
        return self._move_count

#     def make_move(self, m: Move) -> Board:
#         # b = BoardOfBits()
#         # assert Clock.capture_as("Board.make_move: board ctor")
#         # b._init()
#         # assert Clock.capture_as("Board.make_move: board init")
#         # b._fifty_count = self._fifty_count
#         # b._move_count = self._move_count
#         # b._state = self._state
#         # b._colors = self._colors.copy()
#         # b._pieces = self._pieces.copy()
#         # b._castling = self._castling
#         # assert Clock.capture_as("Board.make_move: copy")
#         # increase clock first as it might get zeroed later in logic
#         self._fifty_count += 1

#         # FIXME: en-passant capture
#         myself_piece = self.piece_on(m.src)
#         piece_index = Pieces.to_index(myself_piece.lower())
#         capture_square = m.dst
#         capture_piece = self.piece_on(capture_square)

#         # check e/p before captures, as it affects the capture square
#         if myself_piece in "Pp" and capture_piece == ' ' and capture_square == self._en_passant:
#             if self._en_passant.rank() == '6':
#                 capture_square = capture_square.from_xy(capture_square.x(), capture_square.y()-1)
#                 capture_piece = self.piece_on(capture_square)
#             elif self._en_passant.rank() == '3':
#                 capture_square = capture_square.from_xy(capture_square.x(), capture_square.y()+1)
#                 capture_piece = self.piece_on(capture_square)
#             else:
#                 raise ValueError(f"Invalid en-passant move: {m}")


#         color_index = Sides.to_index(self._turn)
#         if capture_piece != ' ':
#             captured_piece_index = Pieces.to_index(capture_piece.lower())

#             # remove the captured piece (using opponents color)
#             self._pieces[captured_piece_index] ^= capture_square.bit()
#             self._colors[1 - color_index] ^= capture_square.bit()
#             self._fifty_count = 0


#         # if a piece moves TO (=capture) or FROM the rook squares - appropriate castling rights are lost
#         # if a piece moves FROM the kings squares, both castling rights are lost
#         # possible with a rook x rook capture that both sides lose castling rights

#         # FIXME: when a square is a region!

#         # WHITE side rights
#         if m.src.bit() == B.e1.bits:
#             self._castling = self._castling.replace('KQ', '')
#         elif m.src.bit() == B.a1.bits or m.dst.bit() is B.a1.bits:
#             self._castling = self._castling.replace('Q', '')
#         elif m.src.bit() == B.h1.bits or m.dst.bit() == B.h1.bits:
#             self._castling = self._castling.replace('K', '')

#         # BLACK side rights
#         if m.src.bit() == B.e8.bits:
#             self._castling = self._castling.replace('kq', '')
#         elif m.src.bit() == B.a8.bits or m.dst.bit() == B.a8.bits:
#             self._castling = self._castling.replace('q', '')
#         elif m.src.bit() == B.h8.bits or m.dst.bit() == B.h8.bits:
#             self._castling = self._castling.replace('k', '')

#         if myself_piece in "Kk" and abs(m.src.x() - m.dst.x()) > 1:
#             # WHITE castling
#             if m.src.bit() == B.e1.bits and m.dst.bit() == B.g1.bits:
#                 self._castling = self._castling.replace('K', '')
#                 fromToBits = B.h1.bits ^ B.f1.bits
#                 self._pieces[Pieces.ROOK] ^= fromToBits
#                 self._colors[Sides.WHITE] ^= fromToBits
#             elif m.src.bit() == B.e1.bits and m.dst.bit() == B.c1.bits:
#                 self._castling = self._castling.replace('Q', '')
#                 fromToBits = B.a1.bits ^ B.d1.bits
#                 self._pieces[Pieces.ROOK] ^= fromToBits
#                 self._colors[Sides.WHITE] ^= fromToBits

#             # BLACK castling
#             elif m.src.bit() == B.e8.bits and m.dst.bit() == B.g8.bits:
#                 self._castling = self._castling.replace('k', '')
#                 fromToBits = B.h8.bits ^ B.f8.bits
#                 self._pieces[Pieces.ROOK] ^= fromToBits
#                 self._colors[Sides.BLACK] ^= fromToBits
#             elif m.src.bit() == B.e8.bits and m.dst.bit() == B.c8.bits:
#                 self._castling = self._castling.replace('q', '')
#                 fromToBits = B.a8.bits ^ B.d8.bits
#                 self._pieces[Pieces.ROOK] ^= fromToBits
#                 self._colors[Sides.BLACK] ^= fromToBits


#         # set en_passant square
#         if myself_piece in "pP" and abs(m.src.y() - m.dst.y()) == 2:
#             self._en_passant = Square.from_xy(m.dst.x(), (m.dst.y() + m.src.y()) // 2 )  # half way in between
#         else:
#             self._en_passant = None

#         # clock, move and turn
#         if myself_piece.lower() == 'p':
#             self._fifty_count = 0

#         #promo
#         if m.promotion:
#             # change the pawn into a promo-piece on its src square, before doing the move
#             self._pieces[piece_index] ^= m.src.bit()
#             piece_index = Pieces.to_index(m.promotion.lower())
#             self._pieces[piece_index] ^= m.src.bit()

#         # clear one bit and set another for the move
#         fromToBits = m.src.bit() ^ m.dst.bit()
#         self._pieces[piece_index] ^= fromToBits
#         self._colors[color_index] ^= fromToBits
# #
#         # #
#         # b._occupied ^= fromToBits
#         # update hash


#         if self._turn == 'b':
#             self._move_count += 1
#         self._turn = Sides.opposite(self.turn)
#         return self


    def __hash__(self) -> int:
        return self._hash

    def __str__(self) -> str:
        return self.to_fen()

    @property
    def grid(self) -> str:
        g = '\n'.join(reversed(
            [''.join([self.piece_on(Square.from_xy(x, y)) for x in range(8)])
                for y in range(8)]
        )).replace(' ', '.')
        return g

    def to_fen(self) -> str:
        return self.fen_formatter(self)

    @staticmethod
    def fen_formatter(b: Board, omit_counts=False) -> str:
        fen = b.grid.replace('\n', '/')

        # replace continguous spaces by a count
        for sp in range(8, 0, -1):
            fen = fen.replace('.'*sp, str(sp))

        return fen + f' {b.turn} {b.castling_rights or "-"} {b.en_passant or "-"} {b.fifty_halfmove_count} {b.move_number}'

    @staticmethod
    def parse_fen(fen: str) -> Board:
        """
        Parses a FEN string to create a board. FEN format is detailed at https://en.wikipedia.org/wiki/Forsyth–Edwards_Notation
        """

        words = fen.split()
        if len(words) < 6:
            raise ValueError(
                f'Invalid FEN {fen}: expected at least 5 sections after pieces but found only {len(words)-1}')

        grid = Board._parse_pieces_fen(words[0])
        castling = Board._parse_fen_castling(words[2])
        ep = Board._parse_fen_en_passant(words[3])
        turn, fifty_clock, move_number = Board._parse_fen_turn_and_counts(
            words[1], words[4], words[5])
        board = Board()
        board._init(grid, turn, castling, ep, fifty_clock, move_number)
        return board

    @staticmethod
    def _parse_pieces_fen(fen_part1: str) -> str:
        sqs = ''
        valid_digits = "12345678"
        valid_pieces = "pnbrqkPNBRQK"
        ranks_8_to_1 = fen_part1.split('/')
        if len(ranks_8_to_1) != 8:
            raise ValueError(
                f'Invalid FEN {fen_part1}: Expected 8 ranks in position part but found {len(ranks_8_to_1)}')
        for rank in ranks_8_to_1:
            row = ''
            for p in rank:
                if p in valid_digits:
                    row += ' ' * int(p)
                elif p in valid_pieces:
                    row += p
                else:
                    raise ValueError(
                        f'Invalid FEN {fen_part1} in row of "{rank}" unexpected "{p}"')
            # weve captured all the pieces/squares in this row
            if len(row) != 8:
                raise ValueError(
                    f'Invalid FEN {fen_part1} in row of "{rank}" expected 8 pieces but found {len(row)}')
            sqs += row
        grid = "\n".join([sqs[r*8:r*8 + 8] for r in range(8)])
        return grid

    @staticmethod
    def _parse_fen_turn_and_counts(fen_turn: str, fen_halfmove_clock: str, fen_fullmove_number: str):
        if len(fen_turn) == 1 and (fen_turn == 'b' or fen_turn == 'w'):
            pass
        else:
            raise ValueError(
                f'Invalid FEN: expected w/b turn indicator but found "{fen_turn}"')

        try:
            fifty_clock = int(fen_halfmove_clock)
        except ValueError:
            raise ValueError(
                f'Invalid FEN: expected halfmove clock to be a number not "{fen_halfmove_clock}"')
        try:
            move_number = int(fen_fullmove_number)
        except ValueError:
            raise ValueError(
                f'Invalid FEN: expected fullmove count to be a number not "{fen_fullmove_number}"')
        return (fen_turn, fifty_clock, move_number)

    @staticmethod
    def _parse_fen_castling(fen_castling: str):
        if fen_castling == '-':
            return ""
        if fen_castling not in "KQkq":
            raise ValueError(
                f'Invalid FEN: expected castling indicator but found "{fen_castling}"')
        return fen_castling

    @staticmethod
    def _parse_fen_en_passant(fen_en_passant: str) -> Optional[Square]:
        if fen_en_passant == '-':
            return None
        else:
            return Square.parse(fen_en_passant)


DEFAULT_ODONATA_PARAMS = {
    "Hash": 16,
}


class Odonata:

    def __init__(self, path: str = '', debug: bool = False ) -> None:
        self.process: Optional[subprocess.Popen] = None
        if not path:
            # try and look for Odonata executable
            files = ["./odonata.exe", "./odonata", "./target/release/odonata.exe", "./target/release/odonata" ]
            for f in files:
                if os.path.isfile(f):
                    path = f
                    break
            if not path:
                raise ValueError(f"Unable to find executable in {files}")

        self.debug: bool = debug
        self.process = subprocess.Popen(
            path,
            universal_newlines=True,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE
        )
        self._put("uci")
        self._start_new_game()

        # self._odonata_major_version: int = int(self._read_line().split(" ")[1])


        # self.depth = str(depth)
        self.info: str = ""

        # if parameters is None:
        #     parameters = {}
        # self._parameters = DEFAULT_ODONATA_PARAMS
        # self._parameters.update(parameters)
        # for name, value in list(self._parameters.items()):
        #     self._set_option(name, value)

        self._start_new_game()


    # def get_parameters(self) -> dict:
    #     """Returns current board position.
    #     Returns:
    #         Dictionary of current Odonata engine's parameters.
    #     """
    #     return self._parameters

    def _start_new_game(self) -> None:
        self._put("ucinewgame")
        self.is_ready()
        self.info = ""

    def _put(self, command: str) -> None:
        if not self.process.stdin:
            raise BrokenPipeError()
        if self.debug:
            print("  >", command)
        self.process.stdin.write(f"{command}\n")
        self.process.stdin.flush()

    def _read_line(self) -> str:
        if not self.process.stdout:
            raise BrokenPipeError()
        text = self.process.stdout.readline().strip()
        if self.debug:
            print("<", text)
        return text

    def set_option(self, name: str, value: Any) -> None:
        self._put(f"setoption name {name} value {value}")
        self.is_ready()


    def get_best_move(self, b: Board, depth: Optional[int] = None, millis: Optional[int] = None ) -> Optional[str]:
        """Returns best move with current position on the board in uci notation or None if it's a mate."""

        self._put(f"position fen {b.to_fen()}")
        req = "go movetime 1000"
        if depth:
            req = f"go depth {depth}"
        if millis:
            req = f"go movetime {millis}"    

        result = self._command(req, res = "bestmove")
        return None if result == "0000" else result

    def is_ready(self) -> None:
        self._put("isready")
        while True:
            if self._read_line() == "readyok":
                return

    def _command(self, req, res) -> str:
        self._put(req)
        last_text: str = ""
        while True:
            text = self._read_line()
            splitted_text = text.split(" ")
            if splitted_text[0] == res:
                self.info = last_text
                return splitted_text[1]
            last_text = text
    
    def __del__(self) -> None:
        if self.process:
            self._put("quit")
            self.process.kill()


class Test:

    def test_square(self):
        assert str(Square(63)) == 'h8'
        assert str(Square(0)) == 'a1'
        assert str(Square(1)) == 'b1'
        assert str(Square(8)) == 'a2'
        assert str(Square.of(8)) == 'a2'
        assert str(Square.parse('a2')) == 'a2'
        assert Square.parse('h1').index() == 7
        assert Square.parse('a2').name() == 'a2'
        assert str(Square.from_xy(5, 0)) == 'f1'
        assert Square.parse('b7').rank() == '7'
        assert Square.parse('b7').y() == 6
        assert Square.parse('b7').file() == 'b'
        assert Square.parse('b7').x() == 1

        # bitwise
        assert Square.parse('a1').bit() == 1
        assert Square.parse('h1').bit() == 128
        assert Square.parse('a2').bit() == 256
        s = Square.parse('a2')
        s >>= 3
        assert str(s) == 'd2'
        s <<= 1
        assert str(s) == 'c2'

        assert [Square.is_off_board(
            0, -1), Square.is_off_board(0, 1)] == [True, False]
        assert [Square.is_off_board(
            63, -1), Square.is_off_board(63, 1)] == [False, True]

    def test_bitboard(self):
        assert len(Bitboard(bits=3)) == 2
        assert Bitboard(bits=0) == set()
        a1 = Square.of(0)
        b1 = Square.of(1)
        c1 = Square.of(2)
        d1 = Square.of(3)
        e1 = Square.of(4)
        r = Bitboard()
        r2 = r.clone()
        r.add(b1)
        assert b1 in r
        assert [b1] == [s for s in r]
        r2.add(b1)
        r2.add(c1)
        r2.add(c1)
        assert len(r2) == 2
        assert c1 in r2
        assert not r.is_disjoint(r2)
        assert r & r2 == {b1}
        assert r | r2 == {b1, c1}
        assert r ^ r2 == {c1}
        assert r2 - r == {c1}
        r2.add(d1)
        assert r2 == {b1, c1, d1}
        r2 <<= 1
        assert r2 == {c1, d1, e1}
        r2 >>= 1
        r3 = r2.clone()
        r2.remove(c1)  # no error raised
        assert Bitboard(Square.ALL) == B.ALL
        assert r2 == {b1, d1}
        assert r2 < r3
        assert r2 != r3
        assert r3 > r2
        r2.clear()
        assert len(r2) == 0
        assert str(r3) == "[b1, c1, d1]"
        assert repr(r3) == "Bitboard({b1, c1, d1})"
        assert Bitboard.file(Square.parse('c4')) == {Square.parse(
            s.strip()) for s in 'c1, c2, c3, c4, c5, c6, c7, c8'.split(',')}
        assert Bitboard.rank(Square.parse('c4')) == B.a4 + \
            B.b4 + B.c4 + B.d4 + B.e4 + B.f4 + B.g4 + B.h4
        file_a = Bitboard.file(Square.parse('a1'))
        rank_1 = Bitboard.rank(Square.parse('a1'))
        rook_a1 = file_a
        rook_a1 |= rank_1
        rook_a1 = ~rook_a1
        rook_a1 = ~rook_a1
        assert rook_a1 == file_a | rank_1
        not_b1 = ~Bitboard([b1])
        not_b1.add(b1)
        all = not_b1
        assert len(all) == 64

    def test_board(self):
        board = Board()
        board._init()
        assert board.piece_on(Square.of(1)) == 'N'
        assert board.piece_on(Square.of(63)) == 'r'
        assert board[B.a1] == 'R'
        assert board[B.h1] == 'R'
        assert set(board[B.a1 + B.a2 + B.a3]) == set("RP")
        assert set(board[B.RANK_1]) == set("RRBBNNQK")
        # print(f"{board!r}")
        # print(Stringer().pretty_print(board))

        # assert f"{board:f}" == board.to_fen()
        # # check string formatting is applied
        # assert f"{board:g}" == board.to_grid() + "\nwhite to move"

        b = Board()
        b._init()
        assert b.to_fen() == "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"

        b = Board()
        b._init()
        b = b.clone()
        assert b.to_fen() == "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"

        c = Board.parse_fen(b.to_fen())
        assert b == c
        c.remove(S.a1)
        assert c.piece_on(S.a1) == ' '
        c.place(S.a1, 'Q')
        assert c.piece_on(S.a1) == 'Q'
        assert c[B.a1] == 'Q'

        # parser = Parser(cls)
        # epd = '2rr3k/pp3pp1/1nnqbN1p/3pN3/2pP4/2P3Q1/PPB4P/R4RK1 w - - id "WAC.001";'
        # b = parser.parse_board_epd(epd)
        # f = ColourFlipper().flip_board(b)
        # assert b[c1] == f[c8].swapcase()
        # assert b[a1] == 'R'
        # assert f[g8] == 'k'
        # assert str(b) == str(ColourFlipper().flip_board(ColourFlipper().flip_board(b)))


    def test_odonata(self):
        odo = Odonata()
        odo.is_ready()
        board = Board.parse_fen("r6k/8/8/8/8/8/8/R6K w - - 0 30")
        bm = odo.get_best_move(board, millis=200)
        assert bm == "a1a8"





def main():
    test = Test()
    test.test_square()
    test.test_bitboard()
    test.test_board()
    test.test_odonata()

    b = Board()
    print(f'''

board as a FEN string {b.to_fen()}    

board as a grid 

{b.grid}    

knight squares {b.knights}
    
white knigt squares {b.knights & b.w}
    
as a grid 

{(b.knights & b.w).grid}

how many white pawns {len(b.pawns & b.w)}
    ''')






if __name__ == "__main__":
    main()
