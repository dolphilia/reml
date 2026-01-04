#include "reml_embed.h"
#include <stdio.h>
#include <string.h>

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

static const char* safe_error(reml_embed_context_t* context) {
    const char* message = reml_last_error(context);
    return message ? message : "unknown error";
}

int main(void) {
    const char* abi_version = "0.1.0";
    const char* source = "module Examples.Native.Embedding.Basic\n\nfn main() -> Str { \"embedded ok\" }\n";
    reml_embed_context_t* context = NULL;

    reml_embed_status_t status = reml_create_context(abi_version, &context);
    printf("create=%s\n", status_label(status));
    if (status != REML_EMBED_STATUS_OK) {
        fprintf(stderr, "create failed: %s\n", safe_error(context));
        return 1;
    }

    status = reml_load_module(context, (const unsigned char*)source, strlen(source));
    printf("load=%s\n", status_label(status));
    if (status != REML_EMBED_STATUS_OK) {
        fprintf(stderr, "load failed: %s\n", safe_error(context));
        reml_dispose_context(context);
        return 1;
    }

    status = reml_run(context, "main");
    printf("run=%s\n", status_label(status));
    if (status != REML_EMBED_STATUS_OK) {
        fprintf(stderr, "run failed: %s\n", safe_error(context));
        reml_dispose_context(context);
        return 1;
    }

    status = reml_dispose_context(context);
    printf("dispose=%s\n", status_label(status));
    return status == REML_EMBED_STATUS_OK ? 0 : 1;
}
