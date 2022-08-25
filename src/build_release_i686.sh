# Build scrpt for 32-bit Windows.
# Performance is bad since special CPU instructions or even 64-bit instructions are not used.
cargo +stable-i686-pc-windows-msvc build --target i686-pc-windows-msvc --release
