//! A song's tags (title, artist, album, ...): read from any file symphonia opens, and
//! written into the stems as Vorbis comments.

use std::fs::File;
use std::path::Path;

use symphonia::core::formats::FormatOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, MetadataRevision, RawValue, StandardTag, Tag};

use crate::DecodeError;

/// The tags Watershed keeps. Empty fields are `None`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    /// Track number on the album.
    pub track: Option<u32>,
    pub year: Option<u32>,
    pub genre: Option<String>,
}

impl Tags {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The tags as Vorbis comments (`ARTIST=...`), for the stem files. The stem's own
    /// name follows the title: "Paranoid Android (vocals)".
    pub fn vorbis_comments(&self, stem: &str) -> Vec<(&'static str, String)> {
        let mut comments = Vec::new();
        if let Some(title) = &self.title {
            comments.push(("TITLE", format!("{title} ({stem})")));
        }
        let text = [
            ("ARTIST", &self.artist),
            ("ALBUM", &self.album),
            ("GENRE", &self.genre),
        ];
        for (key, value) in text {
            if let Some(value) = value {
                comments.push((key, value.clone()));
            }
        }
        if let Some(track) = self.track {
            comments.push(("TRACKNUMBER", track.to_string()));
        }
        if let Some(year) = self.year {
            comments.push(("DATE", year.to_string()));
        }
        comments
    }

    /// Removes the " (stem)" that [`Tags::vorbis_comments`] added to the title.
    pub fn strip_stem(mut self, stem: &str) -> Self {
        let suffix = format!(" ({stem})");
        if let Some(title) = &mut self.title
            && let Some(cut) = title.strip_suffix(&suffix)
        {
            *title = cut.to_owned();
        }
        self
    }

    /// Fills the fields still empty from one tag.
    fn add(&mut self, tag: &Tag) {
        let text = |value: &str| {
            let value = value.trim();
            (!value.is_empty()).then(|| value.to_owned())
        };
        match &tag.std {
            Some(StandardTag::TrackTitle(v)) => fill(&mut self.title, text(v)),
            Some(StandardTag::Artist(v)) => fill(&mut self.artist, text(v)),
            // Only when there is no track artist (checked once all tags are in).
            Some(StandardTag::AlbumArtist(_)) => {}
            Some(StandardTag::Album(v)) => fill(&mut self.album, text(v)),
            Some(StandardTag::Genre(v)) => fill(&mut self.genre, text(v)),
            Some(StandardTag::TrackNumber(n)) => {
                fill(&mut self.track, u32::try_from(*n).ok().filter(|&n| n > 0));
            }
            Some(
                StandardTag::ReleaseYear(y)
                | StandardTag::RecordingYear(y)
                | StandardTag::OriginalReleaseYear(y),
            ) => fill(&mut self.year, Some(u32::from(*y)).filter(|&y| y > 0)),
            Some(
                StandardTag::ReleaseDate(v)
                | StandardTag::RecordingDate(v)
                | StandardTag::OriginalReleaseDate(v),
            ) => fill(&mut self.year, year_of(v)),
            // Tags symphonia does not map, by their usual raw keys.
            None => match (tag.raw.key.to_ascii_uppercase().as_str(), &tag.raw.value) {
                ("TITLE", RawValue::String(v)) => fill(&mut self.title, text(v)),
                ("ARTIST", RawValue::String(v)) => fill(&mut self.artist, text(v)),
                ("ALBUM", RawValue::String(v)) => fill(&mut self.album, text(v)),
                ("DATE" | "YEAR", RawValue::String(v)) => fill(&mut self.year, year_of(v)),
                _ => {}
            },
            Some(_) => {}
        }
    }

    fn add_revision(&mut self, revision: &MetadataRevision) {
        for tag in all_tags(revision) {
            self.add(tag);
        }
        if self.artist.is_none() {
            self.artist = all_tags(revision).find_map(|tag| match &tag.std {
                Some(StandardTag::AlbumArtist(v)) if !v.trim().is_empty() => {
                    Some(v.trim().to_owned())
                }
                _ => None,
            });
        }
    }
}

/// The revision's tags: the media's, then each track's.
fn all_tags(revision: &MetadataRevision) -> impl Iterator<Item = &Tag> {
    std::iter::once(&revision.media)
        .chain(revision.per_track.iter().map(|t| &t.metadata))
        .flat_map(|c| &c.tags)
}

/// Sets `field` if it is still empty.
fn fill<T>(field: &mut Option<T>, value: Option<T>) {
    if field.is_none() {
        *field = value;
    }
}

/// The year at the start of a date: "1997", "1997-05-21", "1997-05-21T10:00".
fn year_of(date: &str) -> Option<u32> {
    let digits: String = date
        .trim()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    (digits.len() == 4).then(|| digits.parse().ok()).flatten()
}

/// Reads the tags of an audio file without decoding its audio: ID3 (mp3, also at
/// the end), MP4, Vorbis comments (Ogg, FLAC), ...
pub fn read_tags(path: &Path) -> Result<Tags, DecodeError> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|e| DecodeError::Unsupported(e.to_string()))?;
    let mut tags = Tags::default();
    // Every revision, oldest first: tags found before the container (ID3v2), then
    // the container's own; the first value of each field wins.
    let mut metadata = format.metadata();
    loop {
        if let Some(revision) = metadata.current() {
            tags.add_revision(revision);
        }
        if metadata.pop().is_none() {
            break;
        }
    }
    Ok(tags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn years_from_dates() {
        assert_eq!(year_of("1997"), Some(1997));
        assert_eq!(year_of("1997-05-21"), Some(1997));
        assert_eq!(year_of(" 2003-01-01T10:00 "), Some(2003));
        assert_eq!(year_of("97"), None);
        assert_eq!(year_of("May 1997"), None);
    }

    #[test]
    fn stem_suffix_round_trip() {
        let tags = Tags {
            title: Some("Paranoid Android".into()),
            artist: Some("Radiohead".into()),
            year: Some(1997),
            ..Tags::default()
        };
        let comments = tags.vorbis_comments("vocals");
        assert_eq!(comments[0], ("TITLE", "Paranoid Android (vocals)".into()));
        let read = Tags {
            title: Some("Paranoid Android (vocals)".into()),
            ..tags.clone()
        };
        assert_eq!(read.strip_stem("vocals"), tags);
    }

    /// Tags written into Ogg and FLAC stems read back the same, without the stem in
    /// the title.
    #[test]
    fn stems_keep_the_tags() {
        let dir = std::env::temp_dir().join(format!("watershed-tags-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let tags = Tags {
            title: Some("Blue Rondo à la Turk".into()),
            artist: Some("The Dave Brubeck Quartet".into()),
            album: Some("Time Out".into()),
            track: Some(1),
            year: Some(1959),
            genre: Some("Jazz".into()),
        };
        let audio = crate::StereoBuffer::new(44_100, vec![0.0; 2 * 4410]);
        let comments = tags.vorbis_comments("piano");
        let ogg = dir.join("piano.ogg");
        crate::write_vorbis(&ogg, &audio, &comments).unwrap();
        let flac = dir.join("piano.flac");
        crate::write_flac(&flac, &audio, 1.0, &comments).unwrap();
        for path in [ogg, flac] {
            let read = read_tags(&path).unwrap();
            assert_eq!(read.title.as_deref(), Some("Blue Rondo à la Turk (piano)"));
            assert_eq!(read.strip_stem("piano"), tags, "{}", path.display());
        }
        // A stem without tags.
        let bare = dir.join("bare.flac");
        crate::write_flac(&bare, &audio, 1.0, &[]).unwrap();
        assert!(read_tags(&bare).unwrap().is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
