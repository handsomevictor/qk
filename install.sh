#!/usr/bin/env bash
# install.sh — download and install the qk binary from GitHub Releases.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/OWNER/qk/main/install.sh | bash
#   # Or with a specific version:
#   QK_VERSION=v0.2.0 bash install.sh
#
# Env vars:
#   QK_VERSION    — release tag to install (default: latest)
#   QK_INSTALL_DIR — destination directory (default: /usr/local/bin, falls back to ~/.local/bin)

set -euo pipefail

REPO="OWNER/qk"
BINARY="qk"
INSTALL_DIR="${QK_INSTALL_DIR:-}"

# ── Detect OS ────────────────────────────────────────────────────────────────

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)
    case "$ARCH" in
      x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
      aarch64|arm64) TARGET="aarch64-unknown-linux-gnu" ;;
      *)
        echo "Unsupported Linux architecture: $ARCH" >&2
        exit 1
        ;;
    esac
    ARCHIVE_EXT="tar.gz"
    ;;
  Darwin)
    case "$ARCH" in
      x86_64)  TARGET="x86_64-apple-darwin" ;;
      arm64)   TARGET="aarch64-apple-darwin" ;;
      *)
        echo "Unsupported macOS architecture: $ARCH" >&2
        exit 1
        ;;
    esac
    ARCHIVE_EXT="tar.gz"
    ;;
  *)
    echo "Unsupported OS: $OS. Please download manually from https://github.com/$REPO/releases" >&2
    exit 1
    ;;
esac

# ── Resolve version ──────────────────────────────────────────────────────────

if [[ -z "${QK_VERSION:-}" ]]; then
  echo "Fetching latest release version from GitHub..."
  QK_VERSION="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | grep '"tag_name"' \
    | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')"
fi

if [[ -z "$QK_VERSION" ]]; then
  echo "Could not determine version. Set QK_VERSION explicitly." >&2
  exit 1
fi

echo "Installing $BINARY $QK_VERSION for $TARGET..."

# ── Determine install directory ───────────────────────────────────────────────

if [[ -z "$INSTALL_DIR" ]]; then
  if [[ -w /usr/local/bin ]]; then
    INSTALL_DIR="/usr/local/bin"
  else
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
  fi
fi

# ── Download and extract ──────────────────────────────────────────────────────

ARCHIVE_NAME="${BINARY}-${QK_VERSION}-${TARGET}.${ARCHIVE_EXT}"
DOWNLOAD_URL="https://github.com/$REPO/releases/download/$QK_VERSION/$ARCHIVE_NAME"
TMP_DIR="$(mktemp -d)"

trap 'rm -rf "$TMP_DIR"' EXIT

echo "Downloading $DOWNLOAD_URL..."
curl -fsSL --progress-bar "$DOWNLOAD_URL" -o "$TMP_DIR/$ARCHIVE_NAME"

echo "Extracting..."
tar -xzf "$TMP_DIR/$ARCHIVE_NAME" -C "$TMP_DIR"

# ── Install ───────────────────────────────────────────────────────────────────

DEST="$INSTALL_DIR/$BINARY"

if [[ -w "$INSTALL_DIR" ]]; then
  install -m 755 "$TMP_DIR/$BINARY" "$DEST"
else
  echo "Root access required to install to $INSTALL_DIR"
  sudo install -m 755 "$TMP_DIR/$BINARY" "$DEST"
fi

echo ""
echo "✓ $BINARY installed to $DEST"
echo "  Run: $BINARY --help"

# Warn if install dir is not in PATH
if ! command -v "$BINARY" &>/dev/null; then
  echo ""
  echo "NOTE: $INSTALL_DIR is not in your PATH."
  echo "      Add the following to your shell profile:"
  echo "      export PATH=\"\$PATH:$INSTALL_DIR\""
fi
