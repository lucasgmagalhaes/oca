#ifndef OCA_AVBRIDGE_BRIDGE_H
#define OCA_AVBRIDGE_BRIDGE_H

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
} OcaEncodeStatus;

/* Renders `in_path` to `out_path`: video passthrough-copied, audio decoded, normalized
   (loudnorm to target_lufs + a true-peak safety limiter) and re-encoded to AAC 192kbps.
   Equivalent to:
   ffmpeg -i in_path -af "loudnorm=I=<target_lufs>:TP=-1.0:LRA=11,alimiter=limit=0.95:attack=5:release=50"
          -c:v copy -c:a aac -b:a 192k out_path
   Fails with OCA_ENCODE_ERR_NO_AUDIO_STREAM if `in_path` has no audio stream. */
OcaEncodeStatus oca_avbridge_encode_export(const char *in_path, const char *out_path,
                                            float target_lufs);

#endif
