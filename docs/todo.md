# Todo items
[ ] Quick check test by square cache cache[kg.sq - to.sq]
[ ] NMP with 1 minor piece
[ ] MNP with static eval > beta


## Bugs
- [ ] Sort haswell architecure - use AMD64 microarchitecture category
- [ ] bug on ponder? and timeouts
- [ ] need to treat insufficient material as score=0 not end of game
- [ ] 8/8/1K6/4k3/6p1/8/4N3/8 b - problem of draw


## General
- [ ] PV table tidy, truncated PV, score POV?, big test of score vs search
- [ ] Avoid PV for threading and multi pv search
- [ ] debugging info for static eval (template) - json?
- [ ] position features
- [ ] optimise check of repetitions within 3 of ireversible move
- [X ] PV on q search and eval
- [ ] serde as json struct
- [X] triangle stuff
- [ ] NPS accross all threads
- [ ] flamer?
- [ ] x-ray in see algorithm
- [ ] Board/Position/Game refactor
- [ ] tt table. 
  [x] use xor'ing trck for duplicates (https://binarydebt.wordpress.com/2013/09/29/lockless-transposition-tables/). 
  - check legalality on probe.
  - pluggable replacement strats
  - packed structs  like https://docs.rs/packed_struct/0.5.0/packed_struct/ (mv=16, node=2, score=14 [16384], draft=8, generation=8 ) leaving 16 bits for hash? Prob not enough. So basically needs to be Atomic128s or brace of Atomic64s
  - leaves enough room to pack extra collision detection or bucket fields. 
- [ ] Game / pgn export
- [ ] Branching factor bug
- [ ] Killer moves
- [ ] Rust "template specialization" / branching
- [ ] Analyses WAC.259 q search which explodes to 50m posn 
  - r1bq1rk1/ppp2ppp/2np4/2bN1PN1/2B1P3/3p4/PPP2nPP/R1BQ1K1R w - - 0 1
- [ ] REWORK NODE LOGIC. Esp when to write PV and IDS
- [ ] outcome cache on eval. Qsearch eval cache and tt?
- [ ] Pawn chains fixed 
- [ ] Fix board ==
- [ ] E/P when not relelvant and tt
- [ ] Prime the startup on uci with a search
- [ ] EPD modelling like https://docs.rs/http/0.1.5/src/http/extensions.rs.html#19-130


## Todo multi-threading
- [ ] Task control to potentially use channels rather than atomics (stop/ponderhit so far)
- [ ] "Stats" shared class to be split up into components
- [ ] Aggregate class to collect stats from all thread local components
- [ ] Thread Pool - not needed in first instance
- [ ] Pad the read-only data (lookup tables) so CPU cached without fear of neighbouring write (tiny mem I recall)
- [x] Command line or num_cpus (default?) switch for threading
- [ ] Overall design. Pull TaskControl outside of Algo. Engine = Algo+Task control = threaded algorithm. Test code stays (largely)as is. 

## UCI
- [ ] multi PV
- [ ] Searchmoves in UCI
- [ ] Clear hash
- [ ] wdl as in  UCI_ShowWDL from Leela

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
- [x] tt -lets mutex it first before the atomic refactoring above (some already atomic)
- [x] engine wrapper
- [X] Null move pruning
- [x] "Local" caching
- [X] bug on mv 75draws



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
  - [x] Futility pruning
  - [x] Null move pruning
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







