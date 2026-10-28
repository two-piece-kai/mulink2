cross build --release --target aarch64-unknown-linux-gnu --bin rx_simple
scp target/aarch64-unknown-linux-gnu/release/rx* brodie@mulink64bit.local:~/mulink-bin/