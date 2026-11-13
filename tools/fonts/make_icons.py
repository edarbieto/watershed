# /// script
# requires-python = ">=3.10"
# dependencies = ["fonttools>=4.50"]
# ///
"""Builds the app's icon font: Material Symbols Rounded, fixed at one style and cut down
to the icons in ICONS, plus `crates/app/src/icons.rs` with a constant per icon.

Download the variable font and its codepoints from google/material-design-icons
(`variablefont/MaterialSymbolsRounded[FILL,GRAD,opsz,wght].ttf` and `.codepoints`),
then, from the repo root:

    uv run tools/fonts/make_icons.py <font.ttf> <font.codepoints>

To add an icon: add a line to ICONS (names as on fonts.google.com/icons) and run it again.
"""

import sys
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

# Rust constant -> Material Symbols name.
ICONS = {
    "PLAY": "play_arrow",
    "PAUSE": "pause",
    "STOP": "stop",
    "TO_START": "skip_previous",
    "BACK": "fast_rewind",
    "FORWARD": "fast_forward",
    "LOOP": "repeat",
    "CLOSE": "close",
    "CHECK": "check_circle",
    "UNDO": "undo",
    "REDO": "redo",
    "DROP_DOWN": "arrow_drop_down",
    "SEARCH": "search",
    "DELETE": "delete",
    "RESTORE": "restore_from_trash",
    "RESET": "restart_alt",
    "ZOOM_IN": "zoom_in",
    "ZOOM_OUT": "zoom_out",
    "ZOOM_FIT": "fit_screen",
    "LOCK": "lock",
    "MOVE": "swap_horiz",
    "MUTE": "volume_off",
    "SOLO": "headphones",
    "UNSTEADY": "bolt",
    # Material Symbols has no magnet: a grid with guides for snapping to the grid.
    "SNAP": "grid_guides",
}

# The one style the app uses: outlined (not filled), regular weight, drawn for ~20 px.
STYLE = {"FILL": 0, "wght": 400, "GRAD": 0, "opsz": 20}

ROOT = Path(__file__).resolve().parents[2]
FONT_OUT = ROOT / "assets" / "fonts" / "MaterialSymbolsRounded-Watershed.ttf"
RUST_OUT = ROOT / "crates" / "app" / "src" / "icons.rs"


def main() -> None:
    font_path, codepoints_path = sys.argv[1], sys.argv[2]
    codepoints = {}
    for line in Path(codepoints_path).read_text(encoding="utf-8").splitlines():
        name, code = line.split()
        codepoints[name] = int(code, 16)
    missing = [name for name in ICONS.values() if name not in codepoints]
    if missing:
        sys.exit(f"not in the font: {', '.join(missing)}")

    font = TTFont(font_path)
    font = instancer.instantiateVariableFont(font, STYLE)
    options = subset.Options()
    options.layout_features = []  # icons by codepoint only: no ligatures
    options.name_IDs = ["*"]
    options.notdef_outline = True
    subsetter = subset.Subsetter(options)
    subsetter.populate(unicodes=[codepoints[name] for name in ICONS.values()])
    subsetter.subset(font)
    FONT_OUT.parent.mkdir(parents=True, exist_ok=True)
    font.save(FONT_OUT)

    lines = [
        "//! Icons: characters of the icon font (Material Symbols Rounded, cut down by",
        "//! `tools/fonts/make_icons.py`, which writes this file). They are text: they take",
        "//! the color and size of the text around them.",
        "",
    ]
    for const, name in ICONS.items():
        lines.append(f"/// `{name}`")
        lines.append(f'pub const {const}: &str = "\\u{{{codepoints[name]:x}}}";')
    RUST_OUT.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    print(f"{FONT_OUT.name}: {FONT_OUT.stat().st_size} bytes, {len(ICONS)} icons")


if __name__ == "__main__":
    main()
