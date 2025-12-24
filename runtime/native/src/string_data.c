/*
 * string_data.c - 文字列データ参照ユーティリティ
 *
 * LLVM IR 側の `@reml_str_data(Str) -> ptr` に対応し、
 * `reml_string_t` の data フィールドを返す。
 */

#include "../include/reml_runtime.h"

const char* reml_str_data(reml_string_t value) {
    return value.data;
}
