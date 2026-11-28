#!/usr/bin/env bash
# WASM（Component）にビルドして、組み込み Extension のフォルダにコピーする。
set -euo pipefail
cd "$(dirname "$0")"
cargo build --target wasm32-wasip2 --release
cp target/wasm32-wasip2/release/nucleus_git.wasm ../../extensions/nucleus.git/git.wasm
echo "built $(du -h ../../extensions/nucleus.git/git.wasm | cut -f1)"
