<img src="odonata-blue.png" width=150 />


# Odonata
A chess engine written in Rust. 

Odonata was created by [Andy Watkins](https://github.com/akanalytics), with [Mark Raistrick](https://github.com/raistrma) assisting with designs and discussion. Bugs in the Rust code will certainly be mine though - Andy ;-)

My lockdown hobby was writing a chess engine, and learning Python and Rust in the process. I started Decemeber 2020. Python and Rust are very different from Java, which I had programmed maybe 10 years previous. Certainly my first efforts at Rust are not very clean, clever or idiomatic, but the code improves as I revisit areas to build improvements.

I would describe Odonata as original code, with some original ideas, but inspired by other libraries, engines, articles and blogs. 

Andy

## Downloads
Binaries for Linux, Windows and Mac can be found at [Downloads](https://github.com/akanalytics/odonata/releases) along with a changelog detailing the release. Please pick the "modern" binary for best prformance, unless you have a very old PC.

## Challenge Odonata online
You can play Odonata online at [Lichess](https://lichess.org/@/odonata-bot). 
Odonata-Bot only plays blitz, bullet and rapid games right now, as she's running on my media centre, and Netflix stutters if she's thinking too hard...

## Building from source
See [compilation instructions](compilation.md) for compiling Odonata from its Rust source code. 

## Elo Ratings and games

* [CCRL 40/15](https://computerchess.org.uk/ccrl/404/cgi/compare_engines.cgi?family=Odonata) 
* [CCRL Blitz](https://computerchess.org.uk/ccrl/404/cgi/compare_engines.cgi?family=Odonata)
* [Lichess](https://lichess.org/@/odonata-bot) 
* [CEGT](http://www.cegt.net/40_4_Ratinglist/40_4_single/1874.html) 

## Engine options in the UCI interface
```
option name Threads type spin default 1 min 1 max 16
option name Hash type spin default 8 min 0 max 4000
option name MultiPV type spin default 1 min 1 max 64
option name Debug_Log_File type string default ""
option name UCI_AnalyseMode type check default false
option name Analyse_Mode type check default false
option name UCI_Opponent type string default ""
option name Ponder type check default false
option name OwnBook type check default false
option name Book File type string default ""
option name Best Book Move type check default true
option name Clear Hash type button
```

The following non-standard uci commands are also supported
```
'compiler' 
show the settings used to compile odonata 

'd' or 'board'
display the board

'eval'
show the current evaluation 

'perft N'
calculate perft for depth N
```

Additionally, a series of uci commands can be executed directly from the command line  using the "uci" subcommand. For example 
```
odonata.exe uci "perft 6; board; go depth 6"
```

## Chess position evaluation, training and search tuning
Odonata has both a hand-crafted evaluation (HCE) and a [NNUE](https://en.wikipedia.org/w/index.php?title=NNUE) style neural-network. The HCE was tuned using the [L-BFGS](https://en.wikipedia.org/wiki/Limited-memory_BFGS) algorithm and a tuner based upon [ArgMin](https://argmin-rs.org/). The training data was generated from self-play games and from Odonata vs internet opponent games on [Lichess](https://lichess.org/@/odonata-bot). 

The NNUE was trained using 400 million positions from self-play games, and evaluations using Odonata's own HCE. The trainer is a Rust based, self-written CPU trainer using mini-batch AdamW, stepped learning rate, Normal/He-style weight initilisation, Rayon for multi-threading, and Rust's auto-vectorization for SIMD. It's not as fast as a GPU trainer, but manages 3 MM positions/second so a training run is a few hours. 

The initial proof-of-concept 128-node neural-net was trained using an old version of [Bullet trainer](https://github.com/jw1912/bullet), which I can recommend, and which came to my attention via [Leorik](https://github.com/lithander/Leorik).

Odonata's NNUE trainer uses NNUE incremental evaluation during training (necessitating inputs being PGNs rather than unrelated positions) and a bespoke SIMD vector implementation. Once it's tidy, I'll open source it (and the associated pgn writer/parser).

The network is currently a completely vanilla 768 -> 512x2 -> 1 perspective network, whilst I experiment on more exotic approaches.

Odonata's playing strength is assessed using a self-written chess tournament engine, similar to the command-line [cutechess](https://github.com/cutechess/cutechess). It features the ability to run using either "total nodes" or cpu-instruction counts rather than total game time, enabling accurate and fair competitions even when the PC is simultaneously being used for neural-network training, and under load. The ployglot opening book parsing was implemented from the algorithm detailed at http://hgm.nubati.net/book_format.html .

The search algorithm is alpha-beta based, with many of the pruning techniques detailed on the [Chess Programming Wiki](http://www.chessprogramming.org) (null move, late move reduction etc). Tuning of search parameters  was originally performed using [CLOP](https://www.remi-coulom.fr/CLOP/) but now uses a self-written SPSA solver that relies on a "continuous" proxy outcome rather than a discrete WDL result - this speeds convergence, allowing more frequent re-tuning. Many of the search parameters have yet to be quantized though, which possibly slows down the engine's search somewhat.


## Credits
Thanks to my sons - Freddie, Oscar and Hector - for assistance with graphics, some design ideas and testing the engine play; Gabor Szots, Graham Banks and others in the CCRL team for arranging competitive testing, and [Mark Raistrick](https://github.com/raistrma) for chess discussions, design input and much more.

There are many chess libraries, engines and blogs out there. I've listed some at [Credits and Links](credits.md). My apologies if I have not mentioned your project explicitly by name...


## License
The software licence is [AGPL-3](../license.txt), though libraries used are MIT licensed. Trial or experimental versions may lag in terms of source code publication, and the author reserves the right to release versions of the software which are not open source. The intention though, is to give back, and share with the community. Other libraries are listed in the Cargo.toml, with licenses available on Rust's https://crates.io .




