// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! Minimal GStreamer-based preview pipeline — plays a media file via `playbin`, decoding
//! independent of the UI thread (GStreamer drives its own internal threads once the pipeline
//! is `Playing`/`Paused`, no subprocess). Video frames are pulled on demand as raw RGBA
//! ([`Self::current_frame`]) rather than pushed to a live display — [`ui`] is
//! responsible for polling and uploading them to an egui texture; this module doesn't know
//! egui exists.

use std::path::Path;

use gstreamer as gst;
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;

use gst::prelude::*;

use crate::timeline::{ClipInstance, ColorFilter, ShapeClip, TextClip};

#[derive(Debug)]
pub enum PreviewError {
    Init(gst::glib::Error),
    CreateElement(gst::glib::BoolError),
    UriConversion(gst::glib::Error),
    StateChange(gst::StateChangeError),
    Seek(gst::glib::BoolError),
    /// Couldn't probe `path` for its actual decoded resolution — needed to convert
    /// [`ClipInstance`]'s normalized crop rect into `videocrop`'s pixel properties.
    Probe(avbridge::ProbeError),
    /// Failed wiring the per-clip effects filter bin together (linking elements or adding
    /// ghost pads).
    FilterBin(gst::glib::BoolError),
    /// Failed wiring the multi-track compositor pipeline together (adding elements, linking a
    /// branch's chain, or linking a branch into `compositor`).
    Compositing(gst::glib::BoolError),
    /// `compositor` refused to hand out a new `sink_%u` request pad.
    RequestPad,
    /// Failed linking a branch's last element into its `compositor` request pad.
    PadLink(gst::PadLinkError),
    /// [`Preview::open_composited`]'s background input has no video stream to size the canvas
    /// from.
    NoBackgroundVideo,
    /// Failed pushing a rasterized text/shape overlay buffer into its `appsrc`, or signalling
    /// end-of-stream on it (see [`build_static_overlay_branch`]).
    PushBuffer(gst::FlowError),
}

impl std::fmt::Display for PreviewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreviewError::Init(e) => write!(f, "failed to initialize GStreamer: {e}"),
            PreviewError::CreateElement(e) => write!(f, "failed to create playbin element: {e}"),
            PreviewError::UriConversion(e) => {
                write!(f, "failed to convert path to a file URI: {e}")
            }
            PreviewError::StateChange(e) => write!(f, "failed to change pipeline state: {e}"),
            PreviewError::Seek(e) => write!(f, "failed to seek: {e}"),
            PreviewError::Probe(e) => write!(f, "failed to probe for the effects filter bin: {e}"),
            PreviewError::FilterBin(e) => write!(f, "failed to build the effects filter bin: {e}"),
            PreviewError::Compositing(e) => {
                write!(f, "failed to build the compositor pipeline: {e}")
            }
            PreviewError::RequestPad => {
                write!(f, "compositor refused to hand out a sink request pad")
            }
            PreviewError::PadLink(e) => write!(f, "failed to link into compositor: {e:?}"),
            PreviewError::NoBackgroundVideo => {
                write!(
                    f,
                    "background input has no video stream to size the canvas from"
                )
            }
            PreviewError::PushBuffer(e) => {
                write!(f, "failed pushing a static overlay buffer: {e:?}")
            }
        }
    }
}

impl std::error::Error for PreviewError {}

/// One decoded video frame, pixels in tightly-packed row-major RGBA (no stride padding —
/// [`Self::rgba`] is exactly `width * height * 4` bytes, ready to hand to an egui
/// `ColorImage`/texture without extra copying logic).
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Builds a `gst::Bin` chaining the subset of `clip`'s effects GStreamer can apply live,
/// suitable for `playbin`'s `video-filter` property — `None` if every covered effect is
/// neutral (leaves `video-filter` unset). Stage order matches
/// [`crate::timeline::ClipInstance::video_filter_chain`]'s, plus scale keyframes
/// (`ClipInstance::scale_keyframes`) applied first, ahead of everything else — mirroring
/// `ClipInstance::keyframe_video_filter_chain`, which applies it at the canvas level before the
/// per-clip filter chain for export — for consistency with what export applies, even though the
/// element set differs (GStreamer elements here, avfilter there) and the covered subset is
/// narrower (no vignette — no matching element in this GStreamer install; no chroma key — only
/// meaningful once layering exists; no gain — preview has no audio route at all yet; no
/// glitch/transitions — animated per-frame in export via avfilter's `n` frame-count expressions
/// with no static element equivalent GStreamer-side; position keyframes aren't wired to preview
/// — they need a compositing (`overlay`) stage, which this single-clip pipeline doesn't have,
/// the same reason position has no visible effect on a single/background track in export either
/// — see `keyframe` module docs; scale/rotation/opacity each get a pad probe re-evaluating
/// `evaluate_keyframes` per buffer (`rotate`'s `angle` property and `alpha`'s `alpha` property
/// are both settable per-buffer the same way `videocrop`'s edges are), keyed off the buffer's
/// own PTS rather than a frame count, since preview has no fixed canvas fps to convert a frame
/// count against the way export's `N` does).
///
/// `resolution`, if known (`None` for an audio-only source, which shouldn't reach here but is
/// handled by just skipping the zoom/crop/pixelize/shake stages), is the *actual* decoded frame
/// size — needed since `videocrop`'s properties and the zoom/pixelize/shake downscale/upscale
/// target sizes are plain pixel counts, and `path` may be a lower-resolution editing proxy
/// rather than the original asset.
// TODO: transitions (ClipInstance::transition_in / ClipSegment::transition_in — a *different*
// concept from the scale_keyframes Ken-Burns-style animation above: an entry animation over the
// clip's first transition_duration_secs) are not yet covered by preview — the fade/slide/zoom
// avfilter expressions are built in bridge.c's avbridge_encode_timeline_export and only affect
// the exported file. Adding them here would require either a GStreamer element equivalent
// (e.g. `frei0r-filter-cairoimagegraphics` for drawbox, or a custom element) or a manual
// frame-count-driven property update — the same pad-probe technique shake/scale use below would
// work for the crop/scale-based Zoom transition variant, but Fade (alpha ramp) and Slide
// (drawbox wipe) still need their own element equivalents.
//
// TODO: position keyframes aren't wired to preview (see crate::keyframe module docs) — unlike
// rotation/opacity below, position needs a compositing (`overlay`) stage that this single-clip
// pipeline doesn't have at all; adding it would mean building out multi-track preview
// compositing first, not just picking a GStreamer element.
/// `include_opacity` is `false` for an overlay branch in [`Preview::open_composited`]'s
/// pipeline — there, opacity keyframes drive `compositor`'s own per-pad `alpha` property
/// instead (real alpha blending against whatever's under it), so baking a second, redundant
/// uniform-alpha stage in here via the `alpha` element would just double-apply the same ramp.
/// Every other caller (the single-clip [`Preview::open`] path, and a composited pipeline's
/// background branch, which has nothing under it to blend against) keeps the old behavior.
fn build_video_filter_bin(
    clip: &ClipInstance,
    resolution: Option<(u32, u32)>,
    include_opacity: bool,
) -> Result<Option<gst::Element>, PreviewError> {
    let mut elements: Vec<gst::Element> = Vec::new();

    if let Some((width, height)) = resolution {
        if clip.has_scale_keyframes() {
            // Scale keyframes sit before video_filter_chain's own stages in export (see
            // ClipInstance::keyframe_video_filter_chain, applied at the canvas level ahead of
            // the per-clip filter chain) — mirrored here by building it first, ahead of the
            // user-crop stage below. Keyed off the buffer's own PTS (seconds within the *source
            // file*, since ui's preview seeks to clip.source_in_secs + offset rather than 0)
            // instead of a frame counter — preview has no fixed canvas fps to convert a frame
            // count against, unlike export. evaluate_keyframes is the same piecewise-linear
            // interpolation export's scale_filter_expr compiles into an avfilter expression —
            // this reuses it directly instead of duplicating the math, and naturally supports
            // any number of keyframes (not just the old zoom_start/zoom_end two-point case).
            let scale_keyframes = clip.scale_keyframes.clone();
            let source_in_secs = clip.source_in_secs;
            let clip_duration_secs = (clip.source_out_secs - clip.source_in_secs).max(1e-6);

            let zoom_crop = gst::ElementFactory::make("videocrop")
                .build()
                .map_err(PreviewError::CreateElement)?;
            let zoom_crop_for_probe = zoom_crop.clone();
            let sink_pad = zoom_crop
                .static_pad("sink")
                .expect("videocrop always has a sink pad");
            sink_pad.add_probe(gst::PadProbeType::BUFFER, move |_pad, info| {
                let secs = info
                    .buffer()
                    .and_then(|b| b.pts())
                    .map(|t| t.seconds_f64())
                    .unwrap_or(source_in_secs);
                let frac = ((secs - source_in_secs) / clip_duration_secs).clamp(0.0, 1.0) as f32;
                let zoom = crate::keyframe::evaluate_keyframes(&scale_keyframes, frac, 1.0)
                    .clamp(0.1, 20.0) as f64;
                let crop_w = (width as f64 / zoom).round().max(2.0);
                let crop_h = (height as f64 / zoom).round().max(2.0);
                let side_w = ((width as f64 - crop_w) / 2.0).round().max(0.0) as i32;
                let side_h = ((height as f64 - crop_h) / 2.0).round().max(0.0) as i32;
                zoom_crop_for_probe.set_property("left", side_w);
                zoom_crop_for_probe.set_property("right", side_w);
                zoom_crop_for_probe.set_property("top", side_h);
                zoom_crop_for_probe.set_property("bottom", side_h);
                gst::PadProbeReturn::Ok
            });

            let upscale = gst::ElementFactory::make("videoscale")
                .build()
                .map_err(PreviewError::CreateElement)?;
            let full_caps = gst::ElementFactory::make("capsfilter")
                .property(
                    "caps",
                    gst::Caps::builder("video/x-raw")
                        .field("width", width as i32)
                        .field("height", height as i32)
                        .build(),
                )
                .build()
                .map_err(PreviewError::CreateElement)?;
            elements.push(zoom_crop);
            elements.push(upscale);
            elements.push(full_caps);
        }
    }

    if clip.has_rotation_keyframes() {
        // Mirrors export's rotation_filter_angle_expr: keyed off elapsed seconds within the
        // source file (clip.source_in_secs + offset), same PTS-based fraction the scale probe
        // above uses, since preview has no fixed canvas fps to convert a frame count against.
        let rotation_keyframes = clip.rotation_keyframes.clone();
        let source_in_secs = clip.source_in_secs;
        let clip_duration_secs = (clip.source_out_secs - clip.source_in_secs).max(1e-6);

        let rotate = gst::ElementFactory::make("rotate")
            .build()
            .map_err(PreviewError::CreateElement)?;
        let rotate_for_probe = rotate.clone();
        let sink_pad = rotate
            .static_pad("sink")
            .expect("rotate always has a sink pad");
        sink_pad.add_probe(gst::PadProbeType::BUFFER, move |_pad, info| {
            let secs = info
                .buffer()
                .and_then(|b| b.pts())
                .map(|t| t.seconds_f64())
                .unwrap_or(source_in_secs);
            let frac = ((secs - source_in_secs) / clip_duration_secs).clamp(0.0, 1.0) as f32;
            let angle_deg = crate::keyframe::evaluate_keyframes(&rotation_keyframes, frac, 0.0);
            rotate_for_probe.set_property("angle", angle_deg.to_radians() as f64);
            gst::PadProbeReturn::Ok
        });
        elements.push(rotate);
    }

    if include_opacity && clip.has_opacity_keyframes() {
        // "alpha" (gst-plugins-good) sets a uniform per-buffer alpha on its output — the RGBA
        // path is already forced by the AppSink's fixed caps in Preview::open, and egui draws
        // ColorImage::from_rgba_unmultiplied with alpha blending, so a reduced alpha here is
        // actually visible in the preview (unlike position, which would need a compositing
        // stage this single-clip pipeline doesn't have).
        let opacity_keyframes = clip.opacity_keyframes.clone();
        let source_in_secs = clip.source_in_secs;
        let clip_duration_secs = (clip.source_out_secs - clip.source_in_secs).max(1e-6);

        let alpha = gst::ElementFactory::make("alpha")
            .property_from_str("method", "set")
            .build()
            .map_err(PreviewError::CreateElement)?;
        let alpha_for_probe = alpha.clone();
        let sink_pad = alpha
            .static_pad("sink")
            .expect("alpha always has a sink pad");
        sink_pad.add_probe(gst::PadProbeType::BUFFER, move |_pad, info| {
            let secs = info
                .buffer()
                .and_then(|b| b.pts())
                .map(|t| t.seconds_f64())
                .unwrap_or(source_in_secs);
            let frac = ((secs - source_in_secs) / clip_duration_secs).clamp(0.0, 1.0) as f32;
            let a =
                crate::keyframe::evaluate_keyframes(&opacity_keyframes, frac, 1.0).clamp(0.0, 1.0);
            alpha_for_probe.set_property("alpha", a as f64);
            gst::PadProbeReturn::Ok
        });
        elements.push(alpha);
    }

    if let Some((width, height)) = resolution {
        if clip.is_cropped() {
            let (width, height) = (width as f32, height as f32);
            let left = (clip.crop_x * width).round().max(0.0) as i32;
            let top = (clip.crop_y * height).round().max(0.0) as i32;
            let right = ((1.0 - clip.crop_x - clip.crop_w) * width).round().max(0.0) as i32;
            let bottom = ((1.0 - clip.crop_y - clip.crop_h) * height)
                .round()
                .max(0.0) as i32;
            let crop = gst::ElementFactory::make("videocrop")
                .property("left", left)
                .property("top", top)
                .property("right", right)
                .property("bottom", bottom)
                .build()
                .map_err(PreviewError::CreateElement)?;
            elements.push(crop);
        }
    }

    let effective_saturation = if clip.color_filter == ColorFilter::BlackAndWhite {
        0.0
    } else {
        clip.saturation as f64
    };
    if clip.brightness != 0.0 || clip.contrast != 1.0 || effective_saturation != 1.0 {
        let balance = gst::ElementFactory::make("videobalance")
            .property("brightness", clip.brightness as f64)
            .property("contrast", clip.contrast as f64)
            .property("saturation", effective_saturation)
            .build()
            .map_err(PreviewError::CreateElement)?;
        elements.push(balance);
    }

    if clip.color_filter == ColorFilter::Sepia {
        let sepia = gst::ElementFactory::make("coloreffects")
            .property_from_str("preset", "sepia")
            .build()
            .map_err(PreviewError::CreateElement)?;
        elements.push(sepia);
    }

    // gaussianblur's single signed `sigma` covers both blur_intensity (positive) and sharpen
    // (negative) — the *4.0 scale is a judgment call, same style as video_filter_chain's own
    // per-effect scale factors, not a value derived from anything.
    let net_sigma = clip.blur_intensity as f64 * 4.0 - clip.sharpen as f64 * 4.0;
    if net_sigma != 0.0 {
        let blur = gst::ElementFactory::make("gaussianblur")
            .property("sigma", net_sigma)
            .build()
            .map_err(PreviewError::CreateElement)?;
        elements.push(blur);
    }

    if let Some((width, height)) = resolution {
        if clip.pixelize_intensity > 0.0 {
            // Same block-size formula as video_filter_chain's: 2px (subtle) to 50px (heavy
            // censorship). Two videoscale elements (nearest-neighbour, for the hard mosaic
            // look — bilinear would just blur) with a capsfilter pinning each stage's output
            // size stand in for avfilter's single scale-down/scale-up expression pair.
            let block = (2.0 + clip.pixelize_intensity * 48.0).round().max(1.0) as u32;
            let small_w = (width / block).max(1) as i32;
            let small_h = (height / block).max(1) as i32;

            let downscale = gst::ElementFactory::make("videoscale")
                .property_from_str("method", "nearest-neighbour")
                .build()
                .map_err(PreviewError::CreateElement)?;
            let small_caps = gst::ElementFactory::make("capsfilter")
                .property(
                    "caps",
                    gst::Caps::builder("video/x-raw")
                        .field("width", small_w)
                        .field("height", small_h)
                        .build(),
                )
                .build()
                .map_err(PreviewError::CreateElement)?;
            let upscale = gst::ElementFactory::make("videoscale")
                .property_from_str("method", "nearest-neighbour")
                .build()
                .map_err(PreviewError::CreateElement)?;
            let full_caps = gst::ElementFactory::make("capsfilter")
                .property(
                    "caps",
                    gst::Caps::builder("video/x-raw")
                        .field("width", width as i32)
                        .field("height", height as i32)
                        .build(),
                )
                .build()
                .map_err(PreviewError::CreateElement)?;
            elements.push(downscale);
            elements.push(small_caps);
            elements.push(upscale);
            elements.push(full_caps);
        }
    }

    if let Some((width, height)) = resolution {
        if clip.shake_intensity > 0.0 {
            // avfilter's crop=...:iw*margin*(1+sin(n*0.31)):ih*margin*(1+cos(n*0.23)) slides a
            // fixed-size crop *window* around within a margin, keyed off n (frame count).
            // videocrop has no expression support, so a buffer probe on its sink pad recomputes
            // left/top/right/bottom from a running frame counter before every frame — same
            // sinusoids, translated from ffmpeg's "window position" framing to videocrop's
            // "pixels trimmed per edge" one: left + right (and top + bottom) are kept summing
            // to a constant total_trim (only the split between them oscillates), so videocrop's
            // *output* size never changes frame to frame and the fixed-size upscale after it
            // never needs to renegotiate caps mid-stream.
            let margin = clip.shake_intensity * 0.08_f32;
            let total_trim_w =
                ((width as f32 * 2.0 * margin).round() as i32).clamp(0, (width as i32 - 2).max(0));
            let total_trim_h = ((height as f32 * 2.0 * margin).round() as i32)
                .clamp(0, (height as i32 - 2).max(0));

            let crop = gst::ElementFactory::make("videocrop")
                .build()
                .map_err(PreviewError::CreateElement)?;
            let crop_for_probe = crop.clone();
            let frame_counter = std::sync::atomic::AtomicU64::new(0);
            let sink_pad = crop
                .static_pad("sink")
                .expect("videocrop always has a sink pad");
            sink_pad.add_probe(gst::PadProbeType::BUFFER, move |_pad, _info| {
                let n = frame_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) as f64;
                let frac_x = (1.0 + (n * 0.31).sin()) / 2.0;
                let frac_y = (1.0 + (n * 0.23).cos()) / 2.0;
                let left = (total_trim_w as f64 * frac_x).round() as i32;
                let top = (total_trim_h as f64 * frac_y).round() as i32;
                crop_for_probe.set_property("left", left);
                crop_for_probe.set_property("right", total_trim_w - left);
                crop_for_probe.set_property("top", top);
                crop_for_probe.set_property("bottom", total_trim_h - top);
                gst::PadProbeReturn::Ok
            });

            let upscale = gst::ElementFactory::make("videoscale")
                .build()
                .map_err(PreviewError::CreateElement)?;
            let full_caps = gst::ElementFactory::make("capsfilter")
                .property(
                    "caps",
                    gst::Caps::builder("video/x-raw")
                        .field("width", width as i32)
                        .field("height", height as i32)
                        .build(),
                )
                .build()
                .map_err(PreviewError::CreateElement)?;
            elements.push(crop);
            elements.push(upscale);
            elements.push(full_caps);
        }
    }

    if clip.flipped_h {
        let flip = gst::ElementFactory::make("videoflip")
            .property_from_str("method", "horizontal-flip")
            .build()
            .map_err(PreviewError::CreateElement)?;
        elements.push(flip);
    }

    if elements.is_empty() {
        return Ok(None);
    }

    let bin = gst::Bin::new();
    bin.add_many(&elements).map_err(PreviewError::FilterBin)?;
    gst::Element::link_many(&elements).map_err(PreviewError::FilterBin)?;

    let sink_pad = elements
        .first()
        .and_then(|e| e.static_pad("sink"))
        .expect("every filter element above has a static sink pad");
    let src_pad = elements
        .last()
        .and_then(|e| e.static_pad("src"))
        .expect("every filter element above has a static src pad");
    let ghost_sink = gst::GhostPad::with_target(&sink_pad).map_err(PreviewError::FilterBin)?;
    let ghost_src = gst::GhostPad::with_target(&src_pad).map_err(PreviewError::FilterBin)?;
    bin.add_pad(&ghost_sink).map_err(PreviewError::FilterBin)?;
    bin.add_pad(&ghost_src).map_err(PreviewError::FilterBin)?;

    Ok(Some(bin.upcast::<gst::Element>()))
}

/// Builds the `alpha` element's chroma-key configuration for `clip` — `method=custom` against
/// its own `chroma_key_color` rather than the fixed `green`/`blue` presets, so an arbitrary key
/// color (not just a standard green/blue screen) works the same as export's avfilter `colorkey`
/// stage does. `chroma_key_tolerance` (`0.0..=1.0`) maps onto `alpha`'s `black-sensitivity`/
/// `white-sensitivity` (`0..=128`, default `100`) linearly — a judgment call, same as this
/// module's other intensity-to-property scale factors, not a value derived from anything.
fn build_chroma_key_element(clip: &ClipInstance) -> Result<gst::Element, PreviewError> {
    let [r, g, b] = clip.chroma_key_color;
    let sensitivity = (clip.chroma_key_tolerance.clamp(0.0, 1.0) * 128.0).round() as u32;
    gst::ElementFactory::make("alpha")
        .property_from_str("method", "custom")
        .property("target-r", r as u32)
        .property("target-g", g as u32)
        .property("target-b", b as u32)
        .property("black-sensitivity", sensitivity)
        .property("white-sensitivity", sensitivity)
        .build()
        .map_err(PreviewError::CreateElement)
}

/// Links `decodebin`'s first video output pad to `target_sink` once it appears — `decodebin`/
/// `uridecodebin` expose pads dynamically (`pad-added`, possibly more than one: video, audio,
/// subtitle), so this can't be a static link at bin-build time like every other element pair in
/// [`build_video_filter_bin`]. Ignores non-video pads (an audio pad with nothing downstream just
/// sits unused — GStreamer doesn't require every source pad to be linked) and a second video pad
/// if one somehow appears (multi-video-stream files aren't a case this preview handles).
fn connect_decodebin_video_pad(decodebin: &gst::Element, target_sink: gst::Pad) {
    decodebin.connect_pad_added(move |_dbin, src_pad| {
        if target_sink.is_linked() {
            return;
        }
        let is_video = src_pad
            .current_caps()
            .or_else(|| Some(src_pad.query_caps(None)))
            .and_then(|caps| caps.structure(0).map(|s| s.name().starts_with("video/")))
            .unwrap_or(false);
        if !is_video {
            return;
        }
        if let Err(e) = src_pad.link(&target_sink) {
            tracing::warn!(error = ?e, "failed to link decodebin video pad into its branch chain");
        }
    });
}

/// One input (background or overlay) feeding [`Preview::open_composited`]'s `compositor`.
struct CompositeBranch<'a> {
    path: &'a Path,
    clip: Option<&'a ClipInstance>,
    resolution: Option<(u32, u32)>,
    /// `false` for the background (track 0) branch — position/opacity keyframes only drive
    /// `compositor`'s pad properties on an overlay branch, matching export's own
    /// position/opacity-is-overlay-only convention (see `crate::keyframe` module docs).
    is_overlay: bool,
    zorder: u32,
}

/// Builds one branch of [`Preview::open_composited`]'s pipeline — `uridecodebin` (dynamically
/// linked once its video pad appears) through this branch's own effects chain (layer-scale
/// resize, [`build_video_filter_bin`]'s subset, chroma key, background-removal matte) into a
/// freshly requested `compositor` sink pad — and returns that branch's `uridecodebin` element
/// (what [`Preview::seek_composited`] seeks independently, since each branch's source file has
/// its own, unrelated time base — a single pipeline-wide seek would send every branch to the
/// same absolute source time, which is only ever correct by coincidence when clips have
/// different trim points), plus a second `uridecodebin` for the matte file if
/// `clip.background_removal_enabled` and a matte path is set — mirroring export's own
/// `ClipSegment::mask_video_path` gate (see that field's doc comment in `bridge.h`): only
/// consulted on an overlay branch, same as chroma key.
fn build_composite_branch(
    pipeline: &gst::Pipeline,
    compositor: &gst::Element,
    canvas: (u32, u32),
    branch: CompositeBranch,
) -> Result<(gst::Element, Option<gst::Element>), PreviewError> {
    let uri = gst::glib::filename_to_uri(branch.path, None).map_err(PreviewError::UriConversion)?;
    let decodebin = gst::ElementFactory::make("uridecodebin")
        .property("uri", uri.as_str())
        .build()
        .map_err(PreviewError::CreateElement)?;

    let mut chain: Vec<gst::Element> = vec![gst::ElementFactory::make("videoconvert")
        .build()
        .map_err(PreviewError::CreateElement)?];

    if branch.is_overlay {
        if let (Some(clip), Some((width, height))) = (branch.clip, branch.resolution) {
            if clip.has_layer_scale() {
                // Mirrors export's own layer_scale handling (resolve_clip_filters): the
                // overlay's own decoded frame is resized before compositing, not the
                // compositor's placement — `compositor` pad width/height are left at their
                // default (-1, meaning "use the input's own negotiated size").
                let target_w = ((width as f32 * clip.layer_scale_x).round().max(2.0)) as i32;
                let target_h = ((height as f32 * clip.layer_scale_y).round().max(2.0)) as i32;
                chain.push(
                    gst::ElementFactory::make("videoscale")
                        .build()
                        .map_err(PreviewError::CreateElement)?,
                );
                chain.push(
                    gst::ElementFactory::make("capsfilter")
                        .property(
                            "caps",
                            gst::Caps::builder("video/x-raw")
                                .field("width", target_w)
                                .field("height", target_h)
                                .build(),
                        )
                        .build()
                        .map_err(PreviewError::CreateElement)?,
                );
            }
        }
    }

    if let Some(clip) = branch.clip {
        if let Some(filter_bin) =
            build_video_filter_bin(clip, branch.resolution, !branch.is_overlay)?
        {
            chain.push(filter_bin);
        }
        if branch.is_overlay && clip.chroma_key_enabled {
            chain.push(build_chroma_key_element(clip)?);
        }
    }

    chain.push(
        gst::ElementFactory::make("videoconvert")
            .build()
            .map_err(PreviewError::CreateElement)?,
    );

    // Background-removal matte — mirrors export's `ClipSegment::mask_video_path`/`alphamerge`
    // stage: only meaningful on an overlay branch, only when a matte was actually generated for
    // this clip. `alphacombine` (gst-plugins-bad's `codecalpha` plugin, confirmed present via
    // `gst-inspect-1.0` on this dev machine) takes the `sink` pad's own video and the `alpha`
    // pad's luma plane, producing an alpha-capable output (`A420`/etc.) — the GStreamer
    // counterpart to avfilter's `alphamerge`. Both inputs are forced to the branch's own
    // resolution so their planes line up regardless of the matte's own encoded size (see
    // `crate::background_removal`'s doc comment on how it's generated).
    // At this point `chain` already holds this branch's complete pre-matte effects sequence
    // (layer scale, filter bin, chroma key, trailing videoconvert) — every element in it is
    // added/linked together, exactly once, by the single `add_many`/`link_many` call right
    // below. The matte apparatus below is deliberately kept out of `chain` and added/linked
    // separately instead, since it needs its own second input (the matte decode branch) that
    // a plain linear chain can't express — pushing `alphacombine` into `chain` too would give
    // the pipeline two elements sharing the same auto-generated name and fail to add.
    pipeline
        .add(&decodebin)
        .map_err(PreviewError::Compositing)?;
    pipeline
        .add_many(&chain)
        .map_err(PreviewError::Compositing)?;
    gst::Element::link_many(&chain).map_err(PreviewError::Compositing)?;

    let chain_sink = chain
        .first()
        .and_then(|e| e.static_pad("sink"))
        .expect("chain always starts with a videoconvert, which always has a sink pad");
    connect_decodebin_video_pad(&decodebin, chain_sink);

    let chain_tail = chain
        .last()
        .expect("chain always has at least the trailing videoconvert")
        .clone();

    // Background-removal matte — mirrors export's `ClipSegment::mask_video_path`/`alphamerge`
    // stage: only meaningful on an overlay branch, only when a matte was actually generated for
    // this clip. `alphacombine` (gst-plugins-bad's `codecalpha` plugin, confirmed present via
    // `gst-inspect-1.0` on this dev machine) takes the `sink` pad's own video and the `alpha`
    // pad's luma plane, producing an alpha-capable output (`A420`/etc.) — the GStreamer
    // counterpart to avfilter's `alphamerge`. Both inputs are forced to the branch's own
    // resolution so their planes line up regardless of the matte's own encoded size (see
    // `crate::background_removal`'s doc comment on how it's generated). Returns the matte's own
    // `uridecodebin` (for independent seeking, see [`Preview::matte_branches`]) and this
    // branch's true final video element — `chain_tail` itself when there's no matte.
    let (matte_decodebin, branch_output) = if branch.is_overlay
        && branch.clip.is_some_and(|c| {
            c.background_removal_enabled && !c.background_removal_mask_path.is_empty()
        }) {
        let clip = branch.clip.expect("checked by is_some_and above");
        let (width, height) = branch.resolution.unwrap_or(canvas);
        let mask_path = Path::new(&clip.background_removal_mask_path);
        match gst::glib::filename_to_uri(mask_path, None) {
            Ok(uri) => {
                let matte_decodebin = gst::ElementFactory::make("uridecodebin")
                    .property("uri", uri.as_str())
                    .build()
                    .map_err(PreviewError::CreateElement)?;
                let matte_convert = gst::ElementFactory::make("videoconvert")
                    .build()
                    .map_err(PreviewError::CreateElement)?;
                let matte_scale = gst::ElementFactory::make("videoscale")
                    .build()
                    .map_err(PreviewError::CreateElement)?;
                let matte_caps = gst::ElementFactory::make("capsfilter")
                    .property(
                        "caps",
                        gst::Caps::builder("video/x-raw")
                            .field("format", "GRAY8")
                            .field("width", width as i32)
                            .field("height", height as i32)
                            // Explicit, identical colorimetry on both this and `sink_caps`
                            // below — `alphacombine` refuses to combine two inputs with
                            // mismatched color range ("Color range mismatch"), which the
                            // matte's own encode and the main chain's own negotiated caps
                            // otherwise don't guarantee agree on, confirmed empirically
                            // (`gst_alpha_combine_negotiate`'s error message, not guessed).
                            .field("colorimetry", "bt601")
                            .build(),
                    )
                    .build()
                    .map_err(PreviewError::CreateElement)?;
                // Forces `chain_tail`'s output into I420 before it reaches `alphacombine`'s
                // `sink` pad — its pad template doesn't accept the unconstrained/RGBA-negotiated
                // caps `chain_tail` otherwise produces.
                let sink_caps = gst::ElementFactory::make("capsfilter")
                    .property(
                        "caps",
                        gst::Caps::builder("video/x-raw")
                            .field("format", "I420")
                            .field("width", width as i32)
                            .field("height", height as i32)
                            .field("colorimetry", "bt601")
                            .build(),
                    )
                    .build()
                    .map_err(PreviewError::CreateElement)?;
                let alphacombine = gst::ElementFactory::make("alphacombine")
                    .build()
                    .map_err(PreviewError::CreateElement)?;
                let post_convert = gst::ElementFactory::make("videoconvert")
                    .build()
                    .map_err(PreviewError::CreateElement)?;

                pipeline
                    .add_many([
                        &matte_decodebin,
                        &matte_convert,
                        &matte_scale,
                        &matte_caps,
                        &sink_caps,
                        &alphacombine,
                        &post_convert,
                    ])
                    .map_err(PreviewError::Compositing)?;
                gst::Element::link_many([&matte_convert, &matte_scale, &matte_caps])
                    .map_err(PreviewError::Compositing)?;
                let matte_sink = matte_convert
                    .static_pad("sink")
                    .expect("videoconvert always has a sink pad");
                connect_decodebin_video_pad(&matte_decodebin, matte_sink);

                chain_tail
                    .link(&sink_caps)
                    .map_err(PreviewError::Compositing)?;

                let color_src = sink_caps
                    .static_pad("src")
                    .expect("capsfilter always has a src pad");
                let color_sink = alphacombine
                    .static_pad("sink")
                    .expect("alphacombine always has a sink pad");
                color_src.link(&color_sink).map_err(PreviewError::PadLink)?;

                let alpha_src = matte_caps
                    .static_pad("src")
                    .expect("capsfilter always has a src pad");
                let alpha_sink = alphacombine
                    .static_pad("alpha")
                    .expect("alphacombine always has an alpha sink pad");
                alpha_src.link(&alpha_sink).map_err(PreviewError::PadLink)?;

                alphacombine
                    .link(&post_convert)
                    .map_err(PreviewError::Compositing)?;

                (Some(matte_decodebin), post_convert)
            }
            Err(e) => {
                // Same "degrade rather than abort" posture export's matte handling has for a
                // stale/deleted cache file — no matte compositing this branch, not a failed
                // preview pipeline.
                tracing::warn!(error = ?e, path = %mask_path.display(), "failed to resolve matte path for preview, compositing without it");
                (None, chain_tail.clone())
            }
        }
    } else {
        (None, chain_tail.clone())
    };

    let sink_pad = compositor
        .request_pad_simple("sink_%u")
        .ok_or(PreviewError::RequestPad)?;
    sink_pad.set_property("zorder", branch.zorder);
    sink_pad.set_property("xpos", 0i32);
    sink_pad.set_property("ypos", 0i32);
    sink_pad.set_property("alpha", 1.0f64);

    if branch.is_overlay {
        if let Some(clip) = branch.clip {
            let position_keyframes = clip.position_keyframes.clone();
            let opacity_keyframes = clip.opacity_keyframes.clone();
            let source_in_secs = clip.source_in_secs;
            let clip_duration_secs = (clip.source_out_secs - clip.source_in_secs).max(1e-6);
            let (canvas_w, canvas_h) = canvas;
            let pad_for_probe = sink_pad.clone();

            let chain_src = branch_output
                .static_pad("src")
                .expect("branch_output is always a videoconvert, which always has a src pad");
            chain_src.add_probe(gst::PadProbeType::BUFFER, move |_pad, info| {
                let secs = info
                    .buffer()
                    .and_then(|b| b.pts())
                    .map(|t| t.seconds_f64())
                    .unwrap_or(source_in_secs);
                let frac = ((secs - source_in_secs) / clip_duration_secs).clamp(0.0, 1.0) as f32;
                let pos = crate::keyframe::evaluate_keyframes(
                    &position_keyframes,
                    frac,
                    crate::keyframe::Position { x: 0.0, y: 0.0 },
                );
                let alpha = crate::keyframe::evaluate_keyframes(&opacity_keyframes, frac, 1.0)
                    .clamp(0.0, 1.0);
                pad_for_probe.set_property("xpos", (pos.x * canvas_w as f32).round() as i32);
                pad_for_probe.set_property("ypos", (pos.y * canvas_h as f32).round() as i32);
                pad_for_probe.set_property("alpha", alpha as f64);
                gst::PadProbeReturn::Ok
            });
        }
    }

    let chain_out = branch_output
        .static_pad("src")
        .expect("branch_output is always a videoconvert, which always has a src pad");
    chain_out.link(&sink_pad).map_err(PreviewError::PadLink)?;

    Ok((decodebin, matte_decodebin))
}

/// Builds one static text/shape overlay branch of [`Preview::open_composited`]'s pipeline:
/// `appsrc` (pushed exactly one already-rasterized full-canvas RGBA buffer, see
/// [`crate::overlay_render`]) through `imagefreeze` (repeats that single buffer indefinitely,
/// deriving fresh timestamps off the pipeline's own clock) into a freshly requested `compositor`
/// sink pad. Unlike [`build_composite_branch`], there's no keyframe animation and no seeking —
/// [`TextClip`]/[`ShapeClip`] are both static for their whole visible span and have no source
/// file of their own — so this needs neither a pad probe nor an entry in [`Preview::branches`].
/// `rgba` already covers the whole canvas with the text/shape positioned within it (matching
/// export's own absolute-pixel-position convention), so the compositor pad is placed at
/// `(0, 0)` at the canvas's own size rather than needing per-pad position math.
fn build_static_overlay_branch(
    pipeline: &gst::Pipeline,
    compositor: &gst::Element,
    canvas: (u32, u32),
    rgba: Vec<u8>,
    zorder: u32,
) -> Result<(), PreviewError> {
    let (width, height) = canvas;
    let caps = gst::Caps::builder("video/x-raw")
        .field("format", "RGBA")
        .field("width", width as i32)
        .field("height", height as i32)
        // `0/1` is the conventional "still image, not a real framerate" sentinel decoders like
        // `jpegdec`/`pngdec` use ahead of `imagefreeze` — its sink pad's caps template requires
        // a `framerate` field to negotiate at all (a fixed caps with none omitted entirely was
        // rejected outright, confirmed empirically: `imagefreeze0:sink> caps ... not accepted`).
        .field("framerate", gst::Fraction::new(0, 1))
        .build();

    let appsrc = gst_app::AppSrc::builder()
        .caps(&caps)
        .format(gst::Format::Time)
        .build();
    let mut buffer = gst::Buffer::with_size(rgba.len()).map_err(PreviewError::Compositing)?;
    {
        let buffer_mut = buffer.get_mut().expect("freshly created, uniquely owned");
        buffer_mut.set_pts(gst::ClockTime::ZERO);
        let mut map = buffer_mut
            .map_writable()
            .map_err(|_| PreviewError::PushBuffer(gst::FlowError::Error))?;
        map.copy_from_slice(&rgba);
    }
    appsrc
        .push_buffer(buffer)
        .map_err(PreviewError::PushBuffer)?;
    appsrc.end_of_stream().map_err(PreviewError::PushBuffer)?;

    let imagefreeze = gst::ElementFactory::make("imagefreeze")
        .build()
        .map_err(PreviewError::CreateElement)?;
    let appsrc = appsrc.upcast::<gst::Element>();

    pipeline
        .add_many([&appsrc, &imagefreeze])
        .map_err(PreviewError::Compositing)?;
    gst::Element::link_many([&appsrc, &imagefreeze]).map_err(PreviewError::Compositing)?;

    let sink_pad = compositor
        .request_pad_simple("sink_%u")
        .ok_or(PreviewError::RequestPad)?;
    sink_pad.set_property("zorder", zorder);
    sink_pad.set_property("xpos", 0i32);
    sink_pad.set_property("ypos", 0i32);
    sink_pad.set_property("alpha", 1.0f64);

    let src_pad = imagefreeze
        .static_pad("src")
        .expect("imagefreeze always has a src pad");
    src_pad.link(&sink_pad).map_err(PreviewError::PadLink)?;

    Ok(())
}

/// A media pipeline loaded for preview playback — either a single file via `playbin`
/// ([`Self::open`]) or a multi-track `compositor` pipeline ([`Self::open_composited`]). Owns
/// the pipeline; dropping it tears the pipeline down (`State::Null`) so GStreamer releases any
/// decoder/output resources.
pub struct Preview {
    pipeline: gst::Element,
    video_sink: gst_app::AppSink,
    /// Each branch's own `uridecodebin`, in the same order [`Self::open_composited`]'s
    /// `overlays` were given (background first) — what [`Self::seek_composited`] seeks
    /// independently. Empty for a [`Self::open`]-opened single-clip pipeline, which uses the
    /// plain [`Self::seek`]/[`Self::seek_with_rate`] instead.
    branches: Vec<gst::Element>,
    /// `(branch_index, matte uridecodebin, clip.source_in_secs)` for every overlay branch with
    /// a background-removal matte. `branch_index` indexes into [`Self::branches`]/the offsets
    /// [`Self::seek_composited`] is given — the matte plays its own 0-based clip (see
    /// `ClipSegment::mask_video_path`'s doc comment in `bridge.h` for why: it was sampled
    /// directly from the overlay clip's own trimmed source range, not the timeline), so
    /// `seek_composited` derives its seek target from the matching branch's own offset minus
    /// `source_in_secs` rather than taking a separate offset from the caller — `ui`'s `App`
    /// never needs to know mattes exist.
    matte_branches: Vec<(usize, gst::Element, f64)>,
}

impl Preview {
    /// Opens `path` — the clip's resolved source (proxy or original) — and brings the
    /// pipeline up to `Paused` (decodes enough to preroll, so
    /// [`Self::position_secs`]/[`Self::duration_secs`]/[`Self::current_frame`] have something
    /// to report). `clip`, if given, has its effects (the subset [`build_video_filter_bin`]
    /// covers) applied via `playbin`'s `video-filter` property — probed from `path` itself
    /// first (not passed in) since `path` may be a lower-resolution editing proxy, and
    /// `videocrop`'s properties need the actual decoded pixel size. `None` skips both the
    /// probe and any filtering — for callers with no clip in scope at all (e.g. filmstrip
    /// thumbnail extraction, which always shows the raw source regardless of applied effects).
    pub fn open(path: &Path, clip: Option<&ClipInstance>) -> Result<Self, PreviewError> {
        gst::init().map_err(PreviewError::Init)?;

        let uri = gst::glib::filename_to_uri(path, None).map_err(PreviewError::UriConversion)?;

        let pipeline = gst::ElementFactory::make("playbin")
            .build()
            .map_err(PreviewError::CreateElement)?;
        pipeline.set_property("uri", uri.as_str());

        if let Some(clip) = clip {
            let resolution = avbridge::probe(path)
                .map_err(PreviewError::Probe)?
                .resolution;
            if let Some(filter_bin) = build_video_filter_bin(clip, resolution, true)? {
                pipeline.set_property("video-filter", &filter_bin);
            }
        }

        // Fixed RGBA caps: whatever the source's actual pixel format is (planar YUV, etc.),
        // playbin inserts the conversion elements needed to match this — callers of
        // current_frame() never need to handle more than one, simple, packed format.
        let video_caps = gst::Caps::builder("video/x-raw")
            .field("format", "RGBA")
            .build();
        let video_sink = gst_app::AppSink::builder()
            .caps(&video_caps)
            .sync(false)
            .max_buffers(1)
            .drop(true)
            .build();
        pipeline.set_property("video-sink", &video_sink);

        // No audio output wired up yet — fakesink avoids playbin defaulting to
        // autoaudiosink, which would need real audio hardware for something as basic as
        // `open()`.
        let audio_sink = gst::ElementFactory::make("fakesink")
            .build()
            .map_err(PreviewError::CreateElement)?;
        pipeline.set_property("audio-sink", &audio_sink);

        pipeline
            .set_state(gst::State::Paused)
            .map_err(PreviewError::StateChange)?;
        // Block until the Paused transition actually completes (it's commonly Async — the
        // preroll happens on GStreamer's own threads) so duration/position/frame queries made
        // right after `open()` returns are reliable instead of racing the preroll.
        let (result, _current, _pending) = pipeline.state(gst::ClockTime::from_seconds(5));
        result.map_err(PreviewError::StateChange)?;

        Ok(Self {
            pipeline,
            video_sink,
            branches: Vec::new(),
            matte_branches: Vec::new(),
        })
    }

    /// Opens a multi-track composited pipeline: `background` (track 0, `is_overlay: false`)
    /// underneath every entry in `overlays` (in the given order — later entries draw on top,
    /// matching [`crate::render::resolve_timeline_segments_multi`]'s "track index = z-order"
    /// convention), mixed live via `compositor` instead of `playbin`'s single-source decode.
    /// Wires up the subset of overlay-only effects export already gates the same way (see
    /// `crate::keyframe` module docs): position/opacity keyframes drive `compositor`'s own
    /// per-pad `xpos`/`ypos`/`alpha`, layer scale resizes the branch's decoded frame before
    /// compositing, and chroma key removes a background color before the branch reaches the
    /// mixer — all recomputed per buffer off the branch's own PTS, same pad-probe technique
    /// [`build_video_filter_bin`]'s scale/rotation keyframes already use.
    ///
    /// Each branch's `uridecodebin` seeks independently ([`Self::seek_composited`]) rather than
    /// through a single pipeline-wide seek — the branches are different source files with
    /// unrelated time bases (different trim points), so one absolute seek position wouldn't be
    /// simultaneously correct for all of them. Once each branch's own starting offset is set,
    /// `Playing` advances every branch together at the same rate off the pipeline's one shared
    /// clock — no further per-branch bookkeeping needed during playback itself.
    ///
    /// `background`'s resolution sizes the canvas every overlay's layer-scale/position math is
    /// expressed in pixels against — [`PreviewError::NoBackgroundVideo`] if it can't be probed.
    /// Every branch's own `ClipInstance::speed_factor` is honored via
    /// [`Self::seek_composited`]'s `rates` argument, called right after this returns — this
    /// method itself only opens the pipeline (implicitly rate `1.0` until the first seek).
    ///
    /// `text_overlays`/`shape_overlays` — the clips covering the playhead on any
    /// [`crate::timeline::TrackKind::Text`]/[`crate::timeline::TrackKind::Shape`] track, if
    /// any — are rasterized once each ([`crate::overlay_render`]) and composited on top of
    /// every `overlays` video branch (drawn last, matching export's own text/shape
    /// post-processing passes running after the main timeline composite). Static for the whole
    /// branch lifetime, unlike `overlays`' video branches — neither clip type has keyframes or
    /// a source file of its own to seek.
    pub fn open_composited(
        background_path: &Path,
        background_clip: Option<&ClipInstance>,
        overlays: &[(&Path, &ClipInstance)],
        text_overlays: &[&TextClip],
        shape_overlays: &[&ShapeClip],
    ) -> Result<Self, PreviewError> {
        gst::init().map_err(PreviewError::Init)?;

        let canvas = avbridge::probe(background_path)
            .map_err(PreviewError::Probe)?
            .resolution
            .ok_or(PreviewError::NoBackgroundVideo)?;

        let pipeline = gst::Pipeline::new();

        let compositor = gst::ElementFactory::make("compositor")
            .build()
            .map_err(PreviewError::CreateElement)?;
        let out_convert = gst::ElementFactory::make("videoconvert")
            .build()
            .map_err(PreviewError::CreateElement)?;
        let out_caps = gst::ElementFactory::make("capsfilter")
            .property(
                "caps",
                gst::Caps::builder("video/x-raw")
                    .field("format", "RGBA")
                    .build(),
            )
            .build()
            .map_err(PreviewError::CreateElement)?;
        pipeline
            .add_many([&compositor, &out_convert, &out_caps])
            .map_err(PreviewError::Compositing)?;
        gst::Element::link_many([&compositor, &out_convert, &out_caps])
            .map_err(PreviewError::Compositing)?;

        let video_caps = gst::Caps::builder("video/x-raw")
            .field("format", "RGBA")
            .build();
        let video_sink = gst_app::AppSink::builder()
            .caps(&video_caps)
            .sync(false)
            .max_buffers(1)
            .drop(true)
            .build();
        pipeline
            .add(&video_sink)
            .map_err(PreviewError::Compositing)?;
        out_caps
            .link(&video_sink)
            .map_err(PreviewError::Compositing)?;

        let (background_decodebin, background_matte) = build_composite_branch(
            &pipeline,
            &compositor,
            canvas,
            CompositeBranch {
                path: background_path,
                clip: background_clip,
                resolution: Some(canvas),
                is_overlay: false,
                zorder: 0,
            },
        )?;
        let mut branches = vec![background_decodebin];
        let mut matte_branches: Vec<(usize, gst::Element, f64)> = Vec::new();
        if let Some(matte) = background_matte {
            // Unreachable in practice — matte compositing is gated to overlay branches only
            // (see `build_composite_branch`'s doc comment) — but kept for completeness rather
            // than silently dropping a matte decodebin if that gate ever changes.
            matte_branches.push((0, matte, background_clip.map_or(0.0, |c| c.source_in_secs)));
        }
        for (i, (path, clip)) in overlays.iter().enumerate() {
            let resolution = avbridge::probe(path)
                .map_err(PreviewError::Probe)?
                .resolution;
            let (decodebin, matte) = build_composite_branch(
                &pipeline,
                &compositor,
                canvas,
                CompositeBranch {
                    path,
                    clip: Some(clip),
                    resolution,
                    is_overlay: true,
                    zorder: (i + 1) as u32,
                },
            )?;
            let branch_index = branches.len();
            branches.push(decodebin);
            if let Some(matte) = matte {
                matte_branches.push((branch_index, matte, clip.source_in_secs));
            }
        }

        let mut next_zorder = (overlays.len() + 1) as u32;
        for clip in text_overlays {
            let rgba = crate::overlay_render::render_text_clip_rgba(clip, canvas.0, canvas.1);
            build_static_overlay_branch(&pipeline, &compositor, canvas, rgba, next_zorder)?;
            next_zorder += 1;
        }
        for clip in shape_overlays {
            let rgba = crate::overlay_render::render_shape_clip_rgba(clip, canvas.0, canvas.1);
            build_static_overlay_branch(&pipeline, &compositor, canvas, rgba, next_zorder)?;
            next_zorder += 1;
        }

        let pipeline = pipeline.upcast::<gst::Element>();
        pipeline
            .set_state(gst::State::Paused)
            .map_err(PreviewError::StateChange)?;
        let (result, _current, _pending) = pipeline.state(gst::ClockTime::from_seconds(5));
        result.map_err(PreviewError::StateChange)?;

        Ok(Self {
            pipeline,
            video_sink,
            branches,
            matte_branches,
        })
    }

    /// Seeks a [`Self::open_composited`] pipeline's branches independently — `offsets[i]` is
    /// seconds within branch `i`'s own source file (background first, then `overlays` in the
    /// order [`Self::open_composited`] was given), not a shared timeline position, and
    /// `rates[i]` is that branch's own `ClipInstance::speed_factor` (missing/shorter than
    /// `offsets` defaults to `1.0`) — each branch is seeked independently via
    /// `Element::seek` (not `seek_simple`) sent directly to that branch's own `uridecodebin`,
    /// same technique [`Self::seek_with_rate`] uses pipeline-wide for the single-clip path.
    /// Seeking one element's own upstream segment doesn't affect any other branch's rate — this
    /// closes the gap [`Self::open_composited`]'s doc comment used to describe as "every branch
    /// plays at a uniform rate `1.0`". A no-op for any branch beyond `offsets`' length; extra
    /// offsets/rates past [`Self::branches`]' length are ignored. Returns the first branch's
    /// seek error, if any, after attempting every branch (partial application is preferable to
    /// leaving some branches on their old offset/rate with no indication which).
    ///
    /// Any [`Self::matte_branches`] entry tied to a branch that got seeked here is seeked too,
    /// at that same branch's own rate, to that branch's own offset minus its `source_in_secs`
    /// (the matte's own 0-based clip timeline — see [`Self::matte_branches`]' doc comment) —
    /// the caller never passes a separate offset or rate for it.
    pub fn seek_composited(&self, offsets: &[f64], rates: &[f64]) -> Result<(), PreviewError> {
        let mut first_err = None;
        for (i, (branch, &offset)) in self.branches.iter().zip(offsets).enumerate() {
            let rate = rates.get(i).copied().unwrap_or(1.0).max(0.01);
            if let Err(e) = branch.seek(
                rate,
                gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                gst::SeekType::Set,
                gst::ClockTime::from_seconds_f64(offset.max(0.0)),
                gst::SeekType::None,
                gst::ClockTime::NONE,
            ) {
                first_err.get_or_insert(PreviewError::Seek(e));
            }
        }
        for &(branch_index, ref matte, source_in_secs) in &self.matte_branches {
            let Some(&offset) = offsets.get(branch_index) else {
                continue;
            };
            let rate = rates.get(branch_index).copied().unwrap_or(1.0).max(0.01);
            let matte_offset = (offset - source_in_secs).max(0.0);
            if let Err(e) = matte.seek(
                rate,
                gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                gst::SeekType::Set,
                gst::ClockTime::from_seconds_f64(matte_offset),
                gst::SeekType::None,
                gst::ClockTime::NONE,
            ) {
                first_err.get_or_insert(PreviewError::Seek(e));
            }
        }
        match first_err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    pub fn play(&self) -> Result<(), PreviewError> {
        self.pipeline
            .set_state(gst::State::Playing)
            .map(|_| ())
            .map_err(PreviewError::StateChange)
    }

    pub fn pause(&self) -> Result<(), PreviewError> {
        self.pipeline
            .set_state(gst::State::Paused)
            .map(|_| ())
            .map_err(PreviewError::StateChange)
    }

    /// Seeks to `position_secs`, flushing buffered data for an immediate jump to the nearest
    /// key unit. Equivalent to [`Self::seek_with_rate`] at rate `1.0`.
    pub fn seek(&self, position_secs: f64) -> Result<(), PreviewError> {
        self.pipeline
            .seek_simple(
                gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                gst::ClockTime::from_seconds_f64(position_secs.max(0.0)),
            )
            .map_err(PreviewError::Seek)
    }

    /// Seeks to `position_secs` like [`Self::seek`], and additionally sets the pipeline's
    /// playback rate to `rate` — GStreamer's own rate-seek mechanism, not a filter element, so
    /// it covers audio too and stays in effect across a later plain `play()`/`pause()` until
    /// the next seek changes it again. Used to make preview playback/scrubbing honor
    /// [`crate::timeline::ClipInstance::speed_factor`] the same way export's `setpts` scaling
    /// does — without this, a sped-up/slowed-down clip would still visually play at 1x in the
    /// preview even though `ui`'s position math already accounts for speed when converting
    /// between timeline and source time.
    pub fn seek_with_rate(&self, position_secs: f64, rate: f64) -> Result<(), PreviewError> {
        self.pipeline
            .seek(
                rate,
                gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                gst::SeekType::Set,
                gst::ClockTime::from_seconds_f64(position_secs.max(0.0)),
                gst::SeekType::None,
                gst::ClockTime::NONE,
            )
            .map_err(PreviewError::Seek)
    }

    /// Current playhead position, if the pipeline can report one (usually only once it's
    /// reached at least `Paused` and finished prerolling).
    pub fn position_secs(&self) -> Option<f64> {
        self.pipeline
            .query_position::<gst::ClockTime>()
            .map(|t| t.seconds_f64())
    }

    /// Total duration of the loaded file, if known.
    pub fn duration_secs(&self) -> Option<f64> {
        self.pipeline
            .query_duration::<gst::ClockTime>()
            .map(|t| t.seconds_f64())
    }

    /// The most recent video frame the pipeline has decoded, as packed RGBA. `None` if
    /// nothing has arrived yet (e.g. called before the first preroll completes), the file
    /// has no video stream, or (while [`Self::play`]ing) no new sample has been decoded since
    /// the last call.
    ///
    /// Tries the prerolled frame first (what's available right after [`Self::open`] or a
    /// [`Self::seek`] while paused), then falls back to the latest live sample (what arrives
    /// during [`Self::play`]) — covers both without the caller needing to know which state
    /// produced the frame. Non-blocking (zero timeout on both pulls) so callers can poll this
    /// once per UI frame without risking a multi-frame stall waiting for a sample that isn't
    /// ready yet — safe since a decoded-but-not-yet-fetched frame just gets picked up on the
    /// next call instead.
    pub fn current_frame(&self) -> Option<VideoFrame> {
        let timeout = gst::ClockTime::ZERO;
        let sample = self
            .video_sink
            .try_pull_preroll(timeout)
            .or_else(|| self.video_sink.try_pull_sample(timeout))?;

        let buffer = sample.buffer()?;
        let caps = sample.caps()?;
        let info = gst_video::VideoInfo::from_caps(caps).ok()?;
        let frame = gst_video::VideoFrameRef::from_buffer_ref_readable(buffer, &info).ok()?;
        let plane = frame.plane_data(0).ok()?;

        Some(VideoFrame {
            width: info.width(),
            height: info.height(),
            rgba: plane.to_vec(),
        })
    }
}

impl Drop for Preview {
    fn drop(&mut self) {
        // Best-effort: nothing more to do if this fails, we're already tearing down.
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}
