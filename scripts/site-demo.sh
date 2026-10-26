#!/usr/bin/env bash
# Builds the live demo of the website (prompt 011): ed-core → WebAssembly → site/demo/pkg.
# Needs the wasm32 target (`rustup target add wasm32-unknown-unknown`) and wasm-bindgen-cli in the
# version of src-tauri/wasm/Cargo.lock (`cargo install wasm-bindgen-cli --version <v>`).
# CI does the same in .github/workflows/site-demo.yml.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release --locked --target wasm32-unknown-unknown --manifest-path src-tauri/wasm/Cargo.toml
wasm-bindgen --target web --no-typescript --out-dir site/demo/pkg \
  src-tauri/wasm/target/wasm32-unknown-unknown/release/ed_wasm.wasm
if command -v wasm-opt >/dev/null; then
  wasm-opt -Os --enable-bulk-memory --enable-nontrapping-float-to-int -o site/demo/pkg/ed_wasm_bg.wasm site/demo/pkg/ed_wasm_bg.wasm
fi
ls -l site/demo/pkg
