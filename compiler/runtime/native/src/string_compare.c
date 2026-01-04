/* string_compare.c - 文字列比較関数（型クラス実装用）
 *
 * Phase 2 Week 22-23 で追加された型クラスビルトイン実装の一部。
 * Eq<String> および Ord<String> の実装で使用される。
 */

#include "../include/reml_runtime.h"
#include <string.h>

/**
 * 文字列の等価比較（Eq<String>::eq の実装）
 *
 * @param s1 比較対象の文字列1（FAT pointer）
 * @param s2 比較対象の文字列2（FAT pointer）
 * @return 等しければ1（true）、異なれば0（false）
 */
int32_t string_eq(const reml_string_t* s1, const reml_string_t* s2) {
    if (s1 == NULL || s2 == NULL) {
        // NULL チェック（どちらかが NULL なら等しくない）
        return (s1 == s2) ? 1 : 0;
    }

    // 長さが異なれば等しくない
    if (s1->length != s2->length) {
        return 0;
    }

    // 長さが0なら等しい（空文字列）
    if (s1->length == 0) {
        return 1;
    }

    // バイト単位で比較
    return (memcmp(s1->data, s2->data, (size_t)s1->length) == 0) ? 1 : 0;
}

/**
 * 文字列の順序比較（Ord<String>::compare の実装）
 *
 * @param s1 比較対象の文字列1（FAT pointer）
 * @param s2 比較対象の文字列2（FAT pointer）
 * @return s1 < s2 なら負の値、s1 == s2 なら 0、s1 > s2 なら正の値
 */
int32_t string_compare(const reml_string_t* s1, const reml_string_t* s2) {
    if (s1 == NULL || s2 == NULL) {
        // NULL チェック（NULL は小さいとみなす）
        if (s1 == NULL && s2 == NULL) return 0;
        return (s1 == NULL) ? -1 : 1;
    }

    // 共通部分の長さを決定
    int64_t min_len = (s1->length < s2->length) ? s1->length : s2->length;

    // 共通部分をバイト単位で比較
    int cmp = memcmp(s1->data, s2->data, (size_t)min_len);
    if (cmp != 0) {
        return cmp;
    }

    // 共通部分が等しい場合、長さで判定
    if (s1->length < s2->length) return -1;
    if (s1->length > s2->length) return 1;
    return 0;
}
