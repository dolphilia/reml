/**
 * boxing.c - Float/Char のボックス化・アンボックス化
 *
 * Backend からのリテラル構築を支援するための最小 API を提供する。
 * ボックス化された値はヒープオブジェクトとして扱い、参照カウント管理対象となる。
 */

#include "../include/reml_runtime.h"

static int reml_char_is_valid(reml_char_t value) {
    if (value > 0x10FFFFu) {
        return 0;
    }
    if (value >= 0xD800u && value <= 0xDFFFu) {
        return 0;
    }
    return 1;
}

void* reml_box_float(double value) {
    double* payload = (double*)mem_alloc(sizeof(double));
    reml_set_type_tag(payload, REML_TAG_FLOAT);
    *payload = value;
    return payload;
}

double reml_unbox_float(void* ptr) {
    if (ptr == NULL) {
        panic("float unbox target is null");
    }
    if (reml_get_type_tag(ptr) != REML_TAG_FLOAT) {
        panic("float unbox type tag mismatch");
    }
    return *(double*)ptr;
}

void* reml_box_char(reml_char_t value) {
    if (!reml_char_is_valid(value)) {
        panic("char scalar value out of range");
    }
    reml_char_t* payload = (reml_char_t*)mem_alloc(sizeof(reml_char_t));
    reml_set_type_tag(payload, REML_TAG_CHAR);
    *payload = value;
    return payload;
}

reml_char_t reml_unbox_char(void* ptr) {
    if (ptr == NULL) {
        panic("char unbox target is null");
    }
    if (reml_get_type_tag(ptr) != REML_TAG_CHAR) {
        panic("char unbox type tag mismatch");
    }
    return *(reml_char_t*)ptr;
}
