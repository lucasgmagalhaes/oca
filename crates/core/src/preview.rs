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

use crate::timeline::{ClipInstance, ColorFilter};

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
fn build_video_filter_bin(
    clip: &ClipInstance,
    resolution: Option<(u32, u32)>,
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

    if clip.has_opacity_keyframes() {
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

/// A single media file loaded into a `playbin`-based GStreamer pipeline for preview playback.
/// Owns the pipeline; dropping it tears the pipeline down (`State::Null`) so GStreamer releases
/// any decoder/output resources.
pub struct Preview {
    pipeline: gst::Element,
    video_sink: gst_app::AppSink,
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
            if let Some(filter_bin) = build_video_filter_bin(clip, resolution)? {
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
        })
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
