// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.

#include "bridge.h"
#include "bridge_internal.h"

#include <math.h>

#include <libavcodec/avcodec.h>
#include <libavfilter/avfilter.h>
#include <libavfilter/buffersink.h>
#include <libavformat/avformat.h>
#include <libavutil/channel_layout.h>
#include <libavutil/opt.h>

static int first_stream_index(AVFormatContext *ctx, enum AVMediaType type) {
    for (unsigned i = 0; i < ctx->nb_streams; i++) {
        if (ctx->streams[i]->codecpar->codec_type == type) return (int)i;
    }
    return -1;
}

static int source_has_audio(const char *path, int *open_failed) {
    AVFormatContext *ctx = NULL;
    *open_failed = 0;
    if (open_input(path, &ctx) != 0) {
        *open_failed = 1;
        return 0;
    }
    int found = first_stream_index(ctx, AVMEDIA_TYPE_AUDIO) >= 0;
    avformat_close_input(&ctx);
    return found;
}

static int create_filter(AVFilterGraph *graph, AVFilterContext **ctx,
                         const char *filter_name, const char *instance_name,
                         const char *args) {
    const AVFilter *filter = avfilter_get_by_name(filter_name);
    if (!filter) return AVERROR_FILTER_NOT_FOUND;
    return avfilter_graph_create_filter(ctx, filter, instance_name, args, NULL, graph);
}

static int build_mix_graph(const AudioSegment *segments, const uint8_t *valid, int segment_count,
                           int valid_count, double timeline_duration_secs, float target_lufs,
                           AVFilterGraph **out_graph, AVFilterContext **out_sink) {
    int ret = 0;
    int branch = 0;
    AVFilterGraph *graph = avfilter_graph_alloc();
    AVFilterContext *mix = NULL, *noise = NULL, *loudnorm = NULL, *limiter = NULL;
    AVFilterContext *final_trim = NULL, *format = NULL, *sink = NULL;
    if (!graph) return AVERROR(ENOMEM);

    char args[256];
    snprintf(args, sizeof(args),
             "inputs=%d:duration=longest:dropout_transition=0:normalize=0", valid_count);
    ret = create_filter(graph, &mix, "amix", "mix", args);
    if (ret < 0) goto fail;

    for (int i = 0; i < segment_count; i++) {
        if (!valid[i]) continue;
        const AudioSegment *seg = &segments[i];
        char name[64];
        AVFilterContext *movie = NULL, *trim = NULL, *pts = NULL;
        AVFilterContext *tempo = NULL, *volume = NULL, *delay = NULL;

        snprintf(name, sizeof(name), "movie_%d", branch);
        const AVFilter *movie_filter = avfilter_get_by_name("amovie");
        if (!movie_filter) { ret = AVERROR_FILTER_NOT_FOUND; goto fail; }
        movie = avfilter_graph_alloc_filter(graph, movie_filter, name);
        if (!movie) { ret = AVERROR(ENOMEM); goto fail; }
        ret = av_opt_set(movie, "filename", seg->source_path, AV_OPT_SEARCH_CHILDREN);
        if (ret < 0 || (ret = avfilter_init_str(movie, NULL)) < 0) goto fail;

        snprintf(name, sizeof(name), "trim_%d", branch);
        snprintf(args, sizeof(args), "start=%.9f:end=%.9f",
                 seg->source_in_secs, seg->source_out_secs);
        if ((ret = create_filter(graph, &trim, "atrim", name, args)) < 0) goto fail;

        snprintf(name, sizeof(name), "pts_%d", branch);
        if ((ret = create_filter(graph, &pts, "asetpts", name, "PTS-STARTPTS")) < 0) goto fail;

        double speed = seg->speed_factor > 0.0f ? seg->speed_factor : 1.0;
        if (speed < 0.5) speed = 0.5;
        if (speed > 100.0) speed = 100.0;
        snprintf(name, sizeof(name), "tempo_%d", branch);
        snprintf(args, sizeof(args), "tempo=%.9f", speed);
        if ((ret = create_filter(graph, &tempo, "atempo", name, args)) < 0) goto fail;

        snprintf(name, sizeof(name), "volume_%d", branch);
        snprintf(args, sizeof(args), "volume=%.6fdB", (double)seg->gain_db);
        if ((ret = create_filter(graph, &volume, "volume", name, args)) < 0) goto fail;

        double delay_ms = fmax(0.0, seg->timeline_start_secs * 1000.0);
        snprintf(name, sizeof(name), "delay_%d", branch);
        snprintf(args, sizeof(args), "delays=%.3f:all=1", delay_ms);
        if ((ret = create_filter(graph, &delay, "adelay", name, args)) < 0) goto fail;

        if ((ret = avfilter_link(movie, 0, trim, 0)) < 0 ||
            (ret = avfilter_link(trim, 0, pts, 0)) < 0 ||
            (ret = avfilter_link(pts, 0, tempo, 0)) < 0 ||
            (ret = avfilter_link(tempo, 0, volume, 0)) < 0 ||
            (ret = avfilter_link(volume, 0, delay, 0)) < 0 ||
            (ret = avfilter_link(delay, 0, mix, branch)) < 0) goto fail;
        branch++;
    }

    if ((ret = create_filter(graph, &noise, "afftdn", "noise", NULL)) < 0) goto fail;
    snprintf(args, sizeof(args), "I=%.1f:TP=-1.0:LRA=11", (double)target_lufs);
    if ((ret = create_filter(graph, &loudnorm, "loudnorm", "loudnorm", args)) < 0) goto fail;
    if ((ret = create_filter(graph, &limiter, "alimiter", "limiter",
                             "limit=0.95:attack=5:release=50")) < 0) goto fail;
    snprintf(args, sizeof(args), "end=%.9f", timeline_duration_secs);
    if ((ret = create_filter(graph, &final_trim, "atrim", "timeline_trim", args)) < 0) goto fail;
    if ((ret = create_filter(graph, &format, "aformat", "format",
                             "sample_fmts=fltp:sample_rates=48000:channel_layouts=stereo")) < 0)
        goto fail;
    if ((ret = create_filter(graph, &sink, "abuffersink", "out", NULL)) < 0) goto fail;

    if ((ret = avfilter_link(mix, 0, noise, 0)) < 0 ||
        (ret = avfilter_link(noise, 0, loudnorm, 0)) < 0 ||
        (ret = avfilter_link(loudnorm, 0, limiter, 0)) < 0 ||
        (ret = avfilter_link(limiter, 0, final_trim, 0)) < 0 ||
        (ret = avfilter_link(final_trim, 0, format, 0)) < 0 ||
        (ret = avfilter_link(format, 0, sink, 0)) < 0 ||
        (ret = avfilter_graph_config(graph, NULL)) < 0) goto fail;

    *out_graph = graph;
    *out_sink = sink;
    return 0;

fail:
    avfilter_graph_free(&graph);
    return ret;
}

AudioMixStatus avbridge_mix_audio_timeline(
    const AudioSegment *segments, int segment_count, double timeline_duration_secs,
    const char *out_path, float target_lufs, const uint8_t *cancel) {
    if (!segments || segment_count <= 0) return AUDIO_MIX_ERR_NO_AUDIO;

    AudioMixStatus status = AUDIO_MIX_OK;
    uint8_t *valid = av_calloc((size_t)segment_count, sizeof(*valid));
    AVFilterGraph *graph = NULL;
    AVFilterContext *sink = NULL;
    AVFormatContext *out_ctx = NULL;
    AVCodecContext *enc_ctx = NULL;
    AVStream *out_stream = NULL;
    AVFrame *frame = NULL;
    AVPacket *pkt = NULL;
    int valid_count = 0;
    if (!valid) return AUDIO_MIX_ERR_FILTER_GRAPH;

    for (int i = 0; i < segment_count; i++) {
        int open_failed = 0;
        if (source_has_audio(segments[i].source_path, &open_failed)) {
            valid[i] = 1;
            valid_count++;
        } else if (open_failed) {
            status = AUDIO_MIX_ERR_OPEN_INPUT;
            goto cleanup;
        }
    }
    if (valid_count == 0) { status = AUDIO_MIX_ERR_NO_AUDIO; goto cleanup; }

    if (build_mix_graph(segments, valid, segment_count, valid_count,
                        timeline_duration_secs, target_lufs, &graph, &sink) < 0) {
        status = AUDIO_MIX_ERR_FILTER_GRAPH;
        goto cleanup;
    }

    avformat_alloc_output_context2(&out_ctx, NULL, NULL, out_path);
    if (!out_ctx) { status = AUDIO_MIX_ERR_ALLOC_OUTPUT; goto cleanup; }
    const AVCodec *encoder = avcodec_find_encoder(AV_CODEC_ID_AAC);
    if (!encoder) { status = AUDIO_MIX_ERR_ENCODER; goto cleanup; }
    enc_ctx = avcodec_alloc_context3(encoder);
    if (!enc_ctx) { status = AUDIO_MIX_ERR_ENCODER; goto cleanup; }
    enc_ctx->sample_rate = 48000;
    enc_ctx->sample_fmt = AV_SAMPLE_FMT_FLTP;
    av_channel_layout_default(&enc_ctx->ch_layout, 2);
    enc_ctx->bit_rate = 192000;
    enc_ctx->time_base = (AVRational){1, 48000};
    if (out_ctx->oformat->flags & AVFMT_GLOBALHEADER) enc_ctx->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
    if (avcodec_open2(enc_ctx, encoder, NULL) < 0) { status = AUDIO_MIX_ERR_ENCODER; goto cleanup; }
    if (enc_ctx->frame_size > 0) av_buffersink_set_frame_size(sink, (unsigned)enc_ctx->frame_size);

    out_stream = avformat_new_stream(out_ctx, NULL);
    if (!out_stream || avcodec_parameters_from_context(out_stream->codecpar, enc_ctx) < 0) {
        status = AUDIO_MIX_ERR_ALLOC_OUTPUT;
        goto cleanup;
    }
    out_stream->time_base = enc_ctx->time_base;
    if (!(out_ctx->oformat->flags & AVFMT_NOFILE) &&
        avio_open(&out_ctx->pb, out_path, AVIO_FLAG_WRITE) < 0) {
        status = AUDIO_MIX_ERR_OPEN_OUTPUT;
        goto cleanup;
    }
    if (avformat_write_header(out_ctx, NULL) < 0) {
        status = AUDIO_MIX_ERR_WRITE_HEADER;
        goto cleanup;
    }

    frame = av_frame_alloc();
    pkt = av_packet_alloc();
    if (!frame || !pkt) { status = AUDIO_MIX_ERR_PIPELINE; goto cleanup; }
    AVRational sink_tb = av_buffersink_get_time_base(sink);
    while (1) {
        if (cancel && *cancel) { status = AUDIO_MIX_CANCELLED; goto cleanup; }
        int ret = av_buffersink_get_frame(sink, frame);
        if (ret == AVERROR_EOF) break;
        if (ret == AVERROR(EAGAIN)) continue;
        if (ret < 0) { status = AUDIO_MIX_ERR_PIPELINE; goto cleanup; }
        frame->pts = av_rescale_q(frame->pts, sink_tb, enc_ctx->time_base);
        if (encode_write_packet(out_ctx, enc_ctx, out_stream, frame, pkt) < 0) {
            av_frame_unref(frame);
            status = AUDIO_MIX_ERR_PIPELINE;
            goto cleanup;
        }
        av_frame_unref(frame);
    }
    if (encode_write_packet(out_ctx, enc_ctx, out_stream, NULL, pkt) < 0) {
        status = AUDIO_MIX_ERR_PIPELINE;
        goto cleanup;
    }
    if (av_write_trailer(out_ctx) < 0) status = AUDIO_MIX_ERR_PIPELINE;

cleanup:
    av_packet_free(&pkt);
    av_frame_free(&frame);
    avcodec_free_context(&enc_ctx);
    avfilter_graph_free(&graph);
    if (out_ctx && !(out_ctx->oformat->flags & AVFMT_NOFILE) && out_ctx->pb) avio_closep(&out_ctx->pb);
    if (out_ctx) avformat_free_context(out_ctx);
    av_free(valid);
    return status;
}

static int read_next_stream_packet(AVFormatContext *ctx, int stream_index, AVPacket *pkt) {
    while (av_read_frame(ctx, pkt) >= 0) {
        if (pkt->stream_index == stream_index) return 1;
        av_packet_unref(pkt);
    }
    return 0;
}

static int64_t packet_ts(const AVPacket *pkt) {
    return pkt->dts != AV_NOPTS_VALUE ? pkt->dts : pkt->pts;
}

MediaMuxStatus avbridge_mux_video_audio(
    const char *video_path, const char *audio_path, const char *out_path) {
    AVFormatContext *video_ctx = NULL, *audio_ctx = NULL, *out_ctx = NULL;
    AVPacket *video_pkt = NULL, *audio_pkt = NULL;
    MediaMuxStatus status = MEDIA_MUX_OK;

    if (open_input(video_path, &video_ctx) != 0 || open_input(audio_path, &audio_ctx) != 0) {
        status = MEDIA_MUX_ERR_OPEN_INPUT;
        goto cleanup;
    }
    int video_idx = first_stream_index(video_ctx, AVMEDIA_TYPE_VIDEO);
    int audio_idx = first_stream_index(audio_ctx, AVMEDIA_TYPE_AUDIO);
    if (video_idx < 0 || audio_idx < 0) { status = MEDIA_MUX_ERR_MISSING_STREAM; goto cleanup; }

    avformat_alloc_output_context2(&out_ctx, NULL, NULL, out_path);
    if (!out_ctx) { status = MEDIA_MUX_ERR_ALLOC_OUTPUT; goto cleanup; }
    AVStream *video_out = avformat_new_stream(out_ctx, NULL);
    AVStream *audio_out = avformat_new_stream(out_ctx, NULL);
    if (!video_out || !audio_out ||
        avcodec_parameters_copy(video_out->codecpar, video_ctx->streams[video_idx]->codecpar) < 0 ||
        avcodec_parameters_copy(audio_out->codecpar, audio_ctx->streams[audio_idx]->codecpar) < 0) {
        status = MEDIA_MUX_ERR_NEW_STREAM;
        goto cleanup;
    }
    video_out->codecpar->codec_tag = 0;
    audio_out->codecpar->codec_tag = 0;
    video_out->time_base = video_ctx->streams[video_idx]->time_base;
    audio_out->time_base = audio_ctx->streams[audio_idx]->time_base;
    if (!(out_ctx->oformat->flags & AVFMT_NOFILE) &&
        avio_open(&out_ctx->pb, out_path, AVIO_FLAG_WRITE) < 0) {
        status = MEDIA_MUX_ERR_OPEN_OUTPUT;
        goto cleanup;
    }
    if (avformat_write_header(out_ctx, NULL) < 0) {
        status = MEDIA_MUX_ERR_WRITE_HEADER;
        goto cleanup;
    }

    video_pkt = av_packet_alloc();
    audio_pkt = av_packet_alloc();
    if (!video_pkt || !audio_pkt) { status = MEDIA_MUX_ERR_WRITE_FRAME; goto cleanup; }
    int have_video = read_next_stream_packet(video_ctx, video_idx, video_pkt);
    int have_audio = read_next_stream_packet(audio_ctx, audio_idx, audio_pkt);
    while (have_video || have_audio) {
        int write_video = have_video && (!have_audio ||
            av_compare_ts(packet_ts(video_pkt), video_ctx->streams[video_idx]->time_base,
                          packet_ts(audio_pkt), audio_ctx->streams[audio_idx]->time_base) <= 0);
        AVPacket *pkt = write_video ? video_pkt : audio_pkt;
        AVStream *in_stream = write_video ? video_ctx->streams[video_idx]
                                          : audio_ctx->streams[audio_idx];
        AVStream *out_stream = write_video ? video_out : audio_out;
        pkt->stream_index = out_stream->index;
        av_packet_rescale_ts(pkt, in_stream->time_base, out_stream->time_base);
        pkt->pos = -1;
        if (av_interleaved_write_frame(out_ctx, pkt) < 0) {
            status = MEDIA_MUX_ERR_WRITE_FRAME;
            goto cleanup;
        }
        if (write_video) have_video = read_next_stream_packet(video_ctx, video_idx, video_pkt);
        else have_audio = read_next_stream_packet(audio_ctx, audio_idx, audio_pkt);
    }
    if (av_write_trailer(out_ctx) < 0) status = MEDIA_MUX_ERR_WRITE_FRAME;

cleanup:
    av_packet_free(&video_pkt);
    av_packet_free(&audio_pkt);
    if (out_ctx && !(out_ctx->oformat->flags & AVFMT_NOFILE) && out_ctx->pb) avio_closep(&out_ctx->pb);
    if (out_ctx) avformat_free_context(out_ctx);
    avformat_close_input(&video_ctx);
    avformat_close_input(&audio_ctx);
    return status;
}
