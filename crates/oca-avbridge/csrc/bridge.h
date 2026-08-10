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

#endif
