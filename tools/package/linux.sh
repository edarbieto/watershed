#!/bin/sh
# Builds Watershed-<version>-linux-x86_64.AppImage in dist/. Linux x86_64 only.
#
#   tools/package/linux.sh
#
# The binary links against the glibc it is built with, so release builds run on
# an older distribution (see release.yml) to work on newer ones. The window
# system, audio (ALSA) and Vulkan libraries come from the user's system.
set -eu
cd "$(dirname "$0")/../.."

# Pinned AppImage tools: appimagetool and the runtime it puts at the start of the file.
tool_url=https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage
tool_sha=ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0
runtime_url=https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-x86_64
runtime_sha=2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
[ -n "$version" ] || { echo "could not read the version from Cargo.toml" >&2; exit 1; }

cargo build --release --locked -p watershed

tools=target/appimage-tools
mkdir -p "$tools"
fetch() { # url sha256 file
    if ! echo "$2  $3" | sha256sum -c --status 2>/dev/null; then
        curl -fsSL -o "$3" "$1"
        echo "$2  $3" | sha256sum -c
    fi
}
fetch "$tool_url" "$tool_sha" "$tools/appimagetool"
fetch "$runtime_url" "$runtime_sha" "$tools/runtime"
chmod +x "$tools/appimagetool"

appdir=target/Watershed.AppDir
rm -rf "$appdir"
mkdir -p "$appdir/usr/bin" "$appdir/usr/share/icons/hicolor/256x256/apps"
cp target/release/watershed "$appdir/usr/bin/watershed"
cp assets/icon-256.png "$appdir/usr/share/icons/hicolor/256x256/apps/watershed.png"
cp assets/icon-256.png "$appdir/watershed.png"
ln -s usr/bin/watershed "$appdir/AppRun"
cat > "$appdir/watershed.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Watershed
Comment=Separate songs into stems and practice over them
Exec=watershed %F
Icon=watershed
Categories=AudioVideo;Audio;Music;
Terminal=false
DESKTOP

mkdir -p dist
out=dist/Watershed-$version-linux-x86_64.AppImage
# Extract and run: CI machines have no FUSE to mount appimagetool itself.
ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$tools/appimagetool" \
    --runtime-file "$tools/runtime" "$appdir" "$out"
echo "built $out"
