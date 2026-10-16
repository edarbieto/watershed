use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::{self, Align, Key, Layout, RichText};
use watershed_audio::{Engine, Notify, OutputDevice, output_devices};
use watershed_storage::Config;

use crate::loader::{self, LoadedSong, Loading};
use crate::waveform::waveform;

const ROW_HEIGHT: f32 = 64.0;
const CONTROLS_WIDTH: f32 = 240.0;
const PAN_SLIDER_WIDTH: f32 = 40.0;
const MIN_DB: f32 = -60.0;
const MAX_DB: f32 = 6.0;
const FALLBACK_SAMPLE_RATE: u32 = 48_000;

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
    restore: Option<Restore>,
    error: Option<String>,
    /// Engine underrun count already logged.
    xruns_logged: u64,
    /// Wakes the UI when the engine needs a restart, even if nothing else repaints.
    notify: Notify,
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
            restore: None,
            error,
            xruns_logged: 0,
            notify,
        }
    }

    fn sample_rate(&self) -> u32 {
        self.engine
            .as_ref()
            .map_or(FALLBACK_SAMPLE_RATE, Engine::sample_rate)
    }

    pub fn open(&mut self, paths: &[PathBuf], ctx: &egui::Context) {
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
        self.loading = Some(loader::start(files, self.sample_rate(), ctx.clone()));
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

    fn poll_loading(&mut self) {
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
                } else if let Some(engine) = &mut self.engine {
                    engine.load(loaded.song.clone());
                }
                self.loaded = Some(loaded);
            }
            Err(e) => self.error = Some(e),
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
        // Close the old stream first; some drivers allow only one at a time.
        self.engine = None;
        let engine = match Engine::start(Some(id), self.notify.clone()) {
            Ok(engine) => engine,
            Err(e) => {
                self.error = Some(e.to_string());
                self.engine = Engine::start(None, self.notify.clone()).ok();
                return;
            }
        };
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
        let Some(engine) = &mut self.engine else {
            return;
        };
        let rate = engine.sample_rate() as usize;
        ui.input(|i| {
            if i.key_pressed(Key::Space) {
                engine.toggle();
            }
            if i.key_pressed(Key::Home) {
                engine.seek(0);
            }
            if i.key_pressed(Key::ArrowLeft) {
                engine.seek(engine.position().saturating_sub(5 * rate));
            }
            if i.key_pressed(Key::ArrowRight) {
                engine.seek(engine.position() + 5 * rate);
            }
        });
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Open folder…").clicked() {
                self.pick_folder(ui.ctx());
            }
            if let Some(loaded) = &self.loaded {
                ui.label(RichText::new(&loaded.title).strong());
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
            });
        });
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
        ui.horizontal(|ui| {
            let Some(engine) = &mut self.engine else {
                ui.label("No audio output");
                return;
            };
            let enabled = self.loaded.is_some();
            let play_label = if engine.is_playing() { "⏸" } else { "▶" };
            if ui
                .add_enabled(
                    enabled,
                    egui::Button::new(RichText::new(play_label).size(18.0)),
                )
                .on_hover_text("Play / pause (Space)")
                .clicked()
            {
                engine.toggle();
            }
            if ui
                .add_enabled(enabled, egui::Button::new(RichText::new("⏹").size(18.0)))
                .on_hover_text("Stop")
                .clicked()
            {
                engine.stop();
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

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} Hz", engine.sample_rate()))
                        .weak()
                        .small(),
                );
            });
        });
    }

    fn tracks(&mut self, ui: &mut egui::Ui) {
        let Some(loaded) = &self.loaded else {
            ui.centered_and_justified(|ui| {
                ui.label(
                    RichText::new("Drop a folder of stems here, or click “Open folder…”")
                        .weak()
                        .size(18.0),
                );
            });
            return;
        };

        let position = self.engine.as_ref().map_or(0, Engine::position);
        let mut seek = None;
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
                let silent = peaks.max.iter().chain(&peaks.min).all(|s| s.abs() < 1e-4);

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
                    if let Some(frame) = waveform(ui, peaks, color, ROW_HEIGHT, position) {
                        seek = Some(frame);
                    }
                });
                ui.add_space(4.0);
            }
        });

        if let (Some(frame), Some(engine)) = (seek, &mut self.engine) {
            engine.seek(frame);
        }
    }
}

impl eframe::App for WatershedApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_loading();
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
        egui::CentralPanel::default_margins().show(ui, |ui| {
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

        if self.engine.as_ref().is_some_and(Engine::is_playing) {
            ui.ctx().request_repaint();
        }
    }
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
