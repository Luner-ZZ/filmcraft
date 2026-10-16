#!/usr/bin/env bash
# FilmCraft 1-Click Builder for macOS
# Double-click this file in Finder to build FilmCraft and package FilmCraft.app.

set -euo pipefail

# 1. Switch to repository root directory
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$DIR"

echo "===================================================="
echo "          FilmCraft 1-Click Builder (macOS)         "
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

# 3. Prompt build mode or default to Release
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

# 4. Package into FilmCraft.app with official icon and Info.plist
echo ""
echo "[INFO] Assembling FilmCraft.app bundle with official icon..."
APP="FilmCraft.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
mkdir -p "$APP/Contents/Resources"

# Copy binary
cp "target/$TARGET_DIR/filmcraft" "$APP/Contents/MacOS/FilmCraft"
chmod +x "$APP/Contents/MacOS/FilmCraft"

# Copy official macOS icns icon
if [ -f "assets/app-icon/filmcraft.icns" ]; then
  cp "assets/app-icon/filmcraft.icns" "$APP/Contents/Resources/FilmCraft.icns"
fi

# Generate Info.plist
cat > "$APP/Contents/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleDisplayName</key>
  <string>FilmCraft</string>
  <key>CFBundleName</key>
  <string>FilmCraft</string>
  <key>CFBundleExecutable</key>
  <string>FilmCraft</string>
  <key>CFBundleIdentifier</key>
  <string>ai.storyteller.filmcraft</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>0.8.0</string>
  <key>CFBundleVersion</key>
  <string>0.8.0</string>
  <key>CFBundleIconFile</key>
  <string>FilmCraft</string>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.video</string>
  <key>LSMinimumSystemVersion</key>
  <string>11.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>NSSupportsAutomaticGraphicsSwitching</key>
  <true/>
  <key>NSRequiresAquaSystemAppearance</key>
  <false/>
  <key>NSMicrophoneUsageDescription</key>
  <string>FilmCraft uses the microphone to record voice-over onto the timeline.</string>
  <key>NSHumanReadableCopyright</key>
  <string>Copyright © the FilmCraft authors. MIT OR Apache-2.0.</string>
</dict>
</plist>
EOF

# Ad-hoc codesign so macOS allows running locally without warning
if command -v codesign &> /dev/null; then
  echo "[INFO] Applying local ad-hoc signature to FilmCraft.app..."
  codesign --force --deep --sign - "$APP" 2>/dev/null || true
fi

# 5. Clean up target/ to reclaim storage
echo ""
echo "[INFO] Cleaning up intermediate build files to reclaim disk space..."
rm -rf target/
echo "[SUCCESS] Removed target/ folder. Storage reclaimed!"

echo ""
echo "===================================================="
echo "    Build Complete! FilmCraft.app is in this folder "
echo "    with its official icon. Double-click it to run! "
echo "===================================================="
echo ""
read -r -p "Press Enter to exit..." || true
