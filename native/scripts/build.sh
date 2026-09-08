#!/usr/bin/env bash
# Build the native Linux host (Vulkan via wgpu).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export RUSTFLAGS="${RUSTFLAGS:-}"
cargo test --manifest-path Cargo.toml
cargo build --release --manifest-path Cargo.toml
echo "binary: $ROOT/target/release/melee-native"
echo "staticlib: $ROOT/target/release/libmelee_host.a"
echo "run: $ROOT/target/release/melee-native"
echo "headless check: $ROOT/target/release/melee-native --headless"
