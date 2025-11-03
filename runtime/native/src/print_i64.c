/**
 * print_i64.c - デバッグ用整数出力関数
 *
 * Phase 1 での動作確認・テスト用に提供する簡易出力関数。
 * 本格的な I/O は Phase 2 以降で標準ライブラリとして整備する。
 */

#include "../include/reml_runtime.h"
#include "../include/reml_os.h"
#include <stdio.h>
#include <string.h>

void print_i64(int64_t value) {
    char buffer[64];
    int length = snprintf(buffer, sizeof(buffer), "%lld\n", (long long)value);
    if (length <= 0) {
        return;
    }

    reml_os_file_t stdout_file = reml_os_stdout();
    reml_os_file_write_all(&stdout_file, buffer, (size_t)length);
}
