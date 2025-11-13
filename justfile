default:
    just build-web
    cargo run --no-default-features --target-dir target/server

build-web:
    rm -rf dist
    mkdir -p dist
    cp -r assets/* dist/

    RUSTFLAGS="--cfg=web_sys_unstable_apis" cargo build \
        --target wasm32-unknown-unknown \
        --release \
        --target-dir target/wasm
    wasm-bindgen target/wasm/wasm32-unknown-unknown/release/wallpapy.wasm \
        --out-dir dist \
        --target web \
        --no-typescript

    wasm-opt -Oz dist/wallpapy_bg.wasm -o dist/wallpapy_bg.wasm

build-server:
    cargo build --no-default-features --target-dir target/server --release
