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
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <sys/types.h>
#include <unistd.h>

/**
 * パニック（プログラムを異常終了させる）
 *
 * LLVM IR 側では panic(ptr, i64) の FAT ポインタ形式で宣言されているが、
 * C 実装側では const char* として受け取り、NULL 終端を前提とする。
 * 長さパラメータ（i64）は現在の実装では無視される。
 *
 * @param msg エラーメッセージ（NULL 終端文字列）
 */
void panic(const char* msg) {
    // タイムスタンプ取得
    time_t now = time(NULL);
    char time_buf[64];
    struct tm* tm_info = localtime(&now);
    strftime(time_buf, sizeof(time_buf), "%Y-%m-%d %H:%M:%S", tm_info);

    // プロセス ID 取得
    pid_t pid = getpid();

    // エラーメッセージを stderr に出力
    fprintf(stderr, "\n");
    fprintf(stderr, "===============================================\n");
    fprintf(stderr, "PANIC: Runtime Error\n");
    fprintf(stderr, "===============================================\n");
    fprintf(stderr, "Time:    %s\n", time_buf);
    fprintf(stderr, "PID:     %d\n", pid);
    fprintf(stderr, "Message: %s\n", msg ? msg : "(null)");
    fprintf(stderr, "===============================================\n");
    fprintf(stderr, "\n");

    // stderr をフラッシュして確実に出力
    fflush(stderr);

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
    time_t now = time(NULL);
    char time_buf[64];
    struct tm* tm_info = localtime(&now);
    strftime(time_buf, sizeof(time_buf), "%Y-%m-%d %H:%M:%S", tm_info);

    // プロセス ID 取得
    pid_t pid = getpid();

    // エラーメッセージを stderr に出力
    fprintf(stderr, "\n");
    fprintf(stderr, "===============================================\n");
    fprintf(stderr, "PANIC: Runtime Error\n");
    fprintf(stderr, "===============================================\n");
    fprintf(stderr, "Time:     %s\n", time_buf);
    fprintf(stderr, "PID:      %d\n", pid);
    fprintf(stderr, "Location: %s:%d\n", file ? file : "(unknown)", line);
    fprintf(stderr, "Message:  %s\n", msg ? msg : "(null)");
    fprintf(stderr, "===============================================\n");
    fprintf(stderr, "\n");

    // stderr をフラッシュして確実に出力
    fflush(stderr);

    // プログラムを異常終了（終了コード 1）
    exit(1);
}
