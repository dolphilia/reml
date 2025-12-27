/**
 * boxing.c - プリミティブのボックス化・アンボックス化
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

void* reml_box_i64(int64_t value) {
    int64_t* payload = (int64_t*)mem_alloc(sizeof(int64_t));
    reml_set_type_tag(payload, REML_TAG_INT);
    *payload = value;
    return payload;
}

int64_t reml_unbox_i64(void* ptr) {
    if (ptr == NULL) {
        panic("i64 unbox target is null");
    }
    if (reml_get_type_tag(ptr) != REML_TAG_INT) {
        panic("i64 unbox type tag mismatch");
    }
    return *(int64_t*)ptr;
}

void* reml_box_bool(uint8_t value) {
    uint8_t* payload = (uint8_t*)mem_alloc(sizeof(uint8_t));
    reml_set_type_tag(payload, REML_TAG_BOOL);
    *payload = value ? 1u : 0u;
    return payload;
}

uint8_t reml_unbox_bool(void* ptr) {
    if (ptr == NULL) {
        panic("bool unbox target is null");
    }
    if (reml_get_type_tag(ptr) != REML_TAG_BOOL) {
        panic("bool unbox type tag mismatch");
    }
    return *(uint8_t*)ptr ? 1u : 0u;
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

void* reml_box_string(reml_string_t value) {
    reml_string_t* payload = (reml_string_t*)mem_alloc(sizeof(reml_string_t));
    reml_set_type_tag(payload, REML_TAG_STRING);
    *payload = value;
    return payload;
}

reml_string_t reml_unbox_string(void* ptr) {
    if (ptr == NULL) {
        panic("string unbox target is null");
    }
    if (reml_get_type_tag(ptr) != REML_TAG_STRING) {
        panic("string unbox type tag mismatch");
    }
    return *(reml_string_t*)ptr;
}
