#include "bridge.h"

#include <math.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#include <libavcodec/avcodec.h>
#include <libavfilter/avfilter.h>
#include <libavfilter/buffersink.h>
#include <libavfilter/buffersrc.h>
#include <libavformat/avformat.h>
#include <libavutil/channel_layout.h>
#include <libavutil/log.h>
#include <libavutil/opt.h>
#include <libswscale/swscale.h>

uint32_t avbridge_version(void) { return avformat_version(); }

/* Opens `path` and reads its stream info into a new AVFormatContext.
   Returns 0 on success (caller owns *fmt_ctx_out and must close it),
   -1 if avformat_open_input failed, -2 if avformat_find_stream_info failed
   (*fmt_ctx_out is NULL in both error cases). */
static int oca_open_input(const char *path, AVFormatContext **fmt_ctx_out) {
    if (avformat_open_input(fmt_ctx_out, path, NULL, NULL) < 0)
        return -1;
    if (avformat_find_stream_info(*fmt_ctx_out, NULL) < 0) {
        avformat_close_input(fmt_ctx_out);
        return -2;
    }
    return 0;
}

OcaProbeStatus avbridge_probe(const char *path, OcaProbeInfo *out) {
    AVFormatContext *fmt_ctx = NULL;

    switch (oca_open_input(path, &fmt_ctx)) {
        case -1: return OCA_PROBE_ERR_OPEN;
        case -2: return OCA_PROBE_ERR_STREAM_INFO;
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

OcaRemuxStatus avbridge_remux_copy(const char *in_path, const char *out_path) {
    AVFormatContext *in_ctx = NULL;
    AVFormatContext *out_ctx = NULL;
    int *stream_mapping = NULL;
    OcaRemuxStatus status = OCA_REMUX_OK;

    switch (oca_open_input(in_path, &in_ctx)) {
        case -1: return OCA_REMUX_ERR_OPEN_INPUT;
        case -2: return OCA_REMUX_ERR_STREAM_INFO;
    }

    avformat_alloc_output_context2(&out_ctx, NULL, NULL, out_path);
    if (!out_ctx) {
        avformat_close_input(&in_ctx);
        return OCA_REMUX_ERR_ALLOC_OUTPUT;
    }

    stream_mapping = av_calloc(in_ctx->nb_streams, sizeof(*stream_mapping));
    if (!stream_mapping) {
        avformat_free_context(out_ctx);
        avformat_close_input(&in_ctx);
        return OCA_REMUX_ERR_ALLOC_OUTPUT;
    }

    int out_stream_count = 0;
    for (unsigned int i = 0; i < in_ctx->nb_streams; i++) {
        AVStream *in_stream = in_ctx->streams[i];
        AVCodecParameters *in_codecpar = in_stream->codecpar;

        if (in_codecpar->codec_type != AVMEDIA_TYPE_VIDEO &&
            in_codecpar->codec_type != AVMEDIA_TYPE_AUDIO) {
            stream_mapping[i] = -1;
            continue;
        }

        AVStream *out_stream = avformat_new_stream(out_ctx, NULL);
        if (!out_stream || avcodec_parameters_copy(out_stream->codecpar, in_codecpar) < 0) {
            status = OCA_REMUX_ERR_NEW_STREAM;
            goto cleanup;
        }
        out_stream->codecpar->codec_tag = 0;
        out_stream->time_base = in_stream->time_base;
        stream_mapping[i] = out_stream_count++;
    }

    if (!(out_ctx->oformat->flags & AVFMT_NOFILE)) {
        if (avio_open(&out_ctx->pb, out_path, AVIO_FLAG_WRITE) < 0) {
            status = OCA_REMUX_ERR_OPEN_OUTPUT;
            goto cleanup;
        }
    }

    if (avformat_write_header(out_ctx, NULL) < 0) {
        status = OCA_REMUX_ERR_WRITE_HEADER;
        goto cleanup_output_io;
    }

    {
        AVPacket *pkt = av_packet_alloc();
        if (!pkt) {
            status = OCA_REMUX_ERR_WRITE_FRAME;
            goto cleanup_output_io;
        }

        while (av_read_frame(in_ctx, pkt) >= 0) {
            AVStream *in_stream = in_ctx->streams[pkt->stream_index];
            int mapped_index = stream_mapping[pkt->stream_index];
            if (mapped_index < 0) {
                av_packet_unref(pkt);
                continue;
            }

            AVStream *out_stream = out_ctx->streams[mapped_index];
            pkt->stream_index = mapped_index;
            av_packet_rescale_ts(pkt, in_stream->time_base, out_stream->time_base);
            pkt->pos = -1;

            if (av_interleaved_write_frame(out_ctx, pkt) < 0) {
                status = OCA_REMUX_ERR_WRITE_FRAME;
                break;
            }
        }
        av_packet_free(&pkt);

        if (status == OCA_REMUX_OK) {
            av_write_trailer(out_ctx);
        }
    }

cleanup_output_io:
    if (out_ctx && !(out_ctx->oformat->flags & AVFMT_NOFILE)) {
        avio_closep(&out_ctx->pb);
    }
cleanup:
    av_freep(&stream_mapping);
    if (out_ctx) {
        avformat_free_context(out_ctx);
    }
    avformat_close_input(&in_ctx);
    return status;
}

typedef struct {
    AVFilterContext *buffersrc_ctx;
    AVFilterContext *buffersink_ctx;
    AVFilterGraph *graph;
} AudioFilterChain;

static void free_audio_filter_chain(AudioFilterChain *chain) {
    avfilter_graph_free(&chain->graph);
    chain->buffersrc_ctx = NULL;
    chain->buffersink_ctx = NULL;
}

/* Builds `filter_descr` (e.g. "loudnorm=...,alimiter=..." or just "anull" for a passthrough
   format-conversion-only chain) between an abuffer source shaped like dec_ctx's decoded audio
   and an abuffersink constrained to a format `encoder` accepts (so the graph auto-inserts any
   needed aformat/aresample conversion, per FFmpeg's own transcode_aac.c example) — without
   this, avcodec_open2() on the encoder can reject whatever raw format the filter chain
   happens to produce. */
static int init_audio_filter_chain(AVCodecContext *dec_ctx, const AVCodec *encoder,
                                    const char *filter_descr, AudioFilterChain *chain) {
    char args[512];
    char chlayout_str[64];
    const AVFilter *buffersrc = avfilter_get_by_name("abuffer");
    const AVFilter *buffersink = avfilter_get_by_name("abuffersink");
    AVFilterInOut *outputs = avfilter_inout_alloc();
    AVFilterInOut *inputs = avfilter_inout_alloc();
    int ret;

    chain->buffersrc_ctx = NULL;
    chain->buffersink_ctx = NULL;
    chain->graph = avfilter_graph_alloc();
    if (!outputs || !inputs || !chain->graph) {
        ret = AVERROR(ENOMEM);
        goto end;
    }

    av_channel_layout_describe(&dec_ctx->ch_layout, chlayout_str, sizeof(chlayout_str));
    snprintf(args, sizeof(args),
             "time_base=1/%d:sample_rate=%d:sample_fmt=%s:channel_layout=%s",
             dec_ctx->sample_rate, dec_ctx->sample_rate,
             av_get_sample_fmt_name(dec_ctx->sample_fmt), chlayout_str);

    ret = avfilter_graph_create_filter(&chain->buffersrc_ctx, buffersrc, "in", args, NULL,
                                        chain->graph);
    if (ret < 0) goto end;

    /* abuffersink, built in two steps (alloc, then init) instead of the one-shot
       avfilter_graph_create_filter(): "sample_fmts" is a pre-init-only option — setting it
       after the filter is initialized fails with "not a runtime option". */
    chain->buffersink_ctx = avfilter_graph_alloc_filter(chain->graph, buffersink, "out");
    if (!chain->buffersink_ctx) {
        ret = AVERROR(ENOMEM);
        goto end;
    }

    /* Constrain the sink to formats/rates `encoder` actually accepts, so the graph inserts
       any needed aformat/aresample conversion automatically (per FFmpeg's own
       transcode_aac.c example) instead of avcodec_open2() rejecting whatever the filter
       chain happens to produce. As of this FFmpeg version these are proper array-typed
       options set via av_opt_set_array() — the older av_opt_set_bin()-with-a-raw-array
       idiom ("sample_fmts") is deprecated and, here, silently fails to constrain anything. */
    {
        const enum AVSampleFormat *sample_fmts = NULL;
        int num_sample_fmts = 0;
        if (avcodec_get_supported_config(NULL, encoder, AV_CODEC_CONFIG_SAMPLE_FORMAT, 0,
                                          (const void **)&sample_fmts, &num_sample_fmts) == 0 &&
            sample_fmts && num_sample_fmts > 0) {
            ret = av_opt_set_array(chain->buffersink_ctx, "sample_formats",
                                    AV_OPT_SEARCH_CHILDREN, 0, (unsigned)num_sample_fmts,
                                    AV_OPT_TYPE_SAMPLE_FMT, sample_fmts);
            if (ret < 0) goto end;
        }
    }

    {
        const int *sample_rates = NULL;
        int num_sample_rates = 0;
        if (avcodec_get_supported_config(NULL, encoder, AV_CODEC_CONFIG_SAMPLE_RATE, 0,
                                          (const void **)&sample_rates, &num_sample_rates) == 0 &&
            sample_rates && num_sample_rates > 0) {
            /* Prefer the source's own rate when the encoder already accepts it — negotiation
               otherwise doesn't necessarily pick the closest/cheapest match from the allowed
               set, and will happily insert a gratuitous upsample (seen: 44100 -> 96000, the
               first entry in AAC's supported-rates list) when no rate is preferred. */
            int source_rate_ok = 0;
            for (int i = 0; i < num_sample_rates; i++) {
                if (sample_rates[i] == dec_ctx->sample_rate) {
                    source_rate_ok = 1;
                    break;
                }
            }
            if (source_rate_ok) {
                ret = av_opt_set_array(chain->buffersink_ctx, "samplerates",
                                        AV_OPT_SEARCH_CHILDREN, 0, 1, AV_OPT_TYPE_INT,
                                        &dec_ctx->sample_rate);
            } else {
                ret = av_opt_set_array(chain->buffersink_ctx, "samplerates",
                                        AV_OPT_SEARCH_CHILDREN, 0, (unsigned)num_sample_rates,
                                        AV_OPT_TYPE_INT, sample_rates);
            }
            if (ret < 0) goto end;
        }
    }

    ret = avfilter_init_str(chain->buffersink_ctx, NULL);
    if (ret < 0) goto end;

    outputs->name = av_strdup("in");
    outputs->filter_ctx = chain->buffersrc_ctx;
    outputs->pad_idx = 0;
    outputs->next = NULL;

    inputs->name = av_strdup("out");
    inputs->filter_ctx = chain->buffersink_ctx;
    inputs->pad_idx = 0;
    inputs->next = NULL;

    ret = avfilter_graph_parse_ptr(chain->graph, filter_descr, &inputs, &outputs, NULL);
    if (ret < 0) goto end;

    ret = avfilter_graph_config(chain->graph, NULL);

end:
    avfilter_inout_free(&inputs);
    avfilter_inout_free(&outputs);
    if (ret < 0) {
        free_audio_filter_chain(chain);
    }
    return ret;
}

/* Drains every packet the encoder has ready and writes it. Pass frame=NULL once, at the end,
   to flush. */
static int encode_write_packet(AVFormatContext *out_ctx, AVCodecContext *enc_ctx,
                                AVStream *out_stream, AVFrame *frame, AVPacket *enc_pkt) {
    int ret = avcodec_send_frame(enc_ctx, frame);
    if (ret < 0 && ret != AVERROR_EOF) return ret;

    while (1) {
        ret = avcodec_receive_packet(enc_ctx, enc_pkt);
        if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF) {
            return 0;
        } else if (ret < 0) {
            return ret;
        }
        enc_pkt->stream_index = out_stream->index;
        av_packet_rescale_ts(enc_pkt, enc_ctx->time_base, out_stream->time_base);
        ret = av_interleaved_write_frame(out_ctx, enc_pkt);
        if (ret < 0) return ret;
    }
}

/* Pushes one decoded frame through the filter graph and encodes+writes every frame it
   produces in response (a loudnorm/alimiter chain doesn't emit 1:1 with its input). Pass
   frame=NULL once, at the end, to flush the filter graph itself. */
static int filter_encode_write_frame(AVFormatContext *out_ctx, AudioFilterChain *chain,
                                      AVCodecContext *enc_ctx, AVStream *out_stream,
                                      AVFrame *frame, AVFrame *filt_frame, AVPacket *enc_pkt) {
    int ret = av_buffersrc_add_frame(chain->buffersrc_ctx, frame);
    if (ret < 0) return ret;

    while (1) {
        ret = av_buffersink_get_frame(chain->buffersink_ctx, filt_frame);
        if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF) {
            return 0;
        } else if (ret < 0) {
            return ret;
        }
        ret = encode_write_packet(out_ctx, enc_ctx, out_stream, filt_frame, enc_pkt);
        av_frame_unref(filt_frame);
        if (ret < 0) return ret;
    }
}

typedef struct {
    AVFilterContext *buffersrc_ctx;
    AVFilterContext *buffersink_ctx;
    AVFilterGraph *graph;
} VideoFilterChain;

static void free_video_filter_chain(VideoFilterChain *chain) {
    avfilter_graph_free(&chain->graph);
    chain->buffersrc_ctx = NULL;
    chain->buffersink_ctx = NULL;
}

/* Builds `filter_descr` between a "buffer" source shaped like dec_ctx's decoded video and a
   plain "buffersink" — unlike init_audio_filter_chain, no encoder-format negotiation is
   needed here: `filter_descr` itself always ends in an explicit "format=yuv420p" (added by
   the caller), which pins the sink's output format directly instead of constraining the sink
   via options. */
static int init_video_filter_chain(AVCodecContext *dec_ctx, const char *filter_descr,
                                    VideoFilterChain *chain) {
    char args[512];
    const AVFilter *buffersrc = avfilter_get_by_name("buffer");
    const AVFilter *buffersink = avfilter_get_by_name("buffersink");
    AVFilterInOut *outputs = avfilter_inout_alloc();
    AVFilterInOut *inputs = avfilter_inout_alloc();
    AVRational sar = dec_ctx->sample_aspect_ratio;
    int ret;

    if (sar.num <= 0 || sar.den <= 0) {
        sar = (AVRational){1, 1};
    }

    chain->buffersrc_ctx = NULL;
    chain->buffersink_ctx = NULL;
    chain->graph = avfilter_graph_alloc();
    if (!outputs || !inputs || !chain->graph) {
        ret = AVERROR(ENOMEM);
        goto end;
    }

    snprintf(args, sizeof(args),
             "video_size=%dx%d:pix_fmt=%d:time_base=%d/%d:pixel_aspect=%d/%d", dec_ctx->width,
             dec_ctx->height, dec_ctx->pix_fmt, dec_ctx->pkt_timebase.num,
             dec_ctx->pkt_timebase.den, sar.num, sar.den);

    ret = avfilter_graph_create_filter(&chain->buffersrc_ctx, buffersrc, "in", args, NULL,
                                        chain->graph);
    if (ret < 0) goto end;

    ret = avfilter_graph_create_filter(&chain->buffersink_ctx, buffersink, "out", NULL, NULL,
                                        chain->graph);
    if (ret < 0) goto end;

    outputs->name = av_strdup("in");
    outputs->filter_ctx = chain->buffersrc_ctx;
    outputs->pad_idx = 0;
    outputs->next = NULL;

    inputs->name = av_strdup("out");
    inputs->filter_ctx = chain->buffersink_ctx;
    inputs->pad_idx = 0;
    inputs->next = NULL;

    ret = avfilter_graph_parse_ptr(chain->graph, filter_descr, &inputs, &outputs, NULL);
    if (ret < 0) goto end;

    ret = avfilter_graph_config(chain->graph, NULL);

end:
    avfilter_inout_free(&inputs);
    avfilter_inout_free(&outputs);
    if (ret < 0) {
        free_video_filter_chain(chain);
    }
    return ret;
}

/* Pushes one decoded video frame through the filter graph and encodes+writes every frame it
   produces in response (the canvas-conform stage's `fps` filter doesn't emit 1:1 with its
   input — it duplicates/drops frames to hit canvas_fps). Every emitted frame gets a fresh
   sequential pts in `*next_pts` (in venc_ctx's time_base, i.e. one canvas frame) instead of
   whatever pts the filter graph assigns — same idea as scale_video_frame's, extended across
   segment boundaries so concatenated segments produce continuous timestamps. Pass frame=NULL
   once, at the end, to flush the filter graph itself. */
static int filter_encode_write_video_frame(AVFormatContext *out_ctx, VideoFilterChain *chain,
                                            AVCodecContext *enc_ctx, AVStream *out_stream,
                                            AVFrame *frame, AVFrame *filt_frame,
                                            int64_t *next_pts, AVPacket *enc_pkt) {
    int ret = av_buffersrc_add_frame(chain->buffersrc_ctx, frame);
    if (ret < 0) return ret;

    while (1) {
        ret = av_buffersink_get_frame(chain->buffersink_ctx, filt_frame);
        if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF) {
            return 0;
        } else if (ret < 0) {
            return ret;
        }
        filt_frame->pts = (*next_pts)++;
        ret = encode_write_packet(out_ctx, enc_ctx, out_stream, filt_frame, enc_pkt);
        av_frame_unref(filt_frame);
        if (ret < 0) return ret;
    }
}

OcaEncodeStatus avbridge_encode_export(const char *in_path, const char *out_path,
                                            float target_lufs, OcaProgressCallback progress_cb,
                                            void *progress_user_data, const uint8_t *cancel) {
    AVFormatContext *in_ctx = NULL;
    AVFormatContext *out_ctx = NULL;
    AVCodecContext *dec_ctx = NULL;
    AVCodecContext *enc_ctx = NULL;
    int *stream_mapping = NULL;
    AudioFilterChain chain = {0};
    AVPacket *pkt = NULL;
    AVFrame *dec_frame = NULL;
    AVFrame *filt_frame = NULL;
    AVPacket *enc_pkt = NULL;
    OcaEncodeStatus status = OCA_ENCODE_OK;
    int audio_in_index = -1;
    int audio_out_index = -1;

    switch (oca_open_input(in_path, &in_ctx)) {
        case -1: return OCA_ENCODE_ERR_OPEN_INPUT;
        case -2: return OCA_ENCODE_ERR_STREAM_INFO;
    }

    for (unsigned int i = 0; i < in_ctx->nb_streams; i++) {
        if (in_ctx->streams[i]->codecpar->codec_type == AVMEDIA_TYPE_AUDIO) {
            audio_in_index = (int)i;
            break;
        }
    }
    if (audio_in_index < 0) {
        avformat_close_input(&in_ctx);
        return OCA_ENCODE_ERR_NO_AUDIO_STREAM;
    }

    avformat_alloc_output_context2(&out_ctx, NULL, NULL, out_path);
    if (!out_ctx) {
        status = OCA_ENCODE_ERR_ALLOC_OUTPUT;
        goto cleanup;
    }

    stream_mapping = av_calloc(in_ctx->nb_streams, sizeof(*stream_mapping));
    if (!stream_mapping) {
        status = OCA_ENCODE_ERR_ALLOC_OUTPUT;
        goto cleanup;
    }

    {
        const AVCodec *decoder =
            avcodec_find_decoder(in_ctx->streams[audio_in_index]->codecpar->codec_id);
        if (!decoder) {
            status = OCA_ENCODE_ERR_DECODER;
            goto cleanup;
        }
        dec_ctx = avcodec_alloc_context3(decoder);
        if (!dec_ctx || avcodec_parameters_to_context(
                            dec_ctx, in_ctx->streams[audio_in_index]->codecpar) < 0) {
            status = OCA_ENCODE_ERR_DECODER;
            goto cleanup;
        }
        dec_ctx->pkt_timebase = in_ctx->streams[audio_in_index]->time_base;
        if (avcodec_open2(dec_ctx, decoder, NULL) < 0) {
            status = OCA_ENCODE_ERR_DECODER;
            goto cleanup;
        }
    }

    const AVCodec *encoder = avcodec_find_encoder(AV_CODEC_ID_AAC);
    if (!encoder) {
        status = OCA_ENCODE_ERR_ENCODER;
        goto cleanup;
    }

    {
        char filter_descr[256];
        snprintf(filter_descr, sizeof(filter_descr),
                 "loudnorm=I=%.1f:TP=-1.0:LRA=11,alimiter=limit=0.95:attack=5:release=50",
                 (double)target_lufs);
        if (init_audio_filter_chain(dec_ctx, encoder, filter_descr, &chain) < 0) {
            status = OCA_ENCODE_ERR_FILTER_GRAPH;
            goto cleanup;
        }
    }

    enc_ctx = avcodec_alloc_context3(encoder);
    if (!enc_ctx) {
        status = OCA_ENCODE_ERR_ENCODER;
        goto cleanup;
    }
    enc_ctx->sample_rate = av_buffersink_get_sample_rate(chain.buffersink_ctx);
    av_buffersink_get_ch_layout(chain.buffersink_ctx, &enc_ctx->ch_layout);
    enc_ctx->sample_fmt = (enum AVSampleFormat)av_buffersink_get_format(chain.buffersink_ctx);
    enc_ctx->bit_rate = 192000;
    enc_ctx->time_base = av_buffersink_get_time_base(chain.buffersink_ctx);
    if (out_ctx->oformat->flags & AVFMT_GLOBALHEADER) {
        enc_ctx->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
    }
    if (avcodec_open2(enc_ctx, encoder, NULL) < 0) {
        status = OCA_ENCODE_ERR_ENCODER;
        goto cleanup;
    }
    if (enc_ctx->frame_size > 0) {
        /* AAC (and most audio codecs) need exactly frame_size samples per input frame; the
           filter chain doesn't chunk to that on its own — without this, avcodec_send_frame
           on the encoder fails with "nb_samples > frame_size" on anything but a lucky length. */
        av_buffersink_set_frame_size(chain.buffersink_ctx, (unsigned)enc_ctx->frame_size);
    }

    {
        int out_stream_count = 0;
        for (unsigned int i = 0; i < in_ctx->nb_streams; i++) {
            AVStream *in_stream = in_ctx->streams[i];
            enum AVMediaType type = in_stream->codecpar->codec_type;
            AVStream *out_stream;

            if ((int)i == audio_in_index) {
                out_stream = avformat_new_stream(out_ctx, NULL);
                if (!out_stream ||
                    avcodec_parameters_from_context(out_stream->codecpar, enc_ctx) < 0) {
                    status = OCA_ENCODE_ERR_NEW_STREAM;
                    goto cleanup;
                }
                out_stream->time_base = enc_ctx->time_base;
                stream_mapping[i] = out_stream_count++;
                audio_out_index = out_stream->index;
                continue;
            }

            if (type != AVMEDIA_TYPE_VIDEO && type != AVMEDIA_TYPE_AUDIO) {
                stream_mapping[i] = -1;
                continue;
            }

            out_stream = avformat_new_stream(out_ctx, NULL);
            if (!out_stream ||
                avcodec_parameters_copy(out_stream->codecpar, in_stream->codecpar) < 0) {
                status = OCA_ENCODE_ERR_NEW_STREAM;
                goto cleanup;
            }
            out_stream->codecpar->codec_tag = 0;
            out_stream->time_base = in_stream->time_base;
            stream_mapping[i] = out_stream_count++;
        }
    }

    if (!(out_ctx->oformat->flags & AVFMT_NOFILE)) {
        if (avio_open(&out_ctx->pb, out_path, AVIO_FLAG_WRITE) < 0) {
            status = OCA_ENCODE_ERR_OPEN_OUTPUT;
            goto cleanup_output_io;
        }
    }

    if (avformat_write_header(out_ctx, NULL) < 0) {
        status = OCA_ENCODE_ERR_WRITE_HEADER;
        goto cleanup_output_io;
    }

    pkt = av_packet_alloc();
    dec_frame = av_frame_alloc();
    filt_frame = av_frame_alloc();
    enc_pkt = av_packet_alloc();
    if (!pkt || !dec_frame || !filt_frame || !enc_pkt) {
        status = OCA_ENCODE_ERR_PIPELINE;
        goto cleanup_output_io;
    }

    while (av_read_frame(in_ctx, pkt) >= 0) {
        int mapped = stream_mapping[pkt->stream_index];
        if (mapped < 0) {
            av_packet_unref(pkt);
            continue;
        }

        if (cancel && *cancel) {
            av_packet_unref(pkt);
            status = OCA_ENCODE_CANCELLED;
            break;
        }
        if (progress_cb && pkt->pts != AV_NOPTS_VALUE) {
            progress_cb(progress_user_data,
                        pkt->pts * av_q2d(in_ctx->streams[pkt->stream_index]->time_base));
        }

        if (pkt->stream_index == audio_in_index) {
            AVStream *out_stream = out_ctx->streams[audio_out_index];
            int ret = avcodec_send_packet(dec_ctx, pkt);
            av_packet_unref(pkt);
            if (ret < 0) {
                status = OCA_ENCODE_ERR_PIPELINE;
                break;
            }
            while (1) {
                ret = avcodec_receive_frame(dec_ctx, dec_frame);
                if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF) {
                    break;
                } else if (ret < 0) {
                    status = OCA_ENCODE_ERR_PIPELINE;
                    break;
                }
                if (filter_encode_write_frame(out_ctx, &chain, enc_ctx, out_stream, dec_frame,
                                               filt_frame, enc_pkt) < 0) {
                    status = OCA_ENCODE_ERR_PIPELINE;
                    break;
                }
            }
            if (status != OCA_ENCODE_OK) break;
            continue;
        }

        /* Video (or other passthrough) packet — copy through unchanged. */
        {
            AVStream *in_stream = in_ctx->streams[pkt->stream_index];
            AVStream *out_stream = out_ctx->streams[mapped];
            pkt->stream_index = mapped;
            av_packet_rescale_ts(pkt, in_stream->time_base, out_stream->time_base);
            pkt->pos = -1;
            if (av_interleaved_write_frame(out_ctx, pkt) < 0) {
                status = OCA_ENCODE_ERR_WRITE_FRAME;
                break;
            }
        }
    }

    if (status == OCA_ENCODE_OK) {
        /* Flush: decoder -> filter -> encoder, in that order — each stage may be holding
           buffered frames the next stage hasn't seen yet. */
        AVStream *out_stream = out_ctx->streams[audio_out_index];
        avcodec_send_packet(dec_ctx, NULL);
        while (avcodec_receive_frame(dec_ctx, dec_frame) >= 0) {
            if (filter_encode_write_frame(out_ctx, &chain, enc_ctx, out_stream, dec_frame,
                                           filt_frame, enc_pkt) < 0) {
                status = OCA_ENCODE_ERR_PIPELINE;
                break;
            }
        }
        if (status == OCA_ENCODE_OK &&
            filter_encode_write_frame(out_ctx, &chain, enc_ctx, out_stream, NULL, filt_frame,
                                       enc_pkt) < 0) {
            status = OCA_ENCODE_ERR_PIPELINE;
        }
        if (status == OCA_ENCODE_OK &&
            encode_write_packet(out_ctx, enc_ctx, out_stream, NULL, enc_pkt) < 0) {
            status = OCA_ENCODE_ERR_PIPELINE;
        }
    }

    if (status == OCA_ENCODE_OK) {
        av_write_trailer(out_ctx);
    }

cleanup_output_io:
    av_packet_free(&pkt);
    av_packet_free(&enc_pkt);
    av_frame_free(&dec_frame);
    av_frame_free(&filt_frame);
    if (out_ctx && !(out_ctx->oformat->flags & AVFMT_NOFILE)) {
        avio_closep(&out_ctx->pb);
    }
cleanup:
    free_audio_filter_chain(&chain);
    avcodec_free_context(&dec_ctx);
    avcodec_free_context(&enc_ctx);
    av_freep(&stream_mapping);
    if (out_ctx) {
        avformat_free_context(out_ctx);
    }
    avformat_close_input(&in_ctx);
    return status;
}

OcaEncodeStatus avbridge_encode_timeline_export(
    const OcaClipSegment *segments, int segment_count, int canvas_width, int canvas_height,
    int canvas_fps_num, int canvas_fps_den, int64_t canvas_bit_rate_bps, const char *out_path,
    float target_lufs, OcaProgressCallback progress_cb, void *progress_user_data,
    const uint8_t *cancel) {
    if (segment_count <= 0) {
        return OCA_ENCODE_ERR_EMPTY_TIMELINE;
    }

    AVFormatContext *out_ctx = NULL;
    AVCodecContext *venc_ctx = NULL;
    AVCodecContext *aenc_ctx = NULL;
    AudioFilterChain achain = {0};
    AVStream *video_out_stream = NULL;
    AVStream *audio_out_stream = NULL;
    AVPacket *pkt = NULL;
    AVFrame *dec_frame = NULL;
    AVFrame *filt_frame = NULL;
    AVPacket *enc_pkt = NULL;
    OcaEncodeStatus status = OCA_ENCODE_OK;
    AVRational canvas_fps = {canvas_fps_num, canvas_fps_den};
    int64_t next_video_pts = 0;
    double elapsed_before_segment = 0.0;
    int canonical_sample_rate = 0;
    enum AVSampleFormat canonical_sample_fmt = AV_SAMPLE_FMT_NONE;
    AVChannelLayout canonical_ch_layout = {0};

    avformat_alloc_output_context2(&out_ctx, NULL, NULL, out_path);
    if (!out_ctx) {
        return OCA_ENCODE_ERR_ALLOC_OUTPUT;
    }

    /* Fixed video encoder for the whole timeline's canvas — libopenh264, same setup as
       avbridge_generate_proxy's, just at canvas_width/canvas_height/canvas_fps instead of a
       proxy's downscaled size. */
    {
        const AVCodec *venc = avcodec_find_encoder_by_name("libopenh264");
        if (!venc) {
            status = OCA_ENCODE_ERR_ENCODER;
            goto cleanup;
        }
        venc_ctx = avcodec_alloc_context3(venc);
        if (!venc_ctx) {
            status = OCA_ENCODE_ERR_ENCODER;
            goto cleanup;
        }
        venc_ctx->width = canvas_width;
        venc_ctx->height = canvas_height;
        venc_ctx->pix_fmt = AV_PIX_FMT_YUV420P;
        venc_ctx->time_base = av_inv_q(canvas_fps);
        venc_ctx->framerate = canvas_fps;
        venc_ctx->gop_size = (canvas_fps.num / canvas_fps.den) * 2;
        venc_ctx->max_b_frames = 0;
        venc_ctx->bit_rate = canvas_bit_rate_bps;
        if (out_ctx->oformat->flags & AVFMT_GLOBALHEADER) {
            venc_ctx->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
        }
        if (avcodec_open2(venc_ctx, venc, NULL) < 0) {
            status = OCA_ENCODE_ERR_ENCODER;
            goto cleanup;
        }
        video_out_stream = avformat_new_stream(out_ctx, NULL);
        if (!video_out_stream ||
            avcodec_parameters_from_context(video_out_stream->codecpar, venc_ctx) < 0) {
            status = OCA_ENCODE_ERR_NEW_STREAM;
            goto cleanup;
        }
        video_out_stream->time_base = venc_ctx->time_base;
    }

    pkt = av_packet_alloc();
    dec_frame = av_frame_alloc();
    filt_frame = av_frame_alloc();
    enc_pkt = av_packet_alloc();
    if (!pkt || !dec_frame || !filt_frame || !enc_pkt) {
        status = OCA_ENCODE_ERR_PIPELINE;
        goto cleanup;
    }

    for (int seg_i = 0; seg_i < segment_count && status == OCA_ENCODE_OK; seg_i++) {
        const OcaClipSegment *seg = &segments[seg_i];
        AVFormatContext *in_ctx = NULL;
        AVCodecContext *vdec_ctx = NULL;
        AVCodecContext *adec_ctx = NULL;
        VideoFilterChain vchain = {0};
        int video_in_index = -1;
        int audio_in_index = -1;

        switch (oca_open_input(seg->source_path, &in_ctx)) {
            case -1: status = OCA_ENCODE_ERR_OPEN_INPUT; break;
            case -2: status = OCA_ENCODE_ERR_STREAM_INFO; break;
        }
        if (status != OCA_ENCODE_OK) break;
        for (unsigned int s = 0; s < in_ctx->nb_streams; s++) {
            enum AVMediaType type = in_ctx->streams[s]->codecpar->codec_type;
            if (video_in_index < 0 && type == AVMEDIA_TYPE_VIDEO) {
                video_in_index = (int)s;
            } else if (audio_in_index < 0 && type == AVMEDIA_TYPE_AUDIO) {
                audio_in_index = (int)s;
            }
        }
        if (video_in_index < 0) {
            avformat_close_input(&in_ctx);
            status = OCA_ENCODE_ERR_NO_VIDEO_STREAM;
            break;
        }
        if (audio_in_index < 0) {
            avformat_close_input(&in_ctx);
            status = OCA_ENCODE_ERR_NO_AUDIO_STREAM;
            break;
        }

        /* Video decoder for this segment. */
        {
            AVCodecParameters *vpar = in_ctx->streams[video_in_index]->codecpar;
            const AVCodec *vdecoder = avcodec_find_decoder(vpar->codec_id);
            if (!vdecoder) {
                status = OCA_ENCODE_ERR_DECODER;
                goto segment_cleanup;
            }
            vdec_ctx = avcodec_alloc_context3(vdecoder);
            if (!vdec_ctx || avcodec_parameters_to_context(vdec_ctx, vpar) < 0) {
                status = OCA_ENCODE_ERR_DECODER;
                goto segment_cleanup;
            }
            vdec_ctx->pkt_timebase = in_ctx->streams[video_in_index]->time_base;
            if (avcodec_open2(vdec_ctx, vdecoder, NULL) < 0) {
                status = OCA_ENCODE_ERR_DECODER;
                goto segment_cleanup;
            }
        }

        /* Audio decoder for this segment. */
        {
            AVCodecParameters *apar = in_ctx->streams[audio_in_index]->codecpar;
            const AVCodec *adecoder = avcodec_find_decoder(apar->codec_id);
            if (!adecoder) {
                status = OCA_ENCODE_ERR_DECODER;
                goto segment_cleanup;
            }
            adec_ctx = avcodec_alloc_context3(adecoder);
            if (!adec_ctx || avcodec_parameters_to_context(adec_ctx, apar) < 0) {
                status = OCA_ENCODE_ERR_DECODER;
                goto segment_cleanup;
            }
            adec_ctx->pkt_timebase = in_ctx->streams[audio_in_index]->time_base;
            if (avcodec_open2(adec_ctx, adecoder, NULL) < 0) {
                status = OCA_ENCODE_ERR_DECODER;
                goto segment_cleanup;
            }
        }

        if (seg_i == 0) {
            /* First segment sets up the ONE audio filter graph + AAC encoder reused for the
               rest of the timeline — loudnorm needs to see the whole concatenated stream, not
               each clip normalized in isolation, so this graph is never rebuilt per segment
               (unlike the video filter chain, which is). Everything downstream requires every
               later segment's decoded audio to match this format exactly. */
            canonical_sample_rate = adec_ctx->sample_rate;
            canonical_sample_fmt = adec_ctx->sample_fmt;
            av_channel_layout_copy(&canonical_ch_layout, &adec_ctx->ch_layout);

            const AVCodec *aencoder = avcodec_find_encoder(AV_CODEC_ID_AAC);
            if (!aencoder) {
                status = OCA_ENCODE_ERR_ENCODER;
                goto segment_cleanup;
            }

            char afilter_descr[256];
            /* "vol" is a filter-instance name we target later via
               avfilter_graph_send_command() to change each segment's gain without rebuilding
               this graph — the value here is just segment 0's own gain, applied the same way
               right after setup below. */
            snprintf(afilter_descr, sizeof(afilter_descr),
                     "atempo@tempo=1.0,volume@vol=0dB,loudnorm=I=%.1f:TP=-1.0:LRA=11,alimiter="
                     "limit=0.95:attack=5:release=50",
                     (double)target_lufs);
            if (init_audio_filter_chain(adec_ctx, aencoder, afilter_descr, &achain) < 0) {
                status = OCA_ENCODE_ERR_FILTER_GRAPH;
                goto segment_cleanup;
            }

            aenc_ctx = avcodec_alloc_context3(aencoder);
            if (!aenc_ctx) {
                status = OCA_ENCODE_ERR_ENCODER;
                goto segment_cleanup;
            }
            aenc_ctx->sample_rate = av_buffersink_get_sample_rate(achain.buffersink_ctx);
            av_buffersink_get_ch_layout(achain.buffersink_ctx, &aenc_ctx->ch_layout);
            aenc_ctx->sample_fmt =
                (enum AVSampleFormat)av_buffersink_get_format(achain.buffersink_ctx);
            aenc_ctx->bit_rate = 192000;
            aenc_ctx->time_base = av_buffersink_get_time_base(achain.buffersink_ctx);
            if (out_ctx->oformat->flags & AVFMT_GLOBALHEADER) {
                aenc_ctx->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
            }
            if (avcodec_open2(aenc_ctx, aencoder, NULL) < 0) {
                status = OCA_ENCODE_ERR_ENCODER;
                goto segment_cleanup;
            }
            if (aenc_ctx->frame_size > 0) {
                av_buffersink_set_frame_size(achain.buffersink_ctx, (unsigned)aenc_ctx->frame_size);
            }

            audio_out_stream = avformat_new_stream(out_ctx, NULL);
            if (!audio_out_stream ||
                avcodec_parameters_from_context(audio_out_stream->codecpar, aenc_ctx) < 0) {
                status = OCA_ENCODE_ERR_NEW_STREAM;
                goto segment_cleanup;
            }
            audio_out_stream->time_base = aenc_ctx->time_base;

            if (!(out_ctx->oformat->flags & AVFMT_NOFILE)) {
                if (avio_open(&out_ctx->pb, out_path, AVIO_FLAG_WRITE) < 0) {
                    status = OCA_ENCODE_ERR_OPEN_OUTPUT;
                    goto segment_cleanup;
                }
            }
            if (avformat_write_header(out_ctx, NULL) < 0) {
                status = OCA_ENCODE_ERR_WRITE_HEADER;
                goto segment_cleanup;
            }
        } else if (adec_ctx->sample_rate != canonical_sample_rate ||
                   adec_ctx->sample_fmt != canonical_sample_fmt ||
                   av_channel_layout_compare(&adec_ctx->ch_layout, &canonical_ch_layout) != 0) {
            status = OCA_ENCODE_ERR_AUDIO_FORMAT_MISMATCH;
            goto segment_cleanup;
        }

        {
            char gain_str[32];
            snprintf(gain_str, sizeof(gain_str), "%.4fdB", (double)seg->gain_db);
            avfilter_graph_send_command(achain.graph, "vol", "volume", gain_str, NULL, 0, 0);

            float spd = seg->speed_factor > 0.0f ? seg->speed_factor : 1.0f;
            if (spd < 0.5f) spd = 0.5f;
            if (spd > 100.0f) spd = 100.0f;
            char tempo_str[32];
            snprintf(tempo_str, sizeof(tempo_str), "%.6f", (double)spd);
            avfilter_graph_send_command(achain.graph, "tempo", "tempo", tempo_str, NULL, 0, 0);
        }

        /* Per-segment video filter chain: canvas-conform (scale/pad/fps, so every segment
           lands on the same output dimensions/frame rate) + this clip's own effect filters +
           optional entry transition + a final format lock so the encoder always receives
           yuv420p. Rebuilt every segment since clip filters and transitions differ.
           The n counter in avfilter resets to 0 at each segment's graph instantiation, which
           is what drives per-frame transition animation. */
        {
            char vfilter_descr[3072];
            const char *clip_filter =
                (seg->video_filter && seg->video_filter[0]) ? seg->video_filter : "";
            char setpts_str[48] = "";
            if (fabsf(seg->speed_factor - 1.0f) > 1e-4f && seg->speed_factor > 0.0f) {
                snprintf(setpts_str, sizeof(setpts_str), "setpts=PTS/%.6f,",
                         (double)seg->speed_factor);
            }
            char zoom_str[512] = "";
            float zs = seg->zoom_start > 0.0f ? seg->zoom_start : 1.0f;
            float ze = seg->zoom_end > 0.0f ? seg->zoom_end : 1.0f;
            if (zs < 0.1f) zs = 0.1f;  if (zs > 20.0f) zs = 20.0f;
            if (ze < 0.1f) ze = 0.1f;  if (ze > 20.0f) ze = 20.0f;
            if (fabsf(zs - 1.0f) > 1e-4f || fabsf(ze - 1.0f) > 1e-4f) {
                double source_dur = seg->source_out_secs - seg->source_in_secs;
                double speed = seg->speed_factor > 0.0f ? seg->speed_factor : 1.0f;
                double timeline_dur = source_dur / speed;
                double total_frames =
                    timeline_dur * (double)canvas_fps.num / (double)canvas_fps.den;
                if (total_frames < 1.0) total_frames = 1.0;
                double N = total_frames - 1.0;
                if (N < 1.0) N = 1.0;
                double A = zs;
                double B = ((double)ze - (double)zs) / N;
                if (fabs(B) < 1e-9) {
                    snprintf(zoom_str, sizeof(zoom_str),
                             "crop=iw/%.5f:ih/%.5f:iw*(1-1/%.5f)/2:ih*(1-1/%.5f)/2"
                             ",scale=iw*%.5f:ih*%.5f",
                             A, A, A, A, A, A);
                } else {
                    snprintf(zoom_str, sizeof(zoom_str),
                             "crop=iw/(%.7f+%.9f*n):ih/(%.7f+%.9f*n)"
                             ":iw*(1-1/(%.7f+%.9f*n))/2:ih*(1-1/(%.7f+%.9f*n))/2"
                             ",scale=iw*(%.7f+%.9f*n):ih*(%.7f+%.9f*n)",
                             A, B, A, B, A, B, A, B, A, B, A, B);
                }
            }

            /* Build the transition filter string for this segment's entry effect.
               All expressions use arithmetic instead of min()/max()/ite() function calls to
               avoid commas inside option values (which avfilter would misparse as filter
               separators). The pattern (n<TF)*expr_a + (n>=TF)*expr_b evaluates to expr_a
               when n < TF and expr_b when n >= TF, since comparison operators return 0 or 1.
               n resets to 0 at the start of each segment's filter graph, so it counts frames
               from this clip's first frame. */
            char transition_str[384] = "";
            if (seg->transition_in != 0) {
                double tf = (double)seg->transition_duration_secs
                            * (double)canvas_fps.num / (double)canvas_fps.den;
                if (tf < 1.0) tf = 1.0;
                switch (seg->transition_in) {
                    case 1: /* Fade: fade in from black over transition_duration_secs. */
                        snprintf(transition_str, sizeof(transition_str),
                                 "fade=t=in:st=0:d=%.4f",
                                 (double)seg->transition_duration_secs);
                        break;
                    case 2:
                        /* Slide: reveal the clip from left to right using an animated drawbox
                           that covers the frame with black and retreats rightward each frame.
                           x = n*iw/tf when n < tf (box moves right, revealing clip from left);
                           x = iw when n >= tf (box fully off-screen, full clip visible).
                           Arithmetic: (n<tf)*n*iw/tf + (n>=tf)*iw — no commas in expression. */
                        snprintf(transition_str, sizeof(transition_str),
                                 "drawbox=x='(n<%g)*n*iw/%g+(n>=%g)*iw'"
                                 ":y=0:w=iw:h=ih:color=black@1:t=fill",
                                 tf, tf, tf);
                        break;
                    case 3:
                        /* Zoom: scale from 50%% to 100%% of canvas size over transition_duration_secs,
                           then pad back to the canvas dimensions with black borders.
                           Scale factor = 0.5 + 0.5*(n<tf)*n/tf + 0.5*(n>=tf), which is 0.5 at
                           n=0 and 1.0 at n>=tf. eval=frame is required so scale re-evaluates
                           the expression for each output frame. After scaling, iw/ih in the pad
                           expression are the scaled (smaller) dimensions — (W-iw)/2 centres them. */
                        snprintf(transition_str, sizeof(transition_str),
                                 "scale=iw*(0.5+0.5*(n<%g)*n/%g+0.5*(n>=%g))"
                                 ":ih*(0.5+0.5*(n<%g)*n/%g+0.5*(n>=%g))"
                                 ":eval=frame"
                                 ",pad=%d:%d:(%d-iw)/2:(%d-ih)/2:black",
                                 tf, tf, tf, tf, tf, tf,
                                 canvas_width, canvas_height,
                                 canvas_width, canvas_height);
                        break;
                    default:
                        break;
                }
            }

            /* Build the post-fps portion: zoom, clip_filter, and transition, all optional,
               separated by commas only where both neighbours are non-empty. */
            char post_fps[1600] = "";
            if (zoom_str[0] && clip_filter[0]) {
                snprintf(post_fps, sizeof(post_fps), "%s,%s", zoom_str, clip_filter);
            } else if (zoom_str[0]) {
                snprintf(post_fps, sizeof(post_fps), "%s", zoom_str);
            } else if (clip_filter[0]) {
                snprintf(post_fps, sizeof(post_fps), "%s", clip_filter);
            }
            char final_chain[2048] = "";
            if (post_fps[0] && transition_str[0]) {
                snprintf(final_chain, sizeof(final_chain), "%s,%s", post_fps, transition_str);
            } else if (post_fps[0]) {
                snprintf(final_chain, sizeof(final_chain), "%s", post_fps);
            } else if (transition_str[0]) {
                snprintf(final_chain, sizeof(final_chain), "%s", transition_str);
            }
            snprintf(vfilter_descr, sizeof(vfilter_descr),
                     "%sscale=%d:%d:force_original_aspect_ratio=decrease,pad=%d:%d:(ow-iw)/2:(oh-"
                     "ih)/2,fps=%d/%d%s%s,format=yuv420p",
                     setpts_str, canvas_width, canvas_height, canvas_width, canvas_height,
                     canvas_fps.num, canvas_fps.den,
                     final_chain[0] ? "," : "", final_chain);
            if (init_video_filter_chain(vdec_ctx, vfilter_descr, &vchain) < 0) {
                status = OCA_ENCODE_ERR_FILTER_GRAPH;
                goto segment_cleanup;
            }
        }

        /* Seek to source_in_secs (container-wide — av_seek_frame with stream_index=-1 seeks
           every stream approximately together). A keyframe seek commonly lands earlier than
           requested, so frames/samples are still discarded by timestamp below until each
           stream's own decoded time actually reaches source_in_secs. */
        if (seg->source_in_secs > 0.0) {
            av_seek_frame(in_ctx, -1, (int64_t)(seg->source_in_secs * AV_TIME_BASE),
                          AVSEEK_FLAG_BACKWARD);
            avcodec_flush_buffers(vdec_ctx);
            avcodec_flush_buffers(adec_ctx);
        }

        {
            int video_done = 0;
            int audio_done = 0;
            /* Set once the frozen segment's held anchor frame has been synthesized into the
               full hold duration below — later real decoded frames within range are then just
               discarded instead of being pushed through the filter chain again. */
            int frozen_anchor_done = 0;
            while (!video_done || !audio_done) {
                if (cancel && *cancel) {
                    status = OCA_ENCODE_CANCELLED;
                    break;
                }
                if (av_read_frame(in_ctx, pkt) < 0) {
                    break;
                }

                if (pkt->stream_index == video_in_index && !video_done) {
                    int ret = avcodec_send_packet(vdec_ctx, pkt);
                    av_packet_unref(pkt);
                    if (ret < 0) {
                        status = OCA_ENCODE_ERR_PIPELINE;
                        break;
                    }
                    while (1) {
                        ret = avcodec_receive_frame(vdec_ctx, dec_frame);
                        if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF) break;
                        if (ret < 0) {
                            status = OCA_ENCODE_ERR_PIPELINE;
                            break;
                        }
                        double frame_secs = dec_frame->pts * av_q2d(vdec_ctx->pkt_timebase);
                        if (frame_secs < seg->source_in_secs) {
                            av_frame_unref(dec_frame);
                            continue;
                        }
                        if (frame_secs >= seg->source_out_secs) {
                            video_done = 1;
                            av_frame_unref(dec_frame);
                            continue;
                        }

                        if (seg->frozen) {
                            /* Already emitted the held frame for this segment — every later
                               real decoded frame in range is discarded, not re-pushed. */
                            if (frozen_anchor_done) {
                                av_frame_unref(dec_frame);
                                continue;
                            }
                            /* Hold this one anchor frame for the segment's whole trimmed
                               duration by synthesizing duplicate pushes spaced at canvas_fps —
                               same "one decoded frame in, one filtered frame out" contract
                               filter_encode_write_video_frame already relies on elsewhere in
                               this loop, just fed the same content repeatedly instead of newly
                               decoded frames. Driving the duplicate count off canvas_fps (not
                               the source's own frame rate) means the canvas-conform `fps=`
                               stage sees input already arriving near its target rate, so it
                               passes each duplicate through ~1:1 instead of needing to
                               extrapolate — sidesteps relying on this filter graph's EOF/flush
                               semantics, which nothing in this per-segment loop ever triggers
                               (the graph is freed, not flushed, at segment_cleanup). */
                            double hold_secs = seg->source_out_secs - seg->source_in_secs;
                            double canvas_fps_d = av_q2d(canvas_fps);
                            long long hold_frames = llround(hold_secs * canvas_fps_d);
                            if (hold_frames < 1) {
                                hold_frames = 1;
                            }
                            for (long long i = 0; i < hold_frames; i++) {
                                if (cancel && *cancel) {
                                    status = OCA_ENCODE_CANCELLED;
                                    break;
                                }
                                double target_secs =
                                    seg->source_in_secs + (double)i / canvas_fps_d;
                                AVFrame *held_frame = av_frame_clone(dec_frame);
                                if (!held_frame) {
                                    status = OCA_ENCODE_ERR_PIPELINE;
                                    break;
                                }
                                held_frame->pts =
                                    (int64_t)llround(target_secs / av_q2d(vdec_ctx->pkt_timebase));
                                int fret = filter_encode_write_video_frame(
                                    out_ctx, &vchain, venc_ctx, video_out_stream, held_frame,
                                    filt_frame, &next_video_pts, enc_pkt);
                                av_frame_free(&held_frame);
                                if (fret < 0) {
                                    status = OCA_ENCODE_ERR_PIPELINE;
                                    break;
                                }
                                if (progress_cb) {
                                    progress_cb(progress_user_data,
                                                elapsed_before_segment +
                                                    (target_secs - seg->source_in_secs));
                                }
                            }
                            frozen_anchor_done = 1;
                            video_done = 1;
                            av_frame_unref(dec_frame);
                            continue;
                        }

                        if (filter_encode_write_video_frame(out_ctx, &vchain, venc_ctx,
                                                             video_out_stream, dec_frame,
                                                             filt_frame, &next_video_pts,
                                                             enc_pkt) < 0) {
                            status = OCA_ENCODE_ERR_PIPELINE;
                            break;
                        }
                        av_frame_unref(dec_frame);
                        if (progress_cb) {
                            progress_cb(progress_user_data,
                                        elapsed_before_segment + (frame_secs - seg->source_in_secs));
                        }
                    }
                } else if (pkt->stream_index == audio_in_index && !audio_done) {
                    int ret = avcodec_send_packet(adec_ctx, pkt);
                    av_packet_unref(pkt);
                    if (ret < 0) {
                        status = OCA_ENCODE_ERR_PIPELINE;
                        break;
                    }
                    while (1) {
                        ret = avcodec_receive_frame(adec_ctx, dec_frame);
                        if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF) break;
                        if (ret < 0) {
                            status = OCA_ENCODE_ERR_PIPELINE;
                            break;
                        }
                        double frame_secs = dec_frame->pts * av_q2d(adec_ctx->pkt_timebase);
                        if (frame_secs < seg->source_in_secs) {
                            av_frame_unref(dec_frame);
                            continue;
                        }
                        if (frame_secs >= seg->source_out_secs) {
                            audio_done = 1;
                            av_frame_unref(dec_frame);
                            continue;
                        }
                        if (filter_encode_write_frame(out_ctx, &achain, aenc_ctx,
                                                       audio_out_stream, dec_frame, filt_frame,
                                                       enc_pkt) < 0) {
                            status = OCA_ENCODE_ERR_PIPELINE;
                            break;
                        }
                        av_frame_unref(dec_frame);
                    }
                } else {
                    av_packet_unref(pkt);
                }

                if (status != OCA_ENCODE_OK) break;
            }
        }

    segment_cleanup:
        free_video_filter_chain(&vchain);
        avcodec_free_context(&vdec_ctx);
        avcodec_free_context(&adec_ctx);
        avformat_close_input(&in_ctx);
        elapsed_before_segment += seg->source_out_secs - seg->source_in_secs;
    }

    if (status == OCA_ENCODE_OK) {
        /* Flush: decoder(s) already drained per-segment above; only the shared audio filter
           graph and both encoders may still be holding buffered frames. */
        if (filter_encode_write_frame(out_ctx, &achain, aenc_ctx, audio_out_stream, NULL,
                                       filt_frame, enc_pkt) < 0 ||
            encode_write_packet(out_ctx, aenc_ctx, audio_out_stream, NULL, enc_pkt) < 0) {
            status = OCA_ENCODE_ERR_PIPELINE;
        }
    }
    if (status == OCA_ENCODE_OK &&
        encode_write_packet(out_ctx, venc_ctx, video_out_stream, NULL, enc_pkt) < 0) {
        status = OCA_ENCODE_ERR_PIPELINE;
    }

    if (status == OCA_ENCODE_OK) {
        av_write_trailer(out_ctx);
    }

cleanup:
    av_packet_free(&pkt);
    av_packet_free(&enc_pkt);
    av_frame_free(&dec_frame);
    av_frame_free(&filt_frame);
    if (out_ctx && out_ctx->pb && !(out_ctx->oformat->flags & AVFMT_NOFILE)) {
        avio_closep(&out_ctx->pb);
    }
    free_audio_filter_chain(&achain);
    av_channel_layout_uninit(&canonical_ch_layout);
    avcodec_free_context(&venc_ctx);
    avcodec_free_context(&aenc_ctx);
    if (out_ctx) {
        avformat_free_context(out_ctx);
    }
    return status;
}

/* =========================================================================
   avbridge_encode_timeline_export_multi — multi-track overlay compositor.

   Track 0 drives the output: its segments play in order (same as the single-track
   function), providing both audio and the background video.  Tracks 1..n_tracks-1
   are overlaid on top wherever their clips' [timeline_start_secs, timeline_end)
   windows overlap with the current track-0 frame's timeline position.

   When a frame from track 0 is at timeline time T:
   - If track 1 has an active segment at T: open/seek its decoder (once per segment
     boundary), decode a frame, push both frames through a 2-input overlay avfilter
     graph (built once per mode-switch), pull the composited result, encode.
   - Otherwise: fall back to a plain VideoFilterChain for track 0 (same as the
     existing single-track function).

   Audio comes from track 0 only.  Only the first two tracks are composited in this
   implementation; additional tracks beyond index 1 are silently ignored.
   ========================================================================= */

/* Build the complete single-track video filter string for `seg` — identical logic to
   the inline filter-string block inside avbridge_encode_timeline_export, extracted here
   so the multi-track function can reuse it for single-track fallback intervals. */
static void oca_build_vfilter_descr(const OcaClipSegment *seg,
                                     int cw, int ch, int fps_num, int fps_den,
                                     char *buf, size_t cap) {
    AVRational canvas_fps = {fps_num, fps_den};
    const char *cf = (seg->video_filter && seg->video_filter[0]) ? seg->video_filter : "";
    char setpts[48] = "";
    if (fabsf(seg->speed_factor - 1.0f) > 1e-4f && seg->speed_factor > 0.0f)
        snprintf(setpts, sizeof(setpts), "setpts=PTS/%.6f,", (double)seg->speed_factor);

    char zoom[512] = "";
    float zs = seg->zoom_start > 0.0f ? seg->zoom_start : 1.0f;
    float ze = seg->zoom_end > 0.0f ? seg->zoom_end : 1.0f;
    if (zs < 0.1f) zs = 0.1f; if (zs > 20.0f) zs = 20.0f;
    if (ze < 0.1f) ze = 0.1f; if (ze > 20.0f) ze = 20.0f;
    if (fabsf(zs - 1.0f) > 1e-4f || fabsf(ze - 1.0f) > 1e-4f) {
        double sd = seg->source_out_secs - seg->source_in_secs;
        double sp = seg->speed_factor > 0.0f ? seg->speed_factor : 1.0f;
        double tf = (sd / sp) * (double)canvas_fps.num / (double)canvas_fps.den;
        if (tf < 1.0) tf = 1.0;
        double N = tf - 1.0; if (N < 1.0) N = 1.0;
        double A = zs, B = ((double)ze - (double)zs) / N;
        if (fabs(B) < 1e-9)
            snprintf(zoom, sizeof(zoom),
                     "crop=iw/%.5f:ih/%.5f:iw*(1-1/%.5f)/2:ih*(1-1/%.5f)/2,scale=iw*%.5f:ih*%.5f",
                     A, A, A, A, A, A);
        else
            snprintf(zoom, sizeof(zoom),
                     "crop=iw/(%.7f+%.9f*n):ih/(%.7f+%.9f*n)"
                     ":iw*(1-1/(%.7f+%.9f*n))/2:ih*(1-1/(%.7f+%.9f*n))/2"
                     ",scale=iw*(%.7f+%.9f*n):ih*(%.7f+%.9f*n)",
                     A, B, A, B, A, B, A, B, A, B, A, B);
    }

    char trans[384] = "";
    if (seg->transition_in != 0) {
        double tf = (double)seg->transition_duration_secs * (double)fps_num / (double)fps_den;
        if (tf < 1.0) tf = 1.0;
        switch (seg->transition_in) {
            case 1: snprintf(trans, sizeof(trans), "fade=t=in:st=0:d=%.4f",
                             (double)seg->transition_duration_secs); break;
            case 2: snprintf(trans, sizeof(trans),
                             "drawbox=x='(n<%g)*n*iw/%g+(n>=%g)*iw':y=0:w=iw:h=ih:color=black@1:t=fill",
                             tf, tf, tf); break;
            case 3: snprintf(trans, sizeof(trans),
                             "scale=iw*(0.5+0.5*(n<%g)*n/%g+0.5*(n>=%g))"
                             ":ih*(0.5+0.5*(n<%g)*n/%g+0.5*(n>=%g)):eval=frame"
                             ",pad=%d:%d:(%d-iw)/2:(%d-ih)/2:black",
                             tf, tf, tf, tf, tf, tf, cw, ch, cw, ch); break;
            default: break;
        }
    }

    char post[1600] = "";
    if (zoom[0] && cf[0])   snprintf(post, sizeof(post), "%s,%s", zoom, cf);
    else if (zoom[0])        snprintf(post, sizeof(post), "%s", zoom);
    else if (cf[0])          snprintf(post, sizeof(post), "%s", cf);
    char chain[2048] = "";
    if (post[0] && trans[0]) snprintf(chain, sizeof(chain), "%s,%s", post, trans);
    else if (post[0])         snprintf(chain, sizeof(chain), "%s", post);
    else if (trans[0])        snprintf(chain, sizeof(chain), "%s", trans);

    snprintf(buf, cap,
             "%sscale=%d:%d:force_original_aspect_ratio=decrease,pad=%d:%d:(ow-iw)/2:(oh-ih)/2,"
             "fps=%d/%d%s%s,format=yuv420p",
             setpts, cw, ch, cw, ch, fps_num, fps_den, chain[0] ? "," : "", chain);
}

/* Build the per-track filter string for use INSIDE an overlay graph — same as
   oca_build_vfilter_descr but omits the trailing format=yuv420p (added after the
   overlay stage) and omits transitions (n counter semantics differ in multi-input
   graphs). */
static void oca_build_overlay_vfilter(const OcaClipSegment *seg,
                                       int cw, int ch, int fps_num, int fps_den,
                                       char *buf, size_t cap) {
    const char *cf = (seg->video_filter && seg->video_filter[0]) ? seg->video_filter : "";
    char setpts[48] = "";
    if (fabsf(seg->speed_factor - 1.0f) > 1e-4f && seg->speed_factor > 0.0f)
        snprintf(setpts, sizeof(setpts), "setpts=PTS/%.6f,", (double)seg->speed_factor);

    char zoom[512] = "";
    float zs = seg->zoom_start > 0.0f ? seg->zoom_start : 1.0f;
    float ze = seg->zoom_end > 0.0f ? seg->zoom_end : 1.0f;
    if (zs < 0.1f) zs = 0.1f; if (zs > 20.0f) zs = 20.0f;
    if (ze < 0.1f) ze = 0.1f; if (ze > 20.0f) ze = 20.0f;
    if (fabsf(zs - 1.0f) > 1e-4f || fabsf(ze - 1.0f) > 1e-4f) {
        AVRational cfps = {fps_num, fps_den};
        double sd = seg->source_out_secs - seg->source_in_secs;
        double sp = seg->speed_factor > 0.0f ? seg->speed_factor : 1.0f;
        double tf = (sd / sp) * (double)cfps.num / (double)cfps.den;
        if (tf < 1.0) tf = 1.0;
        double N = tf - 1.0; if (N < 1.0) N = 1.0;
        double A = zs, B = ((double)ze - (double)zs) / N;
        if (fabs(B) < 1e-9)
            snprintf(zoom, sizeof(zoom),
                     "crop=iw/%.5f:ih/%.5f:iw*(1-1/%.5f)/2:ih*(1-1/%.5f)/2,scale=iw*%.5f:ih*%.5f",
                     A, A, A, A, A, A);
        else
            snprintf(zoom, sizeof(zoom),
                     "crop=iw/(%.7f+%.9f*n):ih/(%.7f+%.9f*n)"
                     ":iw*(1-1/(%.7f+%.9f*n))/2:ih*(1-1/(%.7f+%.9f*n))/2"
                     ",scale=iw*(%.7f+%.9f*n):ih*(%.7f+%.9f*n)",
                     A, B, A, B, A, B, A, B, A, B, A, B);
    }

    char post[2048] = "";
    if (zoom[0] && cf[0])   snprintf(post, sizeof(post), ",%s,%s", zoom, cf);
    else if (zoom[0])        snprintf(post, sizeof(post), ",%s", zoom);
    else if (cf[0])          snprintf(post, sizeof(post), ",%s", cf);

    snprintf(buf, cap,
             "%sscale=%d:%d:force_original_aspect_ratio=decrease,pad=%d:%d:(ow-iw)/2:(oh-ih)/2,"
             "fps=%d/%d%s",
             setpts, cw, ch, cw, ch, fps_num, fps_den, post);
}

typedef struct {
    AVFormatContext *in_ctx;
    AVCodecContext  *vdec_ctx;
    int              video_in_index;
    int              seg_open;   /* index of the segment this decoder was opened for, or -1 */
    int              done;       /* av_read_frame hit EOF / source_out_secs reached */
    AVFrame         *pending;    /* last decoded frame still within range, or NULL */
} OcaOverlayDecoder;

static void oca_free_overlay_decoder(OcaOverlayDecoder *d) {
    if (d->pending) { av_frame_free(&d->pending); }
    avcodec_free_context(&d->vdec_ctx);
    avformat_close_input(&d->in_ctx);
    d->video_in_index = -1;
    d->seg_open = -1;
    d->done = 0;
}

/* Open the overlay decoder for `seg`, seeking to `src_seek`. Returns 0 on success. */
static int oca_open_overlay_decoder(const OcaClipSegment *seg, double src_seek,
                                     OcaOverlayDecoder *d, int seg_idx) {
    oca_free_overlay_decoder(d);
    switch (oca_open_input(seg->source_path, &d->in_ctx)) {
        case -1: return -1;
        case -2: return -2;
    }
    for (unsigned s = 0; s < d->in_ctx->nb_streams; s++) {
        if (d->in_ctx->streams[s]->codecpar->codec_type == AVMEDIA_TYPE_VIDEO) {
            d->video_in_index = (int)s; break;
        }
    }
    if (d->video_in_index < 0) { avformat_close_input(&d->in_ctx); return -3; }
    AVCodecParameters *vpar = d->in_ctx->streams[d->video_in_index]->codecpar;
    const AVCodec *vd = avcodec_find_decoder(vpar->codec_id);
    if (!vd) { avformat_close_input(&d->in_ctx); return -4; }
    d->vdec_ctx = avcodec_alloc_context3(vd);
    if (!d->vdec_ctx || avcodec_parameters_to_context(d->vdec_ctx, vpar) < 0) {
        avformat_close_input(&d->in_ctx); return -5;
    }
    d->vdec_ctx->pkt_timebase = d->in_ctx->streams[d->video_in_index]->time_base;
    if (avcodec_open2(d->vdec_ctx, vd, NULL) < 0) {
        avcodec_free_context(&d->vdec_ctx); avformat_close_input(&d->in_ctx); return -6;
    }
    if (src_seek > 0.0) {
        av_seek_frame(d->in_ctx, -1, (int64_t)(src_seek * AV_TIME_BASE), AVSEEK_FLAG_BACKWARD);
        avcodec_flush_buffers(d->vdec_ctx);
    }
    d->seg_open = seg_idx;
    d->done = 0;
    return 0;
}

/* Advance `d` until it has a frame at or past `src_target`, within [seg->source_in_secs,
   seg->source_out_secs).  Stores the frame in d->pending (caller must NOT free it —
   oca_free_overlay_decoder handles lifetime).  Returns 1 if a frame is ready, 0 if done. */
static int oca_advance_overlay_decoder(OcaOverlayDecoder *d, const OcaClipSegment *seg,
                                        double src_target, AVPacket *tmp_pkt, AVFrame *tmp_frame) {
    if (d->done) return (d->pending != NULL);
    while (1) {
        if (av_read_frame(d->in_ctx, tmp_pkt) < 0) { d->done = 1; break; }
        if (tmp_pkt->stream_index != d->video_in_index) { av_packet_unref(tmp_pkt); continue; }
        int ret = avcodec_send_packet(d->vdec_ctx, tmp_pkt);
        av_packet_unref(tmp_pkt);
        if (ret < 0) { d->done = 1; break; }
        ret = avcodec_receive_frame(d->vdec_ctx, tmp_frame);
        if (ret == AVERROR(EAGAIN)) continue;
        if (ret < 0) { d->done = 1; break; }
        double fsecs = tmp_frame->pts * av_q2d(d->vdec_ctx->pkt_timebase);
        if (fsecs < seg->source_in_secs) { av_frame_unref(tmp_frame); continue; }
        if (fsecs >= seg->source_out_secs) { av_frame_unref(tmp_frame); d->done = 1; break; }
        /* Store as pending, replacing any previous pending frame. */
        if (d->pending) av_frame_unref(d->pending);
        else            d->pending = av_frame_alloc();
        if (d->pending) av_frame_move_ref(d->pending, tmp_frame);
        else            av_frame_unref(tmp_frame);
        if (fsecs >= src_target - 0.02) break; /* close enough — stop advancing */
    }
    return (d->pending != NULL);
}

/* Build and configure a 2-input overlay AVFilterGraph:
   [in0]<f0>[v0];[in1]<f1>[v1];[v0][v1]overlay=0:0,format=yuv420p[out]
   Caller owns the returned graph + contexts; free with avfilter_graph_free(). */
static int oca_init_overlay_graph(
    AVCodecContext *vdec0, const char *f0,
    AVCodecContext *vdec1, const char *f1,
    AVFilterGraph **out_graph,
    AVFilterContext **out_src0, AVFilterContext **out_src1,
    AVFilterContext **out_sink)
{
    *out_graph = avfilter_graph_alloc();
    *out_src0 = *out_src1 = *out_sink = NULL;
    if (!*out_graph) return AVERROR(ENOMEM);

    const AVFilter *bufsrc  = avfilter_get_by_name("buffer");
    const AVFilter *bufsink = avfilter_get_by_name("buffersink");
    char a0[512], a1[512];
    AVRational sar0 = vdec0->sample_aspect_ratio;
    AVRational sar1 = vdec1->sample_aspect_ratio;
    if (sar0.num <= 0) sar0 = (AVRational){1, 1};
    if (sar1.num <= 0) sar1 = (AVRational){1, 1};

    snprintf(a0, sizeof(a0), "video_size=%dx%d:pix_fmt=%d:time_base=%d/%d:pixel_aspect=%d/%d",
             vdec0->width, vdec0->height, vdec0->pix_fmt,
             vdec0->pkt_timebase.num, vdec0->pkt_timebase.den, sar0.num, sar0.den);
    snprintf(a1, sizeof(a1), "video_size=%dx%d:pix_fmt=%d:time_base=%d/%d:pixel_aspect=%d/%d",
             vdec1->width, vdec1->height, vdec1->pix_fmt,
             vdec1->pkt_timebase.num, vdec1->pkt_timebase.den, sar1.num, sar1.den);

    int ret;
    ret = avfilter_graph_create_filter(out_src0, bufsrc,  "src0", a0,   NULL, *out_graph);
    if (ret < 0) goto fail;
    ret = avfilter_graph_create_filter(out_src1, bufsrc,  "src1", a1,   NULL, *out_graph);
    if (ret < 0) goto fail;
    ret = avfilter_graph_create_filter(out_sink, bufsink, "snk",  NULL, NULL, *out_graph);
    if (ret < 0) goto fail;

    char fstr[8192];
    snprintf(fstr, sizeof(fstr),
             "[in0]%s[v0];[in1]%s[v1];[v0][v1]overlay=0:0,format=yuv420p[out]", f0, f1);

    AVFilterInOut *outs0 = avfilter_inout_alloc();
    AVFilterInOut *outs1 = avfilter_inout_alloc();
    AVFilterInOut *inp   = avfilter_inout_alloc();
    if (!outs0 || !outs1 || !inp) {
        avfilter_inout_free(&outs0); avfilter_inout_free(&outs1); avfilter_inout_free(&inp);
        ret = AVERROR(ENOMEM); goto fail;
    }
    outs0->name = av_strdup("in0"); outs0->filter_ctx = *out_src0; outs0->pad_idx = 0; outs0->next = outs1;
    outs1->name = av_strdup("in1"); outs1->filter_ctx = *out_src1; outs1->pad_idx = 0; outs1->next = NULL;
    inp->name   = av_strdup("out"); inp->filter_ctx   = *out_sink;  inp->pad_idx   = 0; inp->next   = NULL;

    ret = avfilter_graph_parse_ptr(*out_graph, fstr, &inp, &outs0, NULL);
    avfilter_inout_free(&inp);
    avfilter_inout_free(&outs0);
    if (ret < 0) goto fail;
    ret = avfilter_graph_config(*out_graph, NULL);
    if (ret < 0) goto fail;
    return 0;
fail:
    avfilter_graph_free(out_graph);
    *out_src0 = *out_src1 = *out_sink = NULL;
    return ret;
}

OcaEncodeStatus avbridge_encode_timeline_export_multi(
    const OcaClipSegment * const *track_segs, const int *track_n_segs, int n_tracks,
    int canvas_width, int canvas_height, int canvas_fps_num, int canvas_fps_den,
    int64_t canvas_bit_rate_bps, const char *out_path, float target_lufs,
    OcaProgressCallback progress_cb, void *progress_user_data, const uint8_t *cancel)
{
    if (n_tracks <= 0 || track_n_segs[0] <= 0) return OCA_ENCODE_ERR_EMPTY_TIMELINE;

    /* N=1: delegate to the established single-track implementation. */
    if (n_tracks == 1) {
        return avbridge_encode_timeline_export(
            track_segs[0], track_n_segs[0],
            canvas_width, canvas_height, canvas_fps_num, canvas_fps_den,
            canvas_bit_rate_bps, out_path, target_lufs,
            progress_cb, progress_user_data, cancel);
    }

    /* N > 2: only the first two tracks are composited; additional tracks are ignored. */

    AVFormatContext *out_ctx      = NULL;
    AVCodecContext  *venc_ctx     = NULL, *aenc_ctx = NULL;
    AudioFilterChain achain       = {0};
    AVStream        *vout_stream  = NULL, *aout_stream = NULL;
    AVPacket        *pkt          = NULL, *t1_pkt = NULL;
    AVFrame         *dec_frame    = NULL, *filt_frame = NULL, *t1_tmp = NULL;
    AVPacket        *enc_pkt      = NULL;
    OcaEncodeStatus  status       = OCA_ENCODE_OK;
    AVRational       canvas_fps   = {canvas_fps_num, canvas_fps_den};
    int64_t          next_vpts    = 0;
    double           elapsed      = 0.0;
    int              canonical_sr = 0;
    enum AVSampleFormat canonical_fmt = AV_SAMPLE_FMT_NONE;
    AVChannelLayout  canonical_ch = {0};
    OcaOverlayDecoder ov1 = {.seg_open = -1};

    avformat_alloc_output_context2(&out_ctx, NULL, NULL, out_path);
    if (!out_ctx) return OCA_ENCODE_ERR_ALLOC_OUTPUT;

    /* Video encoder — identical setup to single-track function. */
    {
        const AVCodec *ve = avcodec_find_encoder_by_name("libopenh264");
        if (!ve) { status = OCA_ENCODE_ERR_ENCODER; goto cleanup; }
        venc_ctx = avcodec_alloc_context3(ve);
        if (!venc_ctx) { status = OCA_ENCODE_ERR_ENCODER; goto cleanup; }
        venc_ctx->width = canvas_width;  venc_ctx->height = canvas_height;
        venc_ctx->pix_fmt = AV_PIX_FMT_YUV420P;
        venc_ctx->time_base = av_inv_q(canvas_fps);  venc_ctx->framerate = canvas_fps;
        venc_ctx->gop_size = (canvas_fps.num / canvas_fps.den) * 2;
        venc_ctx->max_b_frames = 0;  venc_ctx->bit_rate = canvas_bit_rate_bps;
        if (out_ctx->oformat->flags & AVFMT_GLOBALHEADER) venc_ctx->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
        if (avcodec_open2(venc_ctx, ve, NULL) < 0) { status = OCA_ENCODE_ERR_ENCODER; goto cleanup; }
        vout_stream = avformat_new_stream(out_ctx, NULL);
        if (!vout_stream || avcodec_parameters_from_context(vout_stream->codecpar, venc_ctx) < 0) {
            status = OCA_ENCODE_ERR_NEW_STREAM; goto cleanup;
        }
        vout_stream->time_base = venc_ctx->time_base;
    }

    pkt = av_packet_alloc(); t1_pkt = av_packet_alloc();
    dec_frame = av_frame_alloc(); filt_frame = av_frame_alloc(); t1_tmp = av_frame_alloc();
    enc_pkt = av_packet_alloc();
    if (!pkt || !t1_pkt || !dec_frame || !filt_frame || !t1_tmp || !enc_pkt) {
        status = OCA_ENCODE_ERR_PIPELINE; goto cleanup;
    }

    /* Outer loop: process track-0 segments in order. */
    for (int si = 0; si < track_n_segs[0] && status == OCA_ENCODE_OK; si++) {
        const OcaClipSegment *seg0 = &track_segs[0][si];
        AVFormatContext *in_ctx0   = NULL;
        AVCodecContext  *vdec_ctx0 = NULL, *adec_ctx0 = NULL;
        VideoFilterChain vchain    = {0};
        AVFilterGraph   *ov_graph  = NULL;
        AVFilterContext *ov_src0 = NULL, *ov_src1 = NULL, *ov_sink = NULL;
        int vidx0 = -1, aidx0 = -1;
        /* Filter graph mode: 0 = not built, 1 = single-track vchain, 2 = overlay graph. */
        int cur_mode = 0;
        int64_t ov_frame0 = 0, ov_frame1 = 0; /* synthetic PTS for overlay buffersrc inputs */

        double spd0     = seg0->speed_factor > 0.0f ? seg0->speed_factor : 1.0f;
        double tl_start = seg0->timeline_start_secs;

        switch (oca_open_input(seg0->source_path, &in_ctx0)) {
            case -1: status = OCA_ENCODE_ERR_OPEN_INPUT; break;
            case -2: status = OCA_ENCODE_ERR_STREAM_INFO; break;
        }
        if (status != OCA_ENCODE_OK) break;

        for (unsigned s = 0; s < in_ctx0->nb_streams; s++) {
            enum AVMediaType mt = in_ctx0->streams[s]->codecpar->codec_type;
            if (vidx0 < 0 && mt == AVMEDIA_TYPE_VIDEO) vidx0 = (int)s;
            else if (aidx0 < 0 && mt == AVMEDIA_TYPE_AUDIO) aidx0 = (int)s;
        }
        if (vidx0 < 0) { avformat_close_input(&in_ctx0); status = OCA_ENCODE_ERR_NO_VIDEO_STREAM; break; }
        if (aidx0 < 0) { avformat_close_input(&in_ctx0); status = OCA_ENCODE_ERR_NO_AUDIO_STREAM; break; }

        { /* Video decoder */
            AVCodecParameters *vp = in_ctx0->streams[vidx0]->codecpar;
            const AVCodec *vd = avcodec_find_decoder(vp->codec_id);
            if (!vd) { status = OCA_ENCODE_ERR_DECODER; goto seg_cleanup; }
            vdec_ctx0 = avcodec_alloc_context3(vd);
            if (!vdec_ctx0 || avcodec_parameters_to_context(vdec_ctx0, vp) < 0) { status = OCA_ENCODE_ERR_DECODER; goto seg_cleanup; }
            vdec_ctx0->pkt_timebase = in_ctx0->streams[vidx0]->time_base;
            if (avcodec_open2(vdec_ctx0, vd, NULL) < 0) { status = OCA_ENCODE_ERR_DECODER; goto seg_cleanup; }
        }
        { /* Audio decoder */
            AVCodecParameters *ap = in_ctx0->streams[aidx0]->codecpar;
            const AVCodec *ad = avcodec_find_decoder(ap->codec_id);
            if (!ad) { status = OCA_ENCODE_ERR_DECODER; goto seg_cleanup; }
            adec_ctx0 = avcodec_alloc_context3(ad);
            if (!adec_ctx0 || avcodec_parameters_to_context(adec_ctx0, ap) < 0) { status = OCA_ENCODE_ERR_DECODER; goto seg_cleanup; }
            adec_ctx0->pkt_timebase = in_ctx0->streams[aidx0]->time_base;
            if (avcodec_open2(adec_ctx0, ad, NULL) < 0) { status = OCA_ENCODE_ERR_DECODER; goto seg_cleanup; }
        }

        if (si == 0) { /* First segment: set up audio chain + encoder + output header */
            canonical_sr  = adec_ctx0->sample_rate;
            canonical_fmt = adec_ctx0->sample_fmt;
            av_channel_layout_copy(&canonical_ch, &adec_ctx0->ch_layout);
            const AVCodec *ae = avcodec_find_encoder(AV_CODEC_ID_AAC);
            if (!ae) { status = OCA_ENCODE_ERR_ENCODER; goto seg_cleanup; }
            char afd[256];
            snprintf(afd, sizeof(afd),
                     "atempo@tempo=1.0,volume@vol=0dB,loudnorm=I=%.1f:TP=-1.0:LRA=11,"
                     "alimiter=limit=0.95:attack=5:release=50", (double)target_lufs);
            if (init_audio_filter_chain(adec_ctx0, ae, afd, &achain) < 0) { status = OCA_ENCODE_ERR_FILTER_GRAPH; goto seg_cleanup; }
            aenc_ctx = avcodec_alloc_context3(ae);
            if (!aenc_ctx) { status = OCA_ENCODE_ERR_ENCODER; goto seg_cleanup; }
            aenc_ctx->sample_rate = av_buffersink_get_sample_rate(achain.buffersink_ctx);
            av_buffersink_get_ch_layout(achain.buffersink_ctx, &aenc_ctx->ch_layout);
            aenc_ctx->sample_fmt = (enum AVSampleFormat)av_buffersink_get_format(achain.buffersink_ctx);
            aenc_ctx->bit_rate = 192000;
            aenc_ctx->time_base = av_buffersink_get_time_base(achain.buffersink_ctx);
            if (out_ctx->oformat->flags & AVFMT_GLOBALHEADER) aenc_ctx->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
            if (avcodec_open2(aenc_ctx, ae, NULL) < 0) { status = OCA_ENCODE_ERR_ENCODER; goto seg_cleanup; }
            if (aenc_ctx->frame_size > 0) av_buffersink_set_frame_size(achain.buffersink_ctx, (unsigned)aenc_ctx->frame_size);
            aout_stream = avformat_new_stream(out_ctx, NULL);
            if (!aout_stream || avcodec_parameters_from_context(aout_stream->codecpar, aenc_ctx) < 0) { status = OCA_ENCODE_ERR_NEW_STREAM; goto seg_cleanup; }
            aout_stream->time_base = aenc_ctx->time_base;
            if (!(out_ctx->oformat->flags & AVFMT_NOFILE)) {
                if (avio_open(&out_ctx->pb, out_path, AVIO_FLAG_WRITE) < 0) { status = OCA_ENCODE_ERR_OPEN_OUTPUT; goto seg_cleanup; }
            }
            if (avformat_write_header(out_ctx, NULL) < 0) { status = OCA_ENCODE_ERR_WRITE_HEADER; goto seg_cleanup; }
        } else if (adec_ctx0->sample_rate != canonical_sr ||
                   adec_ctx0->sample_fmt  != canonical_fmt ||
                   av_channel_layout_compare(&adec_ctx0->ch_layout, &canonical_ch) != 0) {
            status = OCA_ENCODE_ERR_AUDIO_FORMAT_MISMATCH; goto seg_cleanup;
        }

        { /* Update gain + tempo for this segment's audio */
            char gs[32], ts[32];
            snprintf(gs, sizeof(gs), "%.4fdB", (double)seg0->gain_db);
            avfilter_graph_send_command(achain.graph, "vol",   "volume", gs, NULL, 0, 0);
            float sp = seg0->speed_factor > 0.0f ? seg0->speed_factor : 1.0f;
            if (sp < 0.5f) sp = 0.5f; if (sp > 100.0f) sp = 100.0f;
            snprintf(ts, sizeof(ts), "%.6f", (double)sp);
            avfilter_graph_send_command(achain.graph, "tempo", "tempo",  ts, NULL, 0, 0);
        }

        if (seg0->source_in_secs > 0.0) {
            av_seek_frame(in_ctx0, -1, (int64_t)(seg0->source_in_secs * AV_TIME_BASE), AVSEEK_FLAG_BACKWARD);
            avcodec_flush_buffers(vdec_ctx0);
            avcodec_flush_buffers(adec_ctx0);
        }

        { /* Per-segment decode loop */
            int vdone = 0, adone = 0;
            while ((!vdone || !adone) && status == OCA_ENCODE_OK) {
                if (cancel && *cancel) { status = OCA_ENCODE_CANCELLED; break; }
                if (av_read_frame(in_ctx0, pkt) < 0) break;

                if (pkt->stream_index == vidx0 && !vdone) {
                    int ret = avcodec_send_packet(vdec_ctx0, pkt);
                    av_packet_unref(pkt);
                    if (ret < 0) { status = OCA_ENCODE_ERR_PIPELINE; break; }
                    while (1) {
                        ret = avcodec_receive_frame(vdec_ctx0, dec_frame);
                        if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF) break;
                        if (ret < 0) { status = OCA_ENCODE_ERR_PIPELINE; break; }
                        double fsrc = dec_frame->pts * av_q2d(vdec_ctx0->pkt_timebase);
                        if (fsrc < seg0->source_in_secs)  { av_frame_unref(dec_frame); continue; }
                        if (fsrc >= seg0->source_out_secs) { vdone = 1; av_frame_unref(dec_frame); continue; }

                        double ftl = tl_start + (fsrc - seg0->source_in_secs) / spd0;

                        /* Find track-1 segment active at timeline time ftl. */
                        int t1_seg_idx = -1;
                        for (int s = 0; s < track_n_segs[1]; s++) {
                            const OcaClipSegment *s1 = &track_segs[1][s];
                            double sp1 = s1->speed_factor > 0.0f ? s1->speed_factor : 1.0f;
                            double s1e = s1->timeline_start_secs + (s1->source_out_secs - s1->source_in_secs) / sp1;
                            if (ftl >= s1->timeline_start_secs && ftl < s1e) { t1_seg_idx = s; break; }
                        }
                        int want_overlay = (t1_seg_idx >= 0);

                        /* Rebuild filter graph if mode or segment changed. */
                        if (want_overlay && (cur_mode != 2 || t1_seg_idx != ov1.seg_open)) {
                            /* Switch to / rebuild overlay mode */
                            if (ov_graph) { avfilter_graph_free(&ov_graph); ov_src0 = ov_src1 = ov_sink = NULL; }
                            free_video_filter_chain(&vchain);

                            const OcaClipSegment *s1 = &track_segs[1][t1_seg_idx];
                            double sp1 = s1->speed_factor > 0.0f ? s1->speed_factor : 1.0f;
                            double t1_seek = s1->source_in_secs + (ftl - s1->timeline_start_secs) * sp1;
                            if (oca_open_overlay_decoder(s1, t1_seek, &ov1, t1_seg_idx) < 0) {
                                status = OCA_ENCODE_ERR_DECODER; av_frame_unref(dec_frame); break;
                            }

                            char f0[4096], f1[4096];
                            oca_build_overlay_vfilter(seg0, canvas_width, canvas_height, canvas_fps_num, canvas_fps_den, f0, sizeof(f0));
                            oca_build_overlay_vfilter(s1,   canvas_width, canvas_height, canvas_fps_num, canvas_fps_den, f1, sizeof(f1));
                            if (oca_init_overlay_graph(vdec_ctx0, f0, ov1.vdec_ctx, f1,
                                                        &ov_graph, &ov_src0, &ov_src1, &ov_sink) < 0) {
                                status = OCA_ENCODE_ERR_FILTER_GRAPH; av_frame_unref(dec_frame); break;
                            }
                            ov_frame0 = ov_frame1 = 0;
                            cur_mode = 2;
                        } else if (!want_overlay && cur_mode != 1) {
                            /* Switch to single-track mode */
                            if (ov_graph) { avfilter_graph_free(&ov_graph); ov_src0 = ov_src1 = ov_sink = NULL; }
                            free_video_filter_chain(&vchain);
                            if (ov1.in_ctx) oca_free_overlay_decoder(&ov1);

                            char vfd[3072];
                            oca_build_vfilter_descr(seg0, canvas_width, canvas_height,
                                                     canvas_fps_num, canvas_fps_den, vfd, sizeof(vfd));
                            if (init_video_filter_chain(vdec_ctx0, vfd, &vchain) < 0) {
                                status = OCA_ENCODE_ERR_FILTER_GRAPH; av_frame_unref(dec_frame); break;
                            }
                            cur_mode = 1;
                        }

                        if (cur_mode == 2) {
                            /* Get overlay frame from track 1 (advance its decoder). */
                            const OcaClipSegment *s1 = &track_segs[1][t1_seg_idx];
                            double sp1 = s1->speed_factor > 0.0f ? s1->speed_factor : 1.0f;
                            double t1_src = s1->source_in_secs + (ftl - s1->timeline_start_secs) * sp1;
                            oca_advance_overlay_decoder(&ov1, s1, t1_src, t1_pkt, t1_tmp);

                            /* Push track-0 frame to overlay graph with synthetic pts. */
                            dec_frame->pts = ov_frame0++;
                            av_buffersrc_add_frame_flags(ov_src0, dec_frame, AV_BUFFERSRC_FLAG_KEEP_REF);

                            /* Push track-1 frame (or repeat last pending frame) to overlay graph. */
                            if (ov1.pending) {
                                ov1.pending->pts = ov_frame1++;
                                av_buffersrc_add_frame_flags(ov_src1, ov1.pending, AV_BUFFERSRC_FLAG_KEEP_REF);
                            }
                            /* (If no track-1 frame: overlay filter holds last known frame.) */

                            /* Pull composite frames from overlay sink. */
                            while (av_buffersink_get_frame(ov_sink, filt_frame) == 0) {
                                filt_frame->pts = next_vpts++;
                                if (encode_write_packet(out_ctx, venc_ctx, vout_stream, filt_frame, enc_pkt) < 0) {
                                    status = OCA_ENCODE_ERR_PIPELINE;
                                }
                                av_frame_unref(filt_frame);
                                if (status != OCA_ENCODE_OK) break;
                            }
                            if (progress_cb)
                                progress_cb(progress_user_data, elapsed + (fsrc - seg0->source_in_secs) / spd0);
                        } else { /* single-track vchain */
                            if (filter_encode_write_video_frame(out_ctx, &vchain, venc_ctx,
                                                                 vout_stream, dec_frame, filt_frame,
                                                                 &next_vpts, enc_pkt) < 0) {
                                status = OCA_ENCODE_ERR_PIPELINE;
                            }
                            if (progress_cb)
                                progress_cb(progress_user_data, elapsed + (fsrc - seg0->source_in_secs) / spd0);
                        }
                        av_frame_unref(dec_frame);
                        if (status != OCA_ENCODE_OK) break;
                    }
                } else if (pkt->stream_index == aidx0 && !adone) {
                    int ret = avcodec_send_packet(adec_ctx0, pkt);
                    av_packet_unref(pkt);
                    if (ret < 0) { status = OCA_ENCODE_ERR_PIPELINE; break; }
                    while (1) {
                        ret = avcodec_receive_frame(adec_ctx0, dec_frame);
                        if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF) break;
                        if (ret < 0) { status = OCA_ENCODE_ERR_PIPELINE; break; }
                        double fs = dec_frame->pts * av_q2d(adec_ctx0->pkt_timebase);
                        if (fs < seg0->source_in_secs)  { av_frame_unref(dec_frame); continue; }
                        if (fs >= seg0->source_out_secs) { adone = 1; av_frame_unref(dec_frame); continue; }
                        if (filter_encode_write_frame(out_ctx, &achain, aenc_ctx, aout_stream,
                                                       dec_frame, filt_frame, enc_pkt) < 0) {
                            status = OCA_ENCODE_ERR_PIPELINE; break;
                        }
                        av_frame_unref(dec_frame);
                    }
                } else {
                    av_packet_unref(pkt);
                }
                if (status != OCA_ENCODE_OK) break;
            }
        }

        elapsed += (seg0->source_out_secs - seg0->source_in_secs) / spd0;

    seg_cleanup:
        if (ov_graph) { avfilter_graph_free(&ov_graph); ov_src0 = ov_src1 = ov_sink = NULL; }
        free_video_filter_chain(&vchain);
        avcodec_free_context(&vdec_ctx0);
        avcodec_free_context(&adec_ctx0);
        avformat_close_input(&in_ctx0);
    }

    /* Flush audio encoder + loudnorm graph. */
    if (status == OCA_ENCODE_OK &&
        (filter_encode_write_frame(out_ctx, &achain, aenc_ctx, aout_stream, NULL, filt_frame, enc_pkt) < 0 ||
         encode_write_packet(out_ctx, aenc_ctx, aout_stream, NULL, enc_pkt) < 0)) {
        status = OCA_ENCODE_ERR_PIPELINE;
    }
    /* Flush video encoder. */
    if (status == OCA_ENCODE_OK &&
        encode_write_packet(out_ctx, venc_ctx, vout_stream, NULL, enc_pkt) < 0) {
        status = OCA_ENCODE_ERR_PIPELINE;
    }
    if (status == OCA_ENCODE_OK) av_write_trailer(out_ctx);

cleanup:
    oca_free_overlay_decoder(&ov1);
    av_packet_free(&pkt);
    av_packet_free(&t1_pkt);
    av_packet_free(&enc_pkt);
    av_frame_free(&dec_frame);
    av_frame_free(&filt_frame);
    av_frame_free(&t1_tmp);
    if (out_ctx && out_ctx->pb && !(out_ctx->oformat->flags & AVFMT_NOFILE))
        avio_closep(&out_ctx->pb);
    free_audio_filter_chain(&achain);
    av_channel_layout_uninit(&canonical_ch);
    avcodec_free_context(&venc_ctx);
    avcodec_free_context(&aenc_ctx);
    if (out_ctx) avformat_free_context(out_ctx);
    return status;
}

/* Global by necessity: av_log_set_callback() takes a plain function pointer with no user-data
   parameter, so there is nowhere else to stash where the current call's capture buffer is.
   Matches the NOT THREAD-SAFE contract documented on avbridge_measure_loudness() in
   bridge.h. */
static char *g_loudness_log_buf = NULL;
static size_t g_loudness_log_cap = 0;
static size_t g_loudness_log_len = 0;
static int g_loudness_log_print_prefix = 1;

static void loudness_log_callback(void *avcl, int level, const char *fmt, va_list vl) {
    /* loudnorm's JSON report prints at AV_LOG_INFO; anything noisier (DEBUG/VERBOSE) is not
       needed and anything we'd want less of would print at WARNING or above regardless. */
    if (level > AV_LOG_INFO || !g_loudness_log_buf || g_loudness_log_len + 1 >= g_loudness_log_cap) {
        return;
    }
    char line[1024];
    int written = av_log_format_line2(avcl, level, fmt, vl, line, sizeof(line),
                                       &g_loudness_log_print_prefix);
    if (written <= 0) {
        return;
    }
    size_t remaining = g_loudness_log_cap - g_loudness_log_len - 1;
    size_t to_copy = (size_t)written < remaining ? (size_t)written : remaining;
    memcpy(g_loudness_log_buf + g_loudness_log_len, line, to_copy);
    g_loudness_log_len += to_copy;
    g_loudness_log_buf[g_loudness_log_len] = '\0';
}

/* Simpler than init_audio_filter_chain(): analysis only, nothing downstream needs the
   filtered samples in a specific format (they're discarded, not encoded), so no
   sample_formats/samplerates constraint on the sink. */
static int init_measure_filter_chain(AVCodecContext *dec_ctx, AudioFilterChain *chain) {
    char args[512];
    char chlayout_str[64];
    const AVFilter *buffersrc = avfilter_get_by_name("abuffer");
    const AVFilter *buffersink = avfilter_get_by_name("abuffersink");
    AVFilterInOut *outputs = avfilter_inout_alloc();
    AVFilterInOut *inputs = avfilter_inout_alloc();
    int ret;

    chain->buffersrc_ctx = NULL;
    chain->buffersink_ctx = NULL;
    chain->graph = avfilter_graph_alloc();
    if (!outputs || !inputs || !chain->graph) {
        ret = AVERROR(ENOMEM);
        goto end;
    }

    av_channel_layout_describe(&dec_ctx->ch_layout, chlayout_str, sizeof(chlayout_str));
    snprintf(args, sizeof(args),
             "time_base=1/%d:sample_rate=%d:sample_fmt=%s:channel_layout=%s",
             dec_ctx->sample_rate, dec_ctx->sample_rate,
             av_get_sample_fmt_name(dec_ctx->sample_fmt), chlayout_str);

    ret = avfilter_graph_create_filter(&chain->buffersrc_ctx, buffersrc, "in", args, NULL,
                                        chain->graph);
    if (ret < 0) goto end;

    ret = avfilter_graph_create_filter(&chain->buffersink_ctx, buffersink, "out", NULL, NULL,
                                        chain->graph);
    if (ret < 0) goto end;

    outputs->name = av_strdup("in");
    outputs->filter_ctx = chain->buffersrc_ctx;
    outputs->pad_idx = 0;
    outputs->next = NULL;

    inputs->name = av_strdup("out");
    inputs->filter_ctx = chain->buffersink_ctx;
    inputs->pad_idx = 0;
    inputs->next = NULL;

    ret = avfilter_graph_parse_ptr(chain->graph, "loudnorm=I=-16:TP=-1.5:LRA=11:print_format=json",
                                    &inputs, &outputs, NULL);
    if (ret < 0) goto end;

    ret = avfilter_graph_config(chain->graph, NULL);

end:
    avfilter_inout_free(&inputs);
    avfilter_inout_free(&outputs);
    if (ret < 0) {
        free_audio_filter_chain(chain);
    }
    return ret;
}

/* Pushes one frame through the filter graph and drains+discards whatever comes out (the
   filtered samples themselves aren't needed, only the JSON report loudnorm logs on EOF).
   Pass frame=NULL to flush the graph. */
static int drain_measure_frame(AudioFilterChain *chain, AVFrame *frame, AVFrame *filt_frame) {
    int ret = av_buffersrc_add_frame(chain->buffersrc_ctx, frame);
    if (ret < 0) return ret;

    while (1) {
        ret = av_buffersink_get_frame(chain->buffersink_ctx, filt_frame);
        if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF) {
            return 0;
        } else if (ret < 0) {
            return ret;
        }
        av_frame_unref(filt_frame);
    }
}

OcaLoudnessStatus avbridge_measure_loudness(const char *in_path, char *out_json,
                                                 size_t out_json_len) {
    AVFormatContext *in_ctx = NULL;
    AVCodecContext *dec_ctx = NULL;
    AudioFilterChain chain = {0};
    AVPacket *pkt = NULL;
    AVFrame *dec_frame = NULL;
    AVFrame *filt_frame = NULL;
    OcaLoudnessStatus status = OCA_LOUDNESS_OK;
    int audio_in_index = -1;
    int log_installed = 0;

    if (out_json && out_json_len > 0) {
        out_json[0] = '\0';
    }

    switch (oca_open_input(in_path, &in_ctx)) {
        case -1: return OCA_LOUDNESS_ERR_OPEN_INPUT;
        case -2: return OCA_LOUDNESS_ERR_STREAM_INFO;
    }

    for (unsigned int i = 0; i < in_ctx->nb_streams; i++) {
        if (in_ctx->streams[i]->codecpar->codec_type == AVMEDIA_TYPE_AUDIO) {
            audio_in_index = (int)i;
            break;
        }
    }
    if (audio_in_index < 0) {
        avformat_close_input(&in_ctx);
        return OCA_LOUDNESS_ERR_NO_AUDIO_STREAM;
    }

    {
        const AVCodec *decoder =
            avcodec_find_decoder(in_ctx->streams[audio_in_index]->codecpar->codec_id);
        if (!decoder) {
            status = OCA_LOUDNESS_ERR_DECODER;
            goto cleanup;
        }
        dec_ctx = avcodec_alloc_context3(decoder);
        if (!dec_ctx || avcodec_parameters_to_context(
                            dec_ctx, in_ctx->streams[audio_in_index]->codecpar) < 0) {
            status = OCA_LOUDNESS_ERR_DECODER;
            goto cleanup;
        }
        dec_ctx->pkt_timebase = in_ctx->streams[audio_in_index]->time_base;
        if (avcodec_open2(dec_ctx, decoder, NULL) < 0) {
            status = OCA_LOUDNESS_ERR_DECODER;
            goto cleanup;
        }
    }

    if (init_measure_filter_chain(dec_ctx, &chain) < 0) {
        status = OCA_LOUDNESS_ERR_FILTER_GRAPH;
        goto cleanup;
    }

    pkt = av_packet_alloc();
    dec_frame = av_frame_alloc();
    filt_frame = av_frame_alloc();
    if (!pkt || !dec_frame || !filt_frame) {
        status = OCA_LOUDNESS_ERR_PIPELINE;
        goto cleanup;
    }

    /* Install the log capture only around the pipeline run — loudnorm prints its JSON report
       via av_log() while processing the flush below, there is no queryable struct API. */
    g_loudness_log_buf = out_json;
    g_loudness_log_cap = out_json_len;
    g_loudness_log_len = 0;
    g_loudness_log_print_prefix = 1;
    av_log_set_callback(loudness_log_callback);
    log_installed = 1;

    while (av_read_frame(in_ctx, pkt) >= 0) {
        if (pkt->stream_index != audio_in_index) {
            av_packet_unref(pkt);
            continue;
        }
        int ret = avcodec_send_packet(dec_ctx, pkt);
        av_packet_unref(pkt);
        if (ret < 0) {
            status = OCA_LOUDNESS_ERR_PIPELINE;
            goto cleanup;
        }
        while (1) {
            ret = avcodec_receive_frame(dec_ctx, dec_frame);
            if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF) {
                break;
            } else if (ret < 0) {
                status = OCA_LOUDNESS_ERR_PIPELINE;
                goto cleanup;
            }
            if (drain_measure_frame(&chain, dec_frame, filt_frame) < 0) {
                status = OCA_LOUDNESS_ERR_PIPELINE;
                goto cleanup;
            }
        }
    }

    avcodec_send_packet(dec_ctx, NULL);
    while (avcodec_receive_frame(dec_ctx, dec_frame) >= 0) {
        if (drain_measure_frame(&chain, dec_frame, filt_frame) < 0) {
            status = OCA_LOUDNESS_ERR_PIPELINE;
            goto cleanup;
        }
    }
    drain_measure_frame(&chain, NULL, filt_frame);

cleanup:
    /* free_audio_filter_chain() (avfilter_graph_free) MUST run while the log capture is still
       installed: loudnorm prints its final JSON report during filter uninit — i.e. graph
       teardown — not while processing the EOF flush above. Resetting the callback first was
       an earlier bug here: the report would print to the console via
       av_log_default_callback instead of landing in out_json. */
    free_audio_filter_chain(&chain);

    if (log_installed) {
        if (status == OCA_LOUDNESS_OK && (!out_json || out_json[0] == '\0')) {
            status = OCA_LOUDNESS_ERR_NO_REPORT;
        }
        av_log_set_callback(av_log_default_callback);
        g_loudness_log_buf = NULL;
        g_loudness_log_cap = 0;
        g_loudness_log_len = 0;
    }
    avcodec_free_context(&dec_ctx);
    av_frame_free(&dec_frame);
    av_frame_free(&filt_frame);
    av_packet_free(&pkt);
    avformat_close_input(&in_ctx);
    return status;
}

/* Same shape as init_measure_filter_chain() but the filter graph downmixes to mono float
   instead of running loudnorm — the filtered samples themselves are what gets read here,
   not discarded. */
static int init_waveform_filter_chain(AVCodecContext *dec_ctx, AudioFilterChain *chain) {
    char args[512];
    char chlayout_str[64];
    const AVFilter *buffersrc = avfilter_get_by_name("abuffer");
    const AVFilter *buffersink = avfilter_get_by_name("abuffersink");
    AVFilterInOut *outputs = avfilter_inout_alloc();
    AVFilterInOut *inputs = avfilter_inout_alloc();
    int ret;

    chain->buffersrc_ctx = NULL;
    chain->buffersink_ctx = NULL;
    chain->graph = avfilter_graph_alloc();
    if (!outputs || !inputs || !chain->graph) {
        ret = AVERROR(ENOMEM);
        goto end;
    }

    av_channel_layout_describe(&dec_ctx->ch_layout, chlayout_str, sizeof(chlayout_str));
    snprintf(args, sizeof(args),
             "time_base=1/%d:sample_rate=%d:sample_fmt=%s:channel_layout=%s",
             dec_ctx->sample_rate, dec_ctx->sample_rate,
             av_get_sample_fmt_name(dec_ctx->sample_fmt), chlayout_str);

    ret = avfilter_graph_create_filter(&chain->buffersrc_ctx, buffersrc, "in", args, NULL,
                                        chain->graph);
    if (ret < 0) goto end;

    ret = avfilter_graph_create_filter(&chain->buffersink_ctx, buffersink, "out", NULL, NULL,
                                        chain->graph);
    if (ret < 0) goto end;

    outputs->name = av_strdup("in");
    outputs->filter_ctx = chain->buffersrc_ctx;
    outputs->pad_idx = 0;
    outputs->next = NULL;

    inputs->name = av_strdup("out");
    inputs->filter_ctx = chain->buffersink_ctx;
    inputs->pad_idx = 0;
    inputs->next = NULL;

    ret = avfilter_graph_parse_ptr(chain->graph, "aformat=sample_fmts=flt:channel_layouts=mono",
                                    &inputs, &outputs, NULL);
    if (ret < 0) goto end;

    ret = avfilter_graph_config(chain->graph, NULL);

end:
    avfilter_inout_free(&inputs);
    avfilter_inout_free(&outputs);
    if (ret < 0) {
        free_audio_filter_chain(chain);
    }
    return ret;
}

/* Peak-accumulation state threaded through the waveform decode/filter loop below — a bucket
   index is purely sample-index math (sample_index / bucket_size_samples), so it can't drift
   from timestamp jitter mid-decode the way a wall-clock computation could. */
typedef struct {
    float *out_min;
    float *out_max;
    int bucket_count;
    int64_t bucket_size_samples;
    int64_t sample_index;
} WaveformAccumulator;

static void accumulate_waveform_frame(WaveformAccumulator *acc, const AVFrame *frame) {
    const float *samples = (const float *)frame->data[0];
    for (int i = 0; i < frame->nb_samples; i++) {
        int64_t bucket = acc->sample_index / acc->bucket_size_samples;
        if (bucket >= acc->bucket_count) {
            bucket = acc->bucket_count - 1;
        }
        float s = samples[i];
        if (s < acc->out_min[bucket]) acc->out_min[bucket] = s;
        if (s > acc->out_max[bucket]) acc->out_max[bucket] = s;
        acc->sample_index++;
    }
}

/* Pushes one decoded frame through the mono-downmix graph and accumulates every filtered
   sample into `acc`'s min/max buckets. Pass frame=NULL to flush. */
static int drain_waveform_frame(AudioFilterChain *chain, AVFrame *frame, AVFrame *filt_frame,
                                 WaveformAccumulator *acc) {
    int ret = av_buffersrc_add_frame(chain->buffersrc_ctx, frame);
    if (ret < 0) return ret;

    while (1) {
        ret = av_buffersink_get_frame(chain->buffersink_ctx, filt_frame);
        if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF) {
            return 0;
        } else if (ret < 0) {
            return ret;
        }
        accumulate_waveform_frame(acc, filt_frame);
        av_frame_unref(filt_frame);
    }
}

OcaWaveformStatus avbridge_generate_waveform(const char *in_path, int bucket_count,
                                                  float *out_min, float *out_max) {
    AVFormatContext *in_ctx = NULL;
    AVCodecContext *dec_ctx = NULL;
    AudioFilterChain chain = {0};
    AVPacket *pkt = NULL;
    AVFrame *dec_frame = NULL;
    AVFrame *filt_frame = NULL;
    WaveformAccumulator acc;
    OcaWaveformStatus status = OCA_WAVEFORM_OK;
    int audio_in_index = -1;
    int64_t total_samples;
    double duration_secs = 0.0;
    AVStream *audio_stream;

    if (bucket_count <= 0) {
        return OCA_WAVEFORM_ERR_PIPELINE;
    }
    for (int i = 0; i < bucket_count; i++) {
        out_min[i] = 0.0f;
        out_max[i] = 0.0f;
    }

    switch (oca_open_input(in_path, &in_ctx)) {
        case -1: return OCA_WAVEFORM_ERR_OPEN_INPUT;
        case -2: return OCA_WAVEFORM_ERR_STREAM_INFO;
    }

    for (unsigned int i = 0; i < in_ctx->nb_streams; i++) {
        if (in_ctx->streams[i]->codecpar->codec_type == AVMEDIA_TYPE_AUDIO) {
            audio_in_index = (int)i;
            break;
        }
    }
    if (audio_in_index < 0) {
        avformat_close_input(&in_ctx);
        return OCA_WAVEFORM_ERR_NO_AUDIO_STREAM;
    }

    audio_stream = in_ctx->streams[audio_in_index];
    if (in_ctx->duration != (int64_t)AV_NOPTS_VALUE) {
        duration_secs = (double)in_ctx->duration / AV_TIME_BASE;
    } else if (audio_stream->duration != (int64_t)AV_NOPTS_VALUE) {
        duration_secs = audio_stream->duration * av_q2d(audio_stream->time_base);
    }

    {
        const AVCodec *decoder = avcodec_find_decoder(audio_stream->codecpar->codec_id);
        if (!decoder) {
            status = OCA_WAVEFORM_ERR_DECODER;
            goto cleanup;
        }
        dec_ctx = avcodec_alloc_context3(decoder);
        if (!dec_ctx || avcodec_parameters_to_context(dec_ctx, audio_stream->codecpar) < 0) {
            status = OCA_WAVEFORM_ERR_DECODER;
            goto cleanup;
        }
        dec_ctx->pkt_timebase = audio_stream->time_base;
        if (avcodec_open2(dec_ctx, decoder, NULL) < 0) {
            status = OCA_WAVEFORM_ERR_DECODER;
            goto cleanup;
        }
    }

    if (init_waveform_filter_chain(dec_ctx, &chain) < 0) {
        status = OCA_WAVEFORM_ERR_FILTER_GRAPH;
        goto cleanup;
    }

    pkt = av_packet_alloc();
    dec_frame = av_frame_alloc();
    filt_frame = av_frame_alloc();
    if (!pkt || !dec_frame || !filt_frame) {
        status = OCA_WAVEFORM_ERR_PIPELINE;
        goto cleanup;
    }

    total_samples = (int64_t)(duration_secs * dec_ctx->sample_rate);
    acc.out_min = out_min;
    acc.out_max = out_max;
    acc.bucket_count = bucket_count;
    acc.bucket_size_samples = total_samples > 0 ? total_samples / bucket_count : 0;
    if (acc.bucket_size_samples <= 0) {
        acc.bucket_size_samples = 1;
    }
    acc.sample_index = 0;

    while (av_read_frame(in_ctx, pkt) >= 0) {
        if (pkt->stream_index != audio_in_index) {
            av_packet_unref(pkt);
            continue;
        }
        int ret = avcodec_send_packet(dec_ctx, pkt);
        av_packet_unref(pkt);
        if (ret < 0) {
            status = OCA_WAVEFORM_ERR_PIPELINE;
            goto cleanup;
        }
        while (1) {
            ret = avcodec_receive_frame(dec_ctx, dec_frame);
            if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF) {
                break;
            } else if (ret < 0) {
                status = OCA_WAVEFORM_ERR_PIPELINE;
                goto cleanup;
            }
            if (drain_waveform_frame(&chain, dec_frame, filt_frame, &acc) < 0) {
                status = OCA_WAVEFORM_ERR_PIPELINE;
                goto cleanup;
            }
        }
    }

    avcodec_send_packet(dec_ctx, NULL);
    while (avcodec_receive_frame(dec_ctx, dec_frame) >= 0) {
        if (drain_waveform_frame(&chain, dec_frame, filt_frame, &acc) < 0) {
            status = OCA_WAVEFORM_ERR_PIPELINE;
            goto cleanup;
        }
    }
    drain_waveform_frame(&chain, NULL, filt_frame, &acc);

cleanup:
    free_audio_filter_chain(&chain);
    avcodec_free_context(&dec_ctx);
    av_frame_free(&dec_frame);
    av_frame_free(&filt_frame);
    av_packet_free(&pkt);
    avformat_close_input(&in_ctx);
    return status;
}

/* Scales one decoded frame into `scaled_frame` (already sized/formatted for the encoder,
   allocated once by the caller) and stamps a sequential pts — proxies don't need to preserve
   the source's original timestamps, just monotonically increasing ones for the encoder/muxer. */
static int scale_video_frame(struct SwsContext *sws_ctx, AVFrame *src_frame,
                              AVFrame *scaled_frame, int64_t *next_pts) {
    int ret = av_frame_make_writable(scaled_frame);
    if (ret < 0) return ret;

    ret = sws_scale_frame(sws_ctx, scaled_frame, src_frame);
    if (ret < 0) return ret;

    scaled_frame->pts = (*next_pts)++;
    return 0;
}

OcaProxyStatus avbridge_generate_proxy(const char *in_path, const char *out_path,
                                            int target_height) {
    AVFormatContext *in_ctx = NULL;
    AVFormatContext *out_ctx = NULL;
    AVCodecContext *vdec_ctx = NULL;
    AVCodecContext *venc_ctx = NULL;
    AVCodecContext *adec_ctx = NULL;
    AVCodecContext *aenc_ctx = NULL;
    struct SwsContext *sws_ctx = NULL;
    AudioFilterChain achain = {0};
    AVPacket *pkt = NULL;
    AVFrame *dec_frame = NULL;
    AVFrame *scaled_frame = NULL;
    AVFrame *filt_frame = NULL;
    AVPacket *enc_pkt = NULL;
    OcaProxyStatus status = OCA_PROXY_OK;
    int video_in_index = -1;
    int audio_in_index = -1;
    int video_out_index = -1;
    int audio_out_index = -1;
    int64_t next_video_pts = 0;

    switch (oca_open_input(in_path, &in_ctx)) {
        case -1: return OCA_PROXY_ERR_OPEN_INPUT;
        case -2: return OCA_PROXY_ERR_STREAM_INFO;
    }

    for (unsigned int i = 0; i < in_ctx->nb_streams; i++) {
        enum AVMediaType type = in_ctx->streams[i]->codecpar->codec_type;
        if (video_in_index < 0 && type == AVMEDIA_TYPE_VIDEO) {
            video_in_index = (int)i;
        } else if (audio_in_index < 0 && type == AVMEDIA_TYPE_AUDIO) {
            audio_in_index = (int)i;
        }
    }
    if (video_in_index < 0) {
        avformat_close_input(&in_ctx);
        return OCA_PROXY_ERR_NO_VIDEO_STREAM;
    }

    avformat_alloc_output_context2(&out_ctx, NULL, NULL, out_path);
    if (!out_ctx) {
        status = OCA_PROXY_ERR_ALLOC_OUTPUT;
        goto cleanup;
    }

    /* Video decoder. */
    {
        AVCodecParameters *vpar = in_ctx->streams[video_in_index]->codecpar;
        const AVCodec *decoder = avcodec_find_decoder(vpar->codec_id);
        if (!decoder) {
            status = OCA_PROXY_ERR_DECODER;
            goto cleanup;
        }
        vdec_ctx = avcodec_alloc_context3(decoder);
        if (!vdec_ctx || avcodec_parameters_to_context(vdec_ctx, vpar) < 0) {
            status = OCA_PROXY_ERR_DECODER;
            goto cleanup;
        }
        vdec_ctx->pkt_timebase = in_ctx->streams[video_in_index]->time_base;
        if (avcodec_open2(vdec_ctx, decoder, NULL) < 0) {
            status = OCA_PROXY_ERR_DECODER;
            goto cleanup;
        }
    }

    /* Video encoder (libopenh264 — BSD-licensed, available in this LGPL FFmpeg build; not
       libx264/GPL) + scaler, dimensions keeping the source's aspect ratio with height fixed
       at target_height, rounded to the nearest even number (odd dimensions are invalid for
       yuv420p — chroma planes are subsampled 2x in both directions). */
    {
        const AVCodec *venc = avcodec_find_encoder_by_name("libopenh264");
        if (!venc) {
            status = OCA_PROXY_ERR_ENCODER;
            goto cleanup;
        }

        int src_w = vdec_ctx->width;
        int src_h = vdec_ctx->height;
        int dst_h = target_height > 0 ? target_height : src_h;
        int dst_w = (int)((int64_t)src_w * dst_h / src_h);
        dst_w += dst_w % 2;
        dst_h += dst_h % 2;

        venc_ctx = avcodec_alloc_context3(venc);
        if (!venc_ctx) {
            status = OCA_PROXY_ERR_ENCODER;
            goto cleanup;
        }
        venc_ctx->width = dst_w;
        venc_ctx->height = dst_h;
        venc_ctx->pix_fmt = AV_PIX_FMT_YUV420P;
        AVRational frame_rate = in_ctx->streams[video_in_index]->avg_frame_rate;
        if (frame_rate.num <= 0 || frame_rate.den <= 0) {
            frame_rate = (AVRational){30, 1};
        }
        venc_ctx->time_base = av_inv_q(frame_rate);
        venc_ctx->framerate = frame_rate;
        venc_ctx->gop_size = (frame_rate.num / frame_rate.den) * 2; /* ~2s keyframe interval */
        venc_ctx->max_b_frames = 0; /* libopenh264 doesn't support B-frames */
        /* Proxy target: small and fast to decode while scrubbing, not archival quality.
           libopenh264 has no CRF-equivalent (unlike libx264, which this build doesn't have —
           see the doc comment on this function) — bitrate is the only practical rate-control
           knob, sized off the (already downscaled) pixel count so both a 540p and a 360p
           proxy land in a reasonable range rather than one fixed number over- or
           under-shooting depending on target_height. */
        venc_ctx->bit_rate = (int64_t)dst_w * dst_h * 2;
        if (out_ctx->oformat->flags & AVFMT_GLOBALHEADER) {
            venc_ctx->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
        }
        if (avcodec_open2(venc_ctx, venc, NULL) < 0) {
            status = OCA_PROXY_ERR_ENCODER;
            goto cleanup;
        }

        sws_ctx = sws_getContext(src_w, src_h, vdec_ctx->pix_fmt, dst_w, dst_h,
                                  AV_PIX_FMT_YUV420P, SWS_BILINEAR, NULL, NULL, NULL);
        if (!sws_ctx) {
            status = OCA_PROXY_ERR_SCALER;
            goto cleanup;
        }

        AVStream *out_stream = avformat_new_stream(out_ctx, NULL);
        if (!out_stream || avcodec_parameters_from_context(out_stream->codecpar, venc_ctx) < 0) {
            status = OCA_PROXY_ERR_NEW_STREAM;
            goto cleanup;
        }
        out_stream->time_base = venc_ctx->time_base;
        video_out_index = out_stream->index;
    }

    /* Audio decoder + encoder + filterless (format-conversion-only) graph, only if the
       source has an audio stream — a video-only source produces a video-only proxy. */
    if (audio_in_index >= 0) {
        AVCodecParameters *apar = in_ctx->streams[audio_in_index]->codecpar;
        const AVCodec *adecoder = avcodec_find_decoder(apar->codec_id);
        if (!adecoder) {
            status = OCA_PROXY_ERR_DECODER;
            goto cleanup;
        }
        adec_ctx = avcodec_alloc_context3(adecoder);
        if (!adec_ctx || avcodec_parameters_to_context(adec_ctx, apar) < 0) {
            status = OCA_PROXY_ERR_DECODER;
            goto cleanup;
        }
        adec_ctx->pkt_timebase = in_ctx->streams[audio_in_index]->time_base;
        if (avcodec_open2(adec_ctx, adecoder, NULL) < 0) {
            status = OCA_PROXY_ERR_DECODER;
            goto cleanup;
        }

        const AVCodec *aenc = avcodec_find_encoder(AV_CODEC_ID_AAC);
        if (!aenc) {
            status = OCA_PROXY_ERR_ENCODER;
            goto cleanup;
        }

        /* "anull": no actual filtering, just reuses init_audio_filter_chain's
           encoder-format-negotiation (sample_formats/samplerates constraint on the sink) so
           avcodec_open2() on the AAC encoder doesn't reject whatever raw format decoding
           happens to produce — see that function's doc comment. */
        if (init_audio_filter_chain(adec_ctx, aenc, "anull", &achain) < 0) {
            status = OCA_PROXY_ERR_FILTER_GRAPH;
            goto cleanup;
        }

        aenc_ctx = avcodec_alloc_context3(aenc);
        if (!aenc_ctx) {
            status = OCA_PROXY_ERR_ENCODER;
            goto cleanup;
        }
        aenc_ctx->sample_rate = av_buffersink_get_sample_rate(achain.buffersink_ctx);
        av_buffersink_get_ch_layout(achain.buffersink_ctx, &aenc_ctx->ch_layout);
        aenc_ctx->sample_fmt = (enum AVSampleFormat)av_buffersink_get_format(achain.buffersink_ctx);
        aenc_ctx->bit_rate = 128000;
        aenc_ctx->time_base = av_buffersink_get_time_base(achain.buffersink_ctx);
        if (out_ctx->oformat->flags & AVFMT_GLOBALHEADER) {
            aenc_ctx->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
        }
        if (avcodec_open2(aenc_ctx, aenc, NULL) < 0) {
            status = OCA_PROXY_ERR_ENCODER;
            goto cleanup;
        }
        if (aenc_ctx->frame_size > 0) {
            av_buffersink_set_frame_size(achain.buffersink_ctx, (unsigned)aenc_ctx->frame_size);
        }

        AVStream *out_stream = avformat_new_stream(out_ctx, NULL);
        if (!out_stream || avcodec_parameters_from_context(out_stream->codecpar, aenc_ctx) < 0) {
            status = OCA_PROXY_ERR_NEW_STREAM;
            goto cleanup;
        }
        out_stream->time_base = aenc_ctx->time_base;
        audio_out_index = out_stream->index;
    }

    if (!(out_ctx->oformat->flags & AVFMT_NOFILE)) {
        if (avio_open(&out_ctx->pb, out_path, AVIO_FLAG_WRITE) < 0) {
            status = OCA_PROXY_ERR_OPEN_OUTPUT;
            goto cleanup_output_io;
        }
    }
    if (avformat_write_header(out_ctx, NULL) < 0) {
        status = OCA_PROXY_ERR_WRITE_HEADER;
        goto cleanup_output_io;
    }

    pkt = av_packet_alloc();
    dec_frame = av_frame_alloc();
    scaled_frame = av_frame_alloc();
    filt_frame = av_frame_alloc();
    enc_pkt = av_packet_alloc();
    if (!pkt || !dec_frame || !scaled_frame || !filt_frame || !enc_pkt) {
        status = OCA_PROXY_ERR_PIPELINE;
        goto cleanup_output_io;
    }
    scaled_frame->format = venc_ctx->pix_fmt;
    scaled_frame->width = venc_ctx->width;
    scaled_frame->height = venc_ctx->height;
    if (av_frame_get_buffer(scaled_frame, 0) < 0) {
        status = OCA_PROXY_ERR_PIPELINE;
        goto cleanup_output_io;
    }

    while (av_read_frame(in_ctx, pkt) >= 0) {
        if (pkt->stream_index == video_in_index) {
            AVStream *out_stream = out_ctx->streams[video_out_index];
            int ret = avcodec_send_packet(vdec_ctx, pkt);
            av_packet_unref(pkt);
            if (ret < 0) {
                status = OCA_PROXY_ERR_PIPELINE;
                goto cleanup_output_io;
            }
            while (1) {
                ret = avcodec_receive_frame(vdec_ctx, dec_frame);
                if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF) break;
                if (ret < 0) {
                    status = OCA_PROXY_ERR_PIPELINE;
                    goto cleanup_output_io;
                }
                if (scale_video_frame(sws_ctx, dec_frame, scaled_frame, &next_video_pts) < 0 ||
                    encode_write_packet(out_ctx, venc_ctx, out_stream, scaled_frame, enc_pkt) <
                        0) {
                    status = OCA_PROXY_ERR_PIPELINE;
                    goto cleanup_output_io;
                }
                av_frame_unref(dec_frame);
            }
        } else if (pkt->stream_index == audio_in_index) {
            AVStream *out_stream = out_ctx->streams[audio_out_index];
            int ret = avcodec_send_packet(adec_ctx, pkt);
            av_packet_unref(pkt);
            if (ret < 0) {
                status = OCA_PROXY_ERR_PIPELINE;
                goto cleanup_output_io;
            }
            while (1) {
                ret = avcodec_receive_frame(adec_ctx, dec_frame);
                if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF) break;
                if (ret < 0) {
                    status = OCA_PROXY_ERR_PIPELINE;
                    goto cleanup_output_io;
                }
                if (filter_encode_write_frame(out_ctx, &achain, aenc_ctx, out_stream, dec_frame,
                                               filt_frame, enc_pkt) < 0) {
                    status = OCA_PROXY_ERR_PIPELINE;
                    goto cleanup_output_io;
                }
            }
        } else {
            av_packet_unref(pkt);
        }
    }

    /* Flush video: decoder -> scale+encode remaining buffered frames -> encoder. */
    {
        AVStream *out_stream = out_ctx->streams[video_out_index];
        avcodec_send_packet(vdec_ctx, NULL);
        while (avcodec_receive_frame(vdec_ctx, dec_frame) >= 0) {
            if (scale_video_frame(sws_ctx, dec_frame, scaled_frame, &next_video_pts) < 0 ||
                encode_write_packet(out_ctx, venc_ctx, out_stream, scaled_frame, enc_pkt) < 0) {
                status = OCA_PROXY_ERR_PIPELINE;
                goto cleanup_output_io;
            }
            av_frame_unref(dec_frame);
        }
        if (encode_write_packet(out_ctx, venc_ctx, out_stream, NULL, enc_pkt) < 0) {
            status = OCA_PROXY_ERR_PIPELINE;
            goto cleanup_output_io;
        }
    }

    /* Flush audio, if present: decoder -> filter -> encoder. */
    if (audio_in_index >= 0) {
        AVStream *out_stream = out_ctx->streams[audio_out_index];
        avcodec_send_packet(adec_ctx, NULL);
        while (avcodec_receive_frame(adec_ctx, dec_frame) >= 0) {
            if (filter_encode_write_frame(out_ctx, &achain, aenc_ctx, out_stream, dec_frame,
                                           filt_frame, enc_pkt) < 0) {
                status = OCA_PROXY_ERR_PIPELINE;
                goto cleanup_output_io;
            }
        }
        if (filter_encode_write_frame(out_ctx, &achain, aenc_ctx, out_stream, NULL, filt_frame,
                                       enc_pkt) < 0 ||
            encode_write_packet(out_ctx, aenc_ctx, out_stream, NULL, enc_pkt) < 0) {
            status = OCA_PROXY_ERR_PIPELINE;
            goto cleanup_output_io;
        }
    }

    if (status == OCA_PROXY_OK) {
        av_write_trailer(out_ctx);
    }

cleanup_output_io:
    av_packet_free(&pkt);
    av_packet_free(&enc_pkt);
    av_frame_free(&dec_frame);
    av_frame_free(&scaled_frame);
    av_frame_free(&filt_frame);
    if (out_ctx && !(out_ctx->oformat->flags & AVFMT_NOFILE)) {
        avio_closep(&out_ctx->pb);
    }
cleanup:
    if (sws_ctx) {
        sws_freeContext(sws_ctx);
    }
    free_audio_filter_chain(&achain);
    avcodec_free_context(&vdec_ctx);
    avcodec_free_context(&venc_ctx);
    avcodec_free_context(&adec_ctx);
    avcodec_free_context(&aenc_ctx);
    if (out_ctx) {
        avformat_free_context(out_ctx);
    }
    avformat_close_input(&in_ctx);
    return status;
}
