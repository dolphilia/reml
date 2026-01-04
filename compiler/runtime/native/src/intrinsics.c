/*
 * intrinsics.c - LLVM lowering intrinsic stubs
 *
 * Backend が生成する暫定 intrinsic を runtime 側で解決するための実装。
 * Phase 1 では identity / 最小限の境界チェックのみを行う。
 */

#include "../include/reml_runtime.h"

int64_t reml_value_i64(int64_t value) {
    return value;
}

uint8_t reml_value_bool(uint8_t value) {
    return value ? 1u : 0u;
}

void* reml_value_ptr(void* value) {
    return value;
}

reml_string_t reml_value_str(reml_string_t value) {
    return value;
}

void* reml_index_access(void* target, int64_t index) {
    if (target == NULL) {
        panic("index target is null");
    }
    if (index < 0) {
        panic("index out of bounds");
    }

    reml_object_header_t* header = REML_GET_HEADER(target);
    if (header->type_tag == REML_TAG_STRING) {
        reml_string_t* value = (reml_string_t*)target;
        if (value->data == NULL) {
            panic("index target string data is null");
        }
        if (index >= value->length) {
            panic("index out of bounds");
        }
        return (void*)(value->data + index);
    }

    reml_list_node_t* node = (reml_list_node_t*)target;
    int64_t cursor = index;
    while (node != NULL && cursor > 0) {
        node = node->tail;
        cursor--;
    }
    if (node == NULL) {
        panic("index out of bounds");
    }
    return node->head;
}
