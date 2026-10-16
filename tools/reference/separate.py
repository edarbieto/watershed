"""Reference stem separation with the original PyTorch code.

Runs ZFTurbo's Music-Source-Separation-Training (MSST) inference code, unmodified,
pinned to a known commit. Its outputs are the ground truth that the native Rust
backend is validated against, and they double as test stems for the player.

Usage:
    uv run separate.py SONG_OR_FOLDER [...] --out OUT_DIR
"""

import argparse
import hashlib
import os
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
MSST_DIR = HERE / ".msst"
MSST_REPO = "https://github.com/ZFTurbo/Music-Source-Separation-Training.git"
MSST_COMMIT = "5a3e16d1f7e6ac489bddbeb25069595527e170ce"
MODELS_DIR = HERE / "models"

MODELS = {
    "bs-roformer-sw": {
        "model_type": "bs_roformer",
        "base_url": "https://huggingface.co/FractalSurfer/BS-RoFormer-SW-6stem/resolve/main",
        "config": ("BS-Rofo-SW-Fixed.yaml", "f9fada9f94e5ba2d2e4600196299459294bc5f532b314c209cc156ac63e4329b"),
        "checkpoint": ("BS-Rofo-SW-Fixed.ckpt", "24e7d35ee9c64415673d3fd33e06a67cac2c103c5df6267ba1576459c775916e"),
    },
}

AUDIO_EXTENSIONS = {".mp3", ".flac", ".wav", ".ogg", ".m4a"}


def ensure_msst() -> None:
    if not MSST_DIR.exists():
        print(f"Cloning MSST into {MSST_DIR}")
        subprocess.run(["git", "clone", "--quiet", MSST_REPO, str(MSST_DIR)], check=True)
    head = subprocess.run(
        ["git", "-C", str(MSST_DIR), "rev-parse", "HEAD"], check=True, capture_output=True, text=True
    ).stdout.strip()
    if head != MSST_COMMIT:
        subprocess.run(["git", "-C", str(MSST_DIR), "fetch", "--quiet", "origin", MSST_COMMIT], check=True)
        subprocess.run(["git", "-C", str(MSST_DIR), "checkout", "--quiet", MSST_COMMIT], check=True)
    sys.path.insert(0, str(MSST_DIR))


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def ensure_file(base_url: str, name: str, expected_sha: str) -> Path:
    path = MODELS_DIR / name
    if path.exists() and sha256(path) == expected_sha:
        return path
    MODELS_DIR.mkdir(parents=True, exist_ok=True)
    print(f"Downloading {name}")
    tmp = path.with_suffix(path.suffix + ".part")
    urllib.request.urlretrieve(f"{base_url}/{name}", tmp)
    actual = sha256(tmp)
    if actual != expected_sha:
        tmp.unlink()
        raise SystemExit(f"sha256 mismatch for {name}: expected {expected_sha}, got {actual}")
    tmp.replace(path)
    return path


def collect_inputs(paths: list[Path]) -> list[Path]:
    songs = []
    for p in paths:
        if p.is_dir():
            songs += sorted(f for f in p.iterdir() if f.suffix.lower() in AUDIO_EXTENSIONS)
        else:
            songs.append(p)
    return songs


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("inputs", nargs="+", type=Path, help="audio files or folders")
    parser.add_argument("--out", type=Path, required=True, help="output folder, one subfolder per song")
    parser.add_argument("--model", default="bs-roformer-sw", choices=MODELS)
    parser.add_argument("--cpu", action="store_true", help="force CPU inference")
    args = parser.parse_args()

    ensure_msst()
    spec = MODELS[args.model]
    config_path = ensure_file(spec["base_url"], *spec["config"])
    ckpt_path = ensure_file(spec["base_url"], *spec["checkpoint"])

    # Run the few operators that MPS lacks on the CPU instead of failing. It must be
    # set before torch is imported.
    os.environ.setdefault("PYTORCH_ENABLE_MPS_FALLBACK", "1")

    # Imported late: these need torch and MSST on sys.path.
    import numpy as np
    import soundfile as sf
    import soxr
    import torch
    from utils.model_utils import demix
    from utils.settings import get_model_from_config

    if args.cpu:
        device = torch.device("cpu")
    elif torch.cuda.is_available():
        device = torch.device("cuda")
    elif torch.backends.mps.is_available():
        device = torch.device("mps")
    else:
        device = torch.device("cpu")
    print(f"Device: {device}" + (f" ({torch.cuda.get_device_name()})" if device.type == "cuda" else ""))

    model, config = get_model_from_config(spec["model_type"], str(config_path))
    state = torch.load(ckpt_path, map_location="cpu", weights_only=True)
    model.load_state_dict(state.get("state_dict", state) if isinstance(state, dict) else state)
    model = model.to(device).eval()
    # MSST's demix uses torch.cuda.amp.autocast, which only does anything on CUDA.
    if device.type != "cuda":
        config.training.use_amp = False

    sample_rate = config.audio.sample_rate
    for song in collect_inputs(args.inputs):
        mix, sr = sf.read(song, dtype="float32", always_2d=True)  # (time, channels)
        if sr != sample_rate:
            mix = soxr.resample(mix, sr, sample_rate)
        if mix.shape[1] == 1:
            mix = np.repeat(mix, 2, axis=1)

        print(f"Separating {song.name} ({mix.shape[0] / sample_rate:.0f} s)")
        start = time.perf_counter()
        stems = demix(config, model, mix.T, device, spec["model_type"], pbar=True)
        elapsed = time.perf_counter() - start

        # FLAC stores integers, so anything above full scale would clip. Stems often
        # exceed it; scale all of them by the same factor to keep their balance.
        peak = max(float(np.abs(audio).max()) for audio in stems.values())
        scale = 1.0 if peak <= 1.0 else 0.999 / peak

        out_dir = args.out / song.stem
        out_dir.mkdir(parents=True, exist_ok=True)
        for name, audio in stems.items():
            sf.write(out_dir / f"{name}.flac", audio.T * scale, sample_rate, subtype="PCM_24")
        gain_note = "" if scale == 1.0 else f", scaled by {20 * np.log10(scale):.1f} dB to avoid clipping"
        print(f"  done in {elapsed:.1f} s{gain_note} -> {out_dir}")


if __name__ == "__main__":
    main()
