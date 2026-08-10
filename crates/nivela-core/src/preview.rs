//! Minimal GStreamer-based preview pipeline — plays a media file via `playbin`, decoding
//! independent of the UI thread (GStreamer drives its own internal threads once the pipeline
//! is `Playing`/`Paused`, no subprocess). No frame extraction/texture upload yet — that's a
//! follow-on task once this proves the open/play/pause/seek/query lifecycle works end to end.

use std::path::Path;

use gstreamer as gst;
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

/// A single media file loaded into a `playbin`-based GStreamer pipeline for preview playback.
/// Owns the pipeline; dropping it tears the pipeline down (`State::Null`) so GStreamer releases
/// any decoder/output resources.
pub struct Preview {
    pipeline: gst::Element,
}

impl Preview {
    /// Opens `path` and brings the pipeline up to `Paused` (decodes enough to preroll, so
    /// [`Self::position_secs`]/[`Self::duration_secs`] have something to report).
    pub fn open(path: &Path) -> Result<Self, PreviewError> {
        gst::init().map_err(PreviewError::Init)?;

        let uri = gst::glib::filename_to_uri(path, None).map_err(PreviewError::UriConversion)?;

        let pipeline = gst::ElementFactory::make("playbin")
            .build()
            .map_err(PreviewError::CreateElement)?;
        pipeline.set_property("uri", uri.as_str());

        // No frame extraction/display wired up yet (see the module doc) — fakesink for both
        // avoids playbin defaulting to autovideosink/autoaudiosink, which would pop an OS
        // window or require real audio hardware during something as basic as `open()`.
        // Swapping video-sink for an appsink is exactly the follow-on task's job.
        let video_sink = gst::ElementFactory::make("fakesink")
            .build()
            .map_err(PreviewError::CreateElement)?;
        let audio_sink = gst::ElementFactory::make("fakesink")
            .build()
            .map_err(PreviewError::CreateElement)?;
        pipeline.set_property("video-sink", &video_sink);
        pipeline.set_property("audio-sink", &audio_sink);

        pipeline
            .set_state(gst::State::Paused)
            .map_err(PreviewError::StateChange)?;
        // Block until the Paused transition actually completes (it's commonly Async — the
        // preroll happens on GStreamer's own threads) so duration/position queries made right
        // after `open()` returns are reliable instead of racing the preroll.
        let (result, _current, _pending) = pipeline.state(gst::ClockTime::from_seconds(5));
        result.map_err(PreviewError::StateChange)?;

        Ok(Self { pipeline })
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
}

impl Drop for Preview {
    fn drop(&mut self) {
        // Best-effort: nothing more to do if this fails, we're already tearing down.
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}
