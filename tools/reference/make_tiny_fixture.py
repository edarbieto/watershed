"""Make the tiny BS-RoFormer that tests crates/roformer in CI.

CI has neither a GPU nor the real checkpoint, so this builds PyTorch's model with the
real model's options, but very small and with random weights, and runs it on seeded
audio. Only rerun it when the fixture has to change; the output is kept in git. The
sizes here must match `tiny_config` in crates/roformer/tests/equivalence.rs.

Usage:
    uv run make_tiny_fixture.py
"""

from pathlib import Path

import dump_tensors
import separate

OUT = Path(__file__).parent.parent.parent / "crates/roformer/tests/fixtures"
HOP = 16
FRAMES = 32


def main() -> None:
    separate.ensure_msst()

    # Imported late: these need torch and MSST on sys.path.
    import torch
    from models.bs_roformer.bs_roformer import BSRoformer

    torch.manual_seed(0)
    model = BSRoformer(
        dim=16,
        depth=2,
        stereo=True,
        num_stems=2,
        time_transformer_depth=1,
        freq_transformer_depth=1,
        linear_transformer_depth=0,
        freqs_per_bands=(2, 2, 4, 8, 17),
        dim_head=8,
        heads=2,
        flash_attn=True,
        dim_freqs_in=33,
        stft_n_fft=64,
        stft_hop_length=HOP,
        stft_win_length=64,
        stft_normalized=False,
        mask_estimator_depth=2,
        mlp_expansion_factor=4,
    ).eval()

    # Two tones and some noise, with a different balance per channel.
    time = torch.arange(HOP * (FRAMES - 1)) / 256
    left = 0.4 * torch.sin(2 * torch.pi * 5 * time) + 0.05 * torch.randn(time.shape)
    right = 0.3 * torch.sin(2 * torch.pi * 17 * time) + 0.05 * torch.randn(time.shape)
    audio = torch.stack([left, right])[None]

    OUT.mkdir(parents=True, exist_ok=True)
    torch.save(model.state_dict(), OUT / "tiny.ckpt")
    dump_tensors.dump(model, audio, OUT / "tiny.safetensors")


if __name__ == "__main__":
    main()
