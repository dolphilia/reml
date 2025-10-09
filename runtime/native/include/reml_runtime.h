#ifndef REML_RUNTIME_H
#define REML_RUNTIME_H

#include <stddef.h>
#include <stdint.h>

/* ========== バージョン定義 ========== */
#define REML_RUNTIME_VERSION_MAJOR 0
#define REML_RUNTIME_VERSION_MINOR 1
#define REML_RUNTIME_VERSION_PATCH 0

/* ========== ヒープオブジェクトヘッダ ========== */

/**
 * ヒープ割り当てされたオブジェクトのヘッダ構造
 *
 * すべてのヒープオブジェクトは先頭にこの構造体を持ち、
 * 参照カウントベースのメモリ管理と型情報の保持を行う。
 *
 * メモリレイアウト:
 *   [reml_object_header_t (8 bytes)] [payload (n bytes)]
 *
 * アラインメント: 8バイト境界
 */
typedef struct {
    uint32_t refcount;  ///< 参照カウント（初期値: 1）
    uint32_t type_tag;  ///< 型タグ（reml_type_tag_t）
} reml_object_header_t;

/* ========== 型タグ定義 ========== */

/**
 * Reml ランタイムで使用する型タグ
 *
 * dec_ref 時の型別デストラクタディスパッチや
 * デバッグ時の型検証に使用する。
 *
 * Phase 1 では基本型のみ定義し、Phase 2 以降で拡張する。
 */
typedef enum {
    REML_TAG_INT       = 1,  ///< 整数型（i32, i64 等）
    REML_TAG_FLOAT     = 2,  ///< 浮動小数点型（f32, f64）
    REML_TAG_BOOL      = 3,  ///< 真偽値型
    REML_TAG_STRING    = 4,  ///< 文字列型（{ptr, len} FAT pointer）
    REML_TAG_TUPLE     = 5,  ///< タプル型
    REML_TAG_RECORD    = 6,  ///< レコード型
    REML_TAG_CLOSURE   = 7,  ///< クロージャ（{env*, code_ptr}）
    REML_TAG_ADT       = 8,  ///< 代数的データ型（{tag, payload}）
    // Phase 2 以降で追加予定:
    // REML_TAG_ARRAY, REML_TAG_SLICE, REML_TAG_CUSTOM, ...
} reml_type_tag_t;

/* ========== メモリ管理 API ========== */

/**
 * ヒープメモリを割り当てる
 *
 * 要求されたサイズに reml_object_header_t のサイズを加えた領域を確保し、
 * 8バイト境界に調整したアドレスを返す。ヘッダは初期化済み（refcount=1）。
 *
 * @param size 要求サイズ（バイト、ヘッダを除く）
 * @return 割り当てられたメモリへのポインタ（ヘッダの直後）
 * @note 割り当て失敗時は panic で異常終了する
 * @note 返されるポインタはヘッダの直後を指す（呼び出し側はヘッダを意識不要）
 */
void* mem_alloc(size_t size);

/**
 * ヒープメモリを解放する
 *
 * mem_alloc で割り当てたメモリを解放する。
 * Phase 1 では主に dec_ref から間接的に呼ばれる。
 *
 * @param ptr 解放するメモリへのポインタ（ヘッダの直後を指すポインタ）
 * @note NULL ポインタを渡した場合は何もしない
 */
void mem_free(void* ptr);

/* ========== 参照カウント API ========== */

/**
 * 参照カウントをインクリメント
 *
 * オブジェクトの参照が増える際（代入、引数渡し等）に呼ばれる。
 *
 * @param ptr オブジェクトへのポインタ（ヘッダの直後）
 * @note NULL ポインタを渡した場合は何もしない
 * @note Phase 1 では単純インクリメント、Phase 2 で並行対応（アトミック操作）
 */
void inc_ref(void* ptr);

/**
 * 参照カウントをデクリメント
 *
 * オブジェクトの参照が減る際（スコープ終了、上書き等）に呼ばれる。
 * カウントが 0 になった場合、型タグに基づいてデストラクタを呼び出し、
 * 子オブジェクトの参照も再帰的に減らした後、メモリを解放する。
 *
 * @param ptr オブジェクトへのポインタ（ヘッダの直後）
 * @note NULL ポインタを渡した場合は何もしない
 */
void dec_ref(void* ptr);

/* ========== エラー処理 API ========== */

/**
 * パニック（プログラムを異常終了させる）
 *
 * 回復不能なエラー（メモリ割り当て失敗、境界外アクセス等）が
 * 発生した際に呼ばれる。エラーメッセージを stderr に出力し、
 * exit(1) でプログラムを終了する。
 *
 * Phase 1 実装注記:
 *   LLVM IR 側では panic(ptr, i64) の FAT ポインタ形式で宣言されているが、
 *   C 実装側では const char* として受け取り、NULL 終端を前提とする。
 *   長さパラメータ（i64）は無視可能。
 *
 * @param msg エラーメッセージ（NULL 終端文字列）
 * @note この関数は決して戻らない（noreturn 属性）
 */
void panic(const char* msg) __attribute__((noreturn));

/* ========== 観測用ユーティリティ ========== */

/**
 * i64 値を標準出力に出力（デバッグ用）
 *
 * Phase 1 での動作確認・テスト用に提供する簡易出力関数。
 * 本格的な I/O は Phase 2 以降で標準ライブラリとして整備する。
 *
 * @param value 出力する整数値
 * @note 改行は自動的に付与される
 */
void print_i64(int64_t value);

/* ========== 内部ヘルパー（実装側で使用） ========== */

/**
 * オブジェクトヘッダを取得
 *
 * ペイロードポインタからヘッダ構造体へのポインタを取得する。
 * 参照カウント操作や型タグ参照時に使用する。
 *
 * @param ptr オブジェクトへのポインタ（ヘッダの直後）
 * @return ヘッダ構造体へのポインタ
 * @note このマクロは実装内部でのみ使用すること
 */
#define REML_GET_HEADER(ptr) \
    ((reml_object_header_t*)((char*)(ptr) - sizeof(reml_object_header_t)))

/* ========== ユーティリティ関数（テスト用） ========== */

/**
 * オブジェクトの型タグを設定
 *
 * @param ptr オブジェクトへのポインタ（ヘッダの直後）
 * @param type_tag 設定する型タグ (reml_type_tag_t)
 */
void reml_set_type_tag(void* ptr, uint32_t type_tag);

/**
 * オブジェクトの型タグを取得
 *
 * @param ptr オブジェクトへのポインタ（ヘッダの直後）
 * @return 型タグ (reml_type_tag_t)
 */
uint32_t reml_get_type_tag(void* ptr);

#ifdef DEBUG
/**
 * アロケーション統計情報を出力（デバッグビルド時のみ）
 */
void reml_debug_print_alloc_stats(void);

/**
 * アロケーション回数を取得（デバッグビルド時のみ）
 */
size_t reml_debug_get_alloc_count(void);

/**
 * 解放回数を取得（デバッグビルド時のみ）
 */
size_t reml_debug_get_free_count(void);

/**
 * 参照カウント統計情報を出力（デバッグビルド時のみ）
 */
void reml_debug_print_refcount_stats(void);
#endif

#endif // REML_RUNTIME_H
