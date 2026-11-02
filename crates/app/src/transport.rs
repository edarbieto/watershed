//! Space and Enter, as in Reaper: Space plays, and pressed again stops and goes back
//! to where playback started; Enter pauses where it is, and either key resumes.

/// What the keys ask the engine to do.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    Play,
    Pause,
    /// Pause and go back to this time, in seconds.
    Stop(f64),
}

#[derive(Default)]
pub struct Transport {
    /// Where Space goes back to: where playback last started, or the last seek made
    /// by the user. Reaper's edit cursor.
    start: f64,
    /// Paused with Enter: the next play resumes and keeps `start`.
    paused: bool,
}

impl Transport {
    pub fn space(&mut self, playing: bool, position: f64) -> Command {
        if playing {
            self.paused = false;
            Command::Stop(self.start)
        } else {
            self.play(position)
        }
    }

    pub fn enter(&mut self, playing: bool, position: f64) -> Command {
        if playing {
            self.paused = true;
            Command::Pause
        } else {
            self.play(position)
        }
    }

    fn play(&mut self, position: f64) -> Command {
        if !self.paused {
            self.start = position;
        }
        self.paused = false;
        Command::Play
    }

    /// A seek made by the user (click, arrow keys, markers, ...): Space comes back here.
    pub fn seeked(&mut self, secs: f64) {
        self.start = secs;
        self.paused = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn space_stops_where_playback_started() {
        let mut transport = Transport::default();
        assert_eq!(transport.space(false, 10.0), Command::Play);
        assert_eq!(transport.space(true, 25.0), Command::Stop(10.0));
        // Played again from there, it still comes back there.
        assert_eq!(transport.space(false, 10.0), Command::Play);
        assert_eq!(transport.space(true, 40.0), Command::Stop(10.0));
    }

    #[test]
    fn enter_pauses_in_place_and_either_key_resumes() {
        let mut transport = Transport::default();
        assert_eq!(transport.enter(false, 10.0), Command::Play);
        assert_eq!(transport.enter(true, 20.0), Command::Pause);
        assert_eq!(transport.space(false, 20.0), Command::Play);
        assert_eq!(transport.enter(true, 30.0), Command::Pause);
        assert_eq!(transport.enter(false, 30.0), Command::Play);
        // Resuming keeps where playback first started.
        assert_eq!(transport.space(true, 45.0), Command::Stop(10.0));
    }

    #[test]
    fn a_seek_moves_the_return_point() {
        let mut transport = Transport::default();
        transport.space(false, 10.0);
        transport.seeked(60.0);
        assert_eq!(transport.space(true, 70.0), Command::Stop(60.0));
        // A seek while paused starts over from there.
        transport.enter(false, 60.0);
        transport.enter(true, 65.0);
        transport.seeked(80.0);
        transport.space(false, 80.0);
        assert_eq!(transport.space(true, 90.0), Command::Stop(80.0));
    }
}
