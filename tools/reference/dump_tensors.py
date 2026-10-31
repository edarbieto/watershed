"""Dump the tensors of one chunk for validating the Rust BS-RoFormer.

Runs a real chunk of a song through the PyTorch model on the CPU in f32 and saves the
input, the output of each layer and the final audio to a safetensors file.

Usage:
    uv run dump_tensors.py SONG --out fixtures/chunk.safetensors [--start SECONDS]
"""

import argparse
from pathlib import Path

import separate


def dump(model, audio, out: Path) -> None:
    """Runs `audio` (1, channels, time) through `model`, saving every layer to `out`."""
    import torch
    from safetensors.torch import save_file

    tensors = {"audio": audio}

    # The same STFT the model does at the start of forward, merged the same way:
    # (batch, frequency * channel, time, complex).
    stft = torch.stft(
        audio.reshape(-1, audio.shape[-1]),
        **model.stft_kwargs,
        window=model.stft_window_fn(),
        return_complex=True,
    )
    stft = torch.view_as_real(stft).permute(1, 2, 3, 0)[None].permute(0, 1, 4, 2, 3)
    tensors["stft"] = stft.reshape(1, -1, stft.shape[3], stft.shape[4]).contiguous()

    def save(name):
        def hook(_module, _input, output):
            tensors[name] = output.detach().clone()

        return hook

    def save_time(name, frames):
        # The time transformer sees (batch * frequency, time, dim).
        def hook(_module, _input, output):
            tensors[name] = output.detach().reshape(1, -1, frames, output.shape[-1]).clone()

        return hook

    def save_freq(name, frames):
        # The frequency transformer sees (batch * time, frequency, dim).
        def hook(_module, _input, output):
            tensors[name] = output.detach().reshape(1, frames, -1, output.shape[-1]).clone()

        return hook

    frames = stft.shape[-2]
    model.band_split.register_forward_hook(save("band_split"))
    for i, (time_transformer, freq_transformer) in enumerate(model.layers):
        time_transformer.register_forward_hook(save_time(f"time_{i}", frames))
        freq_transformer.register_forward_hook(save_freq(f"freq_{i}", frames))
    model.final_norm.register_forward_hook(save("final_norm"))
    for i, estimator in enumerate(model.mask_estimators):
        estimator.register_forward_hook(save(f"mask_{i}"))

    with torch.inference_mode():
        tensors["output"] = model(audio).clone()

    out.parent.mkdir(parents=True, exist_ok=True)
    save_file(tensors, out)
    for name, tensor in tensors.items():
        print(f"{name}: {tuple(tensor.shape)}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("song", type=Path)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--start", type=float, default=60.0, help="where the chunk starts, in seconds")
    parser.add_argument("--model", default="bs-roformer-sw", choices=separate.MODELS)
    args = parser.parse_args()

    separate.ensure_msst()
    spec = separate.MODELS[args.model]
    config_path = separate.ensure_file(spec["base_url"], *spec["config"])
    ckpt_path = separate.ensure_file(spec["base_url"], *spec["checkpoint"])

    # Imported late: these need torch and MSST on sys.path.
    import numpy as np
    import soundfile as sf
    import soxr
    import torch
    from utils.settings import get_model_from_config

    model, config = get_model_from_config(spec["model_type"], str(config_path))
    state = torch.load(ckpt_path, map_location="cpu", weights_only=True)
    model.load_state_dict(state.get("state_dict", state) if isinstance(state, dict) else state)
    model = model.eval()

    sample_rate = config.audio.sample_rate
    mix, sr = sf.read(args.song, dtype="float32", always_2d=True)  # (time, channels)
    if sr != sample_rate:
        mix = soxr.resample(mix, sr, sample_rate)
    if mix.shape[1] == 1:
        mix = np.repeat(mix, 2, axis=1)
    begin = int(args.start * sample_rate)
    chunk = mix[begin : begin + config.audio.chunk_size].T.copy()
    if chunk.shape[1] != config.audio.chunk_size:
        raise SystemExit("the song is too short for a chunk at that start")
    audio = torch.from_numpy(chunk)[None]  # (1, channels, time)

    dump(model, audio, args.out)


if __name__ == "__main__":
    main()
