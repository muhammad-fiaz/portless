#!/usr/bin/env bash
#
# portless installer for macOS and Linux.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/muhammad-fiaz/portless/main/scripts/install.sh | sh
#
# Or, to install a specific version:
#   curl -fsSL .../install.sh | sh -s -- v0.1.0
#
# Or, to install a pre-release:
#   curl -fsSL .../install.sh | sh -s -- --pre
#
# Or, to install a specific release tag from a fork or local checkout:
#   REPO=muhammad-fiaz/portless VERSION=v0.1.0 sh install.sh
#
# Environment variables (optional):
#   INSTALL_DIR  Where to copy the binary (default: $HOME/.local/bin)
#   VERSION      Release tag to install (default: latest stable)
#   REPO         GitHub "owner/name" to fetch from (default: muhammad-fiaz/portless)
#   NO_AUTO_SETUP  If set to 1, skip the binary's first-run auto-setup
#   PORTLESS_NO_AUTO_SETUP  Same flag, exported into the resulting environment

set -euo pipefail

REPO="${REPO:-muhammad-fiaz/portless}"
VERSION="${VERSION:-}"
PRERELEASE_FLAG=""
while [ $# -gt 0 ]; do
  case "$1" in
    -h|--help)
      sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    --pre)
      PRERELEASE_FLAG="--pre"
      shift
      ;;
    -*)
      echo "Unknown flag: $1" >&2
      exit 2
      ;;
    *)
      VERSION="$1"
      shift
      ;;
  esac
done

# --- 1. Pick the install directory ----------------------------------------
if [ -n "${INSTALL_DIR:-}" ]; then
  INSTALL_DIR="$INSTALL_DIR"
elif [ -d "$HOME/.local/bin" ] || [ "${XDG_BIN_HOME:-}" != "" ]; then
  INSTALL_DIR="${XDG_BIN_HOME:-$HOME/.local/bin}"
else
  # Fall back to a directory we know we can create.
  INSTALL_DIR="$HOME/.local/bin"
fi

# --- 2. Detect host platform ---------------------------------------------
uname_s=$(uname -s 2>/dev/null || echo "")
uname_m=$(uname -m 2>/dev/null || echo "")

case "$uname_s" in
  Linux)
    case "$uname_m" in
      x86_64|amd64)   target="portless-linux-amd64.tar.gz"; asset_dir="portless-linux-amd64" ;;
      aarch64|arm64)  target="portless-linux-arm64.tar.gz"; asset_dir="portless-linux-arm64" ;;
      *)
        echo "Unsupported Linux architecture: $uname_m" >&2
        echo "Supported: x86_64, aarch64" >&2
        exit 1
        ;;
    esac
    ;;
  Darwin)
    case "$uname_m" in
      x86_64)         target="portless-macos-amd64.tar.gz"; asset_dir="portless-macos-amd64" ;;
      arm64)          target="portless-macos-arm64.tar.gz"; asset_dir="portless-macos-arm64" ;;
      *)
        echo "Unsupported macOS architecture: $uname_m" >&2
        echo "Supported: x86_64 (Intel), arm64 (Apple Silicon)" >&2
        exit 1
        ;;
    esac
    ;;
  *)
    echo "This installer only supports macOS and Linux." >&2
    echo "On Windows, use scripts/install.ps1 (PowerShell) or scripts/install.bat (cmd)." >&2
    exit 1
    ;;
esac

# --- 3. Resolve version --------------------------------------------------
if [ -z "$VERSION" ]; then
  echo "Resolving latest version for $REPO ..."
  VERSION=$(curl -fsSL \
    -H "Accept: application/vnd.github+json" \
    "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null \
    | grep -o '"tag_name":[[:space:]]*"[^"]*"' \
    | head -1 \
    | sed -E 's/.*"([^"]+)".*/\1/' || true)
  if [ -z "$VERSION" ] && [ -z "$PRERELEASE_FLAG" ]; then
    echo "Could not determine latest version (no internet?)." >&2
    echo "Set VERSION=vX.Y.Z and re-run." >&2
    exit 1
  fi
fi

if [ -z "$VERSION" ]; then
  echo "No release version resolved." >&2
  exit 1
fi

echo "Installing portless ${VERSION} (${uname_s}/${uname_m})"

# --- 4. Download and verify ----------------------------------------------
tmp=$(mktemp -d 2>/dev/null || mktemp -d -t 'portless-install')
trap 'rm -rf "$tmp"' EXIT

url="https://github.com/${REPO}/releases/download/${VERSION}/${target}"
echo "Downloading $url"
curl -fsSL -o "$tmp/$target" "$url"

# Verify SHA256 if a sidecar file is available.
sha_url="https://github.com/${REPO}/releases/download/${VERSION}/${target}.sha256"
if curl -fsSL -o "$tmp/$target.sha256" "$sha_url" 2>/dev/null; then
  echo "Verifying SHA256 checksum..."
  expected=$(awk '{print $1}' "$tmp/$target.sha256")
  if command -v sha256sum >/dev/null 2>&1; then
    actual=$(sha256sum "$tmp/$target" | awk '{print $1}')
  else
    actual=$(shasum -a 256 "$tmp/$target" | awk '{print $1}')
  fi
  if [ "$expected" != "$actual" ]; then
    echo "Checksum mismatch!" >&2
    echo "  expected: $expected" >&2
    echo "  actual:   $actual" >&2
    exit 1
  fi
  echo "Checksum OK"
else
  echo "No checksum file available -- skipping verification."
fi

# --- 5. Extract and install ----------------------------------------------
mkdir -p "$INSTALL_DIR"
tar -xzf "$tmp/$target" -C "$tmp"
chmod +x "$tmp/$asset_dir"
mv "$tmp/$asset_dir" "$INSTALL_DIR/portless"
echo "Installed: $INSTALL_DIR/portless"

# --- 6. Register on PATH (idempotent) ------------------------------------
case ":$PATH:" in
  *":$INSTALL_DIR:"*) already=1 ;;
  *)                 already=0 ;;
esac
if [ "$already" -eq 0 ]; then
  marker="# Added by portless installer"
  export_line="export PATH=\"\$PATH:$INSTALL_DIR\""
  appended=0
  for rc in "$HOME/.profile" "$HOME/.bashrc" "$HOME/.zshrc"; do
    [ -f "$rc" ] || continue
    if ! grep -qF "$marker" "$rc" 2>/dev/null; then
      printf "\n%s\n%s\n" "$marker" "$export_line" >> "$rc"
      appended=1
    fi
  done
  if [ "$appended" -eq 1 ]; then
    echo "Appended $INSTALL_DIR to your shell startup files."
    echo "Open a new terminal (or 'source ~/.bashrc') for the change to take effect."
  fi
fi

# --- 7. Verify -----------------------------------------------------------
"$INSTALL_DIR/portless" --version

echo
echo "✅ portless is ready. Try:"
echo "   $INSTALL_DIR/portless trust"
echo "   $INSTALL_DIR/portless run npm run dev"
echo "   # open https://<your-app>.localhost"
