#!/usr/bin/env bash
set -e

APP_NAME="Marktune"
IDENTIFIER="com.marktune.app"
VERSION="0.2.0"
TARGET_DIR="target/release"
DIST_DIR="dist"
APP_BUNDLE="$DIST_DIR/$APP_NAME.app"
DMG_PATH="$DIST_DIR/$APP_NAME.dmg"

echo "==> Building release binary..."
cargo build --release

echo "==> Cleaning old outputs..."
rm -rf "$APP_BUNDLE" "$DMG_PATH"
mkdir -p "$DIST_DIR"

echo "==> Creating macOS .app bundle..."
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources"

cp "$TARGET_DIR/marktune" "$APP_BUNDLE/Contents/MacOS/$APP_NAME"
chmod +x "$APP_BUNDLE/Contents/MacOS/$APP_NAME"

if [ -f "config.json" ]; then
    cp "config.json" "$APP_BUNDLE/Contents/Resources/"
fi

cat <<EOF > "$APP_BUNDLE/Contents/Info.plist"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>$APP_NAME</string>
    <key>CFBundleIdentifier</key>
    <string>$IDENTIFIER</string>
    <key>CFBundleName</key>
    <string>$APP_NAME</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.13</string>
    <key>LSUIElement</key>
    <true/>
</dict>
</plist>
EOF

echo "==> Packaging into .dmg..."
hdiutil create -volname "$APP_NAME" -srcfolder "$APP_BUNDLE" -ov -format UDZO "$DMG_PATH"

echo "==> Done! DMG created at: $DMG_PATH"
