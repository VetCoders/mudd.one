#!/usr/bin/env bash
# Build Rust FFI library + generate Swift bindings
# Called from Xcode Build Phase
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

echo "=== Building mudd-ffi (release) ==="
cargo build -p mudd-ffi --release

echo "=== Generating Swift bindings ==="
cargo run -p uniffi-bindgen -- generate \
    --library target/release/libmudd_ffi.dylib \
    --language swift \
    --out-dir app/mudd/Bridge/

echo "=== Done ==="
