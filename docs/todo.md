# Todo items


- [x] parsing algebraic notation
- [x] game/match
- [x] lichess bot
- [x] producing algebraic notation
- [x] pgn export
- [x] uci
- [x] iterative deepening
  - [ ] found mate - skip deepening
  - [ ] better carry forward of moves
  - [ ] abandon if not time for another ply
  - [ ] windowed search
    
- [ ] image generation
- [ ] python interface
- [ ] discord bot




# Python Interface

The python interface is a work in progress. 



```board.move("a1a3").move("a3a4")
board['a1'] == "P"
if 'p' in board['a']
if 'R' in board['18']
if 'R' in board['1', '8']

s = square(63)
if 'R' in board[square]

board['a1'] = 'R'

board += "a1a3"
board.move('a1a3.e7e5.a2a4')
board.

board.is_legal_move('a2a4')
board.is_statemate()
board.is_checkmate()
board.antidiag("a3")
board.rank("a3")
board.file("a3")
board.file('a')

board.undo_move()
board.set_name()
board.name()

b = Parser().parse_board_epd("8/p7/8/8/8/8/P7/8 w - - 'one pawn each'")
boards['wac1']






?? board as keys or dict or values


assert board.move('a2a4.b2b4') = board.move('a2a4.b2b4')
assert board.is_legal_move('a2') == False
assert board.is_valid_move('a2') == True

construct boards

board = new BoardOfBits('a1:k a3:K a4:R')
board = new RustBoard('a1:k a3:K a4:R')
board = new PythonChessBoard('a1:k a3:K a4:R')
board.from_fen('pppp')
board.end_game("kKR")
board.wac("1")

## Use list things for game
board = game.at(34)
board = game.end()
board = game.start()

Parser().parse_epd(sdfsfsfs)
b = Parser().fen(dfsdfsfs)
board.convert_to( BoardOfBits() )


board.white.knight_moves

board.white.moves





empty = bl.board()
board = bl.board.starting_pos
board.w.knights.moves == a3 + a4 + a5
board.w.knights[0].moves == a3 + a4
print(board.w.knights[0].moves)

>> a3 + a4

a3 in board.w.knights[0].moves

board.move(a2, a4)
print(board)

```
## Display

```
bl.display=color
bl.display=coords
bl.display.progress = True

print(board)

print(board.occupied)
X X X X X
X X X
(using display from above)


print(board.w)

checks_with_white_knights = w.knights.moves & b.king.knight_moves

legal_moves = w.moves
```


## Eval 

```
f1 = bl.eval.new()
f1.material(p=300, b=400, n=700)
f1.position(endgame)
f1.squares.midgame.pawns[RANK_7] = 50
f1.squares.endgame.knights[a1] = -40
f1.squares.midgame.bishops[a1] = -40

f1.squares.endgame.king[RANK_1] = [1, 20, 30, 40, 40, 30, 20, 1]


 
odo.positions.collection("wac")[1]
odo.positions.starting_position()
odo.positions.chess960.get(3)
odo.positions.chess960.random()


odo.eval.evaluate(board).total_score
f1 = odo.eval.new()
f2 = odo.eval.new()
f1.evaluate(board).total_score

score = f1.evaluate(board)
score.total_score
score3 = score1 + score2
score1 = Score.max(score1 + score2)
```


## Endgame

```
for m in legal_moves:
    f1.evaluate(m)
    f1.evaluate(board + m)
    score1 = odo.eval.evaluate(board + m)
    print(score1)
'''
        w     b  total
pawns   3     5   -200
bishops 1     5   -400
total   -     -   1100
'''
print(score1.total)
print(score1 + score2)
'''
             w     b  total
pawns        3     5   -200
bishops      1     5   -400
passed pawns 4     0     50
total        -     -   1100
'''
```

## Search 

```
scores = odo.search.alphabeta_multipv(board, f1)

scores.pvs[]
scores.best.pv
scores.best.total_score

score  = scores[move]
algo1 = odo.algo.new()
algo1.depth = 5
algo1.quesence=True
algo1.eval.white = f1
algo1.eval.both = f2

score = algo1.search(board)
scores = algo1.search_multi(board)
score = algo1.search_async(board)
algo1.stop()
algo1.stats.nodes
algo1.stats.branching_factor





algo = bl.algo.minmax(ply=3, qiesense=True, max_depth=6)
analysis = algo.apply(board, eval1)

board.play_move(algo)

algo_ab = bl.algo.alphabeta( ply=3, qiesense=True, max_depth=6, config={})
analysis2 = algo.search(board, eval1, time="20s")

score  = analysis[move].score

analysis.max_depth
analysis.cut_nodes
analysis.nodes_per_second
analysis.pv
analysis[pv[0]][pv[1]].nodes ??
```


## Catalog 

```
board = positions_lib2.startpos()

board = bc.positions.board_class = BlunderBoard
board = bc.positions.create(dict, castling, en_passant, fifty_clock, move_number)
board = bc.positions.empty()
board = bc.positions.start960[4]
board = bc.positions.startpos()
board = bc.positions.from_fen("PPPP/etc")

board = bc.positions.perft.compare(depth, board1, board2)
board = bc.positions.perft.benchmark(depth, board)
board = bc.positions.perft.counts()

board = bc.positions.bratko_kopec[1]
board = bc.positions.endgames.krr_kbn[1]
board = bc.positions.endgames.kpp_k[1]
board = bc.positions.from_fen("")
board = bc.board(class, dict, castling, en_passant, fifty_clock, move_number)



board.pieces_on(board.w) == "PPPPNBBRQK"

board.contents(board.w)).count('P') == 4

Counter(board.pieces_on(board.w))['P'] == 4

board.contents(board.w)).count('P') == 4

board[:] = "PPPPPNBRRK"
board.w[:] = "PPPPPNBRRK"  ?

board.w[:] = "PPPPPNBRRK"  ?

board[a1] = "R"
board[a1,a2] = "RN"
board[RANK_2] = "P" * 8
board[a1] in "Pp"
if not board[a1]
board[a1] = ''

if ["k", "r"] < board.w.knights.moves.moves[:]:
if "k" in board.w.knights.moves:
movelist is both a region and a dict
board.b.defended

board.b & board.knights & board.b.attacks
bc.eval.passed_pawns(board).count()
bc.eval.unprotected_pawns(board)


immutable

a4.moves ??? but from where
a4.knight_moves
board.w.knights.knight_moves == a3 + a4 + a5
board.w.knights[0].knight_moves == a3 + a4
print(board.w.knights[0].moves)

>> a3 + a4

a3 in board.w.knights[0].moves


board.w & other_board.b ??

board.turn

board.proponent.knights
board.opponent.knights
board.mover.knights
board.waiter.knights
board.active_player.knights
board.inactive_player.knights
board.is_checkmate
board.is_stalemate
board.in_check
board.resign
board.record_to
board.record_to



board.move(a2, a4)
print(board)

bl.display=color
bl.display=coords
bl.display.progress = True

print(board)

print(board.occupied)
X X X X X
X X X
(using display from above)


print(board.w)

checks_with_white_knights = board.w.knights.moves & board.b.king.knight_moves

legal_moves = w.moves
```

## Config

config = Config()
print(config)
>>> search.threads: default: 4            range: 1..100
>>> search.algo:    default: alphabeta    choices: alphabeta|minmax|mdf
config["search.threads"] = 5
config["search.algo"] = "alphabeta"

eval1 = od.eval.new()
eval1.configure(config)
analysis = algo.apply(board, eval1)





