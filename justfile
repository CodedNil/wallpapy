default:
    RUSTFLAGS="--cfg=web_sys_unstable_apis" trunk build
    cargo run --no-default-features --target-dir target/server

build-web:
    RUSTFLAGS="--cfg=web_sys_unstable_apis" trunk build --release

build-server:
    cargo build --no-default-features --target-dir target/server --release
