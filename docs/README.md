<img src="https://github.com/akanalytics/odonata/blob/main/docs/odonata-blue.png" width=150 />


# Odonata
A chess engine written in Rust.

## Background
My lockdown hobby was writing a chess engine, and learning Python and Rust in the process. Currently Odonata's available to play as a bot on Lichess, though Odonata only plays blitz and bullet games right now, as she's running on my media centre, and Netflix stutters if she's thinking too hard...

I started Decemeber 2020. Python and Rust are very different from Java, which I had programmed maybe 10 years previous. Certainly my first efforts at Rust are not very clean, clever or idiomatic, but the code improves as I revist areas to build improvements.

Im aware that this is now yet another chess engine, with yet another set of Python bindings. In part this is because for me, the project was intended as a learning exercise, but also there did not appear to be a Python native extension available. If any library owners, either Rust or Python, have the time to chat about how I can avoid duplication or rationize things in terms of Python chess extension modules in Rust, please email!   


## Lichess
Odonata is running as a bot on Lichess. You can see her rankings and games here [Lichess](https://lichess.org/@/odonata-bot) .
 

## Todo
See [Todo](/docs/todo.md)

## Credits
A thanks to my sons - Freddie, Oscar and Hector - for assiatance with graphics, some design ideas and testing the engine play. 


## Links
There are some excellent chess engines, libraries  and documentation out there. Please take a look.

Description | Link | Comment  
----------- | ---- | -------
Chess Programming Wiki | https://www.chessprogramming.org/ | Invaluable...
Pure Python Chess Library | https://github.com/niklasf/python-chess |
Fast chess move generation | https://github.com/jordanbray/chess |
Shakmaty - Rust chess library | https://github.com/niklasf/shakmaty |
Rust Chess engine | https://github.com/vinc/littlewing | For making me appreciate ASCII art...
Chesss GUI for UCI engines | http://www.playwitharena.de/ |

## Licence
The software licence is [License](../license.txt) 

Third party libraries used may have differing licenses. In particular... 

Library | URL | License 
----------- | ---- | -------
Bitflags | https://crates.io/crates/bitflags | MIT https://choosealicense.com/licenses/mit/
Once Cell | https://crates.io/crates/once_cell | MIT https://choosealicense.com/licenses/mit/
Criterion | https://crates.io/crates/criterion | MIT https://choosealicense.com/licenses/mit/
Env_Logger | https://crates.io/crates/env_logger | MIT https://choosealicense.com/licenses/mit/


