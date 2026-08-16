//! `.srt` subtitle file export — `request.md`'s Fase 4 ask for subtitles "exportável como
//! texto embutido no vídeo ou como arquivo `.srt` separado". The embedded half already exists
//! (`TextClip`'s `drawtext` rendering on export); this covers the separate-file half. Pure
//! string formatting over already-placed [`crate::timeline::TextClip`]s — no FFI or
//! render-pipeline involvement, so it works the same whether or not this build can decode or
//! encode anything.

use crate::timeline::{TextClip, Timeline, TrackKind};

/// Renders every [`TextClip`] across every [`TrackKind::Text`] track in `timeline` as one
/// SubRip (`.srt`) file, ordered by `start_secs`. Clips whose text is empty/whitespace-only are
/// skipped — an empty caption entry has no meaning in a subtitle file. Returns an empty string
/// if there are no text clips at all.
pub fn export_srt(timeline: &Timeline) -> String {
    let mut clips: Vec<&TextClip> = timeline
        .tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Text)
        .flat_map(|track| track.text_clips.iter())
        .filter(|clip| !clip.text.trim().is_empty())
        .collect();
    clips.sort_by(|a, b| {
        a.start_secs
            .partial_cmp(&b.start_secs)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut out = String::new();
    for (index, clip) in clips.iter().enumerate() {
        out.push_str(&(index + 1).to_string());
        out.push('\n');
        out.push_str(&format_srt_timestamp(clip.start_secs));
        out.push_str(" --> ");
        out.push_str(&format_srt_timestamp(clip.start_secs + clip.duration_secs));
        out.push('\n');
        out.push_str(&clip.text);
        out.push_str("\n\n");
    }
    out
}

/// Formats seconds as SRT's `HH:MM:SS,mmm` timestamp. Negative input clamps to zero — a
/// `start_secs` before the timeline origin shouldn't happen, but a subtitle file has no way to
/// represent a negative time regardless.
fn format_srt_timestamp(total_secs: f64) -> String {
    let total_millis = (total_secs.max(0.0) * 1000.0).round() as u64;
    let millis = total_millis % 1000;
    let total_seconds = total_millis / 1000;
    let secs = total_seconds % 60;
    let total_minutes = total_seconds / 60;
    let mins = total_minutes % 60;
    let hours = total_minutes / 60;
    format!("{hours:02}:{mins:02}:{secs:02},{millis:03}")
}
