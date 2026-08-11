use serde::{Deserialize, Serialize};

/// What a [`Track`] carries. Determines how the timeline widget renders its clips
/// (thumbnails for video, waveforms for audio) and which asset kind can be dropped onto it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackKind {
    Video,
    Audio,
}

/// Layer mask shape for a block, per `request.md`'s Fase 4 "Máscaras" spec — clips a layer to a
/// shape instead of the plain rectangular crop ([`ClipInstance::crop_x`] etc.), e.g. for webcam
/// frames. `Custom` (a user-drawn shape) isn't supported yet — only the two fixed shapes the
/// spec names alongside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MaskShape {
    #[default]
    None,
    Circle,
    RoundedRect,
}

/// Color filter for a block, per `request.md`'s Fase 4 "Efeitos visuais" spec ("Preto e branco
/// e sépia"). A bounded subset of the eventual "Filtros de cor e LUTs" library — just these two
/// fixed looks, no adjustable LUT yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ColorFilter {
    #[default]
    None,
    BlackAndWhite,
    Sepia,
}

/// One placed instance of a `MediaAsset` on the timeline. `source_in_secs`/`source_out_secs`
/// mark the trimmed range within the source asset; `start_secs` is its position on the track.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClipInstance {
    pub id: u64,
    pub asset_id: u64,
    pub start_secs: f64,
    pub source_in_secs: f64,
    pub source_out_secs: f64,
    /// `Some(group_id)` if this clip is a member of a composite block (per `request.md`'s
    /// Fase 3 "blocos compostos" spec) — every clip sharing the same id, always on the same
    /// track (composite blocks don't span tracks yet), moves/splits/deletes together as a
    /// unit (see `ui`'s `OcaApp::merge_into_composite` and the timeline panel's drag/delete
    /// handling). `#[serde(default)]` so a project saved before this field existed still
    /// loads, every clip in it just standalone (`None`).
    #[serde(default)]
    pub composite_id: Option<u64>,
    /// Volume adjustment in decibels applied to this block's audio, independent of every other
    /// clip — per `request.md`'s Fase 4 "ganho de volume por bloco" spec. `0.0` is unity gain.
    /// Currently only feeds the timeline waveform display (`ui`'s `draw_waveform`, scaled by
    /// [`ClipInstance::gain_linear`]); export doesn't mix the timeline yet (it still
    /// passthrough-renders a single source file per job, see `core::render`), so this doesn't
    /// affect exported audio yet. `#[serde(default)]` so older saved projects load at unity
    /// gain.
    #[serde(default)]
    pub gain_db: f32,
    /// `true` if this block is frozen — holds a single still frame
    /// ([`ClipInstance::source_in_secs`]) for its whole displayed duration instead of playing
    /// through the trimmed source range, per `request.md`'s Fase 4 "Congelar" spec ("segura um
    /// quadro específico por uma duração configurável"). The held frame is picked by trimming
    /// `source_in_secs` to it; the hold duration is just the block's existing on-timeline
    /// length ([`ClipInstance::duration_secs`]), adjustable the same way as any other clip via
    /// the existing trim handles — freeze doesn't need its own duration field. Currently only
    /// changes how the timeline draws the block (`ui`'s timeline panel shows a single repeated
    /// poster frame instead of a filmstrip, per-position thumbnails); it doesn't yet affect
    /// preview playback (`avcore::preview::Preview` always plays the real decoded source) or
    /// export (still passthrough-copies video, see `core::render`) — the same kind of
    /// preview/export gap as [`ClipInstance::gain_db`]. `#[serde(default)]` so older saved
    /// projects load unfrozen.
    #[serde(default)]
    pub frozen: bool,
    /// Playback speed multiplier for this block — `2.0` plays twice as fast, `0.5` half speed,
    /// per `request.md`'s Fase 4 "Velocidade" spec. `1.0` is normal speed. Currently only shown
    /// as a badge on the timeline block (`ui`'s timeline panel); it doesn't yet resample audio,
    /// change the block's on-timeline duration, or affect preview playback or export — the same
    /// kind of gap as [`ClipInstance::gain_db`]/[`ClipInstance::frozen`], just earlier: speed
    /// needs the timeline to support a block whose on-screen length differs from
    /// `source_out_secs - source_in_secs`, which nothing here does yet. `#[serde(default = ..)]`
    /// so older saved projects load at normal speed.
    #[serde(default = "default_speed_factor")]
    pub speed_factor: f32,
    /// Normalized crop rectangle within the source frame — `(crop_x, crop_y)` is the visible
    /// sub-rectangle's top-left corner, `(crop_w, crop_h)` its size, all fractions of the full
    /// frame (`0.0..=1.0`). Defaults to `(0.0, 0.0, 1.0, 1.0)` — the whole frame, uncropped —
    /// per `request.md`'s Fase 4 "Recorte (crop)" spec: reframing separate from the time-based
    /// split already covered in Fase 3. Currently only shown as a badge on the timeline block
    /// (`ui`'s timeline panel, via [`ClipInstance::is_cropped`]); doesn't yet affect preview
    /// playback or export — the same kind of gap as [`ClipInstance::gain_db`]. Independently
    /// clamped to `[0.0, 1.0]` when set (`ui`'s `OcaApp::set_selected_clip_crop`); a crop rect
    /// extending past the frame edge (`crop_x + crop_w > 1.0`) isn't rejected — a known
    /// simplification with no visible effect yet since nothing renders the crop.
    /// `#[serde(default = ..)]` so older saved projects load uncropped.
    #[serde(default)]
    pub crop_x: f32,
    #[serde(default)]
    pub crop_y: f32,
    #[serde(default = "default_crop_extent")]
    pub crop_w: f32,
    #[serde(default = "default_crop_extent")]
    pub crop_h: f32,
    /// Layer mask shape ([`MaskShape::None`] by default — unmasked). Independent of the
    /// rectangular crop above; a block can be both cropped and masked. `#[serde(default)]` so
    /// older saved projects load unmasked.
    #[serde(default)]
    pub mask_shape: MaskShape,
    /// Corner radius for [`MaskShape::RoundedRect`], as a fraction (`0.0..=1.0`) of the block's
    /// shorter frame dimension — meaningless for the other shapes. `#[serde(default)]` so older
    /// saved projects load at `0.0` (square corners).
    #[serde(default)]
    pub mask_corner_radius: f32,
    /// `true` if this block's frame is mirrored horizontally, per `request.md`'s Fase 4
    /// "Efeitos visuais" spec ("Espelhar (flip horizontal)"). Currently only shown as a badge
    /// on the timeline block (`ui`'s timeline panel); doesn't yet affect preview playback or
    /// export — the same kind of gap as [`ClipInstance::gain_db`]. `#[serde(default)]` so
    /// older saved projects load unflipped.
    #[serde(default)]
    pub flipped_h: bool,
    /// Color filter applied to this block ([`ColorFilter::None`] by default). Currently only
    /// shown as a tinted timeline-block fill (`ui`'s timeline panel); doesn't yet affect
    /// preview playback or export — the same kind of gap as [`ClipInstance::gain_db`].
    /// `#[serde(default)]` so older saved projects load unfiltered.
    #[serde(default)]
    pub color_filter: ColorFilter,
    /// Vignette strength for this block, `0.0..=1.0` (`0.0` is off) — per `request.md`'s Fase 4
    /// "Efeitos visuais" spec ("Vinheta"). Currently only shown as a darkened border stroke
    /// around the timeline block, scaled by intensity (`ui`'s timeline panel); doesn't yet
    /// affect preview playback or export — the same kind of gap as [`ClipInstance::gain_db`].
    /// `#[serde(default)]` so older saved projects load with no vignette.
    #[serde(default)]
    pub vignette_intensity: f32,
    /// Brightness adjustment for this block, `-1.0..=1.0` (`0.0` is unchanged) — per
    /// `request.md`'s Fase 4 "Efeitos visuais" spec ("Brilho, contraste e saturação").
    /// Currently has no visible effect anywhere (`ui`'s properties panel just exposes the
    /// slider); doesn't yet affect preview playback or export — the same kind of gap as
    /// [`ClipInstance::gain_db`]. `#[serde(default)]` so older saved projects load unchanged.
    #[serde(default)]
    pub brightness: f32,
    /// Contrast multiplier for this block, `0.0..=2.0` (`1.0` is unchanged) — same spec and gap
    /// as [`ClipInstance::brightness`]. `#[serde(default = ..)]` so older saved projects load
    /// unchanged.
    #[serde(default = "default_unity_multiplier")]
    pub contrast: f32,
    /// Saturation multiplier for this block, `0.0..=2.0` (`1.0` is unchanged, `0.0` is
    /// grayscale) — same spec and gap as [`ClipInstance::brightness`]. `#[serde(default = ..)]`
    /// so older saved projects load unchanged.
    #[serde(default = "default_unity_multiplier")]
    pub saturation: f32,
    /// Sharpen strength for this block, `0.0..=1.0` (`0.0` is off) — per `request.md`'s Fase 4
    /// "Efeitos visuais" spec ("Nitidez (sharpen)"). Currently has no visible effect anywhere
    /// (`ui`'s properties panel just exposes the slider); doesn't yet affect preview playback or
    /// export — the same kind of gap as [`ClipInstance::gain_db`]. `#[serde(default)]` so older
    /// saved projects load unsharpened.
    #[serde(default)]
    pub sharpen: f32,
    /// `true` if chroma key (green-screen removal) is enabled for this block, per
    /// `request.md`'s Fase 4 "Efeitos visuais" spec ("Chroma key"). Currently has no visible
    /// effect anywhere (`ui`'s properties panel just exposes the toggle/color/tolerance
    /// controls); doesn't yet affect preview playback or export — the same kind of gap as
    /// [`ClipInstance::gain_db`]. `#[serde(default)]` so older saved projects load disabled.
    #[serde(default)]
    pub chroma_key_enabled: bool,
    /// The key color to remove, as `[r, g, b]` (`0..=255` each) — meaningless while
    /// [`ClipInstance::chroma_key_enabled`] is `false`. `#[serde(default = ..)]` so older saved
    /// projects load at the conventional chroma-green `#00FF00`.
    #[serde(default = "default_chroma_key_color")]
    pub chroma_key_color: [u8; 3],
    /// How close a pixel's color must be to [`ClipInstance::chroma_key_color`] to be keyed out,
    /// `0.0..=1.0` (`0.0` is exact-match-only, `1.0` keys everything) — meaningless while
    /// `chroma_key_enabled` is `false`. `#[serde(default = ..)]` so older saved projects load at
    /// a reasonable default tolerance.
    #[serde(default = "default_chroma_key_tolerance")]
    pub chroma_key_tolerance: f32,
    /// Blur strength for this block, `0.0..=1.0` (`0.0` is off) — per `request.md`'s Fase 4
    /// "Efeitos visuais" spec ("Blur"). Currently has no visible effect anywhere (`ui`'s
    /// properties panel just exposes the slider); doesn't yet affect preview playback or export
    /// — the same kind of gap as [`ClipInstance::gain_db`]. `#[serde(default)]` so older saved
    /// projects load unblurred.
    #[serde(default)]
    pub blur_intensity: f32,
    /// Camera-shake strength for this block, `0.0..=1.0` (`0.0` is off) — per `request.md`'s
    /// Fase 4 "Efeitos visuais" spec ("Shake"), the deliberate counterpart of the eventual video
    /// stabilization feature. Same gap as [`ClipInstance::blur_intensity`]. `#[serde(default)]`
    /// so older saved projects load unshaken.
    #[serde(default)]
    pub shake_intensity: f32,
    /// Glitch strength for this block, `0.0..=1.0` (`0.0` is off) — per `request.md`'s Fase 4
    /// "Efeitos visuais" spec ("Glitch"). Same gap as [`ClipInstance::blur_intensity`].
    /// `#[serde(default)]` so older saved projects load unglitched.
    #[serde(default)]
    pub glitch_intensity: f32,
    /// Pixelize/mosaic-censor strength for this block, `0.0..=1.0` (`0.0` is off) — per
    /// `request.md`'s Fase 4 "Efeitos visuais" spec ("Pixelizar/censura (mosaico)"). Same gap as
    /// [`ClipInstance::blur_intensity`]. `#[serde(default)]` so older saved projects load
    /// unpixelized.
    #[serde(default)]
    pub pixelize_intensity: f32,
}

fn default_speed_factor() -> f32 {
    1.0
}

fn default_crop_extent() -> f32 {
    1.0
}

fn default_unity_multiplier() -> f32 {
    1.0
}

fn default_chroma_key_color() -> [u8; 3] {
    [0, 255, 0]
}

fn default_chroma_key_tolerance() -> f32 {
    0.4
}

impl ClipInstance {
    /// How long this instance plays for, i.e. its trimmed length — not the source asset's
    /// full duration.
    pub fn duration_secs(&self) -> f64 {
        self.source_out_secs - self.source_in_secs
    }

    /// Linear amplitude multiplier for [`ClipInstance::gain_db`] — e.g. `+6.0` dB roughly
    /// doubles amplitude, `-6.0` dB roughly halves it. `0.0` dB gives `1.0` (unity).
    pub fn gain_linear(&self) -> f32 {
        10f32.powf(self.gain_db / 20.0)
    }

    /// `true` if the crop rect isn't the full, uncropped frame.
    pub fn is_cropped(&self) -> bool {
        self.crop_x != 0.0 || self.crop_y != 0.0 || self.crop_w != 1.0 || self.crop_h != 1.0
    }

    /// `true` if a layer mask ([`ClipInstance::mask_shape`]) is applied.
    pub fn is_masked(&self) -> bool {
        self.mask_shape != MaskShape::None
    }

    /// `true` if a color filter ([`ClipInstance::color_filter`]) is applied.
    pub fn is_color_filtered(&self) -> bool {
        self.color_filter != ColorFilter::None
    }

    /// `true` if [`ClipInstance::vignette_intensity`] is above zero.
    pub fn has_vignette(&self) -> bool {
        self.vignette_intensity > 0.0
    }

    /// `true` if chroma key ([`ClipInstance::chroma_key_enabled`]) is on.
    pub fn is_chroma_keyed(&self) -> bool {
        self.chroma_key_enabled
    }

    /// True if `at_secs` (timeline-relative) falls strictly inside this clip's placed range.
    /// Boundary-exact positions return `false` — splitting exactly on an edge would just
    /// produce a zero-length half.
    pub fn contains(&self, at_secs: f64) -> bool {
        at_secs > self.start_secs && at_secs < self.start_secs + self.duration_secs()
    }

    /// Drags the clip's left edge to `new_start_secs`, keeping its end point
    /// (`source_out_secs`) fixed — `source_in_secs` shifts by the same delta as `start_secs`,
    /// since trimming the start plays a later point in the source. No-op (`false`) if that
    /// would put `new_start_secs` or the resulting `source_in_secs` below zero, or shrink the
    /// clip below `min_duration_secs`.
    pub fn trim_start(&mut self, new_start_secs: f64, min_duration_secs: f64) -> bool {
        let delta = new_start_secs - self.start_secs;
        let new_source_in_secs = self.source_in_secs + delta;
        let new_duration_secs = self.source_out_secs - new_source_in_secs;
        if new_start_secs < 0.0 || new_source_in_secs < 0.0 || new_duration_secs < min_duration_secs
        {
            return false;
        }
        self.start_secs = new_start_secs;
        self.source_in_secs = new_source_in_secs;
        true
    }

    /// Drags the clip's right edge to `new_end_secs` (timeline-relative), keeping `start_secs`
    /// and `source_in_secs` fixed. No-op (`false`) if that would shrink the clip below
    /// `min_duration_secs`, or (when `max_source_out_secs` is known — the source asset's own
    /// duration) push `source_out_secs` past the end of the actual source media.
    pub fn trim_end(
        &mut self,
        new_end_secs: f64,
        min_duration_secs: f64,
        max_source_out_secs: Option<f64>,
    ) -> bool {
        let new_duration_secs = new_end_secs - self.start_secs;
        let new_source_out_secs = self.source_in_secs + new_duration_secs;
        if new_duration_secs < min_duration_secs {
            return false;
        }
        if let Some(max) = max_source_out_secs {
            if new_source_out_secs > max {
                return false;
            }
        }
        self.source_out_secs = new_source_out_secs;
        true
    }
}

/// One row of the timeline (e.g. `V1`, `A1`, `A2` in the mockup), holding an ordered list of
/// clips. Tracks don't overlap-check their own clips — that's an editing-time concern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: u64,
    pub name: String,
    pub kind: TrackKind,
    pub clips: Vec<ClipInstance>,
}

impl Track {
    /// Splits the clip covering `at_secs` (timeline-relative) into two: the original keeps
    /// `id` and has its `source_out_secs` trimmed to the split point; a new clip starting at
    /// `at_secs`, with `new_clip_id` and the rest of the original's source range, is inserted
    /// right after it. No-op (`false`) if no clip on this track covers `at_secs`.
    pub fn split_clip_at(&mut self, at_secs: f64, new_clip_id: u64) -> bool {
        let Some(index) = self.clips.iter().position(|c| c.contains(at_secs)) else {
            return false;
        };

        let clip = &mut self.clips[index];
        let split_source_secs = clip.source_in_secs + (at_secs - clip.start_secs);
        let second_half = ClipInstance {
            id: new_clip_id,
            asset_id: clip.asset_id,
            start_secs: at_secs,
            source_in_secs: split_source_secs,
            source_out_secs: clip.source_out_secs,
            // Splitting a composite member must not silently ungroup it from the rest of the
            // block.
            composite_id: clip.composite_id,
            gain_db: clip.gain_db,
            frozen: clip.frozen,
            speed_factor: clip.speed_factor,
            crop_x: clip.crop_x,
            crop_y: clip.crop_y,
            crop_w: clip.crop_w,
            crop_h: clip.crop_h,
            mask_shape: clip.mask_shape,
            mask_corner_radius: clip.mask_corner_radius,
            flipped_h: clip.flipped_h,
            color_filter: clip.color_filter,
            vignette_intensity: clip.vignette_intensity,
            brightness: clip.brightness,
            contrast: clip.contrast,
            saturation: clip.saturation,
            sharpen: clip.sharpen,
            chroma_key_enabled: clip.chroma_key_enabled,
            chroma_key_color: clip.chroma_key_color,
            chroma_key_tolerance: clip.chroma_key_tolerance,
            blur_intensity: clip.blur_intensity,
            shake_intensity: clip.shake_intensity,
            glitch_intensity: clip.glitch_intensity,
            pixelize_intensity: clip.pixelize_intensity,
        };
        clip.source_out_secs = split_source_secs;

        self.clips.insert(index + 1, second_half);
        true
    }

    /// Mutable access to the clip with this id, if it's on this track.
    pub fn clip_mut(&mut self, clip_id: u64) -> Option<&mut ClipInstance> {
        self.clips.iter_mut().find(|c| c.id == clip_id)
    }

    /// Repositions the clip with `clip_id` to `new_start_secs` on this same track — what
    /// dragging a clip's body (not one of its edges) does. Doesn't check for overlap with
    /// neighboring clips (matches this struct's existing no-overlap-checking policy, see
    /// above) — dragging one clip onto another just lets them overlap for now. No-op
    /// (`false`) if the clip isn't on this track or `new_start_secs` is negative.
    pub fn move_clip(&mut self, clip_id: u64, new_start_secs: f64) -> bool {
        if new_start_secs < 0.0 {
            return false;
        }
        let Some(clip) = self.clip_mut(clip_id) else {
            return false;
        };
        clip.start_secs = new_start_secs;
        true
    }

    /// The position, in seconds, where this track's last clip ends. `0.0` for an empty track —
    /// the natural "append here" position for a clip added to this track.
    pub fn duration_secs(&self) -> f64 {
        self.clips
            .iter()
            .map(|c| c.start_secs + c.duration_secs())
            .fold(0.0, f64::max)
    }
}

/// A project's full set of tracks plus the current playhead position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Timeline {
    pub tracks: Vec<Track>,
    pub playhead_secs: f64,
}

impl Timeline {
    /// The position, in seconds, where the last clip on any track ends — i.e. how long the
    /// edited sequence runs for. `0.0` for an empty timeline.
    pub fn duration_secs(&self) -> f64 {
        self.tracks
            .iter()
            .map(Track::duration_secs)
            .fold(0.0, f64::max)
    }

    /// Moves the clip with `clip_id` onto `target_track_id` at `new_start_secs`, removing it
    /// from wherever it currently lives (which may itself be `target_track_id`, for a same-
    /// track reposition — [`Track::move_clip`] is the cheaper path for that specific case, but
    /// this stays correct for it too). No-op (`false`) if the clip or target track don't
    /// exist, `new_start_secs` is negative, or the target track's `kind` doesn't match the
    /// clip's current track's — a video clip can't land on an audio track and vice versa.
    pub fn move_clip_to_track(
        &mut self,
        clip_id: u64,
        target_track_id: u64,
        new_start_secs: f64,
    ) -> bool {
        if new_start_secs < 0.0 {
            return false;
        }
        let Some(source_index) = self
            .tracks
            .iter()
            .position(|t| t.clips.iter().any(|c| c.id == clip_id))
        else {
            return false;
        };
        let Some(target_index) = self.tracks.iter().position(|t| t.id == target_track_id) else {
            return false;
        };
        if self.tracks[source_index].kind != self.tracks[target_index].kind {
            return false;
        }

        let clip_index = self.tracks[source_index]
            .clips
            .iter()
            .position(|c| c.id == clip_id)
            .expect("source_index was found by locating a track containing this clip_id");
        let mut clip = self.tracks[source_index].clips.remove(clip_index);
        clip.start_secs = new_start_secs;
        self.tracks[target_index].clips.push(clip);
        true
    }
}
