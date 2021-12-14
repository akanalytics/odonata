
EXE:
	export RUSTFLAGS="-Ctarget-feature= -Ctarget-cpu=x86-64-v3"
	cargo b --release --features=fast --target x86_64-unknown-linux-musl 
	mv ./target/release/odonata ./$(EXE)