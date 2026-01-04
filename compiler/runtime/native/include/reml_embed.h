#ifndef REML_EMBED_H
#define REML_EMBED_H

/**
 * reml_embed.h — 埋め込み API (Phase 4)
 *
 * Rust 実装の `reml_create_context` / `reml_load_module` / `reml_run` /
 * `reml_dispose_context` を C から呼び出す最小 ABI を提供する。
 */

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct reml_embed_context_t reml_embed_context_t;

typedef enum {
    REML_EMBED_STATUS_OK = 0,
    REML_EMBED_STATUS_ERROR = 1,
    REML_EMBED_STATUS_ABI_MISMATCH = 2,
    REML_EMBED_STATUS_UNSUPPORTED_TARGET = 3,
    REML_EMBED_STATUS_INVALID_ARGUMENT = 4
} reml_embed_status_t;

reml_embed_status_t reml_create_context(
    const char* abi_version,
    reml_embed_context_t** out_context
);

reml_embed_status_t reml_load_module(
    reml_embed_context_t* context,
    const unsigned char* source,
    size_t length
);

reml_embed_status_t reml_run(
    reml_embed_context_t* context,
    const char* entrypoint
);

reml_embed_status_t reml_dispose_context(reml_embed_context_t* context);

const char* reml_last_error(const reml_embed_context_t* context);

#ifdef __cplusplus
}
#endif

#endif /* REML_EMBED_H */
