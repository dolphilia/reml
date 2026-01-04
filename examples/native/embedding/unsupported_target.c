#include "reml_embed.h"
#include <stdio.h>
#include <stdlib.h>

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

static void force_unsupported(void) {
#ifdef _WIN32
    /* テスト用に埋め込み API を未対応ターゲットとして扱う。 */
    _putenv("REML_EMBED_FORCE_UNSUPPORTED=1");
#else
    /* テスト用に埋め込み API を未対応ターゲットとして扱う。 */
    setenv("REML_EMBED_FORCE_UNSUPPORTED", "1", 1);
#endif
}

int main(void) {
    force_unsupported();
    const char* abi_version = "0.1.0";
    reml_embed_context_t* context = NULL;
    reml_embed_status_t status = reml_create_context(abi_version, &context);
    printf("create=%s\n", status_label(status));
    return status == REML_EMBED_STATUS_UNSUPPORTED_TARGET ? 0 : 1;
}
