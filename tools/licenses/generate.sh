#!/bin/sh
# Writes THIRD-PARTY-LICENSES.txt: the licenses of every crate in the app's build
# (cargo-about, for the three release targets) and of the code and fonts that do not
# come from crates. The release packages carry it next to the app.
#
#   tools/licenses/generate.sh           rewrite the file
#   tools/licenses/generate.sh --check   fail if the file is out of date (release.yml)
#
# Needs cargo-about 0.9 (`cargo install cargo-about --locked`). Run it after changing
# dependencies, and commit the result.
set -eu
cd "$(dirname "$0")/../.."

out=THIRD-PARTY-LICENSES.txt
tmp=$(mktemp)
trap 'rm -f "$tmp" "$tmp.crates"' EXIT

cargo about generate --config tools/licenses/about.toml -m crates/app/Cargo.toml \
    tools/licenses/about.hbs -o "$tmp.crates"

section() { # title, where it is used, license file
    printf '\n================================================================================\n%s\n\n%s\n\n' "$1" "$2"
    cat "$3"
}

{
    cat <<'HEADER'
Third-party licenses
====================

Watershed is licensed under MIT OR Apache-2.0 (LICENSE-MIT, LICENSE-APACHE). It is
built on the work below, whose licenses ask for their notices to travel with the
program. The separation model is not part of Watershed: it is downloaded from its
original source on first use and declares no license.

Part 1: code and fonts that do not come from Rust crates.
Part 2: the Rust crates in the build, grouped by license.
HEADER
    printf '\n\nPART 1\n'
    section "BS-RoFormer, by Phil Wang (lucidrains)" \
        "The model in crates/roformer is a Rust port of this PyTorch implementation of
BS-RoFormer (https://github.com/lucidrains/BS-RoFormer), as included in
Music-Source-Separation-Training." \
        tools/licenses/extra/bs-roformer-lucidrains.txt
    section "Music-Source-Separation-Training, by Roman Solovyev (ZFTurbo)" \
        "The model code and the chunked separation (demix) are ported from
https://github.com/ZFTurbo/Music-Source-Separation-Training." \
        tools/licenses/extra/music-source-separation-training.txt
    section "Geist and Geist Mono" \
        "The app's fonts (https://github.com/vercel/geist-font), embedded in the program." \
        assets/fonts/OFL-Geist.txt
    section "Material Symbols, by Google" \
        "The app's icons (https://github.com/google/material-design-icons), a subset of
Material Symbols Rounded embedded in the program." \
        assets/fonts/LICENSE-MaterialSymbols.txt
    printf '\n\nPART 2\n'
    cat "$tmp.crates"
} > "$tmp"

if [ "${1:-}" = "--check" ]; then
    if ! cmp -s "$tmp" "$out"; then
        echo "$out is out of date: run tools/licenses/generate.sh and commit it" >&2
        exit 1
    fi
    echo "$out is up to date"
else
    cp "$tmp" "$out"
    echo "wrote $out"
fi
