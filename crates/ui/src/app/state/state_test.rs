use eframe::egui;

use super::*;

fn input_with_key_press(key: egui::Key, modifiers: egui::Modifiers) -> egui::InputState {
    let mut input = egui::InputState::default();
    input.modifiers = modifiers;
    input.events.push(egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    });
    input
}

fn combo(ctrl: bool, shift: bool, key_name: &str) -> KeyCombo {
    KeyCombo {
        ctrl,
        shift,
        key_name: key_name.to_string(),
    }
}

#[test]
fn display_with_no_modifiers_shows_just_the_key() {
    assert_eq!(combo(false, false, "Space").display(), "Space");
}

#[test]
fn display_with_ctrl_only() {
    assert_eq!(combo(true, false, "B").display(), "Ctrl+B");
}

#[test]
fn display_with_shift_only() {
    assert_eq!(combo(false, true, "C").display(), "Shift+C");
}

#[test]
fn display_with_ctrl_and_shift() {
    assert_eq!(combo(true, true, "V").display(), "Ctrl+Shift+V");
}

#[test]
fn matches_a_key_pressed_with_the_exact_same_modifiers() {
    let bound = combo(true, false, "B");
    let input = input_with_key_press(egui::Key::B, egui::Modifiers::CTRL);
    assert!(bound.matches(&input));
}

#[test]
fn does_not_match_when_ctrl_is_required_but_not_held() {
    let bound = combo(true, false, "B");
    let input = input_with_key_press(egui::Key::B, egui::Modifiers::NONE);
    assert!(!bound.matches(&input));
}

#[test]
fn does_not_match_when_an_extra_modifier_is_held() {
    let bound = combo(true, false, "B");
    let input = input_with_key_press(egui::Key::B, egui::Modifiers::CTRL | egui::Modifiers::SHIFT);
    assert!(!bound.matches(&input));
}

#[test]
fn does_not_match_a_different_key() {
    let bound = combo(true, false, "B");
    let input = input_with_key_press(egui::Key::A, egui::Modifiers::CTRL);
    assert!(!bound.matches(&input));
}

#[test]
fn does_not_match_when_the_key_was_not_pressed_this_frame() {
    let bound = combo(true, false, "B");
    let input = egui::InputState::default();
    assert!(!bound.matches(&input));
}

#[test]
fn an_unrecognized_key_name_never_matches_without_panicking() {
    let bound = combo(false, false, "NotARealKeyName");
    let input = egui::InputState::default();
    assert!(!bound.matches(&input));
}

#[test]
fn key_bindings_default_assigns_a_distinct_combo_to_every_action() {
    let bindings = KeyBindings::default();
    let combos = [
        &bindings.play_pause,
        &bindings.split_at_playhead,
        &bindings.copy_formatting,
        &bindings.paste_formatting,
        &bindings.add_opacity_marker,
        &bindings.undo,
        &bindings.redo,
    ];
    for (i, a) in combos.iter().enumerate() {
        for b in &combos[i + 1..] {
            assert_ne!(
                a, b,
                "two default key bindings must never collide on the same combo"
            );
        }
    }
}
