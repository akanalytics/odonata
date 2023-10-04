<img src="odonata-blue.png" width=150 />


# Odonata
A chess engine written in Rust. 

Odonata is designed by [Andy Watkins](https://github.com/akanalytics) and [Mark Raistrick](https://github.com/raistrma) <br> Bugs in the Rust code will certainly be mine though - Andy ;-)

My lockdown hobby was writing a chess engine, and learning Python and Rust in the process. I started Decemeber 2020. Python and Rust are very different from Java, which I had programmed maybe 10 years previous. Certainly my first efforts at Rust are not very clean, clever or idiomatic, but the code improves as I revisit areas to build improvements.

I would describe Odonata as original code but inspired by other libraries, engines, articles and blogs. There are some utilities code which are more than inspired and hopefully these have been explicitly credited. In particular I use Hoarfrost's magic bitboards for benchmarking (but not movegen), Stockfish wrapper (by zhelyabuzhsky) as a basis for python invocation and JsonRpc (riga) for, well, Json Rpc in the Python interface.   

-Andy

## Links
* [Releases](https://github.com/akanalytics/odonata/releases) - download Windows and Linux executables and see chagelogs.

* Odonata is running as a bot on [Lichess](https://lichess.org/@/odonata-bot). You can challenge her, or see her rankings and games.  <br>
Odonata only plays blitz and bullet games right now, as she's running on my media centre, and Netflix stutters if she's thinking too hard...

* [Compilation](compilation.md) - compile Odonata from source.

* Odonata has a baby [Python](python.md) interface, based on the Stockfish interface by Ilya Zhelyabuzhsky. The interface will sure evolve, and perhaps migrate to a Python extension. Sub-alpha quality, and not actively supported.


* See the CCRL Blitz ratings here https://ccrl.chessdom.com/ccrl/404/cgi/compare_engines.cgi?family=Odonata

## Engine options in the UCI interface
```
option name Threads type spin default 1 min 1 max 16
option name MultiPV type spin default 1 min 1 max 64
option name Hash type spin default 8 min 0 max 4000
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

## Credits
Thanks to my sons - Freddie, Oscar and Hector - for assistance with graphics, some design ideas and testing the engine play; Gabor Szots and the CCRL team for arranging blitz competitive testing; and the many chess libraries, engines and blogs out there. My apologies if I have not mentioned your project explicitly by name...

Description | Link | Comment  
----------- | ---- | -------
Chess Programming Wiki | https://www.chessprogramming.org/ | Invaluable.
Pure Python Chess Library | https://github.com/niklasf/python-chess | Expertly crafted. 
Fast chess move generation | https://github.com/jordanbray/chess | Fast!
Shakmaty - Rust chess library | https://github.com/niklasf/shakmaty | Looks powerful in terms of variants
Rust Chess engine | https://github.com/vinc/littlewing | For making me appreciate ASCII art...
Chesss GUI for UCI engines | http://www.playwitharena.de/ | Have grown to love
CCRL - Computer Chess Ratings Lists | https://www.computerchess.org.uk/ccrl/404/ | Fascinating
Stockfish python interface | https://pypi.org/project/stockfish/ | By Ilya Zhelyabuzhsky. Sweet.
Stockfish | https://stockfishchess.org/ | A giant.
Ethereal / Andrew Grant | - | Helpful PDF on 'texel' style tuning
CLOP / Rémi Coulom | https://www.remi-coulom.fr/CLOP/ | Non-eval parameter tuning
Alcibiades |https://github.com/epandurski/alcibiades | Worthy! Inspired Odonata's q-search 
Hoarfrost | https://github.com/Ravenslofty/Hoarfrost | Used for magic bitboard benchmarking


## License
The software licence is [AGPL-3](../license.txt), though libraries used are MIT licensed. Trial or experimental versions may lag in terms of source code publication, and the author reserves the right to release versions of the software which are not open source. The intention though, is to give back and share with the community which has helped me learn. Other libraries are listed in the Cargo.toml, with licenses availale on Rust's https://crates.io .




