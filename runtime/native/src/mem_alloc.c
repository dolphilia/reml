/**
 * mem_alloc.c - メモリアロケータ実装
 *
 * Phase 1 では malloc ベースの単純実装を提供する。
 * ヒープオブジェクトのヘッダを初期化し、8 バイト境界に調整したメモリを返す。
 * Phase 2 以降でアリーナアロケータや最適化を検討する。
 */

#include "../include/reml_runtime.h"
#include <stdlib.h>
#include <string.h>

/* ========== デバッグ支援 ========== */

#ifdef DEBUG
#include <stdio.h>
#include <stdatomic.h>

// アロケーション追跡カウンタ（デバッグ時のみ）
static atomic_size_t alloc_count = 0;
static atomic_size_t free_count = 0;

void reml_debug_print_alloc_stats(void) {
    size_t allocs = atomic_load(&alloc_count);
    size_t frees = atomic_load(&free_count);
    fprintf(stderr, "[DEBUG] Total allocations: %zu, frees: %zu, leaked: %zu\n",
            allocs, frees, allocs - frees);
}
#endif

/* ========== ヘルパー関数 ========== */

/**
 * サイズを 8 バイト境界に切り上げる
 */
static inline size_t align_to_8(size_t size) {
    return (size + 7) & ~(size_t)7;
}

/* ========== メモリ割り当て ========== */

void* mem_alloc(size_t size) {
    // 8 バイト境界に調整
    size_t aligned_size = align_to_8(size);

    // ヘッダ + ペイロードのサイズを計算
    size_t total_size = sizeof(reml_object_header_t) + aligned_size;

    // malloc でメモリを確保
    void* raw_ptr = malloc(total_size);

    if (raw_ptr == NULL) {
        // アロケーション失敗時は panic で異常終了
        panic("Memory allocation failed");
    }

    // ヘッダを初期化
    reml_object_header_t* header = (reml_object_header_t*)raw_ptr;
    header->refcount = 1;  // 初期参照カウントは 1
    header->type_tag = 0;  // 型タグは呼び出し側で設定する（Phase 1 では未使用も許容）

    // ペイロード領域をゼロクリア（未初期化データ防止）
    void* payload = (char*)raw_ptr + sizeof(reml_object_header_t);
    memset(payload, 0, aligned_size);

#ifdef DEBUG
    atomic_fetch_add(&alloc_count, 1);
    fprintf(stderr, "[DEBUG] mem_alloc: size=%zu, aligned=%zu, ptr=%p\n",
            size, aligned_size, payload);
#endif

    // ペイロード先頭ポインタを返す（呼び出し側はヘッダを意識不要）
    return payload;
}

/* ========== メモリ解放 ========== */

void mem_free(void* ptr) {
    if (ptr == NULL) {
        // NULL ポインタは無視（安全性のため）
        return;
    }

    // ヘッダ位置を逆算
    reml_object_header_t* header = REML_GET_HEADER(ptr);

#ifdef DEBUG
    atomic_fetch_add(&free_count, 1);
    fprintf(stderr, "[DEBUG] mem_free: ptr=%p, refcount=%u, type_tag=%u\n",
            ptr, header->refcount, header->type_tag);

    // 二重解放検出（デバッグ時のみ）
    if (header->refcount == 0xDEADBEEF) {
        fprintf(stderr, "[ERROR] Double free detected at %p\n", ptr);
        panic("Double free detected");
    }

    // 解放済みマーク（デバッグ時のみ）
    header->refcount = 0xDEADBEEF;
#endif

    // ヘッダを含む領域全体を解放
    free(header);
}

/* ========== ユーティリティ関数 ========== */

/**
 * オブジェクトの型タグを設定
 *
 * mem_alloc 直後に呼び出して型タグを設定する。
 * Phase 1 では型タグは参考情報であり、必須ではない。
 *
 * @param ptr オブジェクトへのポインタ（ヘッダの直後）
 * @param type_tag 設定する型タグ (reml_type_tag_t)
 */
void reml_set_type_tag(void* ptr, uint32_t type_tag) {
    if (ptr == NULL) {
        return;
    }

    reml_object_header_t* header = REML_GET_HEADER(ptr);
    header->type_tag = type_tag;
}

/**
 * オブジェクトの型タグを取得
 *
 * @param ptr オブジェクトへのポインタ（ヘッダの直後）
 * @return 型タグ (reml_type_tag_t)
 */
uint32_t reml_get_type_tag(void* ptr) {
    if (ptr == NULL) {
        return 0;
    }

    reml_object_header_t* header = REML_GET_HEADER(ptr);
    return header->type_tag;
}
