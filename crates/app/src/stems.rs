//! Recognizes stem files by name, e.g. `song_bs6stem_mt_0_guitar.mp3` from MVSEP.

use std::path::{Path, PathBuf};

use eframe::egui::Color32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StemKind {
    Vocals,
    Guitar,
    Bass,
    Drums,
    Piano,
    Other,
    /// Everything except vocals. Duplicates the other stems, so it starts muted.
    Instrumental,
    Unknown,
}

impl StemKind {
    /// Recognizes a stem by any word of its file name, in English or Spanish, e.g.
    /// `song_bs6stem_mt_0_guitar.mp3`, `Bateria 2.wav` or `no_vocals.wav`. When
    /// several words match, the last one wins: song titles come first.
    pub fn from_path(path: &Path) -> Self {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let words: Vec<String> = fold(stem)
            .split(|c: char| !c.is_alphabetic())
            .filter(|w| !w.is_empty())
            .map(str::to_owned)
            .collect();
        let mut kind = Self::Unknown;
        for (i, word) in words.iter().enumerate() {
            let found = match word.as_str() {
                "vocals" | "vocal" | "voice" | "voices" | "vox" | "voz" | "voces" | "vocales" => {
                    // Demucs writes `no_vocals`; "sin voz" in Spanish.
                    let negated = i > 0 && matches!(words[i - 1].as_str(), "no" | "sin");
                    if negated {
                        Self::Instrumental
                    } else {
                        Self::Vocals
                    }
                }
                "guitar" | "guitars" | "gtr" | "guitarra" | "guitarras" => Self::Guitar,
                "bass" | "bajo" | "bajos" => Self::Bass,
                "drums" | "drum" | "percussion" | "perc" | "bateria" | "percusion" => Self::Drums,
                "piano" | "keys" | "keyboard" | "keyboards" | "teclado" | "teclados" | "teclas" => {
                    Self::Piano
                }
                "other" | "others" | "otro" | "otros" | "resto" => Self::Other,
                "instrum" | "instrumental" | "inst" | "accompaniment" | "acompanamiento" => {
                    Self::Instrumental
                }
                _ => continue,
            };
            kind = found;
        }
        kind
    }

    /// Display name, or `None` to use the file name.
    pub fn label(self) -> Option<&'static str> {
        Some(match self {
            Self::Vocals => "Vocals",
            Self::Guitar => "Guitar",
            Self::Bass => "Bass",
            Self::Drums => "Drums",
            Self::Piano => "Piano",
            Self::Other => "Other",
            Self::Instrumental => "Instrumental",
            Self::Unknown => return None,
        })
    }

    /// The color of the first stem of this kind; see [`track_colors`].
    pub fn color(self) -> Color32 {
        match self {
            Self::Vocals => Color32::from_rgb(0xe0, 0x6c, 0x75),
            Self::Guitar => Color32::from_rgb(0xe5, 0xa5, 0x4b),
            Self::Bass => Color32::from_rgb(0x98, 0xc3, 0x79),
            Self::Drums => Color32::from_rgb(0x61, 0xaf, 0xef),
            Self::Piano => Color32::from_rgb(0xc6, 0x78, 0xdd),
            Self::Other => Color32::from_rgb(0x56, 0xb6, 0xc2),
            Self::Instrumental | Self::Unknown => Color32::from_rgb(0xab, 0xb2, 0xbf),
        }
    }
}

/// Colors for stems that are not the first of a known kind, used in order and
/// repeated when there are more such stems.
const EXTRA_COLORS: [Color32; 6] = [
    Color32::from_rgb(0xe8, 0xd0, 0x6a), // yellow
    Color32::from_rgb(0xf2, 0x93, 0xc4), // pink
    Color32::from_rgb(0x8f, 0x92, 0xf0), // indigo
    Color32::from_rgb(0x8a, 0xdf, 0xb2), // mint
    Color32::from_rgb(0xc9, 0x9b, 0x6d), // tan
    Color32::from_rgb(0xb8, 0xbf, 0x4f), // olive
];

/// One color per track: the first stem of each known kind gets the kind's color,
/// unknown and repeated stems get the extra colors, so no two share one (up to six
/// of them). The instrumental stays gray: it starts muted.
pub fn track_colors(kinds: &[StemKind]) -> Vec<Color32> {
    let mut extra = EXTRA_COLORS.iter().copied().cycle();
    kinds
        .iter()
        .enumerate()
        .map(|(i, &kind)| match kind {
            StemKind::Instrumental => kind.color(),
            StemKind::Unknown => extra.next().unwrap_or_default(),
            _ if kinds[..i].contains(&kind) => extra.next().unwrap_or_default(),
            _ => kind.color(),
        })
        .collect()
}

/// Default track names: the kind's label, or the file name for unknown stems and
/// for kinds that appear more than once (e.g. guitars from two models), so they
/// can be told apart.
pub fn default_names(files: &[PathBuf], kinds: &[StemKind]) -> Vec<String> {
    files
        .iter()
        .zip(kinds)
        .map(|(file, &kind)| {
            let repeated = kinds.iter().filter(|&&k| k == kind).count() > 1;
            match kind.label() {
                Some(label) if !repeated => label.to_owned(),
                _ => file
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
            }
        })
        .collect()
}

/// Lowercase without Spanish accents, so `Batería` matches `bateria`.
fn fold(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' => 'a',
            'é' | 'è' | 'ë' => 'e',
            'í' | 'ì' | 'ï' => 'i',
            'ó' | 'ò' | 'ö' => 'o',
            'ú' | 'ù' | 'ü' => 'u',
            'ñ' => 'n',
            c => c,
        })
        .collect()
}

/// A human-readable title for a folder of stems.
///
/// MVSEP folders look like `20261015024136-2a83f583f9-artist-song.mp3`; the
/// timestamp, hash and extension are dropped.
pub fn song_title(folder: &Path) -> String {
    let name = folder
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled");
    let mut parts = name.splitn(3, '-');
    let title = match (parts.next(), parts.next(), parts.next()) {
        (Some(stamp), Some(hash), Some(rest))
            if stamp.len() == 14
                && stamp.bytes().all(|b| b.is_ascii_digit())
                && hash.bytes().all(|b| b.is_ascii_hexdigit()) =>
        {
            rest
        }
        _ => name,
    };
    let title = [".mp3", ".flac", ".wav", ".ogg", ".m4a"]
        .iter()
        .find_map(|ext| title.strip_suffix(ext))
        .unwrap_or(title);
    title.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_mvsep_names() {
        let kind = |name: &str| StemKind::from_path(Path::new(name));
        assert_eq!(kind("song_bs6stem_mt_0_guitar.mp3"), StemKind::Guitar);
        assert_eq!(
            kind("song_bs6stem_mt_0_instrum.mp3"),
            StemKind::Instrumental
        );
        assert_eq!(kind("vocals.flac"), StemKind::Vocals);
        assert_eq!(kind("My Song - Drums.wav"), StemKind::Drums);
        assert_eq!(kind("song (bass).wav"), StemKind::Bass);
        assert_eq!(kind("click.wav"), StemKind::Unknown);
    }

    #[test]
    fn recognizes_names_anywhere_and_in_spanish() {
        let kind = |name: &str| StemKind::from_path(Path::new(name));
        assert_eq!(kind("Guitar 2 - take.wav"), StemKind::Guitar);
        assert_eq!(kind("guitar2.wav"), StemKind::Guitar);
        assert_eq!(kind("Batería.flac"), StemKind::Drums);
        assert_eq!(kind("voz principal.wav"), StemKind::Vocals);
        assert_eq!(kind("Bajo la lluvia - guitarra.mp3"), StemKind::Guitar);
        assert_eq!(kind("teclados.ogg"), StemKind::Piano);
        assert_eq!(kind("no_vocals.wav"), StemKind::Instrumental);
        assert_eq!(kind("sin voz.wav"), StemKind::Instrumental);
        assert_eq!(kind("drummer.wav"), StemKind::Unknown);
    }

    #[test]
    fn repeated_and_unknown_stems_get_their_own_colors() {
        use StemKind::*;
        let colors = track_colors(&[Guitar, Bass, Guitar, Unknown, Instrumental]);
        assert_eq!(colors[0], Guitar.color());
        assert_eq!(colors[1], Bass.color());
        assert_eq!(colors[2], EXTRA_COLORS[0]);
        assert_eq!(colors[3], EXTRA_COLORS[1]);
        assert_eq!(colors[4], Instrumental.color());
    }

    #[test]
    fn repeated_kinds_are_named_by_file() {
        let files: Vec<PathBuf> = ["a_guitar.wav", "b_guitar.wav", "drums.wav", "click.wav"]
            .iter()
            .map(PathBuf::from)
            .collect();
        let kinds: Vec<_> = files.iter().map(|f| StemKind::from_path(f)).collect();
        assert_eq!(
            default_names(&files, &kinds),
            ["a_guitar", "b_guitar", "Drums", "click"]
        );
    }

    #[test]
    fn cleans_up_mvsep_folder_names() {
        let title = |name: &str| song_title(Path::new(name));
        assert_eq!(
            title("20261015024136-2a83f583f9-deftones-my-own-summer-shove-it.mp3"),
            "deftones-my-own-summer-shove-it"
        );
        assert_eq!(title("Caifanes - Nubes"), "Caifanes - Nubes");
    }
}
