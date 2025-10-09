/**
 * test_refcount.c - 参照カウント実装のテストスイート
 *
 * inc_ref, dec_ref の動作を検証し、型別デストラクタの正しさを確認する。
 */

#include "../include/reml_runtime.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>

#ifdef DEBUG
#include <stdatomic.h>
#endif

/* ========== テストヘルパー ========== */

static int test_count = 0;
static int test_passed = 0;

#define TEST(name) \
    do { \
        printf("  [%d] %s ... ", ++test_count, name); \
        fflush(stdout); \
    } while (0)

#define PASS() \
    do { \
        printf("OK\n"); \
        test_passed++; \
    } while (0)

#define FAIL(msg) \
    do { \
        printf("FAILED: %s\n", msg); \
        exit(1); \
    } while (0)

#define ASSERT(cond, msg) \
    do { \
        if (!(cond)) { \
            FAIL(msg); \
        } \
    } while (0)

/* ========== テストケース ========== */

/**
 * Test 1: 基本的な inc_ref / dec_ref
 *
 * 単純オブジェクトを作成し、inc_ref でカウントを増やし、
 * dec_ref でカウントを減らす。最終的に解放されることを確認。
 */
void test_basic_inc_dec(void) {
    TEST("基本的な inc_ref / dec_ref");

    // オブジェクトを割り当て（初期 refcount = 1）
    void* obj = mem_alloc(16);
    ASSERT(obj != NULL, "mem_alloc failed");

    reml_object_header_t* header = REML_GET_HEADER(obj);
    ASSERT(header->refcount == 1, "initial refcount should be 1");

    // inc_ref でカウントを増やす
    inc_ref(obj);
    ASSERT(header->refcount == 2, "refcount should be 2 after inc_ref");

    // dec_ref でカウントを減らす（まだ解放されない）
    dec_ref(obj);
    ASSERT(header->refcount == 1, "refcount should be 1 after first dec_ref");

    // 再度 dec_ref でカウントが 0 になり、解放される
    dec_ref(obj);
    // 解放後はヘッダにアクセスできないため、ここでは解放が成功したと仮定

    PASS();
}

/**
 * Test 2: ゼロ到達による自動解放
 *
 * dec_ref でカウントが 0 になった際に、メモリが自動的に解放されることを確認。
 * デバッグモードで alloc/free カウンタを確認する。
 */
void test_auto_free_on_zero(void) {
    TEST("ゼロ到達による自動解放");

#ifdef DEBUG
    // 初期状態のカウンタを記録
    size_t initial_allocs = reml_debug_get_alloc_count();
    size_t initial_frees = reml_debug_get_free_count();
#endif

    // オブジェクトを割り当てて即座に解放
    void* obj = mem_alloc(32);
    ASSERT(obj != NULL, "mem_alloc failed");

    reml_set_type_tag(obj, REML_TAG_INT);  // プリミティブ型（子オブジェクトなし）

    // dec_ref でカウントが 0 になり、解放される
    dec_ref(obj);

#ifdef DEBUG
    // alloc と free のカウンタが一致することを確認
    size_t final_allocs = reml_debug_get_alloc_count();
    size_t final_frees = reml_debug_get_free_count();
    ASSERT(final_allocs == initial_allocs + 1, "alloc count mismatch");
    ASSERT(final_frees == initial_frees + 1, "free count mismatch");
#endif

    PASS();
}

/**
 * Test 3: NULL 安全性
 *
 * inc_ref(NULL) と dec_ref(NULL) が安全に動作することを確認。
 */
void test_null_safety(void) {
    TEST("NULL 安全性");

    // NULL ポインタを渡しても安全であることを確認
    inc_ref(NULL);  // クラッシュしないことを確認
    dec_ref(NULL);  // クラッシュしないことを確認

    PASS();
}

/**
 * Test 4: 型別デストラクタ - プリミティブ型
 *
 * プリミティブ型（INT, FLOAT, BOOL）は子オブジェクトを持たないため、
 * デストラクタが単純に解放されることを確認。
 */
void test_destructor_primitive(void) {
    TEST("型別デストラクタ - プリミティブ型");

    // INT 型オブジェクト
    void* int_obj = mem_alloc(sizeof(int64_t));
    reml_set_type_tag(int_obj, REML_TAG_INT);
    dec_ref(int_obj);  // 解放

    // FLOAT 型オブジェクト
    void* float_obj = mem_alloc(sizeof(double));
    reml_set_type_tag(float_obj, REML_TAG_FLOAT);
    dec_ref(float_obj);  // 解放

    // BOOL 型オブジェクト
    void* bool_obj = mem_alloc(sizeof(uint8_t));
    reml_set_type_tag(bool_obj, REML_TAG_BOOL);
    dec_ref(bool_obj);  // 解放

    PASS();
}

/**
 * Test 5: 型別デストラクタ - クロージャ
 *
 * クロージャ {env*, code_ptr} のデストラクタが env を dec_ref することを確認。
 */
void test_destructor_closure(void) {
    TEST("型別デストラクタ - クロージャ");

    // 環境オブジェクトを作成（env として使用）
    void* env = mem_alloc(64);
    reml_set_type_tag(env, REML_TAG_INT);  // 簡易のため INT 型として扱う
    reml_object_header_t* env_header = REML_GET_HEADER(env);
    ASSERT(env_header->refcount == 1, "env initial refcount should be 1");

    // クロージャオブジェクトを作成
    typedef struct {
        void* env;
        void* code_ptr;
    } closure_t;
    closure_t* closure = (closure_t*)mem_alloc(sizeof(closure_t));
    reml_set_type_tag(closure, REML_TAG_CLOSURE);

    // env を設定し、inc_ref で参照を増やす
    closure->env = env;
    closure->code_ptr = NULL;  // ダミー
    inc_ref(env);  // closure が env を参照するため inc_ref
    ASSERT(env_header->refcount == 2, "env refcount should be 2 after inc_ref");

    // クロージャを解放（env の refcount が 1 に減る）
    dec_ref(closure);
    ASSERT(env_header->refcount == 1, "env refcount should be 1 after closure dec_ref");

    // env を解放
    dec_ref(env);

    PASS();
}

/**
 * Test 6: リークゼロ検証
 *
 * デバッグモードで alloc/free カウンタを確認し、リークがないことを検証。
 */
void test_no_leaks(void) {
    TEST("リークゼロ検証");

#ifdef DEBUG
    // 初期状態のカウンタを記録
    size_t initial_allocs = reml_debug_get_alloc_count();
    size_t initial_frees = reml_debug_get_free_count();

    // 複数のオブジェクトを割り当てて解放
    for (int i = 0; i < 10; i++) {
        void* obj = mem_alloc(128);
        reml_set_type_tag(obj, REML_TAG_INT);
        dec_ref(obj);
    }

    // alloc と free のカウンタが一致することを確認
    size_t final_allocs = reml_debug_get_alloc_count();
    size_t final_frees = reml_debug_get_free_count();
    ASSERT(final_allocs == initial_allocs + 10, "alloc count mismatch");
    ASSERT(final_frees == initial_frees + 10, "free count mismatch");

    printf("(allocs=%zu, frees=%zu, leaked=%zu) ",
           final_allocs, final_frees, final_allocs - final_frees);
#else
    printf("(DEBUG mode not enabled, skipping leak check) ");
#endif

    PASS();
}

/**
 * Test 7: 型別デストラクタ - ADT
 *
 * ADT {tag, payload} のデストラクタが payload を dec_ref することを確認。
 */
void test_destructor_adt(void) {
    TEST("型別デストラクタ - ADT");

    // payload オブジェクトを作成
    void* payload = mem_alloc(32);
    reml_set_type_tag(payload, REML_TAG_INT);
    reml_object_header_t* payload_header = REML_GET_HEADER(payload);
    ASSERT(payload_header->refcount == 1, "payload initial refcount should be 1");

    // ADT オブジェクトを作成
    typedef struct {
        int32_t tag;
        void* payload;
    } adt_t;
    adt_t* adt = (adt_t*)mem_alloc(sizeof(adt_t));
    reml_set_type_tag(adt, REML_TAG_ADT);

    // payload を設定し、inc_ref で参照を増やす
    adt->tag = 42;
    adt->payload = payload;
    inc_ref(payload);  // ADT が payload を参照するため inc_ref
    ASSERT(payload_header->refcount == 2, "payload refcount should be 2 after inc_ref");

    // ADT を解放（payload の refcount が 1 に減る）
    dec_ref(adt);
    ASSERT(payload_header->refcount == 1, "payload refcount should be 1 after ADT dec_ref");

    // payload を解放
    dec_ref(payload);

    PASS();
}

/**
 * Test 8: 複数回の inc_ref / dec_ref
 *
 * 同じオブジェクトに対して複数回 inc_ref と dec_ref を呼び出し、
 * 正しくカウントが管理されることを確認。
 */
void test_multiple_inc_dec(void) {
    TEST("複数回の inc_ref / dec_ref");

    void* obj = mem_alloc(16);
    reml_set_type_tag(obj, REML_TAG_INT);
    reml_object_header_t* header = REML_GET_HEADER(obj);

    // 初期状態: refcount = 1
    ASSERT(header->refcount == 1, "initial refcount should be 1");

    // 3 回 inc_ref
    inc_ref(obj);
    inc_ref(obj);
    inc_ref(obj);
    ASSERT(header->refcount == 4, "refcount should be 4 after 3 inc_ref");

    // 2 回 dec_ref（まだ解放されない）
    dec_ref(obj);
    dec_ref(obj);
    ASSERT(header->refcount == 2, "refcount should be 2 after 2 dec_ref");

    // 残りを dec_ref で解放
    dec_ref(obj);
    dec_ref(obj);

    PASS();
}

/* ========== メイン ========== */

int main(void) {
    printf("Running refcount tests...\n\n");

    test_basic_inc_dec();
    test_auto_free_on_zero();
    test_null_safety();
    test_destructor_primitive();
    test_destructor_closure();
    test_no_leaks();
    test_destructor_adt();
    test_multiple_inc_dec();

    printf("\n========================================\n");
    printf("All %d tests passed!\n", test_passed);
    printf("========================================\n");

#ifdef DEBUG
    printf("\n");
    reml_debug_print_alloc_stats();
    extern void reml_debug_print_refcount_stats(void);
    reml_debug_print_refcount_stats();
#endif

    return 0;
}
