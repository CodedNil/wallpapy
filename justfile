default:
    cargo run --target-dir target/desktop

build-web:
    RUSTFLAGS="--cfg=web_sys_unstable_apis" , trunk build

build-web-release:
    RUSTFLAGS="--cfg=web_sys_unstable_apis" , trunk build --release

serve:
    cargo run --no-default-features --target-dir target/server

serve-release:
    cargo run --no-default-features --target-dir target/server --release

release:
    git pull
    RUSTFLAGS="--cfg=web_sys_unstable_apis" , trunk build --release
    cargo build --release --no-default-features --target-dir target/server
    sudo systemctl restart wallpapy

release-server:
    git pull
    cargo build --release --no-default-features --target-dir target/server
    sudo systemctl restart wallpapy
