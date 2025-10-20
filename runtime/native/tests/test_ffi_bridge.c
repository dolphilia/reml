/**
 * test_ffi_bridge.c - FFI ブリッジ補助 API のテスト
 *
 * Phase 2-3 で追加されたランタイムヘルパが期待通り機能することを検証する。
 */

#include "../include/reml_ffi_bridge.h"
#include <assert.h>
#include <math.h>
#include <stdio.h>

static int test_count = 0;
static int passed = 0;

#define TEST_BEGIN(name)                                 \
    do {                                                 \
        test_count++;                                    \
        printf("[TEST %d] %s ... ", test_count, name);   \
        fflush(stdout);                                  \
    } while (0)

#define TEST_END()                                       \
    do {                                                 \
        printf("OK\n");                                  \
        passed++;                                        \
    } while (0)

static void test_metrics_tracking(void) {
    TEST_BEGIN("FFI bridge metrics tracking");

    reml_ffi_bridge_reset_metrics();
    assert(fabs(reml_ffi_bridge_pass_rate() - 1.0) < 1e-9);

    reml_ffi_bridge_record_success();
    reml_ffi_bridge_record_failure();

    reml_ffi_bridge_metrics_t snapshot = reml_ffi_bridge_get_metrics();
    assert(snapshot.total_calls == 2);
    assert(snapshot.success_calls == 1);
    assert(snapshot.borrowed_results == 0);
    assert(snapshot.transferred_results == 0);
    assert(snapshot.null_results == 0);

    double pass_rate = reml_ffi_bridge_pass_rate();
    assert(fabs(pass_rate - 0.5) < 1e-9);

    TEST_END();
}

static void test_return_metrics(void) {
    TEST_BEGIN("FFI bridge return metrics");

    reml_ffi_bridge_reset_metrics();
    void* payload = mem_alloc(32);
    assert(payload != NULL);
    reml_set_type_tag(payload, REML_TAG_RECORD);

    void* borrowed = reml_ffi_acquire_borrowed_result(payload);
    assert(borrowed == payload);

    void* transferred = reml_ffi_acquire_transferred_result(payload);
    assert(transferred == payload);

    void* null_ret = reml_ffi_acquire_borrowed_result(NULL);
    assert(null_ret == NULL);

    reml_ffi_bridge_record_success();
    reml_ffi_bridge_record_failure();

    reml_ffi_bridge_metrics_t snapshot = reml_ffi_bridge_get_metrics();
    assert(snapshot.total_calls == 2);
    assert(snapshot.success_calls == 1);
    assert(snapshot.borrowed_results == 1);
    assert(snapshot.transferred_results == 1);
    assert(snapshot.null_results == 1);

    dec_ref(payload);

    TEST_END();
}

static void test_string_span_conversion(void) {
    TEST_BEGIN("String/span conversion helpers");

    const char payload[] = "hello";
    reml_string_t source = {.data = payload, .length = 5};

    reml_span_t span = reml_ffi_box_string(&source);
    assert(span.data == source.data);
    assert(span.length == 5);

    reml_string_t restored = reml_ffi_unbox_span(&span);
    assert(restored.data == source.data);
    assert(restored.length == source.length);

    reml_span_t empty_span = reml_ffi_make_span(NULL, 10);
    reml_string_t empty = reml_ffi_unbox_span(&empty_span);
    assert(empty.data == NULL);
    assert(empty.length == 0);

    TEST_END();
}

static void test_borrow_helpers(void) {
    TEST_BEGIN("Borrow/transfer helpers");

    void* payload = mem_alloc(16);
    assert(payload != NULL);
    reml_set_type_tag(payload, REML_TAG_INT);

    reml_object_header_t* header = REML_GET_HEADER(payload);
    assert(header->refcount == 1);

    void* borrowed = reml_ffi_acquire_borrowed(payload);
    assert(borrowed == payload);
    assert(header->refcount == 2);

    void* transferred = reml_ffi_acquire_transferred(payload);
    assert(transferred == payload);
    assert(header->refcount == 2);

    reml_ffi_release_transferred(transferred);
    assert(header->refcount == 1);

    dec_ref(payload);

    TEST_END();
}

int main(void) {
    printf("==================================================\n");
    printf("FFI Bridge Helper Test Suite\n");
    printf("==================================================\n\n");

    test_metrics_tracking();
    test_return_metrics();
    test_string_span_conversion();
    test_borrow_helpers();

    printf("\n==================================================\n");
    printf("Test Summary: %d/%d passed\n", passed, test_count);
    printf("==================================================\n");

    return (passed == test_count) ? 0 : 1;
}
