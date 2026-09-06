use crate::i18n::Text;

pub(super) fn audio_role_icon(role: avcore::AudioRole) -> &'static str {
    match role {
        avcore::AudioRole::Unspecified => "-",
        avcore::AudioRole::GameAudio => "G",
        avcore::AudioRole::Mic => "V",
        avcore::AudioRole::Music => "N",
    }
}

pub(super) fn audio_role_label(
    role: avcore::AudioRole,
    locale: crate::i18n::Locale,
) -> &'static str {
    match role {
        avcore::AudioRole::Unspecified => Text::AudioRoleUnspecified.tr(locale),
        avcore::AudioRole::GameAudio => Text::AudioRoleGameAudio.tr(locale),
        avcore::AudioRole::Mic => Text::AudioRoleMic.tr(locale),
        avcore::AudioRole::Music => Text::AudioRoleMusic.tr(locale),
    }
}
