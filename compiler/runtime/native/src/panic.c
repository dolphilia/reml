/**
 * panic.c - パニックハンドラ実装
 *
 * 回復不能なエラーが発生した際にエラーメッセージを出力し、
 * プログラムを異常終了させる。
 *
 * Phase 1 では基本的な診断情報（メッセージ、タイムスタンプ、PID）を出力し、
 * Phase 2 以降でスタックトレース取得などを拡張する。
 */

#include "../include/reml_runtime.h"
#include "../include/reml_os.h"
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#ifdef REML_PLATFORM_WINDOWS
#include <windows.h>
#else
#include <sys/types.h>
#include <unistd.h>
#endif

static void reml_format_timestamp(char* buffer, size_t buffer_size) {
    time_t now = time(NULL);
    struct tm tm_info;
#ifdef REML_PLATFORM_WINDOWS
    if (localtime_s(&tm_info, &now) != 0) {
        memset(&tm_info, 0, sizeof(tm_info));
    }
#else
    struct tm* tmp = localtime(&now);
    if (tmp == NULL) {
        memset(&tm_info, 0, sizeof(tm_info));
    } else {
        tm_info = *tmp;
    }
#endif
    if (strftime(buffer, buffer_size, "%Y-%m-%d %H:%M:%S", &tm_info) == 0) {
        snprintf(buffer, buffer_size, "unknown");
    }
}

static unsigned long reml_get_process_id(void) {
#ifdef REML_PLATFORM_WINDOWS
    return (unsigned long)GetCurrentProcessId();
#else
    return (unsigned long)getpid();
#endif
}

/**
 * パニック（プログラムを異常終了させる）
 *
 * LLVM IR 側は panic(ptr) の宣言を正準とし、C 実装側では const char* として受け取り、
 * NULL 終端を前提とする。
 *
 * @param msg エラーメッセージ（NULL 終端文字列）
 */
void panic(const char* msg) {
    // タイムスタンプ取得
    char time_buf[64];
    reml_format_timestamp(time_buf, sizeof(time_buf));

    // プロセス ID 取得
    unsigned long pid = reml_get_process_id();

    // エラーメッセージを stderr に出力
    reml_os_file_t stderr_file = reml_os_stderr();

    const char* banner_top = "\n===============================================\n";
    const char* banner_mid = "PANIC: Runtime Error\n===============================================\n";
    const char* banner_bottom = "===============================================\n\n";

    reml_os_file_write_all(&stderr_file, banner_top, strlen(banner_top));
    reml_os_file_write_all(&stderr_file, banner_mid, strlen(banner_mid));

    char line_buffer[256];
    int length = snprintf(line_buffer, sizeof(line_buffer), "Time:    %s\n", time_buf);
    if (length > 0) {
        reml_os_file_write_all(&stderr_file, line_buffer, (size_t)length);
    }
    length = snprintf(line_buffer, sizeof(line_buffer), "PID:     %lu\n", pid);
    if (length > 0) {
        reml_os_file_write_all(&stderr_file, line_buffer, (size_t)length);
    }
    length = snprintf(line_buffer, sizeof(line_buffer), "Message: %s\n", msg ? msg : "(null)");
    if (length > 0) {
        reml_os_file_write_all(&stderr_file, line_buffer, (size_t)length);
    }

    reml_os_file_write_all(&stderr_file, banner_bottom, strlen(banner_bottom));

    // プログラムを異常終了（終了コード 1）
    exit(1);
}

/**
 * パニック（ファイル名と行番号付き）
 *
 * Phase 2 以降で使用する予定の拡張版 panic。
 * コンパイラ側で生成されるエラーメッセージに位置情報を含める。
 *
 * @param msg エラーメッセージ
 * @param file ソースファイル名
 * @param line 行番号
 */
void panic_at(const char* msg, const char* file, int line) {
    // タイムスタンプ取得
    char time_buf[64];
    reml_format_timestamp(time_buf, sizeof(time_buf));

    // プロセス ID 取得
    unsigned long pid = reml_get_process_id();

    // エラーメッセージを stderr に出力
    reml_os_file_t stderr_file = reml_os_stderr();

    const char* banner_top = "\n===============================================\n";
    const char* banner_mid = "PANIC: Runtime Error\n===============================================\n";
    const char* banner_bottom = "===============================================\n\n";

    reml_os_file_write_all(&stderr_file, banner_top, strlen(banner_top));
    reml_os_file_write_all(&stderr_file, banner_mid, strlen(banner_mid));

    char line_buffer[256];
    int length = snprintf(line_buffer, sizeof(line_buffer), "Time:     %s\n", time_buf);
    if (length > 0) {
        reml_os_file_write_all(&stderr_file, line_buffer, (size_t)length);
    }
    length = snprintf(line_buffer, sizeof(line_buffer), "PID:      %lu\n", pid);
    if (length > 0) {
        reml_os_file_write_all(&stderr_file, line_buffer, (size_t)length);
    }
    length = snprintf(line_buffer, sizeof(line_buffer), "Location: %s:%d\n", file ? file : "(unknown)", line);
    if (length > 0) {
        reml_os_file_write_all(&stderr_file, line_buffer, (size_t)length);
    }
    length = snprintf(line_buffer, sizeof(line_buffer), "Message:  %s\n", msg ? msg : "(null)");
    if (length > 0) {
        reml_os_file_write_all(&stderr_file, line_buffer, (size_t)length);
    }

    reml_os_file_write_all(&stderr_file, banner_bottom, strlen(banner_bottom));

    // プログラムを異常終了（終了コード 1）
    exit(1);
}
