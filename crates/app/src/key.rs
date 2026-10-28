//! The song's key: estimated on load, corrected by the user, saved in the session.
//! Stored at the original pitch; what is shown follows the transposition.

use eframe::egui::{self, RichText};
use watershed_dsp::{Key, KeyEstimate};

#[derive(Debug, Default)]
pub struct SongKey {
    /// From the pitched stems, when the song was loaded.
    estimate: Option<KeyEstimate>,
    /// Set by the user; wins over the estimate.
    user: Option<Key>,
}

impl SongKey {
    /// `user` is the session's value; one that does not parse is ignored.
    pub fn new(estimate: Option<KeyEstimate>, user: Option<&str>) -> Self {
        Self {
            estimate,
            user: user.and_then(|text| text.parse().ok()),
        }
    }

    /// The key at the original pitch, if known.
    pub fn value(&self) -> Option<Key> {
        self.user.or(self.estimate.as_ref().map(|e| e.key))
    }

    /// The user's key, to save in the session.
    pub fn user(&self) -> Option<String> {
        self.user.map(|key| key.to_string())
    }
}

/// The key as heard after transposing by `semitones`, with a menu to correct it.
pub fn controls(ui: &mut egui::Ui, key: &mut SongKey, semitones: i32, enabled: bool) {
    let shown = key.value().map(|k| k.transposed(semitones));
    let text = shown.map_or("Key ?".to_owned(), |k| format!("Key {k}"));
    let hint = match (&key.estimate, key.user) {
        (_, Some(_)) => "Key set by you. Click to change it or go back to the estimate.".to_owned(),
        (Some(estimate), None) => format!(
            "Estimated key ({}). Its relative, {}, uses the same notes and is the usual \
             mix-up; the next best guess was {}. Click to correct it.",
            confidence_label(estimate.confidence),
            estimate.key.relative().transposed(semitones),
            estimate.second.transposed(semitones),
        ),
        (None, None) => "No key found. Click to set it.".to_owned(),
    };
    ui.add_enabled_ui(enabled, |ui| {
        let response = ui.add(egui::Button::new(text)).on_hover_text(hint);
        egui::Popup::menu(&response)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| menu(ui, key, semitones));
    });
}

fn menu(ui: &mut egui::Ui, key: &mut SongKey, semitones: i32) {
    if semitones != 0 {
        ui.label(
            RichText::new(format!("As heard, transposed {semitones:+} semitones"))
                .weak()
                .small(),
        );
    }
    if let Some(estimate) = key.estimate.clone() {
        let estimated = estimate.key.transposed(semitones);
        if ui
            .selectable_label(key.user.is_none(), format!("Estimate: {estimated}"))
            .clicked()
        {
            key.user = None;
        }
        let relative = estimate.key.relative();
        if ui
            .selectable_label(
                key.user == Some(relative),
                format!("Relative: {}", relative.transposed(semitones)),
            )
            .on_hover_text("Same notes as the estimate: the usual mix-up")
            .clicked()
        {
            key.user = Some(relative);
        }
        ui.separator();
    }
    // Every key, as heard; stored at the original pitch.
    for minor in [false, true] {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for tonic in 0..12 {
                let heard = Key::new(tonic, minor);
                let original = heard.transposed(-semitones);
                let selected = key.value() == Some(original);
                let label = RichText::new(heard.to_string()).monospace();
                if ui
                    .add(egui::Button::selectable(selected, label).small())
                    .clicked()
                {
                    key.user = Some(original);
                }
            }
        });
    }
}

fn confidence_label(confidence: f32) -> &'static str {
    if confidence >= 0.8 {
        "clear"
    } else if confidence >= 0.6 {
        "likely"
    } else {
        "uncertain"
    }
}
