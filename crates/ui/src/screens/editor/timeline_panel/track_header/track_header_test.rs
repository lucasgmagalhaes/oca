use super::*;

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
