/**
 * array.c - Array リテラル生成
 *
 * Array リテラル用の最小 API を提供する。
 * 要素は RC 対象のヒープポインタとして扱い、生成時に inc_ref する。
 */

#include "../include/reml_runtime.h"
#include <stdarg.h>
#include <stdlib.h>

static void** reml_array_alloc_items(int64_t len) {
    if (len <= 0) {
        return NULL;
    }

    void** items = (void**)calloc((size_t)len, sizeof(void*));
    if (items == NULL) {
        panic("Array allocation failed");
    }
    return items;
}

void* reml_array_from(int64_t len, ...) {
    if (len < 0) {
        panic("array length is negative");
    }

    reml_array_t* array = (reml_array_t*)mem_alloc(sizeof(reml_array_t));
    reml_set_type_tag(array, REML_TAG_ARRAY);
    array->len = len;
    array->items = reml_array_alloc_items(len);

    va_list args;
    va_start(args, len);
    for (int64_t i = 0; i < len; i++) {
        void* item = va_arg(args, void*);
        array->items[i] = item;
        if (item != NULL) {
            inc_ref(item);
        }
    }
    va_end(args);

    return array;
}
