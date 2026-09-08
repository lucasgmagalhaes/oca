use super::*;

#[test]
fn track_header_reserves_the_entire_shared_gutter() {
    let ctx = egui::Context::default();
    let mut reserved_width = 0.0;
    let mut cursor_advance = 0.0;

    let frame = ctx.run_ui(egui::RawInput::default(), |ui| {
        ui.horizontal(|ui| {
            let start = ui.cursor().left();
            let gutter = reserve_track_header(ui, 184.0, 56.0);

            reserved_width = gutter.width();
            cursor_advance = ui.cursor().left() - start;
        });
    });
    frame.drop_without_applying_deltas();

    assert_eq!(reserved_width, 184.0);
    assert!(
        cursor_advance >= 184.0,
        "the next timeline canvas must begin after the full reserved gutter"
    );
}

const ROLES: [avcore::AudioRole; 4] = [
    avcore::AudioRole::Unspecified,
    avcore::AudioRole::GameAudio,
    avcore::AudioRole::Mic,
    avcore::AudioRole::Music,
];

#[test]
fn every_audio_role_has_a_distinct_non_empty_icon() {
    let mut icons: Vec<&str> = ROLES.iter().map(|&r| audio_role_icon(r)).collect();
    for icon in &icons {
        assert!(!icon.is_empty());
    }
    icons.sort_unstable();
    icons.dedup();
    assert_eq!(icons.len(), ROLES.len());
}

#[test]
fn every_audio_role_has_a_distinct_non_empty_label_per_locale() {
    for locale in [Locale::PtBr, Locale::En] {
        let mut labels: Vec<&str> = ROLES.iter().map(|&r| audio_role_label(r, locale)).collect();
        for label in &labels {
            assert!(!label.is_empty());
        }
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), ROLES.len());
    }
}
