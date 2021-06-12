# Todo items

## General
- [ ] x-ray in see algorithm
- [ ] Board/Position/Game refactor
- [ ]  tt table. 
  - use xor'ing trck for duplicates (https://binarydebt.wordpress.com/2013/09/29/lockless-transposition-tables/). 
  - check legalality on probe.
  - pluggable replacement strats
  - packed structs  like https://docs.rs/packed_struct/0.5.0/packed_struct/ (mv=16, node=2, score=14 [16384], draft=8, generation=8 ) leaving 16 bits for hash? Prob not enough. So basically needs to be Atomic128s or brace of Atomic64s
  - leaves enough room to pack extra collision detection or bucket fields. 
- [ ] "Local" caching
- [ ] Game / pgn export
- [ ] Branching factor bug
- [ ] Killer moves
- [ ] Rust "template specialization" / branching
- [ ] Analyses WAC.259 q search which explodes to 50m posn 
  - r1bq1rk1/ppp2ppp/2np4/2bN1PN1/2B1P3/3p4/PPP2nPP/R1BQ1K1R w - - 0 1
- [ ] REWORK NODE LOGIC. Esp when to write PV and IDS
- [ ] outcome cache on eval. Qsearch eval cache and tt?
- [ ] Pawn chains fixed 
- [ ] Null move pruning
- [ ] Fix board ==
- [ ] E/P when not relelvant and tt
- [ ] Prime the startup on uci with a search


## Todo multi-threading
- [ ] Task control to potentially use channels rather than atomics (stop/ponderhit so far)
- [ ] "Stats" shared class to be split up into components
- [ ] Aggregate class to collect stats from all thread local components
- [ ] Thread Pool - not needed in first instance
- [ ] tt -lets mutex it first before the atomic refactoring above (some already atomic)
- [ ] Pad the read-only data (lookup tables) so CPU cached without fear of neighbouring write (tiny mem I recall)
- [ ] Command line or num_cpus (default?) switch for threading
- [ ] Overall design. Pull TaskControl outside of Algo. Engine = Algo+Task control = threaded algorithm. Test code stays (largely)as is. 

## UCI
- [ ] multi PV
- [ ] Searchmoves in UCI
- [ ] Clear hash
- [ ] wdl as in  UCI_ShowWDL from Leela
- [ ] engine wrapper

## Lichess
- [ ] use https://berserk.readthedocs.io/en/master/
- [ ] challenege other bots
- [ ] record stats



# Done
- [X] Logging
- [X] Fix logging via switches
- [X] PV extraction  
- [X] Non linear piece mobility
- [X] Draw repeats with TT
- [X] Config. Like debug. Pass in &ref for defaults, naming etc
- [X] Cancel soon after thread start
- [X] Occasional PV screwup
- [X] Aging tt



- Bug fixes
  - stop IDS if nodes not increasing or max depth < depth>


- Optimization backlog
  - [x] tt aging
  - tt eval caching
  -  board phase
  -  eval position cahing or incremental
  -  [x] 20% has legal moves
  -  [x] indexing
  -  
  - [x] default() and once_cell on Hasher
  - make_move - and move x 2
  - count occurrences of different moves
  - copymake vs undo move 
  - bitmask of moves - bitboard - allows many sq comparisons or sqs class
  - [x] avoid hash_move on is_check move
  - BoardBug set methods call hash_board


  - pgn tags [%eval 3.2,15] [%clk 2:34:56.890]
  - export RUST_TEST_THREADS=1;


# CPW todo list

- Obligatory
  - Futility pruning
  - Null move pruning
  - [x] Transposition Table
  - [x] Iterative Deepening
  - Aspiration Windows

- Selectivity
  - [x] Quiescence Algo
  - [x] static exchange evaluation < 0
  - delta pruning
  - [x] standing pat

- Selectivity
  - Mate Algo

- Scout and Friends
  - Scout
  - NegaScout
  - Principal Variation Algo (=+30%?)

- Alpha-Beta goes Best-First
  - NegaC*
  - MTD(f)
  - Alpha-Beta Conspiracy Algo



- [x] parsing algebraic notation
- [x] game/match
- [x] lichess bot
- [x] producing algebraic notation
- [x] pgn export
- [x] uci
- [x] improve stats collection
- [x] EPD format - catalog and test puzzles
- [x] iterative deepening
  - [x] abandon if not time for another ply
  - [ ] found mate - skip deepening
  - [x] better carry forward of moves 
  - [ ] windowed search
    

- https://www.chessprogramming.org/Eigenmann_Rapid_Engine_Test    
- [ ] flip vertical and test casezs
- [X] Q-search
- [X] Move orderer
- [ ] image generation
- [ ] python interface
- [X] ~~discord bot~~ use Lichess instead


- EPD processing
  - [ ] decide on eager or lazy validation
  - [ ] decide on how to set stuff
  - [ ] Pul from PGN quiet positions (no capture, promo or end or check)
  - [ ] Process EPD files in bulk
    - [ ] clean/validate
    - [ ] evaluate
    - [ ] is quiet or replace with  I replace every position with the leaf of their q-search
    - [ ] material score (plus other operations eg check, draw, outcome, #legal moves, )


  

- Interface Rust + Python
  - [ ] Search rename/refactor
  - [ ] Have search return position
  - [ ] add perft attribute. Elsewhere called D?
  - [ ] how to handle stats/gird like attributes
  - [ ] Have eval set position data
  - [ ] 

- Specific problems
  - [ ] Sort out logging
  - [ ] Under promotion - use Mark's extra attribute approach? 
  - [X] Better draw detection
  - [X] ~~Mitochondria once_cell~~ Used lazy from once_cell instead

- admin
  - [ ] kubernetes as suggested by Tom
  - [ ] AWS Lambdas suggested by Si



# Rust 

All algos have an "off"
Writing methods are not mut and atomic/threadsafe

Has a shared parent ref, so can access everything else


Threads per ab-search. So spawn and die on every ply of IDS. Merge stats after each ply of IDS.
Decide on new IDSlevel before respawing threads.
Shared interior TT and node count.
Shared killer moves.
Seperate stats per thread.
Seperate time limits/controls.
Separate PV tables.


things on board can be incrementally calculated on do / do&undo / never

Repetitions  do&undo
Checkers     never
Hash         do and sometimes undo
Castling     do and sometimes undo
Legal moves  never





Eval
QSearch
AlphaBeta
NullMove
MoveOrdering
IDS
PVS
Stats
Analysis/PV
TranspositionTable
KillerMoves
TimeController/SearchLimit

Threading
Async







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

board.perform(mv)
board.try(mv)
board.play(mv)
board.play_move(mv)

board.apply(mv)
board.execute(mv)
board.undo(mv)




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

evaluate a board, position, set of positions, and a tree of positions
return a score, set of scores, explanation of score

ef.eval(board)
ef.eval(positions)
ef.eval(positions)
ef = functions.material.pawn(3).rook(5)
ef.material.evaluate(board)
ef.position.evaluate(board)
ef.outcome.evaluate(board)

```
f1 = odo.eval.new()
f1.material(p=300, b=400, n=700)
f1.position(endgame)
f1.squares.midgame.pawns[RANK_7] = 50
f1.squares.endgame.knights[a1] = -40
f1.squares.midgame.bishops[a1] = -40

f1.squares.endgame.king[RANK_1] = [1, 20, 30, 40, 40, 30, 20, 1]

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

algo.search(board).best_move

algo.search(board)
algo.stats
algo.analysis



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

engine=uci_engine
engine.settings
engine.configure(props)
engine.search(board)

engine=algo as algo *is* am engine
engine=human
engine=random
engine=opening_book(engine)
engine=remote ?


for mv in board.moves
   score = ef.eval(board + mv)
   if score > best_score:
    (best_mv, best_score) = mv, score
return best_mv

best_mv, best_score = board.search(minmax(1), ef)

position.pv = ?
position.bm = ?
position.perf = ?
position = algo.search(board)


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
odo.positions.collection("wac")[1]
odo.positions.starting_position()
odo.positions.chess960.get(3)
odo.positions.chess960.random()


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





