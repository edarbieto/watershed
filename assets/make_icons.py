"""Render the app icons from the SVGs in this folder.

Needs `resvg` (`cargo install resvg`) and, for the `.icns`, macOS's `iconutil`.
Run from anywhere: `python3 assets/make_icons.py`. Writes:

- icon-256.png: window icon on Windows and Linux, and the AppImage icon
- icon-macos-512.png: window (Dock) icon on macOS
- icon.ico: the Windows .exe icon (PNG entries, 16 to 256 px)
- icon.icns: the macOS .app icon
"""

import shutil
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ICO_SIZES = [16, 24, 32, 48, 64, 128, 256]
ICNS_SIZES = [16, 32, 128, 256, 512]


def render(svg: Path, size: int, out: Path) -> None:
    subprocess.run(["resvg", "-w", str(size), "-h", str(size), str(svg), str(out)], check=True)


def make_ico(svg: Path, out: Path) -> None:
    with tempfile.TemporaryDirectory() as tmp:
        images = []
        for size in ICO_SIZES:
            png = Path(tmp) / f"{size}.png"
            render(svg, size, png)
            images.append((size, png.read_bytes()))
    header = struct.pack("<HHH", 0, 1, len(images))
    offset = len(header) + 16 * len(images)
    entries, data = b"", b""
    for size, png in images:
        # 0 means 256 in the width and height bytes
        entries += struct.pack("<BBBBHHII", size % 256, size % 256, 0, 0, 1, 32, len(png), offset)
        offset += len(png)
        data += png
    out.write_bytes(header + entries + data)


def make_icns(svg: Path, out: Path) -> None:
    with tempfile.TemporaryDirectory() as tmp:
        iconset = Path(tmp) / "icon.iconset"
        iconset.mkdir()
        for size in ICNS_SIZES:
            render(svg, size, iconset / f"icon_{size}x{size}.png")
            render(svg, size * 2, iconset / f"icon_{size}x{size}@2x.png")
        subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(out)], check=True)


def main() -> None:
    if shutil.which("resvg") is None:
        sys.exit("resvg not found: cargo install resvg")
    render(HERE / "icon.svg", 256, HERE / "icon-256.png")
    render(HERE / "icon-macos.svg", 512, HERE / "icon-macos-512.png")
    make_ico(HERE / "icon.svg", HERE / "icon.ico")
    if shutil.which("iconutil"):
        make_icns(HERE / "icon-macos.svg", HERE / "icon.icns")
    else:
        print("iconutil not found (macOS only): icon.icns not updated")


if __name__ == "__main__":
    main()
