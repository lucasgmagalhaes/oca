#ifndef OCA_AVBRIDGE_BRIDGE_INTERNAL_H
#define OCA_AVBRIDGE_BRIDGE_INTERNAL_H

/* Declarations shared across bridge.c's split translation units. Nothing here is part of
   the public API (see bridge.h) — these are internal helpers reused by more than one .c
   file, so they can no longer be `static`. */

#include <libavcodec/avcodec.h>
#include <libavfilter/avfilter.h>
#include <libavformat/avformat.h>

/* Opens `path` and reads its stream info into a new AVFormatContext.
   Returns 0 on success (caller owns *fmt_ctx_out and must close it),
   -1 if avformat_open_input failed, -2 if avformat_find_stream_info failed
   (*fmt_ctx_out is NULL in both error cases). Defined in common.c. */
int oca_open_input(const char *path, AVFormatContext **fmt_ctx_out);

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
