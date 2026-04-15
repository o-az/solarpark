set dotenv-load := true
set positional-arguments := true

default:
    just --list

build:
    cargo build --release --target wasm32-unknown-unknown --locked

fmt:
    cargo fmt --all
    just --fmt --unstable

lint:
    cargo check --all-features
    cargo clippy --all-targets --all-features

check: fmt
    cargo check --all-features --release
    cargo clippy --all-targets --all-features --release

update-wrangler-sha:
    sed -i '' "s/^COMMIT_SHA = .*/COMMIT_SHA = \"$(git rev-parse --short HEAD)\"/" wrangler.toml

# you shouldn't run these yourself
full-build:
    curl --proto '=https' --tlsv1.2 --silent --show-error --fail 'https://sh.rustup.rs' | sh -s -- --yes --profile minimal
    export PATH="$HOME/.cargo/bin:$PATH"
    rustc --version
    rustup update stable
    rustup target add wasm32-unknown-unknown
    cargo install --quiet worker-build
    worker-build --release
