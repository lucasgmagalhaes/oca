#include "bridge_internal.h"

int open_input(const char *path, AVFormatContext **fmt_ctx_out) {
    if (avformat_open_input(fmt_ctx_out, path, NULL, NULL) < 0)
        return -1;
    if (avformat_find_stream_info(*fmt_ctx_out, NULL) < 0) {
        avformat_close_input(fmt_ctx_out);
        return -2;
    }
    return 0;
}
