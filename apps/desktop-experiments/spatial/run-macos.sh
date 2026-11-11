#!/bin/sh
# Build an isolated app bundle; no runtime or session state is loaded.
set -eu
experiment_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cargo build --locked --manifest-path "$experiment_dir/Cargo.toml"
bundle_dir="$experiment_dir/target/AstridSpatial.app"
mkdir -p "$bundle_dir/Contents/MacOS"
cp "$experiment_dir/target/debug/astrid-spatial" "$bundle_dir/Contents/MacOS/AstridSpatial"
cat > "$bundle_dir/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>AstridSpatial</string>
<key>CFBundleIdentifier</key><string>dev.astrid.spatial-experiment</string>
<key>CFBundleName</key><string>Astrid Spatial</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
exec open "$bundle_dir" --args "$@"
