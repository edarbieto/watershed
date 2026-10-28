//! The metronome click and count-in: their settings, what the engine needs to play
//! them, and their controls in the transport bar.

use eframe::egui::{self, RichText};
use watershed_audio::{Beats, Engine};
use watershed_storage::{ClickChannels, ClickSound, Config, Session};

const MIN_VOLUME_DB: f32 = -40.0;
/// +12 dB, the engine's loudest click.
const MAX_VOLUME_DB: f32 = 12.0;

/// The settings saved with each song. The sound, channels and volume are global,
/// in the config.
#[derive(Debug, Default)]
pub struct Metronome {
    pub click: bool,
    pub count_in: bool,
}

impl Metronome {
    pub fn from_session(session: &Session) -> Self {
        Self {
            click: session.click,
            count_in: session.count_in,
        }
    }

    /// Sends everything to the engine. Cheap: called on every frame, so a new engine
    /// (another output device) gets the settings too.
    pub fn sync(&self, engine: &mut Engine, config: &Config, beats: Option<Beats>) {
        engine.set_beats(beats);
        engine.set_click(self.click);
        engine.set_count_in(self.count_in);
        engine.set_click_sound(match config.click_sound {
            ClickSound::Click => watershed_audio::ClickSound::Click,
            ClickSound::Metronome => watershed_audio::ClickSound::Metronome,
            ClickSound::Wood => watershed_audio::ClickSound::Wood,
        });
        engine.set_click_channels(match config.click_channels {
            ClickChannels::Left => watershed_audio::ClickChannels::Left,
            ClickChannels::Both => watershed_audio::ClickChannels::Both,
            ClickChannels::Right => watershed_audio::ClickChannels::Right,
        });
        let db = config.click_volume_db.clamp(MIN_VOLUME_DB, MAX_VOLUME_DB);
        engine.set_click_gain(10f32.powf(db / 20.0));
    }
}

/// The Click and Count-in toggles and a menu with the sound, channels and volume.
/// `available` is false without a grid. Returns true if the config changed.
pub fn controls(
    ui: &mut egui::Ui,
    metronome: &mut Metronome,
    config: &mut Config,
    available: bool,
) -> bool {
    let mut changed = false;
    let disabled_hint = "Needs a BPM and the beat grid on";
    ui.add_enabled_ui(available, |ui| {
        if ui
            .add(egui::Button::selectable(metronome.click, "Click"))
            .on_hover_text("Metronome on the beat grid, first beat of each bar accented (C)")
            .on_disabled_hover_text(disabled_hint)
            .clicked()
        {
            metronome.click = !metronome.click;
        }
        if ui
            .add(egui::Button::selectable(metronome.count_in, "Count-in"))
            .on_hover_text("One bar of clicks before playback starts")
            .on_disabled_hover_text(disabled_hint)
            .clicked()
        {
            metronome.count_in = !metronome.count_in;
        }
        let response = ui
            .add(egui::Button::new("⏷"))
            .on_hover_text("Click sound, channels and volume")
            .on_disabled_hover_text(disabled_hint);
        egui::Popup::menu(&response)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.label(RichText::new("Sound").strong());
                for (sound, name) in [
                    (ClickSound::Click, "Click"),
                    (ClickSound::Metronome, "Metronome"),
                    (ClickSound::Wood, "Wood"),
                ] {
                    changed |= ui
                        .radio_value(&mut config.click_sound, sound, name)
                        .changed();
                }
                ui.add_space(6.0);
                ui.label(RichText::new("Channels").strong());
                ui.horizontal(|ui| {
                    for (channels, name) in [
                        (ClickChannels::Left, "Left"),
                        (ClickChannels::Both, "Both"),
                        (ClickChannels::Right, "Right"),
                    ] {
                        changed |= ui
                            .radio_value(&mut config.click_channels, channels, name)
                            .changed();
                    }
                });
                ui.add_space(6.0);
                ui.label(RichText::new("Volume").strong());
                let response = ui.add(
                    egui::Slider::new(&mut config.click_volume_db, MIN_VOLUME_DB..=MAX_VOLUME_DB)
                        .step_by(0.5)
                        .suffix(" dB"),
                );
                // Saved when the drag ends, not on every step.
                changed |= response.drag_stopped() || (response.changed() && !response.dragged());
            });
    });
    changed
}
