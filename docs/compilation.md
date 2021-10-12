# Compilation

Instruction on how to compile from source.
You will need a recent version of the stable rust compiler. Typically this can be done using rustup.

```
rustup update
```

## Building a portable binary that can be run on other computers with the same operating system

```
cargo b --release --features=fast
```


## Building a non-portable binary that can only be run on the host used to compile (but give best performance)


### On Linux

```
export RUSTFLAGS="-Ctarget-cpu=native"
cargo b --release --features=fast
```


### On Windows

```
set RUSTFLAGS="-Ctarget-cpu=native"
cargo b --release --features=fast
```

# Testing the binary


```
./target/release/odonata --perft 6


./target/release/odonata --search 
```






  

















