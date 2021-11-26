# Changelog


# Release 0.5.0 
- Elo improved by 200+ in self-play vs release 0.4.0 (expected 2500+)
- Late move reductions, razoring, check extensions, time control tuning
- Greatly simplified UCI options
- Rook behind passer eval adjustment
- Phase adjusted logistic k-steepness incorporated into texel-style tuning
- Some items built but not tuned, finished or adding Elo yet. Notably
    - Aspiration searches
    - (Alternate/direct addressed) bucket hashing and aligned hash
    - MultiPV (experimental)
    - TOML configuration/profile files
    - Tri-valued evaluation weights


# Release 0.4.0 
- Evaluation of king tropism, passed pawn on 7th
- Bishop and knight outposts
- Rook batteries
- History table
- Xray attacks considered in SEE
- Texel tuning uses Rayon library for parallel evals (138 M/sec)
- Check for insufficient material in quiesensce
- CLOP tuning of non-eval params
- Support for "searchmoves" and "info hashfull" in UCI interface
- ELO in the region of 2300

# Release 0.3.37
- Texel style tuning for piece values
- Pre-compiled binaries should run on more CPUs (not fully tested)
- Tuning tooling
- Recursive null move pruning
- Material balance, move to compiler version nightly-2021-08-04
- Fix repetition detection bug
- Check extensions and late move reduction
- Piece square table for rook end game, pawn pst corrected
- Futility margin adjustment, PVS re-enabled


# Release 0.3.26

Milestones ('m') are bundled into Releases.  

### Highlights
- Futility pruning
- Null move pruning
- Multi-threading (still experimental)
- Command-line switches for benchmarking and perft (use --help)
- Bishop pair bonus
- Json-rpc for python interface (still experimental)
- Lockless transposition table

#### m 0.3.26
- Quiet move ordering
- 
#### m 0.3.25
- crude futility pruning

#### m 0.3.24
- pre-calculate pawn span bitboards
- tempo used in eval
- tt probe at leaf nodes 

#### m 0.3.23
- use json-rpc to communicate from python
- add python methods for retreiving epd positions - wac, bk test suites etc
- added checkmate and draw detenction to python interface
- added --search as a command-line switch for benchmarking 
- move to cargo nightly-2021-07-06
- added nodes as search constraint on python interface

#### m 0.3.22
- bishop pair bonus in eval
- avoid null pruning near frontier 
- permit different strategies for null move pruning depth reduction
- for repeated positions use tt only for move ordering (configurable) 
- tweaked a few eval parameters
- remove lazy init of logging when features=fast

#### m 0.3.21
- null move pruning
- pickup UCI config options from env vars eg EXPORT odonata_nmp_enabled=false
- statically linked to musl for linux and statically link for windows
- all uci options made consistently lowercase

#### m 0.3.20
- move ordering improved for promos 
- use hash move to help ordering even if draft implies score bounds ignored
- more but smaller move sorts, or partial sorts  


#### m 0.3.19
- multi-threaded through UCI interface
- 'Threads' setoption supported via UCI

#### m 0.3.18
- added splash with compiler features / optimizations
- multi-threading (internally for tests)
- lockless transposition table


# Release 0.3.17
#### m 0.3.17
- pondering 
- fixed bug with pv length on uci info
- Python: added move_attributes and attacks_from
- added mate_in_4 tests
- make_moves taking a variation 
- removed pseudo_legal_move logic
- replace MoveList vec with array 
- added magic bitboards for benchmarking (and completeness). Not my own code. Just a rust "port" of C++.
- cargo features to enable release-like configs
- disable piece mobility for low plys 

#### m 0.3.16
- Python 'interface' - really just some extensions to uci
- Python sample code

#### m 0.3.15
- Bug fix: illegal moves made when cut node stored at root of tt
- UCI centi-pawn scoring pov finally fixed (from egine pov not white's)
- UCI mate in X reporting also finally fixed
- Some temporary changes to facilitate killer move investigations, and move ordering
- Structural changes to faciliate futility pruning

#### m 0.3.14
- Added clap library for command line 

#### m 0.3.13
- Pull pv from transposition table


# Release 0.3.12
- github release
- uci interface

