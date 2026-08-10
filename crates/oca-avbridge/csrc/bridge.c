#include "bridge.h"

#include <string.h>

#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>

uint32_t oca_avbridge_version(void) { return avformat_version(); }

OcaProbeStatus oca_avbridge_probe(const char *path, OcaProbeInfo *out) {
    AVFormatContext *fmt_ctx = NULL;

    if (avformat_open_input(&fmt_ctx, path, NULL, NULL) < 0) {
        return OCA_PROBE_ERR_OPEN;
    }

    if (avformat_find_stream_info(fmt_ctx, NULL) < 0) {
        avformat_close_input(&fmt_ctx);
        return OCA_PROBE_ERR_STREAM_INFO;
    }

    AVStream *video = NULL;
    AVStream *audio = NULL;
    for (unsigned int i = 0; i < fmt_ctx->nb_streams; i++) {
        AVStream *s = fmt_ctx->streams[i];
        if (!video && s->codecpar->codec_type == AVMEDIA_TYPE_VIDEO) {
            video = s;
        } else if (!audio && s->codecpar->codec_type == AVMEDIA_TYPE_AUDIO) {
            audio = s;
        }
    }

    AVStream *stream = video ? video : audio;
    if (!stream) {
        avformat_close_input(&fmt_ctx);
        return OCA_PROBE_ERR_NO_MEDIA_STREAM;
    }

    memset(out, 0, sizeof(*out));
    out->has_video = video ? 1 : 0;

    double duration_secs = 0.0;
    if (fmt_ctx->duration != (int64_t)AV_NOPTS_VALUE) {
        duration_secs = (double)fmt_ctx->duration / AV_TIME_BASE;
    } else if (stream->duration != (int64_t)AV_NOPTS_VALUE) {
        duration_secs = stream->duration * av_q2d(stream->time_base);
    }
    out->duration_secs = duration_secs;

    const char *codec_name = avcodec_get_name(stream->codecpar->codec_id);
    strncpy(out->codec_name, codec_name, sizeof(out->codec_name) - 1);

    int64_t bit_rate = fmt_ctx->bit_rate;
    if (bit_rate <= 0) {
        bit_rate = stream->codecpar->bit_rate;
    }
    out->bit_rate = bit_rate > 0 ? bit_rate : 0;

    if (video) {
        out->width = stream->codecpar->width;
        out->height = stream->codecpar->height;
        AVRational fps =
            stream->avg_frame_rate.num ? stream->avg_frame_rate : stream->r_frame_rate;
        out->fps_num = fps.num;
        out->fps_den = fps.den;
    } else {
        out->sample_rate_hz = stream->codecpar->sample_rate;
    }

    avformat_close_input(&fmt_ctx);
    return OCA_PROBE_OK;
}
