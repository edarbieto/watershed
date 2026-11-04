#!/bin/sh
# Builds Watershed.app and a .dmg with it in dist/.
#
#   tools/package/macos.sh [--no-dmg]
#
# The .app is ad-hoc signed only (no Apple developer account), so on another Mac
# Gatekeeper blocks the first open: see the release notes. Also handy during
# development: an .app shows in Cmd+Tab and the Dock like any other app.
set -eu
cd "$(dirname "$0")/../.."

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
[ -n "$version" ] || { echo "could not read the version from Cargo.toml" >&2; exit 1; }

cargo build --release --locked -p watershed

app=dist/Watershed.app
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp target/release/watershed "$app/Contents/MacOS/watershed"
cp assets/icon.icns "$app/Contents/Resources/icon.icns"
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>Watershed</string>
    <key>CFBundleDisplayName</key><string>Watershed</string>
    <key>CFBundleIdentifier</key><string>app.watershed.Watershed</string>
    <key>CFBundleExecutable</key><string>watershed</string>
    <key>CFBundleIconFile</key><string>icon</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleVersion</key><string>$version</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>LSApplicationCategoryType</key><string>public.app-category.music</string>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST
codesign --force --sign - "$app"
echo "built $app ($version)"

[ "${1:-}" = "--no-dmg" ] && exit 0

dmg=dist/Watershed-$version-macos-$(uname -m).dmg
stage=$(mktemp -d)
cp -R "$app" "$stage/"
ln -s /Applications "$stage/Applications"
rm -f "$dmg"
hdiutil create -volname Watershed -srcfolder "$stage" -format UDZO -ov "$dmg" >/dev/null
rm -rf "$stage"
echo "built $dmg"
