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

#include <libavcodec/avcodec.h>
#include <libavfilter/avfilter.h>
#include <libavfilter/buffersink.h>
#include <libavfilter/buffersrc.h>
#include <libavformat/avformat.h>
#include <libavutil/opt.h>

/* Maximum byte length of one movie+overlay filter fragment (per segment) -- sized to fit the
   escaped path plus up to three up-to-2048-byte escaped expression fragments (opacity, x, y
   position offset) with room to spare; the snprintf truncation check below is the real safety
   net regardless. */
#define TEXT_OVERLAY_SEG_MAX 8192

/* Escapes `in` for use as a single-quoted avfilter movie filename. Backslash, single-quote,
   colon, comma, semicolon, and graph-label brackets each get a backslash prefix.
   Writes into `out` (size `out_size`); returns bytes written excluding NUL, or -1 on
   buffer overflow. */
static int escape_filter_path(char *out, size_t out_size, const char *in) {
    size_t pos = 0;
    for (size_t i = 0; in[i] != '\0'; i++) {
        char c = in[i];
        int needs_escape =
            (c == '\\' || c == '\'' || c == ':' || c == ',' || c == ';' || c == '[' || c == ']');
        if (pos + (size_t)(needs_escape ? 2 : 1) + 1 > out_size) return -1;
        if (needs_escape) out[pos++] = '\\';
        out[pos++] = c;
    }
    out[pos] = '\0';
    return (int)pos;
}

/* Escapes `in` (a geq-expression-language fragment from
   avcore::keyframe::text_opacity_alpha_expr) for embedding as one avfilter option value inside
   the whole-graph chain description this file builds via avfilter_graph_parse_ptr -- only
   commas need escaping (the expression is built purely from digits/letters/operators/parens/
   dots/minus signs plus commas as `if`/`between`'s argument separators), since a bare comma at
   that level would otherwise end the current filter and start a new one in the chain. Writes
   into `out` (size `out_size`); returns bytes written excluding NUL, or -1 on buffer overflow. */
static int escape_filter_expr(char *out, size_t out_size, const char *in) {
    size_t pos = 0;
    for (size_t i = 0; in[i] != '\0'; i++) {
        char c = in[i];
        if (pos + (size_t)(c == ',' ? 2 : 1) + 1 > out_size) return -1;
        if (c == ',') out[pos++] = '\\';
        out[pos++] = c;
    }
    out[pos] = '\0';
    return (int)pos;
}

/* Opens `in_path` (an already-rendered H.264/AAC mp4 produced by
   avbridge_encode_timeline_export), composites pre-rasterized RGBA PNG overlays, and
   writes the result to `out_path`. Video is decoded, filtered, and re-encoded via
   libopenh264; audio is stream-copied unchanged to avoid a lossy second AAC encode.
   canvas_width/height and canvas_fps_num/den must match the rendered video so the output
   encoder is configured correctly.
   Returns TEXT_OVERLAY_OK immediately without touching any files when segment_count <= 0. */
TextOverlayStatus avbridge_apply_text_overlays(const char *in_path, const char *out_path,
                                               const TextSegment *segments, int segment_count,
                                               int canvas_width, int canvas_height,
                                               int canvas_fps_num, int canvas_fps_den) {

    if (segment_count <= 0) return TEXT_OVERLAY_OK;

    TextOverlayStatus status = TEXT_OVERLAY_OK;
    AVFormatContext *in_ctx = NULL;
    AVFormatContext *out_ctx = NULL;
    AVCodecContext *vdec_ctx = NULL;
    AVCodecContext *venc_ctx = NULL;
    AVFilterGraph *filter_graph = NULL;
    AVFilterContext *buffersrc_ctx = NULL;
    AVFilterContext *buffersink_ctx = NULL;
    AVPacket *pkt = NULL;
    AVFrame *dec_frame = NULL;
    AVFrame *filt_frame = NULL;
    AVPacket *enc_pkt = NULL;
    char *filter_str = NULL;
    int video_in_idx = -1, audio_in_idx = -1;
    int video_out_idx = -1, audio_out_idx = -1;
    int64_t next_video_pts = 0;
    AVRational canvas_fps = {canvas_fps_num, canvas_fps_den};

    /* Open input */
    if (open_input(in_path, &in_ctx) != 0) {
        status = TEXT_OVERLAY_ERR_OPEN_INPUT;
        goto cleanup;
    }
    for (unsigned int i = 0; i < in_ctx->nb_streams; i++) {
        enum AVMediaType mt = in_ctx->streams[i]->codecpar->codec_type;
        if (video_in_idx < 0 && mt == AVMEDIA_TYPE_VIDEO) video_in_idx = (int)i;
        if (audio_in_idx < 0 && mt == AVMEDIA_TYPE_AUDIO) audio_in_idx = (int)i;
    }
    if (video_in_idx < 0) {
        status = TEXT_OVERLAY_ERR_OPEN_INPUT;
        goto cleanup;
    }

    /* Open video decoder */
    {
        AVStream *vs = in_ctx->streams[video_in_idx];
        const AVCodec *vdec = avcodec_find_decoder(vs->codecpar->codec_id);
        if (!vdec) {
            status = TEXT_OVERLAY_ERR_OPEN_INPUT;
            goto cleanup;
        }
        vdec_ctx = avcodec_alloc_context3(vdec);
        if (!vdec_ctx) {
            status = TEXT_OVERLAY_ERR_OPEN_INPUT;
            goto cleanup;
        }
        avcodec_parameters_to_context(vdec_ctx, vs->codecpar);
        vdec_ctx->time_base = vs->time_base;
        if (avcodec_open2(vdec_ctx, vdec, NULL) < 0) {
            status = TEXT_OVERLAY_ERR_OPEN_INPUT;
            goto cleanup;
        }
    }

    /* Build one movie source and one timeline-enabled overlay node per pre-rasterized PNG, plus
       an optional alpha-multiply geq stage for segments with an opacity_keyframe_expr. */
    {
        size_t filter_buf_size = (size_t)segment_count * TEXT_OVERLAY_SEG_MAX + 8;
        filter_str = av_malloc(filter_buf_size);
        if (!filter_str) {
            status = TEXT_OVERLAY_ERR_FILTER_GRAPH;
            goto cleanup;
        }

        size_t pos = 0;
        for (int i = 0; i < segment_count; i++) {
            const TextSegment *seg = &segments[i];
            char escaped[1024];
            if (escape_filter_path(escaped, sizeof(escaped), seg->overlay_path) < 0) {
                status = TEXT_OVERLAY_ERR_FILTER_GRAPH;
                goto cleanup;
            }
            int has_fade = seg->opacity_keyframe_expr && seg->opacity_keyframe_expr[0] != '\0';
            char escaped_expr[2048];
            if (has_fade && escape_filter_expr(escaped_expr, sizeof(escaped_expr),
                                               seg->opacity_keyframe_expr) < 0) {
                status = TEXT_OVERLAY_ERR_FILTER_GRAPH;
                goto cleanup;
            }
            /* Unescaped and single-quoted, same convention as timeline_export_multi.c's
               build_overlay_vfilter -- an overlay x=/y= expression sits directly in its own
               quoted option value, not nested inside another quoted expression the way the
               fade's geq a=' ... ' value above is, so it needs no comma-escaping of its own. */
            const char *pos_expr_x =
                (seg->position_keyframe_expr_x && seg->position_keyframe_expr_x[0] != '\0')
                    ? seg->position_keyframe_expr_x
                    : "0";
            const char *pos_expr_y =
                (seg->position_keyframe_expr_y && seg->position_keyframe_expr_y[0] != '\0')
                    ? seg->position_keyframe_expr_y
                    : "0";
            double end_secs = seg->start_secs + seg->duration_secs;
            char main_label[32];
            char out_label[32];
            char text_label[32];
            if (i == 0)
                snprintf(main_label, sizeof(main_label), "in");
            else
                snprintf(main_label, sizeof(main_label), "v%d", i - 1);
            if (i == segment_count - 1)
                snprintf(out_label, sizeof(out_label), "out");
            else
                snprintf(out_label, sizeof(out_label), "v%d", i);
            snprintf(text_label, sizeof(text_label), "text%d", i);

            /* `repeatlast=1` holds the PNG's only frame for the full main-video timeline;
               enable='between(...)' controls the actual clip visibility window. When faded, a
               geq stage sits between the movie source and the overlay: alpha(X,Y) is the raw
               PNG's own per-pixel alpha, multiplied by the fade expression; r/g/b pass through
               unchanged. Confirmed for real against this project's linked FFmpeg build (a
               dedicated avbridge test, not just the docs) that the read-back function is
               spelled `alpha(X,Y)`, not `a(X,Y)` as FFmpeg's own geq docs otherwise imply --
               `a(X,Y)` parses as "Unknown function" here; two other candidates were ruled out
               the same way (colorchannelmixer has no `eval` option at all in this build; its
               `t`/`n` per-frame variables were never reached). See
               keyframe::text_opacity_alpha_expr's doc comment for the full story. */
            int written;
            if (has_fade) {
                written = snprintf(
                    filter_str + pos, filter_buf_size - pos,
                    "movie=filename='%s',format=rgba,"
                    "geq=r='r(X\\,Y)':g='g(X\\,Y)':b='b(X\\,Y)':a='alpha(X\\,Y)*(%s)'[%s];"
                    "[%s][%s]overlay=x='%s':y='%s':format=auto:alpha=straight:"
                    "repeatlast=1:eof_action=repeat:enable='between(t\\,%.4f\\,%.4f)'[%s]%s",
                    escaped, escaped_expr, text_label, main_label, text_label, pos_expr_x,
                    pos_expr_y, seg->start_secs, end_secs, out_label,
                    i == segment_count - 1 ? "" : ";");
            } else {
                written = snprintf(
                    filter_str + pos, filter_buf_size - pos,
                    "movie=filename='%s',format=rgba[%s];"
                    "[%s][%s]overlay=x='%s':y='%s':format=auto:alpha=straight:"
                    "repeatlast=1:eof_action=repeat:enable='between(t\\,%.4f\\,%.4f)'[%s]%s",
                    escaped, text_label, main_label, text_label, pos_expr_x, pos_expr_y,
                    seg->start_secs, end_secs, out_label, i == segment_count - 1 ? "" : ";");
            }
            if (written < 0 || pos + (size_t)written >= filter_buf_size) {
                status = TEXT_OVERLAY_ERR_FILTER_GRAPH;
                goto cleanup;
            }
            pos += (size_t)written;
        }
        filter_str[pos] = '\0';
    }

    /* Set up filtergraph: buffer + PNG movie sources -> overlay chain -> buffersink. */
    {
        AVStream *vs = in_ctx->streams[video_in_idx];
        filter_graph = avfilter_graph_alloc();
        if (!filter_graph) {
            status = TEXT_OVERLAY_ERR_FILTER_GRAPH;
            goto cleanup;
        }

        const AVFilter *buffersrc = avfilter_get_by_name("buffer");
        const AVFilter *buffersink = avfilter_get_by_name("buffersink");
        if (!buffersrc || !buffersink) {
            status = TEXT_OVERLAY_ERR_FILTER_GRAPH;
            goto cleanup;
        }

        char args[256];
        snprintf(args, sizeof(args),
                 "video_size=%dx%d:pix_fmt=%d:time_base=%d/%d:pixel_aspect=%d/%d", vdec_ctx->width,
                 vdec_ctx->height, (int)vdec_ctx->pix_fmt, vs->time_base.num, vs->time_base.den,
                 vdec_ctx->sample_aspect_ratio.num,
                 vdec_ctx->sample_aspect_ratio.den > 0 ? vdec_ctx->sample_aspect_ratio.den : 1);

        if (avfilter_graph_create_filter(&buffersrc_ctx, buffersrc, "in", args, NULL,
                                         filter_graph) < 0) {
            status = TEXT_OVERLAY_ERR_FILTER_GRAPH;
            goto cleanup;
        }

        /* overlay=format=auto may otherwise negotiate a packed/RGBA output when the PNG input
           has no opacity-expression geq stage. FFmpeg's libopenh264 wrapper forwards that
           frame as I420, leaving chroma planes/strides invalid and causing OpenH264's
           BuildSpatialPicList to reject the first frame. Constrain the sink to the same
           planar format configured on venc_ctx so the graph inserts conversion as needed.
           pixel_formats is a pre-init-only option, so allocate and initialize the sink
           separately. */
        buffersink_ctx = avfilter_graph_alloc_filter(filter_graph, buffersink, "out");
        if (!buffersink_ctx) {
            status = TEXT_OVERLAY_ERR_FILTER_GRAPH;
            goto cleanup;
        }
        enum AVPixelFormat sink_pix_fmt = AV_PIX_FMT_YUV420P;
        if (av_opt_set_array(buffersink_ctx, "pixel_formats", AV_OPT_SEARCH_CHILDREN, 0, 1,
                             AV_OPT_TYPE_PIXEL_FMT, &sink_pix_fmt) < 0 ||
            avfilter_init_str(buffersink_ctx, NULL) < 0) {
            status = TEXT_OVERLAY_ERR_FILTER_GRAPH;
            goto cleanup;
        }

        AVFilterInOut *filt_out = avfilter_inout_alloc();
        AVFilterInOut *filt_in = avfilter_inout_alloc();
        if (!filt_out || !filt_in) {
            avfilter_inout_free(&filt_out);
            avfilter_inout_free(&filt_in);
            status = TEXT_OVERLAY_ERR_FILTER_GRAPH;
            goto cleanup;
        }
        filt_out->name = av_strdup("in");
        filt_out->filter_ctx = buffersrc_ctx;
        filt_out->pad_idx = 0;
        filt_out->next = NULL;
        filt_in->name = av_strdup("out");
        filt_in->filter_ctx = buffersink_ctx;
        filt_in->pad_idx = 0;
        filt_in->next = NULL;

        int ret = avfilter_graph_parse_ptr(filter_graph, filter_str, &filt_in, &filt_out, NULL);
        avfilter_inout_free(&filt_in);
        avfilter_inout_free(&filt_out);
        if (ret < 0 || avfilter_graph_config(filter_graph, NULL) < 0) {
            status = TEXT_OVERLAY_ERR_FILTER_GRAPH;
            goto cleanup;
        }
    }

    /* Set up output context: libopenh264 for video, stream-copy for audio */
    {
        avformat_alloc_output_context2(&out_ctx, NULL, NULL, out_path);
        if (!out_ctx) {
            status = TEXT_OVERLAY_ERR_ALLOC_OUTPUT;
            goto cleanup;
        }

        const AVCodec *venc = avcodec_find_encoder_by_name("libopenh264");
        if (!venc) {
            status = TEXT_OVERLAY_ERR_PIPELINE;
            goto cleanup;
        }
        venc_ctx = avcodec_alloc_context3(venc);
        if (!venc_ctx) {
            status = TEXT_OVERLAY_ERR_PIPELINE;
            goto cleanup;
        }

        AVStream *vin = in_ctx->streams[video_in_idx];
        venc_ctx->width = canvas_width;
        venc_ctx->height = canvas_height;
        venc_ctx->pix_fmt = AV_PIX_FMT_YUV420P;
        venc_ctx->time_base = av_inv_q(canvas_fps);
        venc_ctx->framerate = canvas_fps;
        venc_ctx->bit_rate = vin->codecpar->bit_rate > 0 ? vin->codecpar->bit_rate : 4000000LL;
        venc_ctx->gop_size = canvas_fps_den > 0 ? (canvas_fps_num / canvas_fps_den) * 2 : 60;
        venc_ctx->max_b_frames = 0;
        if (out_ctx->oformat->flags & AVFMT_GLOBALHEADER)
            venc_ctx->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
        if (avcodec_open2(venc_ctx, venc, NULL) < 0) {
            status = TEXT_OVERLAY_ERR_PIPELINE;
            goto cleanup;
        }
        AVStream *vout = avformat_new_stream(out_ctx, NULL);
        if (!vout || avcodec_parameters_from_context(vout->codecpar, venc_ctx) < 0) {
            status = TEXT_OVERLAY_ERR_ALLOC_OUTPUT;
            goto cleanup;
        }
        vout->time_base = venc_ctx->time_base;
        video_out_idx = vout->index;

        if (audio_in_idx >= 0) {
            AVStream *ain = in_ctx->streams[audio_in_idx];
            AVStream *aout = avformat_new_stream(out_ctx, NULL);
            if (!aout || avcodec_parameters_copy(aout->codecpar, ain->codecpar) < 0) {
                status = TEXT_OVERLAY_ERR_ALLOC_OUTPUT;
                goto cleanup;
            }
            aout->time_base = ain->time_base;
            audio_out_idx = aout->index;
        }

        if (!(out_ctx->oformat->flags & AVFMT_NOFILE)) {
            if (avio_open(&out_ctx->pb, out_path, AVIO_FLAG_WRITE) < 0) {
                status = TEXT_OVERLAY_ERR_ALLOC_OUTPUT;
                goto cleanup;
            }
        }
        if (avformat_write_header(out_ctx, NULL) < 0) {
            status = TEXT_OVERLAY_ERR_ALLOC_OUTPUT;
            goto cleanup_output_io;
        }
    }

    pkt = av_packet_alloc();
    dec_frame = av_frame_alloc();
    filt_frame = av_frame_alloc();
    enc_pkt = av_packet_alloc();
    if (!pkt || !dec_frame || !filt_frame || !enc_pkt) {
        status = TEXT_OVERLAY_ERR_PIPELINE;
        goto cleanup_output_io;
    }

    /* Main decode -> filter -> encode loop */
    while (av_read_frame(in_ctx, pkt) >= 0) {
        if (pkt->stream_index == video_in_idx) {
            int ret = avcodec_send_packet(vdec_ctx, pkt);
            av_packet_unref(pkt);
            if (ret < 0) {
                status = TEXT_OVERLAY_ERR_PIPELINE;
                goto cleanup_output_io;
            }
            while (1) {
                ret = avcodec_receive_frame(vdec_ctx, dec_frame);
                if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF) break;
                if (ret < 0) {
                    status = TEXT_OVERLAY_ERR_PIPELINE;
                    goto cleanup_output_io;
                }
                dec_frame->pts = dec_frame->best_effort_timestamp;
                ret = av_buffersrc_add_frame_flags(buffersrc_ctx, dec_frame,
                                                   AV_BUFFERSRC_FLAG_KEEP_REF);
                av_frame_unref(dec_frame);
                if (ret < 0) {
                    status = TEXT_OVERLAY_ERR_PIPELINE;
                    goto cleanup_output_io;
                }
                while (1) {
                    ret = av_buffersink_get_frame(buffersink_ctx, filt_frame);
                    if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF) break;
                    if (ret < 0) {
                        av_frame_unref(filt_frame);
                        status = TEXT_OVERLAY_ERR_PIPELINE;
                        goto cleanup_output_io;
                    }
                    filt_frame->pts = next_video_pts++;
                    filt_frame->pict_type = AV_PICTURE_TYPE_NONE;
                    if (encode_write_packet(out_ctx, venc_ctx, out_ctx->streams[video_out_idx],
                                            filt_frame, enc_pkt) < 0) {
                        av_frame_unref(filt_frame);
                        status = TEXT_OVERLAY_ERR_PIPELINE;
                        goto cleanup_output_io;
                    }
                    av_frame_unref(filt_frame);
                }
            }
        } else if (pkt->stream_index == audio_in_idx && audio_out_idx >= 0) {
            /* Audio: rescale timestamps and stream-copy to avoid a second lossy AAC encode */
            AVStream *ain = in_ctx->streams[audio_in_idx];
            AVStream *aout = out_ctx->streams[audio_out_idx];
            av_packet_rescale_ts(pkt, ain->time_base, aout->time_base);
            pkt->stream_index = audio_out_idx;
            if (av_interleaved_write_frame(out_ctx, pkt) < 0) {
                av_packet_unref(pkt);
                status = TEXT_OVERLAY_ERR_PIPELINE;
                goto cleanup_output_io;
            }
        } else {
            av_packet_unref(pkt);
        }
    }

    /* Flush decoder into filtergraph */
    avcodec_send_packet(vdec_ctx, NULL);
    while (avcodec_receive_frame(vdec_ctx, dec_frame) >= 0) {
        dec_frame->pts = dec_frame->best_effort_timestamp;
        if (av_buffersrc_add_frame_flags(buffersrc_ctx, dec_frame, AV_BUFFERSRC_FLAG_KEEP_REF) >=
            0) {
            while (av_buffersink_get_frame(buffersink_ctx, filt_frame) >= 0) {
                filt_frame->pts = next_video_pts++;
                filt_frame->pict_type = AV_PICTURE_TYPE_NONE;
                encode_write_packet(out_ctx, venc_ctx, out_ctx->streams[video_out_idx], filt_frame,
                                    enc_pkt);
                av_frame_unref(filt_frame);
            }
        }
        av_frame_unref(dec_frame);
    }
    /* Flush filtergraph */
    av_buffersrc_add_frame_flags(buffersrc_ctx, NULL, 0);
    while (av_buffersink_get_frame(buffersink_ctx, filt_frame) >= 0) {
        filt_frame->pts = next_video_pts++;
        filt_frame->pict_type = AV_PICTURE_TYPE_NONE;
        encode_write_packet(out_ctx, venc_ctx, out_ctx->streams[video_out_idx], filt_frame,
                            enc_pkt);
        av_frame_unref(filt_frame);
    }
    /* Flush encoder */
    encode_write_packet(out_ctx, venc_ctx, out_ctx->streams[video_out_idx], NULL, enc_pkt);

    if (status == TEXT_OVERLAY_OK) av_write_trailer(out_ctx);

cleanup_output_io:
    av_packet_free(&pkt);
    av_packet_free(&enc_pkt);
    av_frame_free(&dec_frame);
    av_frame_free(&filt_frame);
    if (out_ctx && !(out_ctx->oformat->flags & AVFMT_NOFILE)) avio_closep(&out_ctx->pb);
cleanup:
    av_free(filter_str);
    avfilter_graph_free(&filter_graph);
    avcodec_free_context(&vdec_ctx);
    avcodec_free_context(&venc_ctx);
    if (out_ctx) avformat_free_context(out_ctx);
    avformat_close_input(&in_ctx);
    return status;
}
