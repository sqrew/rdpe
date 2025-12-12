#!/bin/bash
set -e

# Get the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
REPO_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

echo "Building RDPE Editor for WebAssembly..."

# Check for wasm-bindgen-cli
if ! command -v wasm-bindgen &> /dev/null; then
    echo "Installing wasm-bindgen-cli..."
    cargo install wasm-bindgen-cli
fi

# Build in release mode for smaller binary
echo "Compiling to WASM (release mode)..."
cd "$REPO_ROOT"
cargo build --package rdpe-editor --release --target wasm32-unknown-unknown

# Create output directory
mkdir -p "$SCRIPT_DIR/web/dist"

# Run wasm-bindgen to generate JS bindings
echo "Generating JS bindings..."
wasm-bindgen \
    --out-dir "$SCRIPT_DIR/web/dist" \
    --target web \
    --no-typescript \
    "$REPO_ROOT/target/wasm32-unknown-unknown/release/rdpe-editor.wasm"

# Copy index.html
cp "$SCRIPT_DIR/web/index.html" "$SCRIPT_DIR/web/dist/"

# Optimize WASM size (optional, requires wasm-opt)
if command -v wasm-opt &> /dev/null; then
    echo "Optimizing WASM binary..."
    wasm-opt -Oz -o "$SCRIPT_DIR/web/dist/rdpe_editor_bg.wasm" "$SCRIPT_DIR/web/dist/rdpe_editor_bg.wasm"
else
    echo "Note: Install wasm-opt (from binaryen) for smaller WASM files"
fi

echo ""
echo "Build complete! Files are in rdpe-editor/web/dist/"
echo ""
echo "To test locally, run a web server:"
echo "  cd $SCRIPT_DIR/web/dist && python3 -m http.server 8080"
echo ""
echo "Then open http://localhost:8080 in a WebGPU-enabled browser"
echo "(Chrome 113+, Edge 113+, or Firefox Nightly with WebGPU flag)"
