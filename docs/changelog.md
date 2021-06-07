# Changelog

## 0.3.18 (not yet released)
- added splash with compiler features / optimizations

## 0.3.17
- pondering 
- fixed bug with pv lenght on uci info
- Python: added move_attributes and attacks_from
- added mate_in_4 tests
- make_moves taking a variation 
- removed pseudo_legal_move logic
- replace MoveList vec with array 
- added magic bitboards for benchmarking (and completeness). Not my own code. Just a rust "port" of C++.
- cargo features to enable release-like configs
- disable piece mobility for low plys 

## 0.3.16 (internal release only)
- Python 'interface' - really just some extensions to uci
- Python sample code

## 0.3.15
- Bug fix: illegal moves made when cut node stored at root of tt
- UCI centi-pawn scoring pov finally fixed (from egine pov not white's)
- UCI mate in X reporting also finally fixed
- Some temporary changes to facilitate killer move investigations, and move ordering
- Structural changes to faciliate futility pruning

## 0.3.14
- Added clap library for command line 

## 0.3.13
- Pull pv from transposition table
- 

