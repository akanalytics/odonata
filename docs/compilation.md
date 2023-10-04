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
set RUSTFLAGS=-C target-feature=+crt-static -C target-cpu=x86-64-v3 && cargo b --release 

; if the above fails the search test suite below try compiling for an older cpu
set RUSTFLAGS=-C target-feature=+crt-static -C target-cpu=generic && cargo b --release 
```

### Linux
```bash
# on linux you will want to install the musl library for better portability between Linux systems.
rustup target add x86_64-unknown-linux-musl

# modern cpu version
export RUSTFLAGS="-C target-cpu=generic" && cargo b --release --target x86_64-unknown-linux-musl

# older cpu
export RUSTFLAGS="-C target-cpu=x86-64-v3" && cargo b --release --target x86_64-unknown-linux-musl
```

# Testing 


```bash
# help with command line options 
.\target\release\odonata.exe help

# search performance on a test suite
.\target\release\odonata.exe search -t depth=10

# perft performance 
.\target\release\odonata.exe uci "perft 6"

# show compilation flags
.\target\release\odonata.exe uci "compiler"

# evaluate (statically) a board position
.\target\release\odonata.exe uci "position fen r1b2rk1/1p3pp1/pn1b1n1p/2pPp3/Pq6/2N1PNB1/BP2QPPP/3R1RK1 b - - 4 16; eval" 

# run a depth 5 search from a FEN position
.\target\release\odonata.exe uci "position fen r1b2rk1/1p3pp1/pn1b1n1p/2pPp3/Pq6/2N1PNB1/BP2QPPP/3R1RK1 b - - 4 16; go depth 5" 

# run the interactive uci engine (more usually invoked through a chess GUI)
.\target\release\odonata.exe 

```






  

















