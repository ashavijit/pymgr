#!/bin/sh
set -e

REPO="ashavijit/pymgr"

# Determine OS
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

if [ "$OS" = "linux" ]; then
    if [ "$ARCH" = "x86_64" ]; then
        TARGET="linux-x86_64"
    else
        echo "Unsupported architecture: $ARCH on Linux"
        exit 1
    fi
elif [ "$OS" = "darwin" ]; then
    if [ "$ARCH" = "x86_64" ] || [ "$ARCH" = "amd64" ]; then
        TARGET="macos-x86_64"
    elif [ "$ARCH" = "arm64" ] || [ "$ARCH" = "aarch64" ]; then
        TARGET="macos-aarch64"
    else
        echo "Unsupported architecture: $ARCH on macOS"
        exit 1
    fi
else
    echo "Unsupported OS: $OS"
    exit 1
fi

FILE="pymgr-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/latest/download/${FILE}"
TMP_DIR=$(mktemp -d)

echo "Downloading pymgr from $URL..."
curl -sL "$URL" -o "$TMP_DIR/$FILE"

echo "Extracting archive..."
tar -xzf "$TMP_DIR/$FILE" -C "$TMP_DIR"
chmod +x "$TMP_DIR/pymgr"

INSTALL_DIR="$HOME/.cargo/bin"
mkdir -p "$INSTALL_DIR"
mv "$TMP_DIR/pymgr" "$INSTALL_DIR/"

echo "pymgr installed to $INSTALL_DIR/pymgr"
if ! command -v pymgr > /dev/null; then
  echo "Make sure $INSTALL_DIR is in your PATH."
fi

rm -rf "$TMP_DIR"
