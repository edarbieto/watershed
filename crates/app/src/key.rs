//! The song's key: estimated on load, corrected by the user, saved in the session.
//! Stored at the original pitch; what is shown follows the transposition. Also the
//! song's tuning (where its A sits against 440 Hz), measured with the key.

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

    /// How far the song's tuning is from A = 440 Hz, in cents (-50 to 50), when
    /// measured.
    pub fn tuning_cents(&self) -> Option<f32> {
        self.estimate.as_ref().map(|e| e.tuning_cents)
    }

    /// The user's key, to save in the session.
    pub fn user(&self) -> Option<String> {
        self.user.map(|key| key.to_string())
    }
}

/// Common references besides 440 Hz, in Hz, within half a semitone of it.
const NAMED_REFERENCES: [f32; 4] = [432.0, 435.0, 442.0, 444.0];
/// A tuning this close to a named reference, in cents, is shown as it.
const NAMED_CENTS: f32 = 3.0;

/// The pitch of A, in Hz, for a tuning `cents` away from A = 440 Hz.
fn reference_hz(cents: f32) -> f32 {
    440.0 * 2f32.powf(cents / 1200.0)
}

/// A tuning folded to within half a semitone (-50 to 50 cents): a song a whole
/// semitone flat is in tune, in another key.
fn fold_cents(cents: f32) -> f32 {
    cents - 100.0 * (cents / 100.0).round()
}

/// `A = 430.7 Hz`, with the named reference it is close to, if any.
fn describe_tuning(cents: f32) -> String {
    let hz = reference_hz(cents);
    let named = NAMED_REFERENCES
        .iter()
        .find(|&&named| (1200.0 * (hz / named).log2()).abs() <= NAMED_CENTS);
    match named {
        Some(named) => format!("A = {hz:.1} Hz (about {named:.0})"),
        None => format!("A = {hz:.1} Hz"),
    }
}

/// The key as heard after transposing by `semitones` and `cents`, with a menu to
/// correct it and to tune the song to A = 440 Hz. Returns the cents to set, when
/// the user asks for that.
pub fn controls(
    ui: &mut egui::Ui,
    key: &mut SongKey,
    semitones: i32,
    cents: f32,
    enabled: bool,
) -> Option<f32> {
    let shown = key.value().map(|k| k.transposed(semitones));
    let text = shown.map_or("Key ?".to_owned(), |k| format!("Key {k}"));
    let mut hint = match (&key.estimate, key.user) {
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
    if let Some(tuning) = key.tuning_cents() {
        hint.push_str(&format!("\n\nTuning: {}", describe_tuning(tuning)));
        if cents != 0.0 {
            let heard = fold_cents(tuning + cents);
            hint.push_str(&format!(", as heard {}", describe_tuning(heard)));
        }
    }
    let mut set_cents = None;
    ui.add_enabled_ui(enabled, |ui| {
        let response = ui.add(egui::Button::new(text)).on_hover_text(hint);
        egui::Popup::menu(&response)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                set_cents = tuning(ui, key, cents);
                menu(ui, key, semitones);
            });
    });
    set_cents
}

/// The song's tuning and a button to bring it to A = 440 Hz; returns the cents to
/// set when clicked.
fn tuning(ui: &mut egui::Ui, key: &SongKey, cents: f32) -> Option<f32> {
    let tuning = key.tuning_cents()?;
    ui.label(format!("Tuning: {}", describe_tuning(tuning)));
    let heard = fold_cents(tuning + cents);
    if cents != 0.0 {
        ui.label(
            RichText::new(format!("As heard: {}", describe_tuning(heard)))
                .weak()
                .small(),
        );
    }
    let correction = -tuning.round();
    let clicked = ui
        .add_enabled(heard.abs() >= 0.5, egui::Button::new("Set to 440 Hz"))
        .on_hover_text(format!(
            "Sets the fine pitch to {correction:+.0} cents, so the song plays at A = 440 Hz \
             and matches an instrument tuned to it"
        ))
        .clicked();
    ui.separator();
    clicked.then_some(correction)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tunings_read_as_a_in_hz() {
        assert_eq!(describe_tuning(0.0), "A = 440.0 Hz");
        // Black Hole Sun, measured.
        assert_eq!(describe_tuning(-37.0), "A = 430.7 Hz");
        assert_eq!(describe_tuning(-32.0), "A = 431.9 Hz (about 432)");
        assert_eq!(describe_tuning(16.0), "A = 444.1 Hz (about 444)");
    }

    #[test]
    fn tunings_fold_within_half_a_semitone() {
        assert_eq!(fold_cents(-37.0 + 37.0), 0.0);
        assert_eq!(fold_cents(40.0 + 30.0), -30.0);
        assert_eq!(fold_cents(-45.0 - 10.0), 45.0);
    }
}
