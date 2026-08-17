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

#include "bridge.h"
#include "bridge_internal.h"

#include <string.h>

#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>

/* h264_qsv rejects AV_PIX_FMT_YUV420P as a direct software-frame input (empirically observed:
   "Specified pixel format yuv420p is not supported by the h264_qsv encoder", wants nv12/qsv
   instead) — every other candidate here (h264_nvenc, h264_amf, libopenh264) accepts yuv420p,
   so only Quick Sync gets a different answer. See open_video_encoder's doc comment in
   bridge_internal.h for the full story. */
static enum AVPixelFormat pix_fmt_for_encoder_name(const char *encoder_name) {
    if (strcmp(encoder_name, "h264_qsv") == 0) {
        return AV_PIX_FMT_NV12;
    }
    return AV_PIX_FMT_YUV420P;
}

/* Tries to open a video encoder by name at the given canvas geometry. Returns NULL (freeing
   any partially-allocated context) if the encoder isn't compiled into this FFmpeg build, or if
   avcodec_open2 fails — the latter is the common case for a hardware encoder name on a machine
   without that vendor's GPU/driver present. On success, if `out_pix_fmt` is non-NULL, reports
   the pixel format the opened context actually uses — callers need this to make the filter
   graph feeding the encoder target the same format. */
static AVCodecContext *try_open_encoder(const char *encoder_name, int canvas_width,
                                         int canvas_height, AVRational canvas_fps,
                                         int64_t canvas_bit_rate_bps, int global_header,
                                         enum AVPixelFormat *out_pix_fmt) {
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
    ctx->pix_fmt = pix_fmt_for_encoder_name(encoder_name);
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
    if (out_pix_fmt) {
        *out_pix_fmt = ctx->pix_fmt;
    }
    return ctx;
}

AVCodecContext *open_video_encoder(GpuEncoderPreference preference, int canvas_width,
                                        int canvas_height, AVRational canvas_fps,
                                        int64_t canvas_bit_rate_bps, int global_header,
                                        int *out_used_gpu, enum AVPixelFormat *out_pix_fmt) {
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
                                                canvas_fps, canvas_bit_rate_bps, global_header,
                                                out_pix_fmt);
        if (ctx) {
            if (out_used_gpu) {
                *out_used_gpu = 1;
            }
            return ctx;
        }
    }

    AVCodecContext *ctx = try_open_encoder(cpu, canvas_width, canvas_height, canvas_fps,
                                            canvas_bit_rate_bps, global_header, out_pix_fmt);
    if (out_used_gpu) {
        *out_used_gpu = 0;
    }
    return ctx;
}
