#!/bin/sh
# Builds "Astrid Presence.app" so macOS's Screen Recording grant belongs to
# Astrid's presence host (ADR 0027), not to the terminal that launches it.
# The runtime starts the bundle through LaunchServices (`open`) so TCC
# attributes capture to this app. Ad-hoc signed: rebuilding changes the code
# signature, and macOS may ask for the grant again.
set -eu
cd "$(dirname "$0")/.."
swift build -c release
APP="$PWD/.build/Astrid Presence.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
cp .build/release/astrid-presence "$APP/Contents/MacOS/astrid-presence"
cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key><string>dev.astrid.presence</string>
  <key>CFBundleName</key><string>Astrid Presence</string>
  <key>CFBundleDisplayName</key><string>Astrid Presence</string>
  <key>CFBundleExecutable</key><string>astrid-presence</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>LSMinimumSystemVersion</key><string>14.0</string>
  <key>LSUIElement</key><true/>
  <key>NSMicrophoneUsageDescription</key><string>Astrid listens only while you hold the mic. Audio stays on this Mac.</string>
  <key>NSSpeechRecognitionUsageDescription</key><string>Astrid turns what you say into text on this Mac, without sending audio anywhere.</string>
</dict>
</plist>
PLIST
# Ad-hoc signing gives every build a new code hash, and macOS ties Screen
# Recording / Accessibility grants to the signature, so each rebuild would lose
# them. A designated requirement on the bundle id keeps grants across rebuilds.
# Set ASTRID_SIGN_IDENTITY to a real signing identity to use that instead.
if [ -n "${ASTRID_SIGN_IDENTITY:-}" ]; then
  codesign --force --sign "$ASTRID_SIGN_IDENTITY" "$APP" >/dev/null
else
  codesign --force --sign - --identifier dev.astrid.presence \
    -r='designated => identifier "dev.astrid.presence"' "$APP" >/dev/null
fi
echo "$APP"
