#include "bridge.h"
#include "bridge_internal.h"

#include <math.h>

#include <libavcodec/avcodec.h>
#include <libavfilter/avfilter.h>
#include <libavfilter/buffersink.h>
#include <libavformat/avformat.h>
#include <libavutil/channel_layout.h>

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
