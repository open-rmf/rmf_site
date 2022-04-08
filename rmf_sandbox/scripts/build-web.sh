#!/bin/bash
set -o verbose
set -o errexit
export CARGO_PROFILE_RELEASE_LTO=true
export CARGO_PROFILE_RELEASE_OPT_LEVEL=z
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir web target/wasm32-unknown-unknown/release/librmf_sandbox.wasm
cd web
wasm-opt -Oz -o librmf_sandbox_bg_optimized.wasm librmf_sandbox_bg.wasm
