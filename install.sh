#!/bin/sh
set -eu

REPO="brynary/arc"

# Colors (only when stdout is a terminal)
if [ -t 1 ]; then
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  BOLD='\033[1m'
  RESET='\033[0m'
else
  RED=''
  GREEN=''
  BOLD=''
  RESET=''
fi

info()    { printf "${BOLD}%s${RESET}\n" "$1"; }
success() { printf "${GREEN}%s${RESET}\n" "$1"; }
error()   { printf "${RED}error: %s${RESET}\n" "$1" >&2; exit 1; }

# --- Require gh CLI ---
if ! command -v gh >/dev/null 2>&1; then
  error "gh CLI is required but not installed. Install it from https://cli.github.com"
fi

# --- Detect platform ---
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Darwin)
    # Detect Rosetta translation
    if [ "$ARCH" = "x86_64" ]; then
      if sysctl -n sysctl.proc_translated 2>/dev/null | grep -q 1; then
        ARCH="arm64"
      fi
    fi
    case "$ARCH" in
      arm64) TARGET="aarch64-apple-darwin" ;;
      *)     error "Unsupported macOS architecture: $ARCH. Supported: Apple Silicon (arm64)" ;;
    esac
    ;;
  Linux)
    case "$ARCH" in
      x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
      *)      error "Unsupported Linux architecture: $ARCH. Supported: x86_64" ;;
    esac
    ;;
  *)
    error "Unsupported OS: $OS. Supported platforms: macOS (Apple Silicon), Linux (x86_64)"
    ;;
esac

ASSET="arc-${TARGET}.tar.gz"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

info "Downloading latest Arc release for ${TARGET}..."
gh release download --repo "$REPO" --pattern "$ASSET" --dir "$TMPDIR" --clobber

info "Extracting..."
tar xzf "${TMPDIR}/${ASSET}" -C "$TMPDIR"

# --- Install binary ---
INSTALL_DIR="${ARC_INSTALL_DIR:-/usr/local/bin}"

if [ -w "$INSTALL_DIR" ]; then
  mv "${TMPDIR}/arc-${TARGET}/arc" "${INSTALL_DIR}/arc"
else
  info "Installing to ${INSTALL_DIR} (requires sudo)..."
  sudo mv "${TMPDIR}/arc-${TARGET}/arc" "${INSTALL_DIR}/arc"
fi

chmod +x "${INSTALL_DIR}/arc"

# --- Verify ---
VERSION="$("${INSTALL_DIR}/arc" --version 2>/dev/null || true)"
if [ -z "$VERSION" ]; then
  error "Installation failed: could not run arc --version"
fi

success "Successfully installed ${VERSION} to ${INSTALL_DIR}/arc"
echo ""
info "Run 'arc install' to complete setup."
