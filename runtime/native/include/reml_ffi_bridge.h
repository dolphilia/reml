#ifndef REML_FFI_BRIDGE_H
#define REML_FFI_BRIDGE_H

/**
 * reml_ffi_bridge.h — FFI ブリッジ補助 API
 *
 * docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md で定義された
 * ブリッジスタブ生成タスクに向け、ランタイム側で共通利用するヘルパを
 * 提供する。Phase 2-3 の初期段階では参照カウントの連携とシンプルな
 * Span 型の扱いのみを定義し、今後のマーシャリング機構拡張に備える。
 */

#include "reml_runtime.h"

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Span 表現（Borrowed/Transferred 双方で利用）
 *
 * data:   要素先頭へのポインタ（NULL 許容）
 * length: 要素数（バイト単位または要素数、マーシャリング側で解釈）
 */
typedef struct {
    void* data;
    size_t length;
} reml_span_t;

/**
 * 参照カウントを保持したまま値を借用する。
 *
 * FFI 呼び出しに値を渡す際、Reml 側で借用扱いにする場合は inc_ref で
 * 参照カウントを増やした状態でポインタを返す。NULL はそのまま返す。
 */
static inline void* reml_ffi_acquire_borrowed(void* value) {
    if (value != NULL) {
        inc_ref(value);
    }
    return value;
}

/**
 * 所有権を転送する場合の取得ヘルパ。
 *
 * 現段階では追加処理を行わず、将来的に監査フックやメトリクス記録を
 * 挿入するためのフックポイントとして定義する。
 */
static inline void* reml_ffi_acquire_transferred(void* value) {
    return value;
}

/**
 * 転送済みの値を解放するヘルパ。
 *
 * 所有権が戻ってきた際に参照カウントを減らし、必要であれば解放する。
 * 借用 (Borrowed) の場合は呼び出し側でこの関数を用いない。
 */
static inline void reml_ffi_release_transferred(void* value) {
    if (value != NULL) {
        dec_ref(value);
    }
}

/**
 * Span を構築するユーティリティ。
 *
 * data が NULL の場合は length を 0 とすることを推奨する。
 */
static inline reml_span_t reml_ffi_make_span(void* data, size_t length) {
    reml_span_t span;
    span.data = data;
    span.length = data == NULL ? 0 : length;
    return span;
}

#ifdef __cplusplus
}
#endif

#endif /* REML_FFI_BRIDGE_H */
