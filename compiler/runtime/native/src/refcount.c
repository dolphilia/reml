/**
 * refcount.c - 参照カウント実装
 *
 * Phase 1 では参照カウント（RC）ベースのメモリ管理を提供する。
 * ヒープオブジェクトの参照カウントをインクリメント/デクリメントし、
 * カウントが 0 になった際に型別デストラクタを呼び出して再帰的に解放する。
 *
 * Phase 2 以降で並行対応（アトミック操作）や循環参照検出を追加する。
 */

#include "../include/reml_runtime.h"
#include "../include/reml_atomic.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* ========== デバッグ支援 ========== */

#ifdef DEBUG
// 参照カウント操作の追跡カウンタ（デバッグ時のみ）
static atomic_size_t inc_ref_count = 0;
static atomic_size_t dec_ref_count = 0;
static atomic_size_t destroy_count = 0;

void reml_debug_print_refcount_stats(void) {
    size_t incs = atomic_load(&inc_ref_count);
    size_t decs = atomic_load(&dec_ref_count);
    size_t destroys = atomic_load(&destroy_count);
    fprintf(stderr, "[DEBUG] Refcount stats: inc_ref=%zu, dec_ref=%zu, destroy=%zu\n",
            incs, decs, destroys);
}
#endif

/* ========== 型別デストラクタ（前方宣言） ========== */

static void destroy_string(void* ptr);
static void destroy_tuple(void* ptr);
static void destroy_record(void* ptr);
static void destroy_array(void* ptr);
static void destroy_closure(void* ptr);
static void destroy_adt(void* ptr);
static void destroy_set(void* ptr);

/* ========== 参照カウント操作 ========== */

/**
 * 参照カウントをインクリメント
 *
 * オブジェクトの参照が増える際（代入、引数渡し等）に呼ばれる。
 *
 * @param ptr オブジェクトへのポインタ（ヘッダの直後）
 * @note NULL ポインタを渡した場合は何もしない
 * @note Phase 1 では単純インクリメント、Phase 2 で並行対応（アトミック操作）
 */
void inc_ref(void* ptr) {
    if (ptr == NULL) {
        // NULL ポインタは無視（安全性のため）
        return;
    }

    // ヘッダ位置を逆算
    reml_object_header_t* header = REML_GET_HEADER(ptr);

    // 参照カウントをインクリメント
    // Phase 1: 単純インクリメント（単一スレッド前提）
    // Phase 2: __atomic_fetch_add(&header->refcount, 1, __ATOMIC_RELAXED) に置換
    header->refcount++;

#ifdef DEBUG
    atomic_fetch_add(&inc_ref_count, 1);
    fprintf(stderr, "[DEBUG] inc_ref: ptr=%p, refcount=%u -> %u, type_tag=%u\n",
            ptr, header->refcount - 1, header->refcount, header->type_tag);
#endif
}

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
void dec_ref(void* ptr) {
    if (ptr == NULL) {
        // NULL ポインタは無視（安全性のため）
        return;
    }

    // ヘッダ位置を逆算
    reml_object_header_t* header = REML_GET_HEADER(ptr);

#ifdef DEBUG
    fprintf(stderr, "[DEBUG] dec_ref: ptr=%p, refcount=%u -> %u, type_tag=%u\n",
            ptr, header->refcount, header->refcount - 1, header->type_tag);
#endif

    // 参照カウントをデクリメント
    // Phase 1: 単純デクリメント（単一スレッド前提）
    // Phase 2: __atomic_fetch_sub(&header->refcount, 1, __ATOMIC_ACQ_REL) に置換
    header->refcount--;

#ifdef DEBUG
    atomic_fetch_add(&dec_ref_count, 1);
#endif

    // カウントが 0 になった場合、オブジェクトを破棄
    if (header->refcount == 0) {
#ifdef DEBUG
        fprintf(stderr, "[DEBUG] dec_ref: destroying object at %p (type_tag=%u)\n",
                ptr, header->type_tag);
        atomic_fetch_add(&destroy_count, 1);
#endif

        // 型別デストラクタを呼び出し
        switch (header->type_tag) {
            case REML_TAG_STRING:
                destroy_string(ptr);
                break;
            case REML_TAG_TUPLE:
                destroy_tuple(ptr);
                break;
            case REML_TAG_RECORD:
                destroy_record(ptr);
                break;
            case REML_TAG_ARRAY:
                destroy_array(ptr);
                break;
            case REML_TAG_CLOSURE:
                destroy_closure(ptr);
                break;
            case REML_TAG_ADT:
                destroy_adt(ptr);
                break;
            case REML_TAG_SET:
                destroy_set(ptr);
                break;
            case REML_TAG_INT:
            case REML_TAG_FLOAT:
            case REML_TAG_BOOL:
            case REML_TAG_CHAR:
                // プリミティブ型：子オブジェクトなし、デストラクタ不要
                break;
            default:
#ifdef DEBUG
                fprintf(stderr, "[DEBUG] dec_ref: unknown type_tag=%u, skipping destructor\n",
                        header->type_tag);
#endif
                break;
        }

        // メモリを解放
        mem_free(ptr);
    }
}

/* ========== 型別デストラクタ実装 ========== */

/**
 * 文字列オブジェクトのデストラクタ
 *
 * Phase 1 実装注記:
 *   文字列は {ptr data, i64 len} の FAT ポインタ形式で表現される。
 *   現在の実装では data ポインタは別途 mem_alloc で確保されていると仮定し、
 *   dec_ref で再帰的に解放する。
 *   ただし、Phase 1 では文字列リテラルが静的領域に配置される可能性があるため、
 *   実装時に要確認（TODO: コンパイラ側との整合）。
 *
 * @param ptr 文字列オブジェクトへのポインタ
 */
static void destroy_string(void* ptr) {
    // Phase 1: 文字列ペイロードの解放
    // 文字列構造: {i8* data, i64 len}
    typedef struct {
        char* data;
        int64_t len;
    } string_t;
    string_t* str = (string_t*)ptr;

    // data ポインタが別途確保されたヒープ領域の場合、dec_ref で解放
    // Phase 1 では文字列リテラルが静的領域にある可能性があるため、
    // ここでは data が NULL でないことのみ確認し、実際の解放は保留
    // TODO: コンパイラ側で文字列リテラルの所有権モデルを確定後に実装
    (void)str;  // 未使用警告抑制

#ifdef DEBUG
    fprintf(stderr, "[DEBUG] destroy_string: ptr=%p, data=%p, len=%lld\n",
            ptr, str->data, (long long)str->len);
#endif

    // Phase 2 以降: str->data が RC 管理されている場合は dec_ref(str->data)
}

/**
 * タプルオブジェクトのデストラクタ
 *
 * Phase 1 実装注記:
 *   タプルは {T0, T1, ...} の構造体として表現される。
 *   各要素がポインタ型（RC管理対象）の場合、再帰的に dec_ref を呼び出す。
 *   Phase 1 では型メタデータテーブルがないため、型タグのみで判定する。
 *   簡易実装として、すべての要素を void* として扱い、NULL でなければ dec_ref する。
 *
 * @param ptr タプルオブジェクトへのポインタ
 */
static void destroy_tuple(void* ptr) {
    reml_destroy_tuple(ptr);
}

/**
 * レコードオブジェクトのデストラクタ
 *
 * Phase 1 実装注記:
 *   レコードは {field0: T0, field1: T1, ...} の構造体として表現される。
 *   タプルと同様、各フィールドがポインタ型の場合に dec_ref を呼び出す。
 *   Phase 1 では型メタデータがないため、簡易実装とする。
 *
 * @param ptr レコードオブジェクトへのポインタ
 */
static void destroy_record(void* ptr) {
    reml_destroy_record(ptr);
}

/**
 * 配列オブジェクトのデストラクタ
 *
 * Phase 3 実装注記:
 *   配列は {len, items} の最小 ABI を採用し、items は void* 配列。
 *   すべての要素を RC 対象として dec_ref し、配列バッファを解放する。
 *
 * @param ptr 配列オブジェクトへのポインタ
 */
static void destroy_array(void* ptr) {
    reml_destroy_array(ptr);
}

/**
 * クロージャオブジェクトのデストラクタ
 *
 * Phase 1 実装注記:
 *   クロージャは {env*, code_ptr} の構造として表現される。
 *   env が RC 管理されたヒープオブジェクトの場合、dec_ref で解放する。
 *   code_ptr は関数ポインタであり、解放不要。
 *
 * @param ptr クロージャオブジェクトへのポインタ
 */
static void destroy_closure(void* ptr) {
    reml_closure_t* closure = (reml_closure_t*)ptr;

    // 環境ポインタが NULL でなければ dec_ref で解放
    if (closure->env != NULL) {
        dec_ref(closure->env);
    }

#ifdef DEBUG
    fprintf(stderr, "[DEBUG] destroy_closure: ptr=%p, env=%p, code_ptr=%p\n",
            ptr, closure->env, closure->code_ptr);
#endif
}

/**
 * 代数的データ型（ADT）オブジェクトのデストラクタ
 *
 * Phase 1 実装注記:
 *   ADT は {i32 tag, [payload]} の構造として表現される。
 *   payload の型は tag によって決まるが、Phase 1 では型メタデータがないため、
 *   簡易実装として payload 全体を void* として扱う。
 *   Phase 2 以降で tag ごとのペイロード型情報を取得し、適切に dec_ref する。
 *
 * @param ptr ADT オブジェクトへのポインタ
 */
static void destroy_adt(void* ptr) {
    // Phase 1: ADT 構造: {i32 tag, void* payload}
    typedef struct {
        int32_t tag;
        void* payload;
    } adt_t;
    adt_t* adt = (adt_t*)ptr;

    // payload が NULL でなければ dec_ref で解放
    // ただし、payload が常にポインタであるとは限らないため、
    // Phase 2 でメタデータを導入して判定する必要がある
    if (adt->payload != NULL) {
        dec_ref(adt->payload);
    }

#ifdef DEBUG
    fprintf(stderr, "[DEBUG] destroy_adt: ptr=%p, tag=%d, payload=%p\n",
            ptr, adt->tag, adt->payload);
#endif

    // Phase 2 以降: tag に基づいてペイロードの型を判定し、
    // 適切なデストラクタを呼び出す
}

/**
 * Set オブジェクトのデストラクタ
 *
 * Phase 3 実装注記:
 *   Set は {len, capacity, items[]} を持つ。
 *   items はポインタ同値で管理し、要素は RC 管理対象とする。
 *
 * @param ptr Set オブジェクトへのポインタ
 */
static void destroy_set(void* ptr) {
    reml_set_t* set = (reml_set_t*)ptr;
    if (set->items != NULL) {
        for (int64_t i = 0; i < set->len; i++) {
            dec_ref(set->items[i]);
        }
        free(set->items);
        set->items = NULL;
    }
}

/* ========== Phase 3: 破棄 API 実装 ========== */

void reml_destroy_tuple(void* ptr) {
    if (ptr == NULL) {
        return;
    }

    reml_tuple_t* tuple = (reml_tuple_t*)ptr;
    if (tuple->items != NULL) {
        for (int64_t i = 0; i < tuple->len; i++) {
            if (tuple->items[i] != NULL) {
                dec_ref(tuple->items[i]);
            }
        }
        free(tuple->items);
        tuple->items = NULL;
    }
}

void reml_destroy_record(void* ptr) {
    if (ptr == NULL) {
        return;
    }

    reml_record_t* record = (reml_record_t*)ptr;
    if (record->values != NULL) {
        for (int64_t i = 0; i < record->field_count; i++) {
            if (record->values[i] != NULL) {
                dec_ref(record->values[i]);
            }
        }
        free(record->values);
        record->values = NULL;
    }
}

void reml_destroy_array(void* ptr) {
    if (ptr == NULL) {
        return;
    }

    reml_array_t* array = (reml_array_t*)ptr;
    if (array->items != NULL) {
        for (int64_t i = 0; i < array->len; i++) {
            if (array->items[i] != NULL) {
                dec_ref(array->items[i]);
            }
        }
        free(array->items);
        array->items = NULL;
    }
}
