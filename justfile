set dotenv-load := true
set positional-arguments := true

default:
    just --list

install:
    cargo install worker-build

build:
    cargo build --release --target wasm32-unknown-unknown --locked

deploy: install build
    wrangler deploy

dev: install
    wrangler dev --local

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

#
# these are run on Cloudflare when building and deploying the worker

# you shouldn't run these yourself
full-wrangler-build:
    curl --proto '=https' --tlsv1.2 --silent --show-error --fail 'https://sh.rustup.rs' | sh -s -- --yes --profile minimal
    export PATH="$HOME/.cargo/bin:$PATH"
    rustc --version
    rustup update stable
    rustup target add wasm32-unknown-unknown
    cargo install --quiet worker-build
    worker-build --release

full-wrangler-deploy:
    export PATH="$HOME/.cargo/bin:$PATH"
    npx wrangler deploy
