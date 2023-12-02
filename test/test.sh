ID=$(cargo run -- new "C:/source/isaacadams/xbatch/target/debug/xbatch.exe" fake)
cargo run -- stream --db=test/test.db --table=data | \
RUST_LOG=info cargo run -- monitor $ID "C:/source/isaacadams/xbatch/target/debug/xbatch.exe" fake