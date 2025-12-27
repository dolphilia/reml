/**
 * test_boxing.c - Float/Char ボックス化のテストスイート
 *
 * reml_box_* / reml_unbox_* の往復が一致することを検証する。
 */

#include "../include/reml_runtime.h"
#include <stdio.h>
#include <stdlib.h>
#include <assert.h>

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

void test_box_float_roundtrip(void) {
    TEST("Float の boxing/unboxing");

    double value = 1.5;
    void* boxed = reml_box_float(value);
    ASSERT(boxed != NULL, "reml_box_float returned null");
    ASSERT(reml_get_type_tag(boxed) == REML_TAG_FLOAT, "type tag should be float");
    double roundtrip = reml_unbox_float(boxed);
    ASSERT(roundtrip == value, "float roundtrip mismatch");
    dec_ref(boxed);

    PASS();
}

void test_box_char_roundtrip(void) {
    TEST("Char の boxing/unboxing");

    reml_char_t value = 0x41;
    void* boxed = reml_box_char(value);
    ASSERT(boxed != NULL, "reml_box_char returned null");
    ASSERT(reml_get_type_tag(boxed) == REML_TAG_CHAR, "type tag should be char");
    reml_char_t roundtrip = reml_unbox_char(boxed);
    ASSERT(roundtrip == value, "char roundtrip mismatch");
    dec_ref(boxed);

    PASS();
}

/* ========== メイン ========== */

int main(void) {
    printf("Running boxing tests...\n\n");

    test_box_float_roundtrip();
    test_box_char_roundtrip();

    printf("\n========================================\n");
    printf("All %d tests passed!\n", test_passed);
    printf("========================================\n");

    return 0;
}
