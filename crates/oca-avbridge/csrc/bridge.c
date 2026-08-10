#include "bridge.h"

#include <libavformat/avformat.h>

uint32_t oca_avbridge_version(void) { return avformat_version(); }
