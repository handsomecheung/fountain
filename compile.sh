#!/usr/bin/env bash
set -e

cd "$(dirname "${BASH_SOURCE[0]}")"

cargo build --bin cube-encode --release --no-default-features --features encode
cargo build --bin cube-decode --release --features decode

cargo build --target wasm32-unknown-unknown --release --no-default-features --features wasm

wasm-bindgen target/wasm32-unknown-unknown/release/cube.wasm --out-dir www/pkg --target web
