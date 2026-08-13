#ifndef AVBRIDGE_BRIDGE_INTERNAL_H
#define AVBRIDGE_BRIDGE_INTERNAL_H

/* Declarations shared across bridge.c's split translation units. Nothing here is part of
   the public API (see bridge.h) — these are internal helpers reused by more than one .c
   file, so they can no longer be `static`. */

#include <libavcodec/avcodec.h>
#include <libavfilter/avfilter.h>
#include <libavformat/avformat.h>

#include "bridge.h"

typedef enum {
    /* Try hardware encoders in a fixed order (NVENC, then Quick Sync, then AMF), falling back
       to the CPU (libopenh264) encoder if none of them open successfully. */
    GPU_ENCODER_AUTO = 0,
    /* Force the CPU (libopenh264) encoder — no hardware attempt. */
    GPU_ENCODER_CPU = 1,
    /* Force NVIDIA NVENC (h264_nvenc), falling back to CPU if it can't open. */
    GPU_ENCODER_NVENC = 2,
    /* Force Intel Quick Sync (h264_qsv), falling back to CPU if it can't open. */
    GPU_ENCODER_QUICKSYNC = 3,
    /* Force AMD AMF (h264_amf), falling back to CPU if it can't open. */
    GPU_ENCODER_AMF = 4,
} GpuEncoderPreference;

/* Opens a video H.264 encoder AVCodecContext sized for canvas_width/canvas_height/canvas_fps
   at canvas_bit_rate_bps, honoring `preference`. Tries the requested hardware encoder(s) first
   (AUTO tries all three in a fixed order); each attempt uses AV_PIX_FMT_YUV420P as the input
   pixel format — the same format the existing filter chains already conform every segment to
   (see e.g. timeline_export.c's final `format=yuv420p` stage), so no filter-graph changes are
   needed to feed a hardware encoder instead of libopenh264, WHEN that encoder accepts yuv420p
   as a direct software-frame input. avcodec_open2() fails at runtime (not just "encoder missing
   from this FFmpeg build") both when no compatible GPU/driver is present AND when the encoder
   is present but rejects this pixel format — either way it's treated identically here as
   "unavailable, fall back to CPU". This function can't be exercised against real GPU hardware
   in CI/dev sandboxes without one, but the pix_fmt-rejection path WAS empirically observed on
   this dev machine's FFmpeg build even without real hardware: h264_qsv logs "Specified pixel
   format yuv420p is not supported by the h264_qsv encoder" and lists only nv12/qsv as
   supported, then avcodec_open2 fails and the CPU fallback engages — so on a machine that DOES
   have real Intel Quick Sync hardware, OCA_GPU_ENCODER_QUICKSYNC would currently still silently
   fall back to CPU every time with this implementation, never actually engaging the hardware
   encoder, since converting to nv12 (a swscale/format-filter stage before avcodec_send_frame)
   isn't implemented — a known, deliberate limitation, not an oversight. NVENC/AMF didn't reach
   the pix_fmt check at all on this machine (no NVIDIA GPU; AMD's amfrt64.dll isn't present) so
   whether they'd accept yuv420p directly on real hardware is unverified either way. The CPU
   fallback path itself IS exercised end-to-end for every preference value (see
   encode_test.rs's every_gpu_encoder_preference_falls_back_to_a_working_export), since forcing
   any hardware preference on this machine deterministically falls through to it.

   If none of the attempted encoders open, falls back to the CPU (libopenh264) encoder — the
   "fallback pro encode por CPU" from the Fase 5 spec. Sets *out_used_gpu to 1 if a hardware
   encoder was actually opened, 0 if the CPU fallback (or explicit GPU_ENCODER_CPU) was
   used. `global_header` should be nonzero when the output format wants
   AV_CODEC_FLAG_GLOBAL_HEADER set (out_ctx->oformat->flags & AVFMT_GLOBALHEADER).

   Returns NULL only if even the CPU fallback couldn't open (should only happen if libopenh264
   itself is missing from this FFmpeg build). Caller owns the returned AVCodecContext (not yet
   attached to any AVStream). Defined in gpu_encoder.c. */
AVCodecContext *open_video_encoder(GpuEncoderPreference preference, int canvas_width,
                                        int canvas_height, AVRational canvas_fps,
                                        int64_t canvas_bit_rate_bps, int global_header,
                                        int *out_used_gpu);

/* Builds the animated Ken-Burns zoom filter stage (ClipSegment::zoom_start/zoom_end) into
   `buf` (capacity `cap`) — empty string if both are ~1.0 (no zoom configured). A geq
   inverse-sample: `buf`'s caller just splices it into a comma-joined filter chain, no `n`/`N`
   frame-counter reset concerns beyond the usual "fresh per segment's filter graph" ones.
   Shared by timeline_export.c and timeline_export_multi.c's two filter-string builders, all
   three of which need the same fix (see its definition in timeline_export_multi.c for why the
   original crop/scale-based version reliably failed or crashed instead of animating). */
void build_kenburns_zoom(const ClipSegment *seg, int fps_num, int fps_den,
                              char *buf, size_t cap);

/* Opens `path` and reads its stream info into a new AVFormatContext.
   Returns 0 on success (caller owns *fmt_ctx_out and must close it),
   -1 if avformat_open_input failed, -2 if avformat_find_stream_info failed
   (*fmt_ctx_out is NULL in both error cases). Defined in common.c. */
int open_input(const char *path, AVFormatContext **fmt_ctx_out);

typedef struct {
    AVFilterContext *buffersrc_ctx;
    AVFilterContext *buffersink_ctx;
    AVFilterGraph *graph;
} AudioFilterChain;

/* All defined in filters.c. */
void free_audio_filter_chain(AudioFilterChain *chain);
int init_audio_filter_chain(AVCodecContext *dec_ctx, const AVCodec *encoder,
                             const char *filter_descr, AudioFilterChain *chain);
int encode_write_packet(AVFormatContext *out_ctx, AVCodecContext *enc_ctx,
                         AVStream *out_stream, AVFrame *frame, AVPacket *enc_pkt);
int filter_encode_write_frame(AVFormatContext *out_ctx, AudioFilterChain *chain,
                               AVCodecContext *enc_ctx, AVStream *out_stream,
                               AVFrame *frame, AVFrame *filt_frame, AVPacket *enc_pkt);

typedef struct {
    AVFilterContext *buffersrc_ctx;
    AVFilterContext *buffersink_ctx;
    AVFilterGraph *graph;
} VideoFilterChain;

/* All defined in filters.c. */
void free_video_filter_chain(VideoFilterChain *chain);
int init_video_filter_chain(AVCodecContext *dec_ctx, const char *filter_descr,
                             VideoFilterChain *chain);
int filter_encode_write_video_frame(AVFormatContext *out_ctx, VideoFilterChain *chain,
                                     AVCodecContext *enc_ctx, AVStream *out_stream,
                                     AVFrame *frame, AVFrame *filt_frame,
                                     int64_t *next_pts, AVPacket *enc_pkt);

#endif
