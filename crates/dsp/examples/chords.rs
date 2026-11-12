//! Prints the estimated chords of a folder of stems, bar by bar, on the beats tracked
//! from the audio: `cargo run --release -p watershed-dsp --example chords -- <folder>
//! [--no-vocals] [--bars 16]`. The BPM, bar 1 and beats per bar come from the
//! folder's `.watershed.toml` when it has them; else they are estimated, in 4/4.
//! Tempo and time signature changes are not read. `RATE=48000` resamples the stems
//! first, as the app does to the output device's rate.

use std::path::{Path, PathBuf};
use std::time::Instant;

use watershed_dsp::{
    Anchor, Chroma, Chromagram, OnsetEnvelope, VOCALS_TEMPO_WEIGHT, decode_file, estimate_chords,
    half_bars,
};

/// Bars per printed line.
const BARS_PER_LINE: usize = 4;

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let no_vocals = take_flag(&mut args, "--no-vocals");
    let max_bars = take_option(&mut args, "--bars").map(|b| b as usize);
    let [folder] = args.as_slice() else {
        eprintln!("usage: chords <folder of stems> [--no-vocals] [--bars <n>]");
        std::process::exit(2);
    };
    let folder = Path::new(folder);
    let session = std::fs::read_to_string(folder.join(".watershed.toml")).unwrap_or_default();

    let start = Instant::now();
    let mut onsets = OnsetEnvelope::default();
    let mut treble = Chromagram::default();
    let mut bass = None;
    let mut chroma = Chroma::default();
    for file in stems(folder) {
        let name = file.file_stem().unwrap().to_string_lossy().to_lowercase();
        let audio = decode_file(&file).unwrap_or_else(|e| {
            eprintln!("{}: {e}", file.display());
            std::process::exit(1);
        });
        let audio = match std::env::var("RATE").ok().and_then(|r| r.parse().ok()) {
            Some(rate) => watershed_dsp::resample(audio, rate).unwrap(),
            None => audio,
        };
        let weight = if name.contains("vocals") {
            VOCALS_TEMPO_WEIGHT
        } else {
            1.0
        };
        onsets.add_weighted(
            &OnsetEnvelope::compute(&audio.samples, audio.sample_rate),
            weight,
        );
        if name.contains("drums") {
            continue;
        }
        let (gram, own) = Chromagram::compute(&audio.samples, audio.sample_rate);
        chroma.add(&own);
        if name.contains("bass") {
            bass = Some(Chromagram::compute_bass(&audio.samples, audio.sample_rate));
        } else if !(no_vocals && name.contains("vocals")) {
            treble.add(&gram);
        }
    }
    let analysed = start.elapsed();

    let bpm = number(&session, "bpm").or_else(|| onsets.bpm());
    let Some(bpm) = bpm else {
        println!("no steady beat found");
        return;
    };
    let bar_one = number(&session, "grid_start").or_else(|| onsets.first_beat(bpm));
    let per_bar = number(&session, "beats_per_bar").map_or(4, |b| b as usize);
    let anchors: Vec<Anchor> = bar_one
        .map(|time| Anchor {
            time,
            pace: 1.0,
            tempo: None,
        })
        .into_iter()
        .collect();
    let all = onsets.track_beats(bpm, &anchors);
    // From bar 1 on, as the grid draws it.
    let first = bar_one.map_or(0, |t| {
        all.iter().position(|&b| b >= t - 0.1).unwrap_or(all.len())
    });
    let beats = &all[first..];
    if beats.len() < 2 {
        println!("too few beats");
        return;
    }
    let bar_starts: Vec<usize> = (0..beats.len()).step_by(per_bar).collect();
    let spans = half_bars(beats, &bar_starts);
    let key = chroma.key();
    let flats = key.as_ref().is_some_and(|k| k.key.uses_flats());
    let timed = Instant::now();
    let chords = estimate_chords(&treble, bass.as_ref(), &chroma, &spans);
    let estimated = timed.elapsed();

    println!(
        "{} — {bpm:.1} BPM, {per_bar} beats per bar, bar 1 at {:.2} s, key {} \
         (analysis {analysed:.0?}, chords {estimated:.1?})",
        folder.file_name().unwrap().to_string_lossy(),
        beats[0],
        key.map_or("?".to_owned(), |k| k.key.to_string()),
    );
    let name = |i: usize| {
        chords
            .get(i)
            .copied()
            .flatten()
            .map_or("-".to_owned(), |c| c.name(flats))
    };
    // One cell per bar: its chord, or both halves when they differ.
    let mut cells = Vec::new();
    let mut i = 0;
    while i < spans.len() {
        let time = spans[i].start;
        let whole = spans.get(i + 1).is_none_or(|next| next.on_bar);
        let cell = if whole || name(i) == name(i + 1) {
            name(i)
        } else {
            format!("{} {}", name(i), name(i + 1))
        };
        cells.push((time, cell));
        i += if whole { 1 } else { 2 };
    }
    let shown = max_bars.unwrap_or(cells.len()).min(cells.len());
    for (line, chunk) in cells[..shown].chunks(BARS_PER_LINE).enumerate() {
        let bar = line * BARS_PER_LINE + 1;
        let (minutes, seconds) = ((chunk[0].0 / 60.0) as u32, chunk[0].0 % 60.0);
        let text: Vec<String> = chunk.iter().map(|(_, c)| format!("{c:<9}")).collect();
        println!(
            "{bar:>4} {minutes}:{seconds:05.2}  | {}",
            text.join("| ").trim_end()
        );
    }
}

/// The audio files of `folder`, sorted.
fn stems(folder: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(folder)
        .unwrap_or_else(|e| {
            eprintln!("{}: {e}", folder.display());
            std::process::exit(1);
        })
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| ["ogg", "flac", "wav", "mp3"].contains(&e))
        })
        .collect();
    files.sort();
    files
}

/// A top-level `key = number` line of a TOML file, read without a TOML parser.
fn number(text: &str, key: &str) -> Option<f64> {
    text.lines()
        .take_while(|line| !line.starts_with('['))
        .find_map(|line| {
            let (name, value) = line.split_once('=')?;
            (name.trim() == key).then(|| value.trim().parse().ok())?
        })
}

fn take_flag(args: &mut Vec<String>, name: &str) -> bool {
    let found = args.iter().position(|a| a == name);
    if let Some(i) = found {
        args.remove(i);
    }
    found.is_some()
}

fn take_option(args: &mut Vec<String>, name: &str) -> Option<f64> {
    let i = args.iter().position(|a| a == name)?;
    args.remove(i);
    (i < args.len()).then(|| args.remove(i).parse().ok())?
}
