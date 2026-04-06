#!/usr/bin/env bash
set -euo pipefail

APP_NAME="Snapdash"
BIN_NAME="snapdash"
BUNDLE_ID="com.snapdash.Snapdash"
VERSION="0.1.0"

DIST_DIR="dist"
APP_DIR="${DIST_DIR}/${APP_NAME}.app"
CONTENTS="${APP_DIR}/Contents"
MACOS_DIR="${CONTENTS}/MacOS"
RES_DIR="${CONTENTS}/Resources"

rm -rf "${APP_DIR}"
mkdir -p "${MACOS_DIR}" "${RES_DIR}"

# Build
cargo build --release

# Binárka
cp "target/release/${BIN_NAME}" "${MACOS_DIR}/${APP_NAME}"
chmod +x "${MACOS_DIR}/${APP_NAME}"

# Info.plist (vygeneruj, nebo cp)
cat > "${CONTENTS}/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
 "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>${APP_NAME}</string>
  <key>CFBundleDisplayName</key><string>${APP_NAME}</string>
  <key>CFBundleIdentifier</key><string>${BUNDLE_ID}</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundleExecutable</key><string>${APP_NAME}</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
</dict>
</plist>
EOF

if [[ -f "${DIST_DIR}/AppIcon.icns" ]]; then
  cp "${DIST_DIR}/AppIcon.icns" "${RES_DIR}/AppIcon.icns"
fi

echo "✅ Built: ${APP_DIR}"
echo "Run: open \"${APP_DIR}\""