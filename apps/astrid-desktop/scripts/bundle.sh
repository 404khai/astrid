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
# The app icon: an Icon Composer file (Liquid Glass layers, light/dark/tinted),
# compiled by actool into Assets.car plus a fallback .icns. It is read from
# ASTRID_ICON, else AstridIcon.icon at the repo root; without one the app
# keeps the generic icon.
ICON="${ASTRID_ICON:-$PWD/../../AstridIcon.icon}"
ICON_KEYS=""
if [ -d "$ICON" ]; then
  mkdir -p "$APP/Contents/Resources"
  xcrun actool "$ICON" --compile "$APP/Contents/Resources" --app-icon AstridIcon \
    --platform macosx --minimum-deployment-target 26.0 \
    --output-partial-info-plist "$APP/Contents/icon-partial.plist" >/dev/null
  rm -f "$APP/Contents/icon-partial.plist"
  ICON_KEYS="<key>CFBundleIconName</key><string>AstridIcon</string><key>CFBundleIconFile</key><string>AstridIcon</string>"
else
  echo "no app icon at $ICON; using the generic icon" >&2
fi
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
  <key>NSMicrophoneUsageDescription</key><string>Astrid listens only in voice mode. Audio stays on this Mac.</string>
  <key>NSSpeechRecognitionUsageDescription</key><string>Astrid turns what you say into text on this Mac, without sending audio anywhere.</string>
  ICON_KEYS_HERE
</dict>
</plist>
PLIST
# Fill in the icon keys (empty without an icon).
sed -i '' "s|  ICON_KEYS_HERE|  $ICON_KEYS|" "$APP/Contents/Info.plist"
codesign --force --sign - "$APP" >/dev/null
echo "$APP"
