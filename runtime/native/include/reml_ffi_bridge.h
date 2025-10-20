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
 * FFI ブリッジ呼び出し結果のステータス
 */
typedef enum {
    REML_FFI_BRIDGE_STATUS_SUCCESS = 0,
    REML_FFI_BRIDGE_STATUS_FAILURE = 1
} reml_ffi_bridge_status_t;

/**
 * FFI ブリッジ計測値
 *
 * total_calls:  呼び出し総数
 * success_calls: 成功と判定された呼び出し数
 */
typedef struct {
    uint64_t total_calls;
    uint64_t success_calls;
    uint64_t borrowed_results;
    uint64_t transferred_results;
    uint64_t null_results;
} reml_ffi_bridge_metrics_t;

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

/**
 * Reml 文字列を FFI 向け Span に変換する。
 *
 * data が NULL または長さが負の場合は空 Span を返す。
 */
reml_span_t reml_ffi_box_string(const reml_string_t* source);

/**
 * FFI から渡された Span を Reml 文字列表現へ復元する。
 *
 * span が NULL または data が NULL の場合は空文字列を返す。
 */
reml_string_t reml_ffi_unbox_span(const reml_span_t* span);

/**
 * Borrowed な返り値を記録するヘルパ。
 *
 * 返り値が NULL の場合は null_results カウンタを更新する。
 * それ以外は borrowed_results を更新し、値をそのまま返す。
 */
void* reml_ffi_acquire_borrowed_result(void* value);

/**
 * Transferred な返り値を記録するヘルパ。
 *
 * 返り値が NULL の場合は null_results カウンタを更新する。
 * それ以外は transferred_results を更新し、値をそのまま返す。
 */
void* reml_ffi_acquire_transferred_result(void* value);

/**
 * FFI 呼び出し結果を記録する。
 *
 * メトリクスは CI の `ffi_bridge.audit_pass_rate` 算出に利用される。
 */
void reml_ffi_bridge_record_status(reml_ffi_bridge_status_t status);

/**
 * FFI ブリッジ計測値をリセットする。
 */
void reml_ffi_bridge_reset_metrics(void);

/**
 * 現在の FFI ブリッジ計測値を取得する。
 *
 * @return 計測値のスナップショット（値渡し）
 */
reml_ffi_bridge_metrics_t reml_ffi_bridge_get_metrics(void);

/**
 * 成功率（0.0〜1.0）を算出する。
 * 呼び出しが存在しない場合は 1.0 を返す。
 */
double reml_ffi_bridge_pass_rate(void);

/**
 * 便宜用の成功記録ヘルパ。
 */
static inline void reml_ffi_bridge_record_success(void) {
    reml_ffi_bridge_record_status(REML_FFI_BRIDGE_STATUS_SUCCESS);
}

/**
 * 便宜用の失敗記録ヘルパ。
 */
static inline void reml_ffi_bridge_record_failure(void) {
    reml_ffi_bridge_record_status(REML_FFI_BRIDGE_STATUS_FAILURE);
}

#ifdef __cplusplus
}
#endif

#endif /* REML_FFI_BRIDGE_H */
