RUSTFLAGS="--emit asm -Ctarget-cpu=haswell -Ctarget-feature=+sse2,+sse3,+ssse3,+sse4.1,+sse4.2,+aes" cargo build --release
