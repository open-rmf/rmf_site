#!/bin/bash
set -o verbose
set -o errexit
CRATE=rmf_site_editor_web
if [ "$1" = "--viz" ]; then
  CRATE=rmf_site_viz_web
fi
export CARGO_PROFILE_RELEASE_LTO=true
export CARGO_PROFILE_RELEASE_OPT_LEVEL=z
cargo build -p $CRATE --target wasm32-unknown-unknown --release
RUST_BACKTRACE=full wasm-bindgen --target web --out-dir web target/wasm32-unknown-unknown/release/$CRATE.wasm
ln -sfn ../assets web/assets
cd web
wasm-opt -Oz -o ${CRATE}_bg_optimized.wasm ${CRATE}_bg.wasm
