#!/bin/sh
# Builds "Astrid.app", the desktop chat app (ADR 0037): its own bundle id
# (dev.astrid.desktop), a normal Dock app, separate from "Astrid Presence.app"
# (dev.astrid.presence, a menu-bar accessory). Quitting or closing one never
# affects the other. Ad-hoc signed.
set -eu
cd "$(dirname "$0")/.."
swift build -c release
APP="$PWD/.build/Astrid.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
cp .build/release/astrid-desktop "$APP/Contents/MacOS/astrid-desktop"
cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key><string>dev.astrid.desktop</string>
  <key>CFBundleName</key><string>Astrid</string>
  <key>CFBundleDisplayName</key><string>Astrid</string>
  <key>CFBundleExecutable</key><string>astrid-desktop</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>LSMinimumSystemVersion</key><string>26.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.productivity</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST
codesign --force --sign - "$APP" >/dev/null
echo "$APP"
