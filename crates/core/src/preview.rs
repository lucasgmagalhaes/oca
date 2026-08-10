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

#[derive(Debug)]
pub enum PreviewError {
    Init(gst::glib::Error),
    CreateElement(gst::glib::BoolError),
    UriConversion(gst::glib::Error),
    StateChange(gst::StateChangeError),
    Seek(gst::glib::BoolError),
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

/// A single media file loaded into a `playbin`-based GStreamer pipeline for preview playback.
/// Owns the pipeline; dropping it tears the pipeline down (`State::Null`) so GStreamer releases
/// any decoder/output resources.
pub struct Preview {
    pipeline: gst::Element,
    video_sink: gst_app::AppSink,
}

impl Preview {
    /// Opens `path` and brings the pipeline up to `Paused` (decodes enough to preroll, so
    /// [`Self::position_secs`]/[`Self::duration_secs`]/[`Self::current_frame`] have something
    /// to report).
    pub fn open(path: &Path) -> Result<Self, PreviewError> {
        gst::init().map_err(PreviewError::Init)?;

        let uri = gst::glib::filename_to_uri(path, None).map_err(PreviewError::UriConversion)?;

        let pipeline = gst::ElementFactory::make("playbin")
            .build()
            .map_err(PreviewError::CreateElement)?;
        pipeline.set_property("uri", uri.as_str());

        // Fixed RGBA caps: whatever the source's actual pixel format is (planar YUV, etc.),
        // playbin inserts the conversion elements needed to match this — callers of
        // current_frame() never need to handle more than one, simple, packed format.
        let video_caps = gst::Caps::builder("video/x-raw").field("format", "RGBA").build();
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

        Ok(Self { pipeline, video_sink })
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
    /// key unit.
    pub fn seek(&self, position_secs: f64) -> Result<(), PreviewError> {
        self.pipeline
            .seek_simple(
                gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                gst::ClockTime::from_seconds_f64(position_secs.max(0.0)),
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
    /// nothing has arrived yet (e.g. called before the first preroll completes) or the file
    /// has no video stream.
    ///
    /// Tries the prerolled frame first (what's available right after [`Self::open`] or a
    /// [`Self::seek`] while paused), then falls back to the latest live sample (what arrives
    /// during [`Self::play`]) — covers both without the caller needing to know which state
    /// produced the frame.
    pub fn current_frame(&self) -> Option<VideoFrame> {
        let timeout = gst::ClockTime::from_mseconds(100);
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
