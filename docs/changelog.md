# Changelog

Milestones ('m') are bundled into Releases.  

# Unreleased
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

