<img src="https://github.com/akanalytics/odonata/blob/main/docs/odonata-blue.png" width=150 />


# Odonata
A simple chess engine written in Rust.

## Background
My lockdown hobby was writing a chess engine, and learning Python and Rust in the process. Currently Odonata is available to play as a bot on Lichess, though Odonata only plays blitz and bullet games right now, as she's running on my media centre, and Netflix stutters if she's thinking too hard...

I started Decemeber 2020. Python and Rust are very different from Java, which I had programmed maybe 10 years previous. Certainly my first efforts at Rust are not very clean, clever or idiomatic, but the code improves as I revist areas to build improvements.

I'm aware that this is now yet another chess engine, with yet another set of Python bindings. In part this is because the project was intended as a learning exercise, but also there did not appear to be a Python native extension available. If any library owners, either Rust or Python, have or plan native extensions and want to chat about how I can avoid duplication or rationize things in terms of Python chess extension modules, please email (andy at webcalculator dot com)!

I would describe Odonata as original code but inspired by other libraries, engines, articles and blogs. There are some utilities code which are more than inspired and hopefully these have been explicitly credited. In particular I use Hoarfrost's magic bitboards for benchmarking (but not movegen), Stockfish wrapper (by zhelyabuzhsky) as a basis for python invocation and JsonRpc (riga) for, well, Json Rpc in the Python interface.   


## Lichess
Odonata is running as a bot on Lichess. You can see her rankings and games here [Lichess](https://lichess.org/@/odonata-bot) .

## CCRL
See the CCRL Blitz ratings here https://ccrl.chessdom.com/ccrl/404/cgi/compare_engines.cgi?family=Odonata

## Python interface
Odonata has a baby python interface, based on the Stockfish interface by Ilya Zhelyabuzhsky. Take a look at [Python](/docs/python.md). The interface will sure evolve, and perhaps migrate to a Python extension.

## Todo
See [Todo](/docs/todo.md)

## Changelog
In particular - please note that Odonata's github/main uses rust *nightly* 2021-07-06

See [Changelog](/docs/changelog.md)

## Credits
- A thanks to 
  - My sons - Freddie, Oscar and Hector - for assistance with graphics, some design ideas and testing the engine play
  - Gabor Szots and the CCRL team for arranging blitz competitive testing
  - The many chess libraries, engines and blogs out there. My apologies if I have not mentioned your project explicitly by name...


## Links
There are some excellent chess engines, libraries  and documentation out there. Please take a look.

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


## License
The software licence is [AGPL-3](../license.txt), though libraries used are MIT licensed. Trial or experimental versions may lag in terms of source code publication, and the author reserves the right to release versions of the software which are not open source. The intention though, is to give back and share with the community which has helped me learn.


Library | URL | License 
----------- | ---- | -------
Bitflags | https://crates.io/crates/bitflags | MIT https://choosealicense.com/licenses/mit/ | 
Once Cell | https://crates.io/crates/once_cell | MIT https://choosealicense.com/licenses/mit/ |
Criterion | https://crates.io/crates/criterion | MIT https://choosealicense.com/licenses/mit/ |
Env_Logger | https://crates.io/crates/env_logger | MIT https://choosealicense.com/licenses/mit/ |
Stockfish | https://github.com/zhelyabuzhsky/stockfish | MIT https://choosealicense.com/licenses/mit/ | This is a python wrapper for Stockfish. 
Hoarfrost | https://github.com/Ravenslofty/Hoarfrost | | MIT https://choosealicense.com/licenses/mit/ | Used for magic bitboard benchmarking
JsonRpyc | https://github.com/riga/jsonrpyc | MIT https://choosealicense.com/licenses/mit/ | 
Alcibiades |https://github.com/epandurski/alcibiades |  MIT https://choosealicense.com/licenses/mit/ | Worthy! Inspired Odonata's q-search 
