// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use super::App;

/// Which color field of a text overlay is being edited by the shared color modal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextColorTarget {
    Foreground,
    Background,
    Highlight,
}

/// Transactional state for the text color modal. Project and sequence ids prevent a clip id
/// from resolving to a different clip after switching context, because clip ids are local.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextColorEdit {
    pub project_id: u64,
    pub sequence_id: u64,
    pub clip_id: u64,
    pub target: TextColorTarget,
    pub rgba: [u8; 4],
    pub manual_input: String,
    pub manual_invalid: bool,
}

/// Common editor-friendly colors shown as one-click swatches in the color modal.
pub(crate) const COLOR_PRESETS: [[u8; 4]; 16] = [
    [255, 255, 255, 255],
    [192, 192, 192, 255],
    [96, 96, 96, 255],
    [0, 0, 0, 255],
    [239, 68, 68, 255],
    [249, 115, 22, 255],
    [250, 204, 21, 255],
    [34, 197, 94, 255],
    [20, 184, 166, 255],
    [6, 182, 212, 255],
    [59, 130, 246, 255],
    [99, 102, 241, 255],
    [168, 85, 247, 255],
    [236, 72, 153, 255],
    [244, 63, 94, 255],
    [0, 0, 0, 0],
];

/// Formats an RGBA value without losing alpha. The parser accepts this output verbatim.
pub(crate) fn format_color_hex(rgba: [u8; 4]) -> String {
    format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        rgba[0], rgba[1], rgba[2], rgba[3]
    )
}

/// Parses common manually-entered color formats:
/// `#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA`, `rgb(r,g,b)`, `rgba(r,g,b,a)`, and bare comma-
/// separated RGB(A). Alpha accepts 0..1, 0..100%, or 0..255.
pub(crate) fn parse_color_value(input: &str) -> Result<[u8; 4], ()> {
    let input = input.trim();
    if input.starts_with('#') || input.starts_with("0x") || input.starts_with("0X") {
        return parse_hex_color(input);
    }

    let lower = input.to_ascii_lowercase();
    let body = if lower.starts_with("rgba(") && input.ends_with(')') {
        &input[5..input.len() - 1]
    } else if lower.starts_with("rgb(") && input.ends_with(')') {
        &input[4..input.len() - 1]
    } else {
        input
    };
    let parts: Vec<_> = body.split(',').map(str::trim).collect();
    if !(parts.len() == 3 || parts.len() == 4) {
        return Err(());
    }
    let r = parse_byte(parts[0])?;
    let g = parse_byte(parts[1])?;
    let b = parse_byte(parts[2])?;
    let a = if parts.len() == 4 {
        parse_alpha(parts[3])?
    } else {
        255
    };
    Ok([r, g, b, a])
}

fn parse_hex_color(input: &str) -> Result<[u8; 4], ()> {
    let hex = input
        .strip_prefix('#')
        .or_else(|| input.strip_prefix("0x"))
        .or_else(|| input.strip_prefix("0X"))
        .ok_or(())?;
    match hex.len() {
        3 | 4 => {
            let mut channels = [255_u8; 4];
            for (index, digit) in hex.bytes().enumerate() {
                let nibble = (digit as char).to_digit(16).ok_or(())? as u8;
                channels[index] = nibble * 17;
            }
            Ok(channels)
        }
        6 | 8 => {
            let mut channels = [255_u8; 4];
            for (index, pair) in hex.as_bytes().chunks_exact(2).enumerate() {
                let pair = std::str::from_utf8(pair).map_err(|_| ())?;
                channels[index] = u8::from_str_radix(pair, 16).map_err(|_| ())?;
            }
            Ok(channels)
        }
        _ => Err(()),
    }
}

fn parse_byte(input: &str) -> Result<u8, ()> {
    input.parse::<u8>().map_err(|_| ())
}

fn parse_alpha(input: &str) -> Result<u8, ()> {
    if let Some(percent) = input.strip_suffix('%') {
        let value = percent.trim().parse::<f32>().map_err(|_| ())?;
        if !(0.0..=100.0).contains(&value) {
            return Err(());
        }
        return Ok((value * 2.55).round() as u8);
    }

    let value = input.parse::<f32>().map_err(|_| ())?;
    if !(0.0..=255.0).contains(&value) {
        return Err(());
    }
    if value <= 1.0 {
        Ok((value * 255.0).round() as u8)
    } else {
        Ok(value.round() as u8)
    }
}

impl App {
    /// Opens the text color modal with a detached working copy of the selected field.
    pub(crate) fn begin_text_color_edit(
        &mut self,
        clip_id: u64,
        target: TextColorTarget,
        rgba: [u8; 4],
    ) {
        let project_id = self.active_project().id;
        let sequence_id = self.active_project().sequences[self.active_project().active_sequence].id;
        self.text_color_edit = Some(TextColorEdit {
            project_id,
            sequence_id,
            clip_id,
            target,
            rgba,
            manual_input: format_color_hex(rgba),
            manual_invalid: false,
        });
    }

    /// Commits the staged modal value to the exact text clip and sequence that opened it.
    /// Returns whether a clip was changed.
    pub(crate) fn confirm_text_color_edit(&mut self) -> bool {
        let Some(edit) = self.text_color_edit.take() else {
            return false;
        };
        if self.active_project().id != edit.project_id {
            return false;
        }
        let active_sequence_id =
            self.active_project().sequences[self.active_project().active_sequence].id;
        if active_sequence_id != edit.sequence_id {
            return false;
        }
        let exists = self
            .active_project()
            .timeline()
            .tracks
            .iter()
            .flat_map(|track| &track.text_clips)
            .any(|clip| clip.id == edit.clip_id);
        if !exists {
            return false;
        }

        self.push_undo_snapshot();
        let timeline = self.active_project_mut().timeline_mut();
        let clip = timeline
            .tracks
            .iter_mut()
            .flat_map(|track| &mut track.text_clips)
            .find(|clip| clip.id == edit.clip_id)
            .expect("text clip existence was checked before taking a mutable project borrow");
        match edit.target {
            TextColorTarget::Foreground => clip.color_rgba = edit.rgba,
            TextColorTarget::Background => clip.background_rgba = edit.rgba,
            TextColorTarget::Highlight => clip.highlight_color_rgba = edit.rgba,
        }
        // A color is a content-only change, same category as font/text/position edits in
        // `properties_panel::text_clip_properties` — a fresh raster into the already-open
        // branch, not a full pipeline reopen.
        self.refresh_preview_text_content(edit.clip_id);
        true
    }
}

#[cfg(test)]
#[path = "color/color_test.rs"]
mod tests;
