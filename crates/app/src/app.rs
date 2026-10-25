use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use eframe::egui::{self, Align, Key, Layout, Rect, RichText};
use watershed_audio::{Engine, Notify, OutputDevice, output_devices};
use watershed_dsp::Peaks;
use watershed_storage::{Config, Session, TrackState};

use crate::loader::{self, LoadedSong, Loading};
use crate::loops::Loops;
use crate::markers::Markers;
use crate::playhead::Playhead;
use crate::separate::{self, Job, Target};
use crate::stems::song_title;
use crate::usage::{self, Usage};
use crate::view::View;
use crate::waveform::{Overlay, RulerAction, WaveformAction, ruler, scrollbar, waveform};

const ROW_HEIGHT: f32 = 64.0;
const RULER_HEIGHT: f32 = 32.0;
const SCROLLBAR_HEIGHT: f32 = 8.0;
/// Zoom factor of the zoom buttons and keys.
const ZOOM_STEP: f64 = 1.5;
const CONTROLS_WIDTH: f32 = 240.0;
const PAN_SLIDER_WIDTH: f32 = 40.0;
const MIN_DB: f32 = -60.0;
const MAX_DB: f32 = 6.0;
/// Tempo slider range, in percent of the original speed.
const MIN_TEMPO_PCT: f32 = 50.0;
const MAX_TEMPO_PCT: f32 = 125.0;
/// Transposition slider range, in semitones.
const MAX_SEMITONES: f32 = 12.0;
const FALLBACK_SAMPLE_RATE: u32 = 48_000;
/// How far the skip buttons and arrow keys move, in seconds.
const SKIP_SECS: f64 = 5.0;
const SIDE_PANEL_WIDTH: f32 = 280.0;
/// Below this, the song title is left out of the top bar (it is in the window title).
const MIN_TITLE_WIDTH: f32 = 100.0;
/// Number keys jump to markers 1 to 9, like Reaper.
const MARKER_KEYS: [Key; 9] = [
    Key::Num1,
    Key::Num2,
    Key::Num3,
    Key::Num4,
    Key::Num5,
    Key::Num6,
    Key::Num7,
    Key::Num8,
    Key::Num9,
];
/// How long the session must stay unchanged before it is written, so dragging a
/// slider does not write the file on every frame.
const SAVE_DELAY: Duration = Duration::from_secs(1);
/// A saved position this close to the end restarts the song from the beginning.
const RESUME_END_MARGIN_SECS: f64 = 1.0;

/// Transport commands shared by the buttons and the keyboard.
enum Action {
    /// Back to the loop start when a loop is on, else to the start of the song.
    ToStart,
    Skip(f64),
    SetA,
    SetB,
    ToggleLoop,
    ClearLoop,
    /// Adds a marker at the playhead.
    AddMarker,
    /// Moves the playhead to the marker with this index.
    GoToMarker(usize),
    /// Zooms by this factor (above 1 zooms in) around the playhead.
    Zoom(f64),
    /// Shows the whole song.
    ZoomToFit,
}

/// Mixer settings carried over when a song is reloaded, e.g. after switching to a
/// device with a different sample rate.
struct Restore {
    position_secs: f64,
    playing: bool,
    params: Vec<(f32, f32, bool, bool)>,
}

pub struct WatershedApp {
    config: Config,
    engine: Option<Engine>,
    devices: Vec<OutputDevice>,
    loaded: Option<LoadedSong>,
    loading: Option<Loading>,
    /// Song being separated.
    separating: Option<Job>,
    /// A separated song not opened yet because another one was playing.
    ready: Option<PathBuf>,
    restore: Option<Restore>,
    /// Set while the open folder is read again: whether the song was playing.
    reloading: Option<bool>,
    error: Option<String>,
    /// Engine underrun count already logged.
    xruns_logged: u64,
    /// Wakes the UI when the engine needs a restart, even if nothing else repaints.
    notify: Notify,
    loops: Loops,
    markers: Markers,
    /// Visible part of the song.
    view: View,
    /// Playhead position in the previous frame, in seconds, to follow it.
    last_position: f64,
    /// Smoothed playhead, for drawing.
    playhead: Playhead,
    /// When the previous frame was drawn, to advance the playhead.
    last_frame: Option<Instant>,
    /// CPU and memory of the process, shown in the transport bar.
    usage: Usage,
    /// Active loop last seen by [`WatershedApp::sync_loop`].
    last_loop: Option<(f64, f64)>,
    /// Session of the loaded song as last written (or read), to detect changes.
    saved_session: Option<Session>,
    /// When the session first differed from `saved_session`.
    changed_at: Option<Instant>,
}

impl WatershedApp {
    pub fn new(ctx: &egui::Context) -> Self {
        let config = Config::load();
        let ctx = ctx.clone();
        let notify: Notify = Arc::new(move || ctx.request_repaint());
        let (engine, error) = match Engine::start(config.output_device.as_deref(), notify.clone()) {
            Ok(engine) => (Some(engine), None),
            Err(e) => (None, Some(e.to_string())),
        };
        Self {
            config,
            engine,
            devices: output_devices(),
            loaded: None,
            loading: None,
            separating: None,
            ready: None,
            restore: None,
            reloading: None,
            error,
            xruns_logged: 0,
            notify,
            loops: Loops::default(),
            markers: Markers::default(),
            view: View::new(0.0),
            last_position: 0.0,
            playhead: Playhead::default(),
            last_frame: None,
            usage: Usage::new(),
            last_loop: None,
            saved_session: None,
            changed_at: None,
        }
    }

    fn sample_rate(&self) -> u32 {
        self.engine
            .as_ref()
            .map_or(FALLBACK_SAMPLE_RATE, Engine::sample_rate)
    }

    /// Opens a folder of stems or stem files, or separates a single song file.
    pub fn open(&mut self, paths: &[PathBuf], ctx: &egui::Context) {
        if let [path] = paths
            && path.is_file()
            && loader::is_audio(path)
        {
            self.separate(path.clone(), ctx);
            return;
        }
        let files = loader::collect_files(paths);
        if files.is_empty() {
            self.error = Some("No audio files found there.".into());
            return;
        }
        if let Some(folder) = files[0].parent() {
            self.config.last_folder = Some(folder.to_owned());
            self.config.save();
        }
        self.error = None;
        self.restore = None;
        self.reloading = None;
        self.loading = Some(loader::start(files, self.sample_rate(), ctx.clone()));
    }

    /// Reads the open folder again, to pick up new, removed or renamed stems. The
    /// session is written first, so tracks whose file is still there keep their
    /// mixer settings; the position, the zoom and playback carry over too.
    fn reload(&mut self, ctx: &egui::Context) {
        let Some(folder) = self
            .loaded
            .as_ref()
            .and_then(|loaded| loaded.files[0].parent())
            .map(PathBuf::from)
        else {
            return;
        };
        self.save_session();
        let playing = self.engine.as_ref().is_some_and(Engine::is_playing);
        self.open(&[folder], ctx);
        if self.loading.is_some() {
            self.reloading = Some(playing);
        }
    }

    /// Separates `song` into a new folder in the library, or opens the folder of an
    /// earlier separation of it.
    fn separate(&mut self, song: PathBuf, ctx: &egui::Context) {
        if let Some(job) = &self.separating {
            self.error = Some(format!(
                "Already separating {}. Wait for it or cancel it first.",
                job.title
            ));
            return;
        }
        let (Some(library), Some(models_dir)) = (
            self.config.library_folder(),
            watershed_storage::models_dir(),
        ) else {
            self.error = Some("No folder to keep separated songs or models in.".into());
            return;
        };
        if let Some(folder) = song.parent() {
            self.config.last_folder = Some(folder.to_owned());
            self.config.save();
        }
        self.error = None;
        match separate::target_folder(&library, &song) {
            Target::Existing(folder) => self.open(&[folder], ctx),
            Target::New(folder) => {
                self.separating = Some(separate::start(song, folder, models_dir, ctx.clone()));
            }
        }
    }

    fn pick_song(&mut self, ctx: &egui::Context) {
        let mut dialog = rfd::FileDialog::new()
            .set_title("Separate a song")
            .add_filter("Audio", loader::AUDIO_EXTENSIONS);
        if let Some(dir) = &self.config.last_folder {
            dialog = dialog.set_directory(dir);
        }
        if let Some(song) = dialog.pick_file() {
            self.separate(song, ctx);
        }
    }

    fn pick_library(&mut self) {
        let mut dialog = rfd::FileDialog::new().set_title("Folder for separated songs");
        if let Some(dir) = self.config.library_folder().filter(|d| d.is_dir()) {
            dialog = dialog.set_directory(dir);
        }
        if let Some(folder) = dialog.pick_folder() {
            self.config.library_folder = Some(folder);
            self.config.save();
        }
    }

    /// Opens a finished separation, unless another song is playing: then it waits
    /// in [`Self::ready`] for the user.
    fn poll_separation(&mut self, ctx: &egui::Context) {
        let Some(job) = &self.separating else {
            return;
        };
        let Some(result) = job.poll() else {
            return;
        };
        let cancelled = job.is_cancelling();
        self.separating = None;
        match result {
            Ok(folder) => {
                self.config.remember(&folder, unix_now());
                self.config.save();
                if self.engine.as_ref().is_some_and(Engine::is_playing) {
                    self.ready = Some(folder);
                } else {
                    self.open(&[folder], ctx);
                }
            }
            Err(_) if cancelled => {}
            Err(e) => self.error = Some(e),
        }
    }

    /// Progress of the separation, or the separated song waiting to be opened.
    fn separation_status(&mut self, ui: &mut egui::Ui) {
        if let Some(job) = &self.separating {
            let step = job.step();
            let mut text = step.label();
            if let Some(left) = job.remaining() {
                text.push_str(&format!(" ({} left)", separate::format_eta(left)));
            }
            let mut cancel = false;
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Separating {}", job.title)).strong());
                let bar = match step.fraction() {
                    Some(fraction) => egui::ProgressBar::new(fraction),
                    None => egui::ProgressBar::new(0.0).animate(true),
                };
                let response = ui.add(bar.desired_width(360.0).text(text));
                if let Some(device) = step.device() {
                    response.on_hover_text(format!("Running on {device}"));
                }
                if job.is_cancelling() {
                    ui.label(RichText::new("Cancelling…").weak());
                } else {
                    cancel = ui.button("Cancel").clicked();
                }
            });
            if cancel {
                job.cancel();
            }
            ui.ctx().request_repaint_after(Duration::from_millis(100));
            ui.add_space(4.0);
        } else if let Some(folder) = &self.ready {
            let mut open = false;
            let mut dismiss = false;
            ui.horizontal(|ui| {
                ui.label(format!("✔ {} is separated.", song_title(folder)));
                open = ui.button("Open").clicked();
                dismiss = ui
                    .small_button("✕")
                    .on_hover_text("Open it later from Recent")
                    .clicked();
            });
            if open {
                let folder = folder.clone();
                self.ready = None;
                self.open(&[folder], ui.ctx());
            } else if dismiss {
                self.ready = None;
            }
            ui.add_space(4.0);
        }
    }

    fn pick_folder(&mut self, ctx: &egui::Context) {
        let mut dialog = rfd::FileDialog::new().set_title("Open a folder of stems");
        if let Some(dir) = &self.config.last_folder {
            dialog = dialog.set_directory(dir);
        }
        if let Some(folder) = dialog.pick_folder() {
            self.open(&[folder], ctx);
        }
    }

    fn poll_loading(&mut self, ctx: &egui::Context) {
        let Some(result) = self.loading.as_ref().and_then(Loading::poll) else {
            return;
        };
        self.loading = None;
        match result {
            Ok(loaded) => {
                if let Some(restore) = self.restore.take() {
                    for (track, &(gain, pan, mute, solo)) in
                        loaded.song.tracks.iter().zip(&restore.params)
                    {
                        track.params.set_gain(gain);
                        track.params.set_pan(pan);
                        track.params.set_mute(mute);
                        track.params.set_solo(solo);
                    }
                    if let Some(engine) = &mut self.engine {
                        engine.load(loaded.song.clone());
                        let frame = restore.position_secs * f64::from(engine.sample_rate());
                        engine.seek(frame as usize);
                        if restore.playing {
                            engine.play();
                        }
                    }
                    self.loaded = Some(loaded);
                } else {
                    if let Some(folder) = loaded.files[0].parent() {
                        self.config.remember(folder, unix_now());
                        self.config.save();
                    }
                    // Write the session of the song being replaced before switching.
                    self.save_session();
                    let view = self.view;
                    self.apply_session(&loaded);
                    if let Some(playing) = self.reloading.take() {
                        if view.duration() == loaded.song.duration_secs() {
                            self.view = view;
                        }
                        if playing && let Some(engine) = &mut self.engine {
                            engine.play();
                        }
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
                        "Watershed — {}",
                        loaded.title
                    )));
                    self.loaded = Some(loaded);
                    self.saved_session = self.session().map(|(_, session)| session);
                    self.changed_at = None;
                }
            }
            Err(e) => {
                self.reloading = None;
                self.error = Some(e);
            }
        }
    }

    /// Restores a newly loaded song's session (mixer, tempo, pitch, loops and
    /// position) from its folder, and sends the song to the engine.
    fn apply_session(&mut self, loaded: &LoadedSong) {
        let session = Session::load_from(&session_path(loaded)).unwrap_or_default();
        for (track, file) in loaded.song.tracks.iter().zip(&loaded.files) {
            if let Some(state) = session.track(&file_name(file)) {
                let params = &track.params;
                params.set_gain(state.gain.clamp(0.0, db_to_gain(MAX_DB)));
                params.set_pan(state.pan.clamp(-1.0, 1.0));
                params.set_mute(state.mute);
                params.set_solo(state.solo);
            }
        }
        self.loops = Loops::from_session(&session);
        self.markers = Markers::from_session(&session.markers);
        self.view = View::new(loaded.song.duration_secs());
        self.last_loop = None;
        let Some(engine) = &mut self.engine else {
            return;
        };
        engine.set_tempo(
            session
                .tempo
                .clamp(MIN_TEMPO_PCT / 100.0, MAX_TEMPO_PCT / 100.0),
        );
        engine.set_semitones(session.semitones.clamp(-MAX_SEMITONES, MAX_SEMITONES));
        engine.load(loaded.song.clone());
        let duration = loaded.song.duration_secs();
        if session.position > 0.0 && session.position < duration - RESUME_END_MARGIN_SECS {
            engine.seek((session.position * f64::from(engine.sample_rate())) as usize);
        }
    }

    /// The session of the loaded song as it is now, with the path to write it to.
    fn session(&self) -> Option<(PathBuf, Session)> {
        let loaded = self.loaded.as_ref()?;
        let engine = self.engine.as_ref()?;
        let tracks = loaded
            .song
            .tracks
            .iter()
            .zip(&loaded.files)
            .map(|(track, file)| {
                let p = &track.params;
                TrackState {
                    file: file_name(file),
                    gain: p.gain(),
                    pan: p.pan(),
                    mute: p.mute(),
                    solo: p.solo(),
                }
            })
            .collect();
        let session = Session {
            position: engine.position() as f64 / f64::from(engine.sample_rate()),
            tempo: engine.tempo(),
            semitones: engine.semitones(),
            loop_a: self.loops.a,
            loop_b: self.loops.b,
            looping: self.loops.enabled,
            tracks,
            loops: self.loops.saved.clone(),
            markers: self.markers.list.clone(),
            stem_gain: loaded.stem_gain,
            ..Session::default()
        };
        Some((session_path(loaded), session))
    }

    /// Writes the session if anything, including the position, changed since the
    /// last write. Failures are logged, not fatal (e.g. a read-only folder).
    fn save_session(&mut self) {
        // While the stems are reloaded for another device the engine has no song.
        if self.restore.is_some() {
            return;
        }
        let Some((path, session)) = self.session() else {
            return;
        };
        if self.saved_session.as_ref() == Some(&session) {
            return;
        }
        if let Err(e) = session.save_to(&path) {
            log::warn!("cannot save session to {}: {e}", path.display());
        }
        self.saved_session = Some(session);
        self.changed_at = None;
    }

    /// Saves the session once it has stayed changed for [`SAVE_DELAY`]. The position
    /// alone does not count as a change: it moves all the time while playing, and
    /// is written with the next change or when the song is closed.
    fn autosave_session(&mut self, ctx: &egui::Context) {
        let (Some((_, mut session)), Some(saved)) = (self.session(), &self.saved_session) else {
            return;
        };
        session.position = saved.position;
        if session == *saved {
            self.changed_at = None;
            return;
        }
        let changed_at = *self.changed_at.get_or_insert_with(Instant::now);
        if changed_at.elapsed() >= SAVE_DELAY {
            self.save_session();
        } else {
            ctx.request_repaint_after(SAVE_DELAY);
        }
    }

    /// Opens the device with the given id and moves the current song to it, keeping
    /// the position, the mixer settings and whether it was playing.
    fn switch_device(&mut self, id: &str, ctx: &egui::Context) {
        let position_secs = self
            .engine
            .as_ref()
            .map_or(0.0, |e| e.position() as f64 / f64::from(e.sample_rate()));
        let playing = self.engine.as_ref().is_some_and(Engine::is_playing);
        let master = self
            .engine
            .as_ref()
            .map(|e| (e.master_gain(), e.tempo(), e.semitones()));
        let restore_master = |engine: &Engine| {
            if let Some((gain, tempo, semitones)) = master {
                engine.set_master_gain(gain);
                engine.set_tempo(tempo);
                engine.set_semitones(semitones);
            }
        };
        // Close the old stream first; some drivers allow only one at a time.
        self.engine = None;
        let engine = match Engine::start(Some(id), self.notify.clone()) {
            Ok(engine) => engine,
            Err(e) => {
                self.error = Some(e.to_string());
                self.engine = Engine::start(None, self.notify.clone()).ok();
                if let Some(engine) = &self.engine {
                    restore_master(engine);
                }
                return;
            }
        };
        restore_master(&engine);
        self.error = None;
        self.config.output_device = Some(engine.device().id.clone());
        self.config.save();
        let engine = self.engine.insert(engine);

        let Some(loaded) = &self.loaded else { return };
        if loaded.song.sample_rate == engine.sample_rate() {
            engine.load(loaded.song.clone());
            engine.seek((position_secs * f64::from(engine.sample_rate())) as usize);
            if playing {
                engine.play();
            }
        } else {
            // The stems must be resampled to the new device rate.
            self.restore = Some(Restore {
                position_secs,
                playing,
                params: loaded
                    .song
                    .tracks
                    .iter()
                    .map(|t| {
                        let p = &t.params;
                        (p.gain(), p.pan(), p.mute(), p.solo())
                    })
                    .collect(),
            });
            let files = loaded.files.clone();
            self.loading = Some(loader::start(files, engine.sample_rate(), ctx.clone()));
        }
    }

    fn handle_input(&mut self, ui: &egui::Ui) {
        let dropped: Vec<PathBuf> = ui.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_owned())
                .filter(|p| !p.as_os_str().is_empty())
                .collect()
        });
        if !dropped.is_empty() {
            self.open(&dropped, ui.ctx());
        }

        if ui.ctx().egui_wants_keyboard_input() {
            return;
        }
        if ui.input(|i| i.key_pressed(Key::F5)) {
            self.reload(ui.ctx());
        }
        if ui.input(|i| i.key_pressed(Key::Space))
            && let Some(engine) = &mut self.engine
        {
            engine.toggle();
        }
        let keys = [
            (Key::Home, Action::ToStart),
            (Key::ArrowLeft, Action::Skip(-SKIP_SECS)),
            (Key::ArrowRight, Action::Skip(SKIP_SECS)),
            (Key::A, Action::SetA),
            (Key::B, Action::SetB),
            (Key::L, Action::ToggleLoop),
            (Key::M, Action::AddMarker),
            (Key::Plus, Action::Zoom(ZOOM_STEP)),
            (Key::Equals, Action::Zoom(ZOOM_STEP)),
            (Key::Minus, Action::Zoom(1.0 / ZOOM_STEP)),
        ];
        for (key, action) in keys {
            if ui.input(|i| i.key_pressed(key)) {
                self.apply(action);
            }
        }
        for (index, key) in MARKER_KEYS.into_iter().enumerate() {
            if ui.input(|i| i.key_pressed(key)) {
                self.apply(Action::GoToMarker(index));
            }
        }
    }

    fn position_secs(&self) -> f64 {
        self.engine
            .as_ref()
            .map_or(0.0, |e| e.position() as f64 / f64::from(e.sample_rate()))
    }

    fn apply(&mut self, action: Action) {
        let position = self.position_secs();
        match action {
            Action::ToStart => {
                let start = self.loops.active().map_or(0.0, |(a, _)| a);
                self.seek_secs(start);
            }
            Action::Skip(secs) => self.seek_secs((position + secs).max(0.0)),
            Action::SetA => self.loops.set_a(position),
            Action::SetB => self.loops.set_b(position),
            Action::ToggleLoop => self.loops.toggle(),
            Action::ClearLoop => self.loops.clear(),
            Action::AddMarker => {
                if self.loaded.is_some() {
                    self.markers.add(position);
                }
            }
            Action::GoToMarker(index) => {
                if let Some(marker) = self.markers.list.get(index) {
                    self.seek_secs(marker.time);
                }
            }
            Action::Zoom(factor) => {
                let anchor = if self.view.contains(position) {
                    position
                } else {
                    self.view.start + self.view.span / 2.0
                };
                self.view.zoom(factor, anchor);
            }
            Action::ZoomToFit => self.view = View::new(self.view.duration()),
        }
    }

    /// Advances the smoothed playhead to this frame and keeps it in view.
    fn update_playhead(&mut self) {
        let now = Instant::now();
        let dt = self
            .last_frame
            .map_or(0.0, |last| now.duration_since(last).as_secs_f64());
        self.last_frame = Some(now);
        let (playing, tempo) = self
            .engine
            .as_ref()
            .map_or((false, 1.0), |e| (e.is_playing(), f64::from(e.tempo())));
        let position = self
            .playhead
            .update(self.position_secs(), dt, playing, tempo);
        self.view.follow(self.last_position, position, playing);
        self.last_position = position;
    }

    /// Jumps to the start of the active loop and plays it.
    fn play_loop(&mut self) {
        let Some((start, _)) = self.loops.active() else {
            return;
        };
        self.seek_secs(start);
        // Already at the start: `sync_loop` need not jump there again.
        self.last_loop = self.loops.active();
        if let Some(engine) = &mut self.engine {
            engine.play();
        }
    }

    /// Plays from `secs`. Turns the loop off if it is outside it, or playback would
    /// be caught by the loop once it reaches it.
    fn play_from(&mut self, secs: f64) {
        if self
            .loops
            .active()
            .is_some_and(|(a, b)| !(a..b).contains(&secs))
        {
            self.loops.enabled = false;
        }
        self.seek_secs(secs);
        if let Some(engine) = &mut self.engine {
            engine.play();
        }
    }

    fn seek_secs(&mut self, secs: f64) {
        if let Some(engine) = &mut self.engine {
            engine.seek((secs * f64::from(engine.sample_rate())) as usize);
        }
    }

    /// Sends the active loop to the engine. When a loop is turned on while playing
    /// elsewhere, jumps to its start once the mouse is released.
    fn sync_loop(&mut self, ui: &egui::Ui) {
        let Some(engine) = &mut self.engine else {
            return;
        };
        let rate = f64::from(engine.sample_rate());
        let active = self.loops.active();
        engine.set_loop(active.map(|(a, b)| (a * rate) as usize..(b * rate) as usize));

        if active != self.last_loop && !ui.input(|i| i.pointer.any_down()) {
            self.last_loop = active;
            let position = engine.position() as f64 / rate;
            if let Some((a, b)) = active
                && !(a..b).contains(&position)
            {
                engine.seek((a * rate) as usize);
            }
        }
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Open folder…").clicked() {
                self.pick_folder(ui.ctx());
            }
            if ui
                .button("Separate song…")
                .on_hover_text("Split a song file into stems (vocals, drums, bass, …)")
                .clicked()
            {
                self.pick_song(ui.ctx());
            }
            self.recent_menu(ui);
            if ui
                .add_enabled(self.loaded.is_some(), egui::Button::new("Reload"))
                .on_hover_text(
                    "Read the song folder again, for added, removed or renamed stems                      (F5). Tracks that are still there keep their settings.",
                )
                .clicked()
            {
                self.reload(ui.ctx());
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if let Some(engine) = &self.engine {
                    let mut db = gain_to_db(engine.master_gain());
                    let response = ui.add(volume_slider(&mut db)).on_hover_text(VOLUME_HINT);
                    if double_clicked(&response) {
                        engine.set_master_gain(1.0);
                    } else if response.changed() {
                        engine.set_master_gain(db_to_gain(db));
                    }
                    ui.label("Master");
                    ui.separator();
                }
                self.device_picker(ui);
                self.usage_label(ui);
                // Last, so a long title gets what is left and is cut short instead
                // of overlapping the controls on the right. The window title has it
                // too, for when there is no room here.
                if let Some(loaded) = &self.loaded
                    && ui.available_width() > MIN_TITLE_WIDTH
                {
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        ui.add(egui::Label::new(RichText::new(&loaded.title).strong()).truncate())
                            .on_hover_text(&loaded.title);
                    });
                }
            });
        });
    }

    /// CPU, GPU and RAM used by the process, refreshed every [`usage::INTERVAL`].
    fn usage_label(&mut self, ui: &mut egui::Ui) {
        let Some(sample) = self.usage.sample() else {
            return;
        };
        let gpu = sample
            .gpu_percent
            .map_or_else(|| "  —".to_owned(), |percent| format!("{percent:3.0}"));
        let gpu_note = if usage::WHOLE_GPU {
            "GPU is the busiest GPU engine's share of time, for the whole GPU: macOS does \
             not report it per app."
        } else {
            "GPU is the busiest GPU engine's share of time (shown as — when the driver \
             does not report it)."
        };
        ui.separator();
        ui.label(
            RichText::new(format!(
                "CPU {:4.1} %   GPU {gpu} %   RAM {:.0} MiB ({:.1} %)",
                sample.cpu_percent,
                sample.memory as f64 / (1024.0 * 1024.0),
                sample.memory_percent
            ))
            .weak()
            .monospace()
            .size(10.0),
        )
        .on_hover_text(format!(
            "Used by Watershed as a whole (all its windows and threads). CPU is a share \
             of all cores, averaged over the last second; RAM is the memory it holds, and \
             its share of the installed RAM. {gpu_note}"
        ));
    }

    fn recent_menu(&mut self, ui: &mut egui::Ui) {
        let mut open = None;
        let mut clear = false;
        ui.add_enabled_ui(!self.config.recent.is_empty(), |ui| {
            ui.menu_button("Recent", |ui| {
                let current = self
                    .loaded
                    .as_ref()
                    .and_then(|loaded| loaded.files[0].parent());
                for recent in &self.config.recent {
                    let exists = recent.path.is_dir();
                    let is_current = current == Some(recent.path.as_path());
                    let item = egui::RadioButton::new(is_current, song_title(&recent.path));
                    // The open song is already there: clicking it does nothing (the
                    // Reload button reads its folder again).
                    if ui
                        .add_enabled(exists, item)
                        .on_hover_text(recent.path.display().to_string())
                        .on_disabled_hover_text(format!("Not found: {}", recent.path.display()))
                        .clicked()
                        && !is_current
                    {
                        open = Some(recent.path.clone());
                    }
                }
                ui.separator();
                if ui.button("Clear list").clicked() {
                    clear = true;
                }
            });
        });
        if let Some(path) = open {
            self.open(&[path], ui.ctx());
        }
        if clear {
            self.config.recent.clear();
            self.config.save();
        }
    }

    /// Start screen, before a song is opened: where to drop files, and the recent
    /// songs.
    fn welcome(&mut self, ui: &mut egui::Ui) {
        let mut open = None;
        let mut forget = None;
        let mut change_library = false;
        ui.vertical_centered(|ui| {
            ui.add_space(48.0);
            ui.label(
                RichText::new("Drop a song here to separate it, or a folder of stems to open it")
                    .weak()
                    .size(18.0),
            );
            ui.add_space(8.0);
            let library = self.config.library_folder();
            let shown = library.as_deref().map_or("nowhere".into(), short_path);
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Separated songs go to {shown}")).weak())
                    .on_hover_text(library.map(|l| l.display().to_string()).unwrap_or_default());
                if ui.small_button("Change…").clicked() {
                    change_library = true;
                }
            });
            if self.config.recent.is_empty() {
                return;
            }
            ui.add_space(32.0);
            ui.heading("Recent songs");
            ui.add_space(8.0);
        });
        if change_library {
            self.pick_library();
        }
        if self.config.recent.is_empty() {
            return;
        }
        let now = unix_now();
        let width = ui.available_width().min(720.0);
        let margin = (ui.available_width() - width) / 2.0;
        ui.horizontal(|ui| {
            ui.add_space(margin);
            ui.vertical(|ui| {
                ui.set_width(width);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("recent_songs")
                        .num_columns(4)
                        .striped(true)
                        .spacing([16.0, 6.0])
                        .show(ui, |ui| {
                            for recent in &self.config.recent {
                                let exists = recent.path.is_dir();
                                let title = song_title(&recent.path);
                                if exists {
                                    if ui.link(RichText::new(title).size(15.0)).clicked() {
                                        open = Some(recent.path.clone());
                                    }
                                } else {
                                    ui.label(RichText::new(title).size(15.0).weak())
                                        .on_hover_text("Folder not found");
                                }
                                let folder = recent.path.parent().unwrap_or(&recent.path);
                                ui.label(RichText::new(short_path(folder)).weak().small());
                                ui.label(
                                    RichText::new(format_ago(now.saturating_sub(recent.opened)))
                                        .weak()
                                        .small(),
                                );
                                if ui
                                    .small_button("🗑")
                                    .on_hover_text("Remove from the list (the files stay)")
                                    .clicked()
                                {
                                    forget = Some(recent.path.clone());
                                }
                                ui.end_row();
                            }
                        });
                });
            });
        });
        if let Some(path) = open {
            self.open(&[path], ui.ctx());
        }
        if let Some(path) = forget {
            self.config.forget(&path);
            self.config.save();
        }
    }

    fn device_picker(&mut self, ui: &mut egui::Ui) {
        let current = self.engine.as_ref().map(|e| e.device().clone());
        let mut selected = None;
        let response = egui::ComboBox::from_id_salt("output_device")
            .width(220.0)
            .selected_text(current.as_ref().map_or("No device", |d| d.name.as_str()))
            .show_ui(ui, |ui| {
                for device in &self.devices {
                    let is_current = current.as_ref().is_some_and(|c| c.id == device.id);
                    if ui.selectable_label(is_current, &device.name).clicked() && !is_current {
                        selected = Some(device.id.clone());
                    }
                }
            });
        // Refresh the list when the menu opens, in case devices were plugged in.
        if response.response.clicked() {
            self.devices = output_devices();
        }
        ui.label("Output");
        if let Some(id) = selected {
            self.switch_device(&id, ui.ctx());
        }
    }

    fn transport(&mut self, ui: &mut egui::Ui) {
        let mut action = None;
        ui.horizontal(|ui| {
            let Some(engine) = &mut self.engine else {
                ui.label("No audio output");
                return;
            };
            let enabled = self.loaded.is_some();
            let button = |ui: &mut egui::Ui, label: &str, hint: &str| {
                ui.add_enabled(enabled, egui::Button::new(RichText::new(label).size(18.0)))
                    .on_hover_text(hint)
                    .clicked()
            };
            if button(ui, "⏮", "Back to the start, or to the loop start (Home)") {
                action = Some(Action::ToStart);
            }
            if button(ui, "⏪", "Back 5 seconds (Left arrow)") {
                action = Some(Action::Skip(-SKIP_SECS));
            }
            let play_label = if engine.is_playing() { "⏸" } else { "▶" };
            if button(ui, play_label, "Play / pause (Space)") {
                engine.toggle();
            }
            if button(ui, "⏹", "Stop") {
                engine.stop();
            }
            if button(ui, "⏩", "Forward 5 seconds (Right arrow)") {
                action = Some(Action::Skip(SKIP_SECS));
            }

            if let Some(loaded) = &self.loaded {
                let rate = f64::from(engine.sample_rate());
                ui.label(
                    RichText::new(format!(
                        "{} / {}",
                        format_time(engine.position() as f64 / rate),
                        format_time(loaded.song.duration_secs())
                    ))
                    .monospace(),
                );
            }

            ui.separator();
            if ui
                .add_enabled(enabled, egui::Button::new("A"))
                .on_hover_text("Start a new loop here (A)")
                .clicked()
            {
                action = Some(Action::SetA);
            }
            if ui
                .add_enabled(enabled, egui::Button::new("B"))
                .on_hover_text("Set the loop end here and start looping (B)")
                .clicked()
            {
                action = Some(Action::SetB);
            }
            let can_loop = self.loops.region().is_some();
            if ui
                .add_enabled(
                    can_loop,
                    egui::Button::selectable(self.loops.enabled, "Loop"),
                )
                .on_hover_text("Loop between A and B (L). Drag on a waveform to select a loop.")
                .clicked()
            {
                action = Some(Action::ToggleLoop);
            }
            let has_ends = self.loops.a.is_some() || self.loops.b.is_some();
            if ui
                .add_enabled(has_ends, egui::Button::new("Clear"))
                .on_hover_text("Forget A and B (saved loops stay)")
                .clicked()
            {
                action = Some(Action::ClearLoop);
            }
            if has_ends {
                let end = |t: Option<f64>| t.map_or("?".to_owned(), format_time);
                ui.label(
                    RichText::new(format!("{} → {}", end(self.loops.a), end(self.loops.b)))
                        .monospace()
                        .weak(),
                );
            }

            ui.separator();
            let mut percent = engine.tempo() * 100.0;
            let response = ui
                .add(
                    egui::Slider::new(&mut percent, MIN_TEMPO_PCT..=MAX_TEMPO_PCT)
                        .step_by(1.0)
                        .suffix(" %"),
                )
                .on_hover_text("Tempo, without changing the pitch (double-click to reset)");
            if double_clicked(&response) {
                engine.set_tempo(1.0);
            } else if response.changed() {
                engine.set_tempo(percent / 100.0);
            }
            ui.label("Tempo");

            ui.separator();
            let mut semitones = engine.semitones();
            let response = ui
                .add(
                    egui::Slider::new(&mut semitones, -MAX_SEMITONES..=MAX_SEMITONES)
                        .step_by(1.0)
                        .custom_formatter(|v, _| format!("{v:+.0} st"))
                        .custom_parser(|t| t.trim().trim_end_matches("st").trim().parse().ok()),
                )
                .on_hover_text("Transpose in semitones (double-click to reset)");
            if double_clicked(&response) {
                engine.set_semitones(0.0);
            } else if response.changed() {
                engine.set_semitones(semitones);
            }
            ui.label("Pitch");

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} Hz", engine.sample_rate()))
                        .weak()
                        .small(),
                );
            });
        });
        if let Some(action) = action {
            self.apply(action);
        }
    }

    fn side_panel(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            self.markers_section(ui);
            ui.add_space(16.0);
            self.loops_section(ui);
        });
    }

    fn markers_section(&mut self, ui: &mut egui::Ui) {
        let Some(loaded) = &self.loaded else {
            return;
        };
        let duration = loaded.song.duration_secs();
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.heading("Markers");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .button("Add")
                    .on_hover_text("Add a marker at the playhead (M)")
                    .clicked()
                {
                    self.apply(Action::AddMarker);
                }
            });
        });
        ui.separator();
        let sections = self.markers.sections(duration);
        if sections.is_empty() {
            ui.label(
                RichText::new(
                    "Press M or click Add to mark the playhead, or double-click the ruler. \
                     Each marker starts a section.",
                )
                .weak()
                .small(),
            );
            return;
        }

        let position = self.position_secs();
        let mut play = None;
        let mut loop_section = None;
        let mut remove = None;
        for section in &sections {
            let current = section.contains(position);
            ui.horizontal(|ui| {
                let number = section
                    .marker
                    .filter(|&i| i < MARKER_KEYS.len())
                    .map_or(String::new(), |i| (i + 1).to_string());
                let number = RichText::new(format!("{number:>2}")).monospace();
                ui.label(if current {
                    number.strong()
                } else {
                    number.weak()
                })
                .on_hover_text("Press this number to jump here");
                if ui
                    .selectable_label(current, "▶")
                    .on_hover_text("Play from here")
                    .clicked()
                {
                    play = Some(section.start);
                }
                let looping = self.loops.active() == Some((section.start, section.end));
                if ui
                    .selectable_label(looping, "🔁")
                    .on_hover_text("Loop this section, up to the next marker")
                    .clicked()
                {
                    loop_section = Some((section.start, section.end));
                }
                match section.marker {
                    Some(index) => {
                        let marker = &mut self.markers.list[index];
                        ui.add(egui::TextEdit::singleline(&mut marker.name).desired_width(90.0));
                    }
                    None => {
                        ui.add_sized(
                            [90.0, ui.spacing().interact_size.y],
                            egui::Label::new(RichText::new("[Start]").weak()),
                        );
                    }
                }
                ui.label(
                    RichText::new(format_time(section.start))
                        .monospace()
                        .small(),
                );
                if let Some(index) = section.marker
                    && ui.small_button("🗑").on_hover_text("Delete").clicked()
                {
                    remove = Some(index);
                }
            });
        }
        if let Some(secs) = play {
            self.play_from(secs);
        }
        if let Some((start, end)) = loop_section {
            self.loops.select(start, end);
            self.play_loop();
        }
        if let Some(index) = remove {
            self.markers.remove(index);
        }
    }

    fn loops_section(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.heading("Loops");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_enabled(self.loops.region().is_some(), egui::Button::new("Save"))
                    .on_hover_text("Save the current A-B loop")
                    .clicked()
                {
                    self.loops.save();
                }
            });
        });
        ui.separator();
        if self.loops.saved.is_empty() {
            ui.label(
                RichText::new("Set A and B, or drag on a waveform, then click Save.")
                    .weak()
                    .small(),
            );
        }

        let mut recall = None;
        let mut remove = None;
        for (index, saved) in self.loops.saved.iter_mut().enumerate() {
            let current = self.loops.a == Some(saved.start) && self.loops.b == Some(saved.end);
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(current, "▶")
                    .on_hover_text("Play this loop from its start")
                    .clicked()
                {
                    recall = Some(index);
                }
                ui.add(egui::TextEdit::singleline(&mut saved.name).desired_width(90.0));
                ui.label(
                    RichText::new(format!(
                        "{}–{}",
                        format_time(saved.start),
                        format_time(saved.end)
                    ))
                    .monospace()
                    .small(),
                );
                if ui.small_button("🗑").on_hover_text("Delete").clicked() {
                    remove = Some(index);
                }
            });
        }
        if let Some(index) = recall {
            self.loops.recall(index);
            self.play_loop();
        }
        if let Some(index) = remove {
            self.loops.saved.remove(index);
        }
    }

    fn tracks(&mut self, ui: &mut egui::Ui) {
        let Some(loaded) = &self.loaded else {
            self.welcome(ui);
            return;
        };

        let rate = f64::from(loaded.song.sample_rate);
        let position = (self.playhead.secs() * rate) as usize;
        let loop_region = self.loops.region().map(|(a, b)| {
            let frame = |secs: f64| (secs * rate) as usize;
            (frame(a), frame(b), self.loops.enabled)
        });
        let markers: Vec<(usize, String)> = self
            .markers
            .list
            .iter()
            .enumerate()
            .map(|(i, m)| ((m.time * rate) as usize, format!("{} {}", i + 1, m.name)))
            .collect();
        let visible = (self.view.start * rate) as usize..(self.view.end() * rate) as usize;
        let overlay = Overlay {
            visible,
            position,
            loop_region,
            markers: &markers,
        };
        let mut wave_action = None;
        let mut zoom_action = None;
        // The ruler and the scrollbar are drawn once the rows have shown where the
        // waveforms start; the space beside the ruler holds the zoom buttons.
        let ruler_response = ui
            .horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(CONTROLS_WIDTH, RULER_HEIGHT),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.label(RichText::new("Zoom").weak().small());
                        if ui.small_button("−").on_hover_text("Zoom out (-)").clicked() {
                            zoom_action = Some(Action::Zoom(1.0 / ZOOM_STEP));
                        }
                        if ui.small_button("+").on_hover_text("Zoom in (+)").clicked() {
                            zoom_action = Some(Action::Zoom(ZOOM_STEP));
                        }
                        if ui
                            .add_enabled(!self.view.is_full(), egui::Button::new("Fit").small())
                            .on_hover_text(
                                "Show the whole song. Zoom with Cmd + scroll or a pinch, \
                                 scroll sideways with Shift + scroll or two fingers.",
                            )
                            .clicked()
                        {
                            zoom_action = Some(Action::ZoomToFit);
                        }
                    },
                );
                ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), RULER_HEIGHT),
                    egui::Sense::click_and_drag(),
                )
                .1
            })
            .inner;
        let (_, scrollbar_response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), SCROLLBAR_HEIGHT),
            egui::Sense::click_and_drag(),
        );
        let mut wave_x = None;
        let any_solo = loaded.song.tracks.iter().any(|t| t.params.solo());

        egui::ScrollArea::vertical().show(ui, |ui| {
            for ((track, kind), peaks) in loaded
                .song
                .tracks
                .iter()
                .zip(&loaded.kinds)
                .zip(&loaded.peaks)
            {
                let params = &track.params;
                let audible = !params.mute() && (!any_solo || params.solo());
                let silent = peaks.iter().all(Peaks::is_silent);

                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(CONTROLS_WIDTH, ROW_HEIGHT),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&track.name).color(kind.color()).strong());
                                if silent {
                                    ui.label(RichText::new("silent").weak().small());
                                }
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui
                                        .selectable_label(params.solo(), "S")
                                        .on_hover_text("Solo")
                                        .clicked()
                                    {
                                        params.set_solo(!params.solo());
                                    }
                                    if ui
                                        .selectable_label(params.mute(), "M")
                                        .on_hover_text("Mute")
                                        .clicked()
                                    {
                                        params.set_mute(!params.mute());
                                    }
                                });
                            });
                            ui.horizontal(|ui| {
                                let mut db = gain_to_db(params.gain());
                                let response =
                                    ui.add(volume_slider(&mut db)).on_hover_text(VOLUME_HINT);
                                if double_clicked(&response) {
                                    params.set_gain(1.0);
                                } else if response.changed() {
                                    params.set_gain(db_to_gain(db));
                                }
                                let mut pan = params.pan();
                                let pan_slider = egui::Slider::new(&mut pan, -1.0..=1.0)
                                    .show_value(true)
                                    .custom_formatter(|v, _| format_pan(v as f32))
                                    .custom_parser(parse_pan);
                                let response = ui
                                    .scope(|ui| {
                                        ui.spacing_mut().slider_width = PAN_SLIDER_WIDTH;
                                        ui.add(pan_slider)
                                    })
                                    .inner
                                    .on_hover_text("Pan (double-click to center)");
                                if double_clicked(&response) {
                                    params.set_pan(0.0);
                                } else if response.changed() {
                                    params.set_pan(pan);
                                }
                            });
                        },
                    );

                    let color = if audible {
                        kind.color()
                    } else {
                        kind.color().gamma_multiply(0.3)
                    };
                    let (rect, action) = waveform(ui, peaks, color, ROW_HEIGHT, &overlay);
                    wave_x.get_or_insert(rect.x_range());
                    if action.is_some() {
                        wave_action = action;
                    }
                });
                ui.add_space(4.0);
            }
        });

        let ruler_action = wave_x.and_then(|x_range| {
            ruler(
                ui,
                &ruler_response,
                x_range,
                loaded.song.sample_rate,
                &overlay,
            )
        });
        if let Some(x_range) = wave_x {
            scrollbar(ui, &scrollbar_response, x_range, &mut self.view);
            // Cmd + scroll or a pinch zooms around the pointer; sideways scrolling
            // (Shift + wheel, two fingers) scrolls, over the ruler or the waveforms.
            let area =
                Rect::from_x_y_ranges(x_range, ruler_response.rect.top()..=ui.min_rect().bottom());
            let (pointer, zoom, scroll_x) = ui.input(|i| {
                (
                    i.pointer.hover_pos(),
                    i.zoom_delta(),
                    i.smooth_scroll_delta.x,
                )
            });
            if let Some(pointer) = pointer.filter(|p| area.contains(*p)) {
                let secs_per_point = self.view.span / f64::from(x_range.span().max(1.0));
                if zoom != 1.0 {
                    let anchor =
                        self.view.start + f64::from(pointer.x - x_range.min) * secs_per_point;
                    self.view.zoom(f64::from(zoom), anchor);
                }
                if scroll_x != 0.0 {
                    self.view.pan(-f64::from(scroll_x) * secs_per_point);
                }
            }
        }
        if let Some(action) = zoom_action {
            self.apply(action);
        }
        match ruler_action {
            Some(RulerAction::Seek(frame)) => wave_action = Some(WaveformAction::Seek(frame)),
            Some(RulerAction::AddMarker(frame)) => {
                self.markers.add(frame as f64 / rate);
            }
            None => {}
        }

        match wave_action {
            Some(WaveformAction::Seek(frame)) => {
                if let Some(engine) = &mut self.engine {
                    engine.seek(frame);
                }
            }
            Some(WaveformAction::Select(start, end)) => {
                self.loops.select(start as f64 / rate, end as f64 / rate);
            }
            None => {}
        }
    }
}

impl eframe::App for WatershedApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_loading(ui.ctx());
        self.poll_separation(ui.ctx());
        self.handle_input(ui);
        if let Some(engine) = &mut self.engine {
            engine.collect_garbage();
            if engine.failed() && self.error.is_none() {
                self.error =
                    Some("The audio device stopped working. Choose an output device.".into());
            }
            let xruns = engine.xruns();
            if xruns > self.xruns_logged {
                log::warn!("audio dropouts: {xruns} so far");
            }
            self.xruns_logged = xruns;
        }
        if let Some(engine) = &self.engine
            && engine.needs_restart()
        {
            let id = engine.device().id.clone();
            log::info!("restarting audio on {}", engine.device().name);
            self.switch_device(&id, ui.ctx());
        }

        self.update_playhead();

        egui::Panel::top("top_bar").show(ui, |ui| {
            ui.add_space(4.0);
            self.top_bar(ui);
            ui.add_space(2.0);
        });
        egui::Panel::bottom("transport").show(ui, |ui| {
            ui.add_space(4.0);
            self.transport(ui);
            if let Some(error) = &self.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            ui.add_space(2.0);
        });
        if self.loaded.is_some() {
            egui::Panel::right("side_panel")
                .resizable(true)
                .default_size(SIDE_PANEL_WIDTH)
                .show(ui, |ui| self.side_panel(ui));
        }
        egui::CentralPanel::default_margins().show(ui, |ui| {
            self.separation_status(ui);
            if let Some(loading) = &self.loading {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(format!(
                        "Loading {}… {}/{}",
                        loading.title,
                        loading.done(),
                        loading.total
                    ));
                });
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(100));
            }
            self.tracks(ui);
        });
        self.sync_loop(ui);
        self.autosave_session(ui.ctx());

        if self.engine.as_ref().is_some_and(Engine::is_playing) {
            ui.ctx().request_repaint();
        } else {
            // Keeps the CPU and RAM numbers current while nothing else redraws.
            ui.ctx().request_repaint_after(usage::INTERVAL);
        }
    }

    fn on_exit(&mut self) {
        self.save_session();
    }
}

/// Where the session of a song lives: its folder, next to the first stem.
fn session_path(loaded: &LoadedSong) -> PathBuf {
    let folder = loaded.files[0]
        .parent()
        .unwrap_or(std::path::Path::new("."));
    Session::path_in(folder)
}

/// A stem's file name, which identifies its track in the session.
fn file_name(path: &std::path::Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

const VOLUME_HINT: &str = "Double-click to reset to 0 dB";

/// Sliders only sense drags, so `Response::double_clicked` is always false on them.
/// This adds click sensing to the same widget to detect a double-click for reset.
fn double_clicked(response: &egui::Response) -> bool {
    response.interact(egui::Sense::click()).double_clicked()
}

fn volume_slider(db: &mut f32) -> egui::Slider<'_> {
    egui::Slider::new(db, MIN_DB..=MAX_DB)
        .show_value(true)
        .custom_formatter(|v, _| {
            if v <= f64::from(MIN_DB) {
                "-inf dB".into()
            } else {
                format!("{v:+.1} dB")
            }
        })
}

fn gain_to_db(gain: f32) -> f32 {
    if gain <= 0.0 {
        MIN_DB
    } else {
        (20.0 * gain.log10()).clamp(MIN_DB, MAX_DB)
    }
}

fn db_to_gain(db: f32) -> f32 {
    if db <= MIN_DB {
        0.0
    } else {
        10f32.powf(db / 20.0)
    }
}

fn format_pan(pan: f32) -> String {
    let pct = (pan * 100.0).round() as i32;
    match pct {
        0 => "C".into(),
        p if p < 0 => format!("{}L", -p),
        p => format!("{p}R"),
    }
}

/// Parses a pan value typed by the user: `C`, `30L`, `30R`, or a signed percentage.
fn parse_pan(text: &str) -> Option<f64> {
    let text = text.trim().to_ascii_uppercase();
    let percent = |n: &str| n.trim().parse::<f64>().ok().map(|v| v / 100.0);
    if text == "C" {
        Some(0.0)
    } else if let Some(n) = text.strip_suffix('L') {
        percent(n).map(|v| -v)
    } else if let Some(n) = text.strip_suffix('R') {
        percent(n)
    } else {
        percent(&text)
    }
}

/// A path for display, with the home folder shortened to `~`.
fn short_path(path: &std::path::Path) -> String {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    match home.and_then(|home| path.strip_prefix(home).ok()) {
        Some(rest) => format!("~{}{}", std::path::MAIN_SEPARATOR, rest.display()),
        None => path.display().to_string(),
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// How long ago something happened, given the seconds elapsed: "3 days ago".
fn format_ago(secs: u64) -> String {
    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;
    let plural = |n: u64, unit: &str| {
        let s = if n == 1 { "" } else { "s" };
        format!("{n} {unit}{s} ago")
    };
    match secs {
        s if s < MINUTE => "just now".into(),
        s if s < HOUR => plural(s / MINUTE, "minute"),
        s if s < DAY => plural(s / HOUR, "hour"),
        s if s < 2 * DAY => "yesterday".into(),
        s if s < 14 * DAY => plural(s / DAY, "day"),
        s if s < 60 * DAY => plural(s / (7 * DAY), "week"),
        s => plural(s / (30 * DAY), "month"),
    }
}

fn format_time(secs: f64) -> String {
    let secs = secs.max(0.0);
    format!("{}:{:04.1}", (secs / 60.0) as u64, secs % 60.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_round_trip() {
        assert_eq!(db_to_gain(MIN_DB), 0.0);
        assert_eq!(gain_to_db(0.0), MIN_DB);
        assert!((db_to_gain(0.0) - 1.0).abs() < 1e-6);
        assert!((gain_to_db(db_to_gain(-6.0)) + 6.0).abs() < 1e-4);
    }

    #[test]
    fn formats() {
        assert_eq!(format_time(83.25), "1:23.2");
        assert_eq!(format_pan(0.0), "C");
        assert_eq!(format_pan(-0.5), "50L");
        assert_eq!(format_pan(1.0), "100R");
        assert_eq!(format_ago(5), "just now");
        assert_eq!(format_ago(60), "1 minute ago");
        assert_eq!(format_ago(3 * 3600 + 5), "3 hours ago");
        assert_eq!(format_ago(30 * 3600), "yesterday");
        assert_eq!(format_ago(5 * 86_400), "5 days ago");
        assert_eq!(format_ago(20 * 86_400), "2 weeks ago");
        assert_eq!(format_ago(90 * 86_400), "3 months ago");
    }

    #[test]
    fn parses_pan() {
        assert_eq!(parse_pan("C"), Some(0.0));
        assert_eq!(parse_pan(" 30l "), Some(-0.3));
        assert_eq!(parse_pan("30R"), Some(0.3));
        assert_eq!(parse_pan("-50"), Some(-0.5));
        assert_eq!(parse_pan("left"), None);
    }
}
