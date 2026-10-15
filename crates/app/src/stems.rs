//! Recognizes stem files by name, e.g. `song_bs6stem_mt_0_guitar.mp3` from MVSEP.

use std::path::Path;

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
    pub fn from_path(path: &Path) -> Self {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_lowercase();
        let last = stem
            .rsplit(['_', '-', ' ', '.', '(', ')', '[', ']'])
            .find(|t| !t.is_empty())
            .unwrap_or_default();
        match last {
            "vocals" | "vocal" | "voice" => Self::Vocals,
            "guitar" | "guitars" => Self::Guitar,
            "bass" => Self::Bass,
            "drums" | "drum" => Self::Drums,
            "piano" | "keys" => Self::Piano,
            "other" | "others" => Self::Other,
            "instrum" | "instrumental" | "inst" | "accompaniment" => Self::Instrumental,
            _ => Self::Unknown,
        }
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
    fn cleans_up_mvsep_folder_names() {
        let title = |name: &str| song_title(Path::new(name));
        assert_eq!(
            title("20261015024136-2a83f583f9-deftones-my-own-summer-shove-it.mp3"),
            "deftones-my-own-summer-shove-it"
        );
        assert_eq!(title("Caifanes - Nubes"), "Caifanes - Nubes");
    }
}
