#ifndef REML_RUNTIME_H
#define REML_RUNTIME_H

#include <stddef.h>
#include <stdint.h>

#include "reml_platform.h"

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
    REML_TAG_SET       = 9,  ///< Set 型（Phase 3: 最小 ABI）
    REML_TAG_CHAR      = 10, ///< 文字型（Unicode scalar）
    REML_TAG_ARRAY     = 11, ///< 配列型
    // Phase 2 以降で追加予定:
    // REML_TAG_SLICE, REML_TAG_CUSTOM, ...
} reml_type_tag_t;

/* ========== リテラル ABI（Phase 2） ========== */

/**
 * Reml Char 型（Unicode scalar value）
 *
 * 有効範囲は U+0000..U+10FFFF（サロゲート除外）。
 * ABI 上は 32bit のスカラ値として扱う。
 */
typedef uint32_t reml_char_t;

/**
 * Reml Tuple 型の最小 ABI
 *
 * メモリレイアウト:
 *   [reml_object_header_t] [reml_tuple_t payload]
 *
 * items は void* 配列へのポインタを保持し、各スロットは RC 対象の
 * ヒープポインタを格納する。非ポインタ値はボックス化して格納する。
 * items 配列の確保は malloc/calloc を想定し、破棄時に free で解放する。
 */
typedef struct {
    int64_t len;    ///< 要素数
    void** items;   ///< 要素ポインタ配列
} reml_tuple_t;

/**
 * Reml Record 型の最小 ABI
 *
 * メモリレイアウト:
 *   [reml_object_header_t] [reml_record_t payload]
 *
 * values の順序は Backend が確定する（フィールド名の正規化順）。
 * 値スロットは RC 対象のヒープポインタを格納する。
 * values 配列の確保は malloc/calloc を想定し、破棄時に free で解放する。
 */
typedef struct {
    int64_t field_count; ///< フィールド数
    void** values;       ///< 値ポインタ配列
} reml_record_t;

/**
 * Reml Array 型の最小 ABI
 *
 * メモリレイアウト:
 *   [reml_object_header_t] [reml_array_t payload]
 *
 * items は void* 配列へのポインタを保持し、RC 対象の要素を格納する。
 * items 配列の確保は malloc/calloc を想定し、破棄時に free で解放する。
 * 要素の非ポインタ化は Phase 3 でメタデータ化を検討する。
 */
typedef struct {
    int64_t len;    ///< 要素数
    void** items;   ///< 要素ポインタ配列
} reml_array_t;

/**
 * Reml Closure 型の最小 ABI
 *
 * メモリレイアウト:
 *   [reml_object_header_t] [reml_closure_t payload]
 *
 * env はクロージャ環境へのポインタ。RC 管理対象のヒープオブジェクトか NULL。
 * code_ptr は関数ポインタ（具体的な呼び出し規約は Backend で定義）。
 */
typedef struct {
    void* env;       ///< 環境ポインタ（NULL 可）
    void* code_ptr;  ///< 関数ポインタ（不透明）
} reml_closure_t;

/* ========== Record API（Phase 3） ========== */

/**
 * Record リテラル用のレコードを生成する
 *
 * @param field_count フィールド数
 * @return 新しい Record オブジェクト（不透明ポインタとして扱う）
 *
 * @note 可変長引数には正規化順の値スロットを渡す（RC 対象）。
 * @note フィールド値は `inc_ref` され、破棄時に `dec_ref` される。
 */
void* reml_record_from(int64_t field_count, ...);

/* ========== Array API（Phase 3） ========== */

/**
 * Array リテラル用の配列を生成する
 *
 * @param len 要素数
 * @return 新しい Array オブジェクト（不透明ポインタとして扱う）
 *
 * @note 可変長引数には各要素のヒープポインタを渡す（RC 対象）。
 * @note 要素は `inc_ref` され、破棄時に `dec_ref` される。
 */
void* reml_array_from(int64_t len, ...);

/* ========== Closure API（Phase 4） ========== */

/**
 * クロージャオブジェクトを生成する
 *
 * @param env クロージャ環境（RC 管理対象のヒープポインタ or NULL）
 * @param code_ptr 関数ポインタ（不透明）
 * @return 新しい Closure オブジェクト（不透明ポインタとして扱う）
 *
 * @note env が NULL でない場合は inc_ref で参照を保持する。
 * @note destroy 時に env は dec_ref される。
 */
void* reml_closure_new(void* env, void* code_ptr);

/**
 * クロージャの環境ポインタを取得する
 *
 * @param closure_ptr Closure オブジェクトへのポインタ
 * @return 環境ポインタ（NULL 可）
 */
void* reml_closure_env(void* closure_ptr);

/**
 * クロージャの関数ポインタを取得する
 *
 * @param closure_ptr Closure オブジェクトへのポインタ
 * @return 関数ポインタ
 */
void* reml_closure_code_ptr(void* closure_ptr);

/* ========== 参照カウント対象の区分（Phase 3） ========== */

/**
 * 参照カウント管理の対象
 *
 * - mem_alloc で確保したヒープオブジェクトは RC 対象。
 * - 即値（i64/bool/float/char の生値）は RC 対象外。
 * - box_* API を通じてボックス化されたプリミティブは RC 対象。
 * - Tuple/Record/Array の items/values 配列は RC 対象の要素ポインタを保持する。
 */

/* ========== String ABI（Phase 2） ========== */

/**
 * Reml String 型の構造（C側での便宜的定義）
 * LLVM IR では { ptr, i64 } として表現される
 */
typedef struct {
    const char* data;  ///< 文字列データへのポインタ
    int64_t length;    ///< 文字列の長さ（バイト数）
} reml_string_t;

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

/* ========== 破棄処理インタフェース（Phase 2 定義） ========== */

/**
 * タプルの破棄処理
 *
 * @param ptr タプル payload へのポインタ（ヘッダ直後）
 * @note Phase 3 で実装し、各要素の dec_ref と配列解放を行う
 */
void reml_destroy_tuple(void* ptr);

/**
 * レコードの破棄処理
 *
 * @param ptr レコード payload へのポインタ（ヘッダ直後）
 * @note Phase 3 で実装し、各フィールドの dec_ref と配列解放を行う
 */
void reml_destroy_record(void* ptr);

/**
 * 配列の破棄処理
 *
 * @param ptr 配列 payload へのポインタ（ヘッダ直後）
 * @note Phase 3 で実装し、各要素の dec_ref と配列解放を行う
 */
void reml_destroy_array(void* ptr);

/* ========== プリミティブの boxing API（Phase 3） ========== */

/**
 * i64 値をボックス化する
 *
 * @param value i64 値
 * @return ボックス化されたヒープポインタ
 */
void* reml_box_i64(int64_t value);

/**
 * i64 値をアンボックス化する
 *
 * @param ptr REML_TAG_INT のヒープポインタ
 * @return i64 値
 */
int64_t reml_unbox_i64(void* ptr);

/**
 * Bool 値をボックス化する
 *
 * @param value bool 値（0/1）
 * @return ボックス化されたヒープポインタ
 */
void* reml_box_bool(uint8_t value);

/**
 * Bool 値をアンボックス化する
 *
 * @param ptr REML_TAG_BOOL のヒープポインタ
 * @return bool 値（0/1）
 */
uint8_t reml_unbox_bool(void* ptr);

/**
 * 浮動小数点値をボックス化する
 *
 * @param value f64 値
 * @return ボックス化されたヒープポインタ
 */
void* reml_box_float(double value);

/**
 * 浮動小数点値をアンボックス化する
 *
 * @param ptr REML_TAG_FLOAT のヒープポインタ
 * @return f64 値
 */
double reml_unbox_float(void* ptr);

/**
 * Char 値をボックス化する
 *
 * @param value Unicode scalar value
 * @return ボックス化されたヒープポインタ
 */
void* reml_box_char(reml_char_t value);

/**
 * Char 値をアンボックス化する
 *
 * @param ptr REML_TAG_CHAR のヒープポインタ
 * @return Unicode scalar value
 */
reml_char_t reml_unbox_char(void* ptr);

/**
 * String 値をボックス化する
 *
 * @param value 文字列値
 * @return ボックス化されたヒープポインタ
 */
void* reml_box_string(reml_string_t value);

/**
 * String 値をアンボックス化する
 *
 * @param ptr REML_TAG_STRING のヒープポインタ
 * @return 文字列値
 */
reml_string_t reml_unbox_string(void* ptr);

/* ========== エラー処理 API ========== */

/**
 * パニック（プログラムを異常終了させる）
 *
 * 回復不能なエラー（メモリ割り当て失敗、境界外アクセス等）が
 * 発生した際に呼ばれる。エラーメッセージを stderr に出力し、
 * exit(1) でプログラムを終了する。
 *
 * Phase 1 実装注記:
 *   LLVM IR 側は panic(ptr) の宣言を正準とし、C 側では const char* として受け取り
 *   NULL 終端を前提とする。
 *
 * @param msg エラーメッセージ（NULL 終端文字列）
 * @note この関数は決して戻らない（noreturn 属性）
 */
void panic(const char* msg) REML_NORETURN;

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

/* ========== Set ABI（Phase 3: 最小実装） ========== */

typedef struct {
    int64_t len;        ///< 要素数
    int64_t capacity;   ///< バッファ容量
    void** items;       ///< 要素配列（ポインタ同値で比較）
} reml_set_t;

/**
 * 空の Set を生成する
 *
 * @return 新しい Set オブジェクト（不透明ポインタとして扱う）
 */
void* reml_set_new(void);

/**
 * Set に要素を追加する（永続データ構造として新しい Set を返す）
 *
 * @param set_ptr 既存の Set
 * @param value_ptr 追加する要素
 * @return 追加後の新しい Set
 */
void* reml_set_insert(void* set_ptr, void* value_ptr);

/**
 * Set に要素が含まれているかを判定する
 *
 * @param set_ptr Set
 * @param value_ptr 判定対象
 * @return 含まれていれば 1、含まれていなければ 0
 */
int32_t reml_set_contains(void* set_ptr, void* value_ptr);

/**
 * Set の要素数を返す
 *
 * @param set_ptr Set
 * @return 要素数
 */
int64_t reml_set_len(void* set_ptr);

/* ========== 型クラスビルトイン実装（Phase 2 Week 22-23） ========== */

/**
 * 文字列の等価比較（Eq<String>::eq の実装）
 *
 * @param s1 比較対象の文字列1
 * @param s2 比較対象の文字列2
 * @return 等しければ1（true）、異なれば0（false）
 */
int32_t string_eq(const reml_string_t* s1, const reml_string_t* s2);

/**
 * 文字列の順序比較（Ord<String>::compare の実装）
 *
 * @param s1 比較対象の文字列1
 * @param s2 比較対象の文字列2
 * @return s1 < s2 なら負の値、s1 == s2 なら 0、s1 > s2 なら正の値
 */
int32_t string_compare(const reml_string_t* s1, const reml_string_t* s2);

/**
 * 文字列データの取得（IR lowering 用の補助）
 *
 * LLVM IR 側の `@reml_str_data(Str) -> ptr` に対応し、
 * `reml_string_t` からデータポインタを取り出す。
 *
 * @param value 文字列値（構造体）
 * @return 文字列データへのポインタ（NULL 終端を想定）
 */
const char* reml_str_data(reml_string_t value);

/* ========== LLVM lowering intrinsic (Phase 1) ========== */

/**
 * Reml List の暫定ノード表現
 *
 * Phase 1 の LLVM lowering 向けに、単純な連結リストとして扱う。
 * Nil は NULL とし、index は先頭から線形走査で評価する。
 * 将来の永続データ構造へ移行する際は ABI を再定義する。
 */
typedef struct reml_list_node {
    void* head;                  ///< 要素の payload ポインタ
    struct reml_list_node* tail; ///< 次ノード（NULL で終端）
} reml_list_node_t;

/**
 * 型整形 intrinsic（Phase 1: identity stub）
 *
 * LLVM IR 側での cast/unbox 置換前の暫定シンボルとして提供する。
 */
int64_t reml_value_i64(int64_t value);
uint8_t reml_value_bool(uint8_t value);
void* reml_value_ptr(void* value);
reml_string_t reml_value_str(reml_string_t value);

/**
 * index アクセス intrinsic（Phase 1）
 *
 * - target は List/Str のいずれかを想定（暫定）。
 * - Str は byte index を採用し、境界外は panic。
 * - List は先頭から線形走査し、境界外は panic。
 */
void* reml_index_access(void* target, int64_t index);

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
