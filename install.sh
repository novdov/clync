#!/bin/bash
set -e

REPO="novdov/claudy"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

if ! command -v cargo &> /dev/null; then
    echo "Error: cargo is required"
    echo "Install Rust: https://rustup.rs"
    exit 1
fi

if ! command -v git &> /dev/null; then
    echo "Error: git is required"
    exit 1
fi

TMP_DIR=$(mktemp -d)
trap "rm -rf $TMP_DIR" EXIT

echo "Cloning repository..."
git clone --depth 1 "https://github.com/${REPO}.git" "$TMP_DIR/repo"

echo "Building clync..."
cargo build --release --manifest-path "$TMP_DIR/repo/Cargo.toml"

mkdir -p "$INSTALL_DIR"
cp "$TMP_DIR/repo/target/release/clync" "${INSTALL_DIR}/clync"
chmod +x "${INSTALL_DIR}/clync"

VERSION=$("${INSTALL_DIR}/clync" --version 2>/dev/null || echo "unknown")
echo ""
echo "clync ${VERSION} installed to ${INSTALL_DIR}/clync"

if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
    echo ""
    echo "Add the following to your shell profile (.zshrc, .bashrc, etc.):"
    echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
fi

echo ""
echo "Usage:"
echo "  clync --help"
