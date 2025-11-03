/**
 * ffi_bridge.c — FFI ブリッジ補助実装
 *
 * docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md で定義された
 * 計測フックとマーシャリングヘルパーを実装する。Phase 2-3 では呼び出し
 * 成功率の計測と Span/文字列の相互変換のみ提供し、今後のスタブ生成と
 * 監査統合へ備える。
 */

#include "../include/reml_ffi_bridge.h"
#include "../include/reml_atomic.h"
#include <limits.h>

/* ========== 計測カウンタ ========== */

static atomic_uint_fast64_t bridge_total_calls = 0;
static atomic_uint_fast64_t bridge_success_calls = 0;
static atomic_uint_fast64_t bridge_borrowed_results = 0;
static atomic_uint_fast64_t bridge_transferred_results = 0;
static atomic_uint_fast64_t bridge_null_results = 0;

/* ========== マーシャリングヘルパー ========== */

reml_span_t reml_ffi_box_string(const reml_string_t* source) {
    if (source == NULL || source->data == NULL) {
        return reml_ffi_make_span(NULL, 0);
    }

    size_t length = 0;
    if (source->length > 0) {
        length = (size_t)source->length;
    }

    return reml_ffi_make_span((void*)source->data, length);
}

reml_string_t reml_ffi_unbox_span(const reml_span_t* span) {
    reml_string_t result;
    result.data = NULL;
    result.length = 0;

    if (span == NULL || span->data == NULL) {
        return result;
    }

    result.data = (const char*)span->data;
    if (span->length > (size_t)INT64_MAX) {
        result.length = INT64_MAX;
    } else {
        result.length = (int64_t)span->length;
    }

    return result;
}

void* reml_ffi_acquire_borrowed_result(void* value) {
    if (value == NULL) {
        atomic_fetch_add_explicit(&bridge_null_results, 1, memory_order_relaxed);
        return NULL;
    }

    atomic_fetch_add_explicit(&bridge_borrowed_results, 1, memory_order_relaxed);
    return value;
}

void* reml_ffi_acquire_transferred_result(void* value) {
    if (value == NULL) {
        atomic_fetch_add_explicit(&bridge_null_results, 1, memory_order_relaxed);
        return NULL;
    }

    atomic_fetch_add_explicit(&bridge_transferred_results, 1, memory_order_relaxed);
    return value;
}

/* ========== 計測 API ========== */

void reml_ffi_bridge_record_status(reml_ffi_bridge_status_t status) {
    atomic_fetch_add_explicit(&bridge_total_calls, 1, memory_order_relaxed);
    if (status == REML_FFI_BRIDGE_STATUS_SUCCESS) {
        atomic_fetch_add_explicit(&bridge_success_calls, 1, memory_order_relaxed);
    }
}

void reml_ffi_bridge_reset_metrics(void) {
    atomic_store_explicit(&bridge_total_calls, 0, memory_order_relaxed);
    atomic_store_explicit(&bridge_success_calls, 0, memory_order_relaxed);
    atomic_store_explicit(&bridge_borrowed_results, 0, memory_order_relaxed);
    atomic_store_explicit(&bridge_transferred_results, 0, memory_order_relaxed);
    atomic_store_explicit(&bridge_null_results, 0, memory_order_relaxed);
}

reml_ffi_bridge_metrics_t reml_ffi_bridge_get_metrics(void) {
    reml_ffi_bridge_metrics_t snapshot;
    snapshot.total_calls =
        atomic_load_explicit(&bridge_total_calls, memory_order_relaxed);
    snapshot.success_calls =
        atomic_load_explicit(&bridge_success_calls, memory_order_relaxed);
    snapshot.borrowed_results =
        atomic_load_explicit(&bridge_borrowed_results, memory_order_relaxed);
    snapshot.transferred_results =
        atomic_load_explicit(&bridge_transferred_results, memory_order_relaxed);
    snapshot.null_results =
        atomic_load_explicit(&bridge_null_results, memory_order_relaxed);
    return snapshot;
}

double reml_ffi_bridge_pass_rate(void) {
    uint64_t total =
        atomic_load_explicit(&bridge_total_calls, memory_order_relaxed);
    uint64_t success =
        atomic_load_explicit(&bridge_success_calls, memory_order_relaxed);
    if (total == 0) {
        return 1.0;
    }
    return (double)success / (double)total;
}
