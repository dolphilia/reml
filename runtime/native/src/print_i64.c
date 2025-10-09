/**
 * print_i64.c - デバッグ用整数出力関数
 *
 * Phase 1 での動作確認・テスト用に提供する簡易出力関数。
 * 本格的な I/O は Phase 2 以降で標準ライブラリとして整備する。
 */

#include "../include/reml_runtime.h"
#include <stdio.h>

void print_i64(int64_t value) {
    printf("%lld\n", (long long)value);
    fflush(stdout);
}
