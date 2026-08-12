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
                     "volume@vol=0dB,loudnorm=I=%.1f:TP=-1.0:LRA=11,alimiter=limit=0.95:attack="
                     "5:release=50",
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
        }

        /* Per-segment video filter chain: canvas-conform (scale/pad/fps, so every segment
           lands on the same output dimensions/frame rate) + this clip's own effect filters +
           a final format lock so the encoder always receives yuv420p regardless of what the
           clip filters produce. Rebuilt every segment since the clip filter differs. */
        {
            char vfilter_descr[1024];
            const char *clip_filter =
                (seg->video_filter && seg->video_filter[0]) ? seg->video_filter : "";
            snprintf(vfilter_descr, sizeof(vfilter_descr),
                     "scale=%d:%d:force_original_aspect_ratio=decrease,pad=%d:%d:(ow-iw)/2:(oh-"
                     "ih)/2,fps=%d/%d%s%s,format=yuv420p",
                     canvas_width, canvas_height, canvas_width, canvas_height, canvas_fps.num,
                     canvas_fps.den, clip_filter[0] ? "," : "", clip_filter);
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
