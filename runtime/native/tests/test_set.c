/**
 * test_set.c - Set 実装のテストスイート
 *
 * 最小 ABI の Set 機能（new/insert/contains/len）を検証する。
 */

#include "../include/reml_runtime.h"
#include <stdio.h>
#include <stdlib.h>

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

static void test_basic_set_ops(void) {
    TEST("基本操作 (new/insert/contains/len)");

    void* set0 = reml_set_new();
    ASSERT(set0 != NULL, "set0 is null");
    ASSERT(reml_set_len(set0) == 0, "set0 len should be 0");

    void* value1 = mem_alloc(8);
    void* value2 = mem_alloc(8);
    ASSERT(value1 != NULL, "value1 alloc failed");
    ASSERT(value2 != NULL, "value2 alloc failed");

    void* set1 = reml_set_insert(set0, value1);
    ASSERT(reml_set_len(set1) == 1, "set1 len should be 1");
    ASSERT(reml_set_contains(set1, value1) == 1, "set1 should contain value1");
    ASSERT(reml_set_contains(set1, value2) == 0, "set1 should not contain value2");

    void* set2 = reml_set_insert(set1, value2);
    ASSERT(reml_set_len(set2) == 2, "set2 len should be 2");
    ASSERT(reml_set_contains(set2, value2) == 1, "set2 should contain value2");

    void* set3 = reml_set_insert(set2, value1);
    ASSERT(reml_set_len(set3) == 2, "set3 len should remain 2");
    ASSERT(reml_set_contains(set3, value1) == 1, "set3 should contain value1");

    dec_ref(set3);
    dec_ref(set2);
    dec_ref(set1);
    dec_ref(set0);
    dec_ref(value1);
    dec_ref(value2);

    PASS();
}

int main(void) {
    printf("Running set tests...\n\n");

    test_basic_set_ops();

    printf("\n========================================\n");
    printf("All %d tests passed!\n", test_passed);
    printf("========================================\n");

    return 0;
}
