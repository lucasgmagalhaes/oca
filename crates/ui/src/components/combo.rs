use eframe::egui::{self, Ui};

/// A dropdown over a fixed, known set of `Copy + PartialEq` enum values (mask shape, color
/// filter, transition type, ...), each rendered via `label`. Returns whether the selection
/// changed.
pub fn enum_combo<T: Copy + PartialEq>(
    ui: &mut Ui,
    id_salt: &str,
    options: &[T],
    selected: &mut T,
    label: impl Fn(T) -> String,
) -> bool {
    let mut changed = false;
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(label(*selected))
        .show_ui(ui, |ui| {
            for &option in options {
                changed |= ui
                    .selectable_value(selected, option, label(option))
                    .changed();
            }
        });
    changed
}
