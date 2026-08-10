#ifndef OCA_AVBRIDGE_BRIDGE_H
#define OCA_AVBRIDGE_BRIDGE_H

#include <stddef.h>
#include <stdint.h>

/* Returns libavformat's packed version number (same encoding as avformat_version()). */
uint32_t oca_avbridge_version(void);

typedef enum {
    OCA_PROBE_OK = 0,
    /* avformat_open_input() failed — bad path, unreadable file, unrecognized container. */
    OCA_PROBE_ERR_OPEN = 1,
    /* avformat_find_stream_info() failed — container opened but streams couldn't be read. */
    OCA_PROBE_ERR_STREAM_INFO = 2,
    /* No video or audio stream in the file. */
    OCA_PROBE_ERR_NO_MEDIA_STREAM = 3,
} OcaProbeStatus;

typedef struct {
    /* 1 if the picked stream is video, 0 if audio. */
    int has_video;
    double duration_secs;
    /* Short codec name (e.g. "h264", "aac") — always NUL-terminated. */
    char codec_name[32];
    /* Bits per second; 0 if neither the container nor the stream reports one. */
    int64_t bit_rate;
    /* Both 0 when has_video is 0. */
    int width;
    int height;
    /* Frame rate as a fraction; both 0 when unknown or has_video is 0. */
    int fps_num;
    int fps_den;
    /* 0 when has_video is 1 or the rate is unknown. */
    int sample_rate_hz;
} OcaProbeInfo;

/* Probes the media file at `path` (UTF-8, NUL-terminated) and fills `out` on success.
   Picks the first video stream if there is one, otherwise the first audio stream. */
OcaProbeStatus oca_avbridge_probe(const char *path, OcaProbeInfo *out);

typedef enum {
    OCA_REMUX_OK = 0,
    /* avformat_open_input() failed on in_path. */
    OCA_REMUX_ERR_OPEN_INPUT = 1,
    /* avformat_find_stream_info() failed. */
    OCA_REMUX_ERR_STREAM_INFO = 2,
    /* Couldn't allocate/guess an output format for out_path. */
    OCA_REMUX_ERR_ALLOC_OUTPUT = 3,
    /* Failed to create an output stream matching one of the input's streams. */
    OCA_REMUX_ERR_NEW_STREAM = 4,
    /* avio_open() failed on out_path (e.g. unwritable directory). */
    OCA_REMUX_ERR_OPEN_OUTPUT = 5,
    /* avformat_write_header() failed. */
    OCA_REMUX_ERR_WRITE_HEADER = 6,
    /* av_interleaved_write_frame() failed partway through. */
    OCA_REMUX_ERR_WRITE_FRAME = 7,
} OcaRemuxStatus;

/* Demuxes `in_path` and remuxes every stream to `out_path` unchanged — no decode, no encode,
   no filtering. Equivalent to `ffmpeg -i in_path -c copy out_path`. */
OcaRemuxStatus oca_avbridge_remux_copy(const char *in_path, const char *out_path);

typedef enum {
    OCA_ENCODE_OK = 0,
    OCA_ENCODE_ERR_OPEN_INPUT = 1,
    OCA_ENCODE_ERR_STREAM_INFO = 2,
    OCA_ENCODE_ERR_ALLOC_OUTPUT = 3,
    OCA_ENCODE_ERR_NEW_STREAM = 4,
    OCA_ENCODE_ERR_OPEN_OUTPUT = 5,
    OCA_ENCODE_ERR_WRITE_HEADER = 6,
    OCA_ENCODE_ERR_WRITE_FRAME = 7,
    /* The input has no audio stream to normalize. */
    OCA_ENCODE_ERR_NO_AUDIO_STREAM = 8,
    /* Couldn't find/open the audio decoder. */
    OCA_ENCODE_ERR_DECODER = 9,
    /* Couldn't build the loudnorm/limiter filter graph. */
    OCA_ENCODE_ERR_FILTER_GRAPH = 10,
    /* Couldn't find/open the AAC encoder. */
    OCA_ENCODE_ERR_ENCODER = 11,
    /* A decode/filter/encode call failed mid-stream (not at setup). */
    OCA_ENCODE_ERR_PIPELINE = 12,
    /* *cancel became nonzero mid-render — out_path is a truncated/invalid file, not written
       past the header. Not a failure: matches render.rs's RenderOutcome::Cancelled. */
    OCA_ENCODE_CANCELLED = 13,
} OcaEncodeStatus;

/* Called periodically during the read/decode loop with the input packet's position, in
   seconds, that's just been processed. Never called with a value past the file's duration. */
typedef void (*OcaProgressCallback)(void *user_data, double seconds_processed);

/* Renders `in_path` to `out_path`: video passthrough-copied, audio decoded, normalized
   (loudnorm to target_lufs + a true-peak safety limiter) and re-encoded to AAC 192kbps.
   Equivalent to:
   ffmpeg -i in_path -af "loudnorm=I=<target_lufs>:TP=-1.0:LRA=11,alimiter=limit=0.95:attack=5:release=50"
          -c:v copy -c:a aac -b:a 192k out_path
   Fails with OCA_ENCODE_ERR_NO_AUDIO_STREAM if `in_path` has no audio stream.

   progress_cb/progress_user_data may both be NULL to skip progress reporting.
   cancel may be NULL to disable cancellation; otherwise checked between packets — once
   `*cancel` is nonzero, stops and returns OCA_ENCODE_CANCELLED without writing a trailer. */
OcaEncodeStatus oca_avbridge_encode_export(const char *in_path, const char *out_path,
                                            float target_lufs, OcaProgressCallback progress_cb,
                                            void *progress_user_data, const uint8_t *cancel);

typedef enum {
    OCA_LOUDNESS_OK = 0,
    OCA_LOUDNESS_ERR_OPEN_INPUT = 1,
    OCA_LOUDNESS_ERR_STREAM_INFO = 2,
    OCA_LOUDNESS_ERR_NO_AUDIO_STREAM = 3,
    OCA_LOUDNESS_ERR_DECODER = 4,
    OCA_LOUDNESS_ERR_FILTER_GRAPH = 5,
    /* A decode/filter call failed mid-stream (not at setup). */
    OCA_LOUDNESS_ERR_PIPELINE = 6,
    /* The pipeline ran to completion but no loudnorm JSON report was captured. */
    OCA_LOUDNESS_ERR_NO_REPORT = 7,
} OcaLoudnessStatus;

/* Measures integrated loudness / true peak / loudness range via a single-pass `loudnorm`
   analysis (I=-16:TP=-1.5:LRA=11 — matches ffmpeg's
   `-af loudnorm=I=-16:TP=-1.5:LRA=11:print_format=json -f null -`). Nothing is written or
   re-encoded — analysis only.

   On success, writes the raw JSON report (NUL-terminated, truncated to fit if longer than
   out_json_len - 1 bytes) into `out_json` for the caller to parse. The loudnorm filter has no
   queryable struct API for its final stats — it only prints them via av_log() when it
   processes EOF — so this is the only way to get them out of libavfilter directly.

   NOT thread-safe: installs a process-global libavutil log callback for the call's duration
   (reset to av_log_default_callback before returning) — do not call this from multiple
   threads concurrently. */
OcaLoudnessStatus oca_avbridge_measure_loudness(const char *in_path, char *out_json,
                                                 size_t out_json_len);

typedef enum {
    OCA_PROXY_OK = 0,
    OCA_PROXY_ERR_OPEN_INPUT = 1,
    OCA_PROXY_ERR_STREAM_INFO = 2,
    /* The input has no video stream to make a proxy of. */
    OCA_PROXY_ERR_NO_VIDEO_STREAM = 3,
    OCA_PROXY_ERR_ALLOC_OUTPUT = 4,
    OCA_PROXY_ERR_NEW_STREAM = 5,
    OCA_PROXY_ERR_OPEN_OUTPUT = 6,
    OCA_PROXY_ERR_WRITE_HEADER = 7,
    OCA_PROXY_ERR_WRITE_FRAME = 8,
    /* Couldn't find/open the video or audio decoder. */
    OCA_PROXY_ERR_DECODER = 9,
    /* Couldn't find/open the libopenh264 video encoder or the AAC audio encoder. */
    OCA_PROXY_ERR_ENCODER = 10,
    /* Couldn't build the video scaler (libswscale). */
    OCA_PROXY_ERR_SCALER = 11,
    /* Couldn't build the (filterless, format-conversion-only) audio graph. */
    OCA_PROXY_ERR_FILTER_GRAPH = 12,
    /* A decode/scale/encode call failed mid-stream (not at setup). */
    OCA_PROXY_ERR_PIPELINE = 13,
} OcaProxyStatus;

/* Generates a downscaled editing proxy of in_path's video (height = target_height, width
   computed to preserve source aspect ratio and rounded to the nearest even number, matching
   ffmpeg's `scale=-2:<height>`) via libopenh264 (BSD-licensed — not libx264/GPL, which this
   LGPL FFmpeg build doesn't have). Any audio stream is re-encoded to AAC 128kbps unchanged
   otherwise (decode -> format-match the encoder -> encode, no filtering). Equivalent to:
   ffmpeg -i in_path -vf scale=-2:<target_height> -c:v libopenh264 -c:a aac -b:a 128k out_path
   Fails with OCA_PROXY_ERR_NO_VIDEO_STREAM if in_path has no video stream. Audio is optional —
   a video-only input produces a video-only proxy, no error. */
OcaProxyStatus oca_avbridge_generate_proxy(const char *in_path, const char *out_path,
                                            int target_height);

#endif
