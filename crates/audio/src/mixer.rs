use std::sync::Arc;

use crate::Song;

/// Largest block the mixer renders at once, in frames. Longer device buffers are
/// processed in several blocks.
pub(crate) const MAX_BLOCK: usize = 2048;

/// A song plus the mixer state that belongs to the audio thread. Built on the UI
/// thread so that every allocation happens there.
pub(crate) struct Voice {
    pub song: Arc<Song>,
    /// Per-track left/right gains applied at the end of the previous block.
    gains: Vec<[f32; 2]>,
}

impl Voice {
    pub fn new(song: Arc<Song>) -> Self {
        let any_solo = song.tracks.iter().any(|t| t.params.solo());
        let gains = song
            .tracks
            .iter()
            .map(|t| t.params.channel_gains(any_solo))
            .collect();
        Self { song, gains }
    }

    /// Jumps to the current gains without a ramp. For a seek, where the output starts
    /// over anyway.
    pub fn snap_gains(&mut self) {
        let any_solo = self.song.tracks.iter().any(|t| t.params.solo());
        for (track, current) in self.song.tracks.iter().zip(self.gains.iter_mut()) {
            *current = track.params.channel_gains(any_solo);
        }
    }

    /// Adds `out.len() / 2` stereo frames starting at `position` into `out`.
    ///
    /// Gain changes are ramped linearly across the block to avoid clicks. Frames past
    /// the end of the song are left untouched.
    pub fn mix_into(&mut self, position: usize, out: &mut [f32]) {
        let frames = out.len() / 2;
        let available = self.song.frames().saturating_sub(position).min(frames);
        if available == 0 {
            return;
        }
        let any_solo = self.song.tracks.iter().any(|t| t.params.solo());

        for (track, current) in self.song.tracks.iter().zip(self.gains.iter_mut()) {
            let target = track.params.channel_gains(any_solo);
            let start = *current;
            *current = target;
            if start == [0.0, 0.0] && target == [0.0, 0.0] {
                continue;
            }

            let step = [
                (target[0] - start[0]) / frames as f32,
                (target[1] - start[1]) / frames as f32,
            ];
            let src = &track.samples[position * 2..(position + available) * 2];
            for (i, (o, s)) in out
                .as_chunks_mut::<2>()
                .0
                .iter_mut()
                .zip(src.as_chunks::<2>().0)
                .enumerate()
            {
                // Ends exactly on the target so the next block continues seamlessly.
                let t = (i + 1) as f32;
                o[0] += s[0] * (start[0] + step[0] * t);
                o[1] += s[1] * (start[1] + step[1] * t);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A song where track `i` is a constant `values[i]` on both channels.
    fn song(values: &[f32], frames: usize) -> Arc<Song> {
        let tracks = values
            .iter()
            .enumerate()
            .map(|(i, &v)| (format!("t{i}"), vec![v; frames * 2]))
            .collect();
        Arc::new(Song::new(48_000, tracks))
    }

    /// Mixes one block to settle the gain ramps, then returns the next block.
    fn render(voice: &mut Voice, position: usize, frames: usize) -> Vec<f32> {
        let mut out = vec![0.0; frames * 2];
        voice.mix_into(position, &mut out);
        out.fill(0.0);
        voice.mix_into(position, &mut out);
        out
    }

    #[test]
    fn sums_tracks_at_unity() {
        let mut voice = Voice::new(song(&[0.25, 0.5], 64));
        let out = render(&mut voice, 0, 16);
        assert!(out.iter().all(|&s| (s - 0.75).abs() < 1e-6));
    }

    #[test]
    fn mute_and_solo() {
        let song = song(&[0.25, 0.5, 0.125], 64);
        let mut voice = Voice::new(song.clone());

        song.tracks[0].params.set_mute(true);
        assert!((render(&mut voice, 0, 16)[0] - 0.625).abs() < 1e-6);

        // Solo wins over the unsoloed tracks; mute still wins over solo.
        song.tracks[1].params.set_solo(true);
        assert!((render(&mut voice, 0, 16)[0] - 0.5).abs() < 1e-6);
        song.tracks[0].params.set_solo(true);
        assert!((render(&mut voice, 0, 16)[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn gain_and_pan() {
        let song = song(&[1.0], 64);
        let mut voice = Voice::new(song.clone());

        song.tracks[0].params.set_gain(0.5);
        song.tracks[0].params.set_pan(1.0);
        let out = render(&mut voice, 0, 16);
        assert_eq!((out[0], out[1]), (0.0, 0.5));

        song.tracks[0].params.set_pan(-0.5);
        let out = render(&mut voice, 0, 16);
        assert_eq!((out[0], out[1]), (0.5, 0.25));
    }

    #[test]
    fn gain_changes_are_ramped() {
        let song = song(&[1.0], 64);
        let mut voice = Voice::new(song.clone());
        song.tracks[0].params.set_gain(0.0);

        let mut out = vec![0.0; 8 * 2];
        voice.mix_into(0, &mut out);
        let left: Vec<f32> = out.iter().step_by(2).copied().collect();
        assert!(left.windows(2).all(|w| w[1] < w[0]), "{left:?}");
        assert!(left[0] > 0.8 && left[7] == 0.0, "{left:?}");
    }

    #[test]
    fn stops_at_end_of_song() {
        let mut voice = Voice::new(song(&[1.0], 10));
        let out = render(&mut voice, 6, 8);
        let left: Vec<f32> = out.iter().step_by(2).copied().collect();
        assert_eq!(left, [1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0]);
        assert!(render(&mut voice, 20, 8).iter().all(|&s| s == 0.0));
    }

    #[test]
    fn pads_shorter_tracks() {
        let song = Song::new(
            48_000,
            vec![("a".into(), vec![0.0; 20]), ("b".into(), vec![0.0; 8])],
        );
        assert_eq!(song.frames(), 10);
        assert!(song.tracks.iter().all(|t| t.samples.len() == 20));
    }
}
