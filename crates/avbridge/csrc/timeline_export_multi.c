#include "bridge.h"
#include "bridge_internal.h"

#include <math.h>

#include <libavcodec/avcodec.h>
#include <libavfilter/avfilter.h>
#include <libavfilter/buffersink.h>
#include <libavfilter/buffersrc.h>
#include <libavformat/avformat.h>
#include <libavutil/channel_layout.h>

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

void oca_build_kenburns_zoom(const OcaClipSegment *seg, int fps_num, int fps_den,
                              char *buf, size_t cap) {
    buf[0] = '\0';
    float zs = seg->zoom_start > 0.0f ? seg->zoom_start : 1.0f;
    float ze = seg->zoom_end > 0.0f ? seg->zoom_end : 1.0f;
    if (zs < 0.1f) zs = 0.1f; if (zs > 20.0f) zs = 20.0f;
    if (ze < 0.1f) ze = 0.1f; if (ze > 20.0f) ze = 20.0f;
    if (fabsf(zs - 1.0f) <= 1e-4f && fabsf(ze - 1.0f) <= 1e-4f) {
        return;
    }

    AVRational canvas_fps = {fps_num, fps_den};
    double source_dur = seg->source_out_secs - seg->source_in_secs;
    double speed = seg->speed_factor > 0.0f ? seg->speed_factor : 1.0f;
    double timeline_dur = source_dur / speed;
    double total_frames = timeline_dur * (double)canvas_fps.num / (double)canvas_fps.den;
    if (total_frames < 1.0) total_frames = 1.0;
    double N = total_frames - 1.0;
    if (N < 1.0) N = 1.0;
    double A = zs, B = ((double)ze - (double)zs) / N;

    if (fabs(B) < 1e-9) {
        /* Static zoom (zoom_start == zoom_end): a plain crop+scale, no frame variable needed,
           so none of the animated case's concerns below apply. */
        snprintf(buf, cap,
                 "crop=iw/%.5f:ih/%.5f:iw*(1-1/%.5f)/2:ih*(1-1/%.5f)/2,scale=iw*%.5f:ih*%.5f",
                 A, A, A, A, A, A);
        return;
    }

    /* Animated Ken-Burns zoom. Originally `crop=iw/(A+B*n):...,scale=...` — neither `crop` nor
       `scale` here set `eval=frame`, and this FFmpeg build flatly rejects a frame variable
       ("n") in a filter's default "init" eval mode ("Expressions with frame variables 'n',
       't', 'pos' are not valid in init eval_mode") — so any export actually using a
       non-degenerate zoom (B != 0) failed outright with OCA_ENCODE_ERR_FILTER_GRAPH. No
       existing test caught this: every zoom-bearing fixture in this codebase happens to use
       zoom_start == zoom_end (the B == 0 branch above). Reimplemented as a geq inverse-sample,
       the same technique the Slide/Zoom transition cases use (see
       avbridge_encode_timeline_export's per-segment transition block) and for the same reason:
       letting crop/scale actually renegotiate output size per frame is what reliably corrupted
       the heap there, not just a syntax problem. z is the same A+B*N zoom factor the old
       crop/scale pair used; (sx,sy) is (X,Y) mapped back through an inverse zoom around the
       frame center by z. The "inside" clamp only matters for a downward zoom (z<1, "zoom out
       past 1.0") which the original crop=iw/z formula couldn't represent either (crop can't
       grow past its input size) — here it just shows black padding instead of undefined
       behavior; it's a no-op multiplier (always 1) for the far more common z>=1 "push in" case
       this feature is meant for. */
    char z[220], sx[280], sy[280], inside[820];
    snprintf(z, sizeof(z), "(%.7f+%.9f*N)", A, B);
    snprintf(sx, sizeof(sx), "((X-W/2)/%s+W/2)", z);
    snprintf(sy, sizeof(sy), "((Y-H/2)/%s+H/2)", z);
    snprintf(inside, sizeof(inside), "(1-lt(%s,0))*lt(%s,W)*(1-lt(%s,0))*lt(%s,H)",
             sx, sx, sy, sy);
    snprintf(buf, cap,
             "geq=lum='p(%s,%s)*%s'"
             ":cb='128+(cb(%s,%s)-128)*%s'"
             ":cr='128+(cr(%s,%s)-128)*%s'",
             sx, sy, inside, sx, sy, inside, sx, sy, inside);
}

/* Build the complete single-track video filter string for `seg` — identical logic to
   the inline filter-string block inside avbridge_encode_timeline_export, extracted here
   so the multi-track function can reuse it for single-track fallback intervals. */
static void oca_build_vfilter_descr(const OcaClipSegment *seg,
                                     int cw, int ch, int fps_num, int fps_den,
                                     char *buf, size_t cap) {
    const char *cf = (seg->video_filter && seg->video_filter[0]) ? seg->video_filter : "";
    char setpts[48] = "";
    if (fabsf(seg->speed_factor - 1.0f) > 1e-4f && seg->speed_factor > 0.0f)
        snprintf(setpts, sizeof(setpts), "setpts=PTS/%.6f,", (double)seg->speed_factor);

    char zoom[2048];
    oca_build_kenburns_zoom(seg, fps_num, fps_den, zoom, sizeof(zoom));

    char trans[2048] = "";
    if (seg->transition_in != 0) {
        double tf = (double)seg->transition_duration_secs * (double)fps_num / (double)fps_den;
        if (tf < 1.0) tf = 1.0;
        switch (seg->transition_in) {
            case 1: snprintf(trans, sizeof(trans), "fade=t=in:st=0:d=%.4f",
                             (double)seg->transition_duration_secs); break;
            /* See the matching cases in timeline_export.c's per-segment transition block for
               why Slide uses geq instead of drawbox, and why Zoom uses a geq inverse-sample
               instead of a dynamically resizing scale+pad (the latter reliably corrupted the
               heap when actually run through a real export). */
            case 2: snprintf(trans, sizeof(trans),
                             "geq=lum='p(X,Y)*lt(X,min(W,N*W/%g))'"
                             ":cb='128+(cb(X,Y)-128)*lt(X,min(W,N*W/%g))'"
                             ":cr='128+(cr(X,Y)-128)*lt(X,min(W,N*W/%g))'",
                             tf, tf, tf); break;
            case 3: {
                char z[160], sx[224], sy[224], inside[768];
                snprintf(z, sizeof(z), "(0.5+0.5*lt(N,%g)*N/%g+0.5*gte(N,%g))", tf, tf, tf);
                snprintf(sx, sizeof(sx), "((X-W/2)/%s+W/2)", z);
                snprintf(sy, sizeof(sy), "((Y-H/2)/%s+H/2)", z);
                snprintf(inside, sizeof(inside), "(1-lt(%s,0))*lt(%s,W)*(1-lt(%s,0))*lt(%s,H)",
                         sx, sx, sy, sy);
                snprintf(trans, sizeof(trans),
                         "geq=lum='p(%s,%s)*%s'"
                         ":cb='128+(cb(%s,%s)-128)*%s'"
                         ":cr='128+(cr(%s,%s)-128)*%s'",
                         sx, sy, inside, sx, sy, inside, sx, sy, inside);
                break;
            }
            default: break;
        }
    }

    char post[4096] = "";
    if (zoom[0] && cf[0])   snprintf(post, sizeof(post), "%s,%s", zoom, cf);
    else if (zoom[0])        snprintf(post, sizeof(post), "%s", zoom);
    else if (cf[0])          snprintf(post, sizeof(post), "%s", cf);
    char chain[8192] = "";
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

    char zoom[2048];
    oca_build_kenburns_zoom(seg, fps_num, fps_den, zoom, sizeof(zoom));

    char post[4096] = "";
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

    /* f0/f1 (the per-track filter strings passed in) can each run up to their own 8192-byte
       capacity now (a RoundedRect mask combined with a Ken-Burns zoom on the same clip) —
       generous margin over the 2x8192 + a short template that implies. */
    char fstr[20480];
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

                            char f0[8192], f1[8192];
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

                            char vfd[16384];
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
