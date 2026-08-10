#include "bridge.h"

#include <stdio.h>
#include <string.h>

#include <libavcodec/avcodec.h>
#include <libavfilter/avfilter.h>
#include <libavfilter/buffersink.h>
#include <libavfilter/buffersrc.h>
#include <libavformat/avformat.h>
#include <libavutil/channel_layout.h>
#include <libavutil/opt.h>

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

OcaRemuxStatus oca_avbridge_remux_copy(const char *in_path, const char *out_path) {
    AVFormatContext *in_ctx = NULL;
    AVFormatContext *out_ctx = NULL;
    int *stream_mapping = NULL;
    OcaRemuxStatus status = OCA_REMUX_OK;

    if (avformat_open_input(&in_ctx, in_path, NULL, NULL) < 0) {
        return OCA_REMUX_ERR_OPEN_INPUT;
    }
    if (avformat_find_stream_info(in_ctx, NULL) < 0) {
        avformat_close_input(&in_ctx);
        return OCA_REMUX_ERR_STREAM_INFO;
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

/* Builds `loudnorm=I=<target>:TP=-1.0:LRA=11,alimiter=...` between an abuffer source shaped
   like dec_ctx's decoded audio and an abuffersink constrained to a format `encoder` accepts
   (so the graph auto-inserts any needed aformat/aresample conversion, per FFmpeg's own
   transcode_aac.c example) — without this, avcodec_open2() on the AAC encoder can reject
   whatever raw format the filter chain happens to produce. */
static int init_audio_filter_chain(AVCodecContext *dec_ctx, const AVCodec *encoder,
                                    float target_lufs, AudioFilterChain *chain) {
    char args[512];
    char filter_descr[256];
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

    snprintf(filter_descr, sizeof(filter_descr),
             "loudnorm=I=%.1f:TP=-1.0:LRA=11,alimiter=limit=0.95:attack=5:release=50",
             (double)target_lufs);

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

OcaEncodeStatus oca_avbridge_encode_export(const char *in_path, const char *out_path,
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

    if (avformat_open_input(&in_ctx, in_path, NULL, NULL) < 0) {
        return OCA_ENCODE_ERR_OPEN_INPUT;
    }
    if (avformat_find_stream_info(in_ctx, NULL) < 0) {
        avformat_close_input(&in_ctx);
        return OCA_ENCODE_ERR_STREAM_INFO;
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

    if (init_audio_filter_chain(dec_ctx, encoder, target_lufs, &chain) < 0) {
        status = OCA_ENCODE_ERR_FILTER_GRAPH;
        goto cleanup;
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
