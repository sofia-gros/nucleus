#!/usr/bin/env bash
# WASM（Component）にビルドして、組み込み Extension のフォルダにコピーする。
set -euo pipefail
cd "$(dirname "$0")"
cargo build --target wasm32-wasip2 --release
cp target/wasm32-wasip2/release/nucleus_minimap.wasm ../../extensions/nucleus.minimap/minimap.wasm
echo "built $(du -h ../../extensions/nucleus.minimap/minimap.wasm | cut -f1)"
