#include "reml_embed.h"
#include <stdio.h>

static const char* status_label(reml_embed_status_t status) {
    switch (status) {
        case REML_EMBED_STATUS_OK:
            return "ok";
        case REML_EMBED_STATUS_ERROR:
            return "error";
        case REML_EMBED_STATUS_ABI_MISMATCH:
            return "abi_mismatch";
        case REML_EMBED_STATUS_UNSUPPORTED_TARGET:
            return "unsupported_target";
        case REML_EMBED_STATUS_INVALID_ARGUMENT:
            return "invalid_argument";
        default:
            return "unknown";
    }
}

int main(void) {
    const char* abi_version = "9.9.9";
    reml_embed_context_t* context = NULL;
    reml_embed_status_t status = reml_create_context(abi_version, &context);
    printf("create=%s\n", status_label(status));
    return status == REML_EMBED_STATUS_ABI_MISMATCH ? 0 : 1;
}
