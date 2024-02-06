# Compilation

Instruction on how to compile from source.<br>
You will need a recent version of the stable rust compiler. Typically this can be done using rustup.

```
rustup update
```

Then you will need to download or sync the repository from github. 

Finally, use cargo, with some options, to compile.

### Windows
```cmd
; modern cpu version
set RUSTFLAGS=-C target-feature=+crt-static -C target-cpu=x86-64-v3 && cargo b --profile=tournament 

; if the above fails the search test suite below try compiling for an older cpu
set RUSTFLAGS=-C target-feature=+crt-static -C target-cpu=generic && cargo b --profile=tournament 
```

### Linux
```bash
# on linux you will want to install the musl library for better portability between Linux systems.
rustup target add x86_64-unknown-linux-musl

# modern cpu version
RUSTFLAGS="-C target-cpu=x86-64-v3" cargo b --profile=tournament --target x86_64-unknown-linux-musl

# older cpu
RUSTFLAGS="-C target-cpu=generic" cargo b --profile=tournament --target x86_64-unknown-linux-musl

# remove debug symbols
strip ./target/x86_64-unknown-linux-musl/tournament/odonata 
```

# Testing 


```bash
# help with command line options 
.\target\tournament\odonata.exe help

# search performance on a test suite (on Windows)
.\target\tournament\odonata.exe search -t depth=10

# search performance on a test suite (on Linux)
./target/x86_64-unknown-linux-musl/tournament/odonata search -t depth=10

# perft performance 
.\target\tournament\odonata.exe uci "perft 6"

# show compilation flags
.\target\tournament\odonata.exe uci "compiler"

# evaluate (statically) a board position
.\target\tournament\odonata.exe uci "position fen r1b2rk1/1p3pp1/pn1b1n1p/2pPp3/Pq6/2N1PNB1/BP2QPPP/3R1RK1 b - - 4 16; eval" 

# run a depth 5 search from a FEN position
.\target\tournament\odonata.exe uci "position fen r1b2rk1/1p3pp1/pn1b1n1p/2pPp3/Pq6/2N1PNB1/BP2QPPP/3R1RK1 b - - 4 16; go depth 5" 

# run the interactive uci engine (more usually invoked through a chess GUI)
.\target\tournament\odonata.exe 

```






  

















