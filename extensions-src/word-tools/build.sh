#!/usr/bin/env bash
# WASM（Component）にビルドして、組み込み Extension のフォルダにコピーする。
set -euo pipefail
cd "$(dirname "$0")"
cargo build --target wasm32-wasip2 --release
cp target/wasm32-wasip2/release/word_tools.wasm ../../extensions/nucleus.word-tools/word_tools.wasm
echo "built $(du -h ../../extensions/nucleus.word-tools/word_tools.wasm | cut -f1)"
