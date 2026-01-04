/**
 * test_mem_alloc.c - メモリアロケータのテスト
 *
 * mem_alloc / mem_free の基本的な動作を確認する。
 * Phase 1 では単体テストの雛形を準備し、Phase 2 以降で拡充する。
 */

#include "../include/reml_runtime.h"
#include <stdio.h>
#include <assert.h>
#include <string.h>

// テストケース番号
static int test_count = 0;
static int passed = 0;

#define TEST_BEGIN(name) \
    do { \
        test_count++; \
        printf("[TEST %d] %s ... ", test_count, name); \
        fflush(stdout); \
    } while(0)

#define TEST_END() \
    do { \
        printf("OK\n"); \
        passed++; \
    } while(0)

/**
 * テスト1: 基本的なアロケーションと解放
 */
void test_basic_alloc_free(void) {
    TEST_BEGIN("Basic allocation and free");

    // 64 バイトを割り当て
    void* ptr = mem_alloc(64);
    assert(ptr != NULL);

    // ヘッダが正しく初期化されているか確認
    reml_object_header_t* header = REML_GET_HEADER(ptr);
    assert(header->refcount == 1);

    // 解放
    mem_free(ptr);

    TEST_END();
}

/**
 * テスト2: 8 バイト境界のアラインメント
 */
void test_alignment(void) {
    TEST_BEGIN("8-byte alignment");

    // 様々なサイズで割り当てて、アドレスが 8 バイト境界であることを確認
    size_t sizes[] = {1, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65};
    for (size_t i = 0; i < sizeof(sizes) / sizeof(sizes[0]); i++) {
        void* ptr = mem_alloc(sizes[i]);
        assert(ptr != NULL);

        // アドレスが 8 バイト境界か確認
        uintptr_t addr = (uintptr_t)ptr;
        assert(addr % 8 == 0);

        mem_free(ptr);
    }

    TEST_END();
}

/**
 * テスト3: NULL ポインタの解放
 */
void test_free_null(void) {
    TEST_BEGIN("Free NULL pointer");

    // NULL を解放しても問題ないことを確認
    mem_free(NULL);

    TEST_END();
}

/**
 * テスト4: 大きなメモリの割り当て
 */
void test_large_allocation(void) {
    TEST_BEGIN("Large memory allocation");

    // 1MB を割り当て
    void* ptr = mem_alloc(1024 * 1024);
    assert(ptr != NULL);

    // メモリに書き込めることを確認
    memset(ptr, 0xFF, 1024 * 1024);

    mem_free(ptr);

    TEST_END();
}

/**
 * テスト5: 型タグの設定と取得
 */
void test_type_tag(void) {
    TEST_BEGIN("Type tag set/get");

    void* ptr = mem_alloc(32);
    assert(ptr != NULL);

    // 型タグを設定
    reml_set_type_tag(ptr, REML_TAG_STRING);

    // 型タグを取得
    uint32_t tag = reml_get_type_tag(ptr);
    assert(tag == REML_TAG_STRING);

    mem_free(ptr);

    TEST_END();
}

/**
 * テスト6: 複数のアロケーション
 */
void test_multiple_allocations(void) {
    TEST_BEGIN("Multiple allocations");

    const int count = 100;
    void* ptrs[count];

    // 複数のメモリを割り当て
    for (int i = 0; i < count; i++) {
        ptrs[i] = mem_alloc(64);
        assert(ptrs[i] != NULL);
    }

    // すべて解放
    for (int i = 0; i < count; i++) {
        mem_free(ptrs[i]);
    }

    TEST_END();
}

/**
 * メイン関数
 */
int main(void) {
    printf("==================================================\n");
    printf("Memory Allocator Test Suite\n");
    printf("==================================================\n\n");

    test_basic_alloc_free();
    test_alignment();
    test_free_null();
    test_large_allocation();
    test_type_tag();
    test_multiple_allocations();

    printf("\n==================================================\n");
    printf("Test Results: %d/%d passed\n", passed, test_count);
    printf("==================================================\n");

#ifdef DEBUG
    printf("\n");
    reml_debug_print_alloc_stats();
#endif

    return (passed == test_count) ? 0 : 1;
}
