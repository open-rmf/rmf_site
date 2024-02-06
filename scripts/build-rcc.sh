#!/bin/bash
set -o verbose
set -o errexit

export CARGO_PROFILE_RELEASE_LTO=true
export CARGO_PROFILE_RELEASE_OPT_LEVEL=z

cargo build --target wasm32-unknown-unknown --release
RUST_BACKTRACE=full wasm-bindgen --target web --out-dir ../src/site_editor/ target/wasm32-unknown-unknown/release/librmf_site_editor.wasm

cd ../src/site_editor/
wasm-opt -Oz -o librmf_site_editor_bg_optimized.wasm librmf_site_editor_bg.wasm
