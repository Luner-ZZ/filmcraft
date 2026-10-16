#!/usr/bin/env bash
# FilmCraft 1-Click Builder for Linux
# Run this script or double-click build-linux.desktop to build and package FilmCraft.

set -euo pipefail

# 1. Switch to repository root directory
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$DIR"

echo "===================================================="
echo "          FilmCraft 1-Click Builder (Linux)         "
echo "===================================================="
echo ""

# 2. Check for Cargo / Rust
if ! command -v cargo &> /dev/null; then
  if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
  fi
fi

if ! command -v cargo &> /dev/null; then
  echo "[INFO] Rust / Cargo was not found."
  echo "Installing Rust toolchain via rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # shellcheck source=/dev/null
  source "$HOME/.cargo/env"
fi

echo "[INFO] Using Cargo: $(command -v cargo)"
echo ""

# 3. Check for essential system development libraries on Debian/Ubuntu/Fedora/Arch
if command -v apt-get &> /dev/null; then
  MISSING=()
  for pkg in libasound2-dev libfontconfig1-dev libvulkan-dev pkg-config; do
    if ! dpkg -s "$pkg" &> /dev/null; then
      MISSING+=("$pkg")
    fi
  done
  if [ ${#MISSING[@]} -gt 0 ]; then
    echo "[WARN] Missing recommended development packages: ${MISSING[*]}"
    echo "You may install them with: sudo apt install -y ${MISSING[*]}"
  fi
fi

# 4. Prompt build mode or default to Release
echo "Select build mode:"
echo "  [1] Release (Optimized, smoothest playback - RECOMMENDED)"
echo "  [2] Debug   (Faster compile time)"
read -r -p "Enter choice (1 or 2, default is 1): " choice || choice="1"

BUILD_FLAG="--release"
TARGET_DIR="release"
if [[ "$choice" == "2" ]]; then
  BUILD_FLAG=""
  TARGET_DIR="debug"
fi

echo ""
echo "[INFO] Building FilmCraft..."
if [[ -n "$BUILD_FLAG" ]]; then
  cargo build "$BUILD_FLAG" -p filmcraft
else
  cargo build -p filmcraft
fi

# 5. Place binary in root folder and install icon
echo ""
echo "[INFO] Copying FilmCraft binary to root folder..."
cp "target/$TARGET_DIR/filmcraft" "./FilmCraft"
chmod +x "./FilmCraft"

# Copy official app icon
if [ -f "assets/app-icon/filmcraft-1024.png" ]; then
  cp "assets/app-icon/filmcraft-1024.png" "./FilmCraft.png"
fi

# 6. Create a desktop launcher in this folder
cat > "FilmCraft.desktop" << EOF
[Desktop Entry]
Version=1.0
Type=Application
Name=FilmCraft
Comment=FilmCraft Video Editor
Exec=$DIR/FilmCraft
Icon=$DIR/FilmCraft.png
Terminal=false
Categories=AudioVideo;Video;AudioVideoEditing;
StartupNotify=true
EOF
chmod +x "FilmCraft.desktop"

# 7. Clean up target/ to reclaim storage
echo ""
echo "[INFO] Cleaning up intermediate build files to reclaim disk space..."
rm -rf target/
echo "[SUCCESS] Removed target/ folder. Storage reclaimed!"

echo ""
echo "===================================================="
echo "    Build Complete! FilmCraft and FilmCraft.desktop  "
echo "    are ready in this folder with their official icon"
echo "===================================================="
echo ""
read -r -p "Press Enter to exit..." || true
