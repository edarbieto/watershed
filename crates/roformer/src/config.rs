//! Model parameters, named as in the checkpoint's `.yaml`.

/// What the checkpoint's weights do not say by themselves.
#[derive(Debug, Clone)]
pub struct Config {
    pub dim: usize,
    pub depth: usize,
    pub num_stems: usize,
    pub heads: usize,
    pub dim_head: usize,
    pub time_transformer_depth: usize,
    pub freq_transformer_depth: usize,
    /// STFT bins per band, for one channel.
    pub freqs_per_bands: Vec<usize>,
    pub mask_estimator_depth: usize,
    pub mlp_expansion_factor: usize,
}

impl Config {
    /// BS-RoFormer SW. Stands in until the `.yaml` is read.
    pub fn bs_roformer_sw() -> Self {
        let mut freqs_per_bands = vec![2; 24];
        freqs_per_bands.extend([4; 12]);
        freqs_per_bands.extend([12; 8]);
        freqs_per_bands.extend([24; 8]);
        freqs_per_bands.extend([48; 8]);
        freqs_per_bands.extend([128, 129]);
        Self {
            dim: 256,
            depth: 12,
            num_stems: 6,
            heads: 8,
            dim_head: 64,
            time_transformer_depth: 1,
            freq_transformer_depth: 1,
            freqs_per_bands,
            mask_estimator_depth: 2,
            mlp_expansion_factor: 4,
        }
    }

    /// Size of each band in the model's input: complex values of both channels.
    pub(crate) fn band_sizes(&self) -> Vec<usize> {
        self.freqs_per_bands.iter().map(|f| f * 2 * 2).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bands_cover_the_whole_spectrum() {
        let config = Config::bs_roformer_sw();
        assert_eq!(config.freqs_per_bands.len(), 62);
        assert_eq!(config.freqs_per_bands.iter().sum::<usize>(), 1025);
        assert_eq!(config.band_sizes().iter().sum::<usize>(), 4100);
    }
}
