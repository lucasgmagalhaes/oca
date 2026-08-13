#include "bridge.h"
#include "bridge_internal.h"

#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>

/* Tries to open a video encoder by name at the given canvas geometry. Returns NULL (freeing
   any partially-allocated context) if the encoder isn't compiled into this FFmpeg build, or if
   avcodec_open2 fails — the latter is the common case for a hardware encoder name on a machine
   without that vendor's GPU/driver present. */
static AVCodecContext *try_open_encoder(const char *encoder_name, int canvas_width,
                                         int canvas_height, AVRational canvas_fps,
                                         int64_t canvas_bit_rate_bps, int global_header) {
    const AVCodec *enc = avcodec_find_encoder_by_name(encoder_name);
    if (!enc) {
        return NULL;
    }
    AVCodecContext *ctx = avcodec_alloc_context3(enc);
    if (!ctx) {
        return NULL;
    }
    ctx->width = canvas_width;
    ctx->height = canvas_height;
    ctx->pix_fmt = AV_PIX_FMT_YUV420P;
    ctx->time_base = av_inv_q(canvas_fps);
    ctx->framerate = canvas_fps;
    ctx->gop_size = (canvas_fps.num / canvas_fps.den) * 2;
    ctx->max_b_frames = 0;
    ctx->bit_rate = canvas_bit_rate_bps;
    if (global_header) {
        ctx->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
    }
    if (avcodec_open2(ctx, enc, NULL) < 0) {
        avcodec_free_context(&ctx);
        return NULL;
    }
    return ctx;
}

AVCodecContext *open_video_encoder(GpuEncoderPreference preference, int canvas_width,
                                        int canvas_height, AVRational canvas_fps,
                                        int64_t canvas_bit_rate_bps, int global_header,
                                        int *out_used_gpu) {
    static const char *const nvenc = "h264_nvenc";
    static const char *const quicksync = "h264_qsv";
    static const char *const amf = "h264_amf";
    static const char *const cpu = "libopenh264";

    const char *hw_candidates[3] = {NULL, NULL, NULL};
    int hw_candidate_count = 0;
    switch (preference) {
        case GPU_ENCODER_NVENC:
            hw_candidates[hw_candidate_count++] = nvenc;
            break;
        case GPU_ENCODER_QUICKSYNC:
            hw_candidates[hw_candidate_count++] = quicksync;
            break;
        case GPU_ENCODER_AMF:
            hw_candidates[hw_candidate_count++] = amf;
            break;
        case GPU_ENCODER_AUTO:
            hw_candidates[hw_candidate_count++] = nvenc;
            hw_candidates[hw_candidate_count++] = quicksync;
            hw_candidates[hw_candidate_count++] = amf;
            break;
        case GPU_ENCODER_CPU:
        default:
            break;
    }

    for (int i = 0; i < hw_candidate_count; i++) {
        AVCodecContext *ctx = try_open_encoder(hw_candidates[i], canvas_width, canvas_height,
                                                canvas_fps, canvas_bit_rate_bps, global_header);
        if (ctx) {
            if (out_used_gpu) {
                *out_used_gpu = 1;
            }
            return ctx;
        }
    }

    AVCodecContext *ctx = try_open_encoder(cpu, canvas_width, canvas_height, canvas_fps,
                                            canvas_bit_rate_bps, global_header);
    if (out_used_gpu) {
        *out_used_gpu = 0;
    }
    return ctx;
}
