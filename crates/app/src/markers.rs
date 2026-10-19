//! Markers and the sections between them. Times are in seconds.

use watershed_storage::Marker;

use crate::loops::highest_number;

/// Markers closer than this to an existing one are not added: the existing one is
/// reused instead, so pressing M twice does not stack two markers.
const MIN_GAP_SECS: f64 = 0.1;

/// A stretch of the song, from a marker (or the start) to the next marker (or the end).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Section {
    /// Index of the marker it starts at; `None` for the implicit start section
    /// before the first marker.
    pub marker: Option<usize>,
    pub start: f64,
    pub end: f64,
}

impl Section {
    pub fn contains(&self, secs: f64) -> bool {
        (self.start..self.end).contains(&secs)
    }
}

#[derive(Debug, Default)]
pub struct Markers {
    /// In time order.
    pub list: Vec<Marker>,
    /// Number for the next default name, "Marker N".
    count: usize,
}

impl Markers {
    pub fn from_session(markers: &[Marker]) -> Self {
        let mut list = markers.to_vec();
        list.sort_by(|a, b| a.time.total_cmp(&b.time));
        let count = highest_number(list.iter().map(|m| m.name.as_str()), "Marker ");
        Self { list, count }
    }

    /// Adds a marker at `secs` with a default name, unless one is already there.
    /// Returns the index of the marker at that point.
    pub fn add(&mut self, secs: f64) -> usize {
        if let Some(index) = self
            .list
            .iter()
            .position(|m| (m.time - secs).abs() < MIN_GAP_SECS)
        {
            return index;
        }
        self.count += 1;
        let index = self.list.partition_point(|m| m.time < secs);
        self.list.insert(
            index,
            Marker {
                name: format!("Marker {}", self.count),
                time: secs,
            },
        );
        index
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.list.len() {
            self.list.remove(index);
        }
    }

    /// The sections of a song lasting `duration` seconds. The stretch before the
    /// first marker is an implicit section, unless the first marker is at the start.
    /// Empty without markers.
    pub fn sections(&self, duration: f64) -> Vec<Section> {
        let mut sections = Vec::with_capacity(self.list.len() + 1);
        if let Some(first) = self.list.first()
            && first.time >= MIN_GAP_SECS
        {
            sections.push(Section {
                marker: None,
                start: 0.0,
                end: first.time,
            });
        }
        for (index, marker) in self.list.iter().enumerate() {
            let end = self.list.get(index + 1).map_or(duration, |m| m.time);
            sections.push(Section {
                marker: Some(index),
                start: marker.time,
                end,
            });
        }
        sections
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_stay_in_time_order() {
        let mut markers = Markers::default();
        assert_eq!(markers.add(30.0), 0);
        assert_eq!(markers.add(10.0), 0);
        assert_eq!(markers.add(20.0), 1);
        let times: Vec<f64> = markers.list.iter().map(|m| m.time).collect();
        assert_eq!(times, [10.0, 20.0, 30.0]);
        assert_eq!(markers.list[2].name, "Marker 1");
    }

    #[test]
    fn no_marker_on_top_of_another() {
        let mut markers = Markers::default();
        markers.add(10.0);
        assert_eq!(markers.add(10.05), 0);
        assert_eq!(markers.list.len(), 1);
    }

    #[test]
    fn sections_cover_the_song() {
        let mut markers = Markers::default();
        assert!(markers.sections(100.0).is_empty());

        markers.add(40.0);
        markers.add(10.0);
        let sections = markers.sections(100.0);
        let ranges: Vec<_> = sections
            .iter()
            .map(|s| (s.marker, s.start, s.end))
            .collect();
        assert_eq!(
            ranges,
            [
                (None, 0.0, 10.0),
                (Some(0), 10.0, 40.0),
                (Some(1), 40.0, 100.0)
            ]
        );
        assert!(sections[1].contains(10.0));
        assert!(!sections[1].contains(40.0));
    }

    #[test]
    fn no_start_section_with_a_marker_at_zero() {
        let mut markers = Markers::default();
        markers.add(0.0);
        markers.add(50.0);
        let sections = markers.sections(100.0);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].marker, Some(0));
    }

    #[test]
    fn from_session_sorts_and_continues_numbering() {
        let marker = |name: &str, time| Marker {
            name: name.into(),
            time,
        };
        let mut markers =
            Markers::from_session(&[marker("Chorus", 60.0), marker("Marker 4", 20.0)]);
        assert_eq!(markers.list[0].name, "Marker 4");
        let index = markers.add(90.0);
        assert_eq!(markers.list[index].name, "Marker 5");
        markers.remove(0);
        assert_eq!(markers.list[0].name, "Chorus");
    }
}
