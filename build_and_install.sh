#!/bin/bash
set -e

INSTALL_DIR="${ANVL_INSTALL_DIR:-$HOME/.local/bin}"

echo "Building anvl (release)..."
cargo build --release -p tui

VERSION=$(cargo metadata --no-deps --format-version 1 | grep -o '"version":"[^"]*"' | head -1 | cut -d'"' -f4)

mkdir -p "$INSTALL_DIR"
rm -f "$INSTALL_DIR/anvl"
cp target/release/anvl "$INSTALL_DIR/anvl"
chmod +x "$INSTALL_DIR/anvl"

echo "Installed anvl ${VERSION} to ${INSTALL_DIR}/anvl"

case ":$PATH:" in
  *":${INSTALL_DIR}:"*) ;;
  *) echo "Note: ${INSTALL_DIR} is not in your PATH. Add it with:"
     echo "  export PATH=\"${INSTALL_DIR}:\$PATH\"" ;;
esac
