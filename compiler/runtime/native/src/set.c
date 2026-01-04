/**
 * set.c - Set 実装（最小 ABI）
 *
 * Phase 3 ではポインタ同値で判定する単純な永続 Set を提供する。
 * Ord 比較やハッシュは Phase 4 以降で ABI 拡張により導入する。
 */

#include "../include/reml_runtime.h"
#include <stdlib.h>

static void** reml_set_alloc_items(int64_t capacity) {
    if (capacity <= 0) {
        return NULL;
    }

    void** items = (void**)calloc((size_t)capacity, sizeof(void*));
    if (items == NULL) {
        panic("Set allocation failed");
    }
    return items;
}

void* reml_set_new(void) {
    reml_set_t* set = (reml_set_t*)mem_alloc(sizeof(reml_set_t));
    reml_set_type_tag(set, REML_TAG_SET);
    set->len = 0;
    set->capacity = 0;
    set->items = NULL;
    return set;
}

int32_t reml_set_contains(void* set_ptr, void* value_ptr) {
    if (set_ptr == NULL) {
        panic("set contains target is null");
    }

    reml_set_t* set = (reml_set_t*)set_ptr;
    for (int64_t i = 0; i < set->len; i++) {
        if (set->items[i] == value_ptr) {
            return 1;
        }
    }
    return 0;
}

int64_t reml_set_len(void* set_ptr) {
    if (set_ptr == NULL) {
        panic("set len target is null");
    }

    reml_set_t* set = (reml_set_t*)set_ptr;
    return set->len;
}

void* reml_set_insert(void* set_ptr, void* value_ptr) {
    if (set_ptr == NULL) {
        panic("set insert target is null");
    }

    reml_set_t* set = (reml_set_t*)set_ptr;
    int exists = 0;
    for (int64_t i = 0; i < set->len; i++) {
        if (set->items[i] == value_ptr) {
            exists = 1;
            break;
        }
    }

    int64_t new_len = set->len + (exists ? 0 : 1);
    reml_set_t* next = (reml_set_t*)mem_alloc(sizeof(reml_set_t));
    reml_set_type_tag(next, REML_TAG_SET);
    next->len = new_len;
    next->capacity = new_len;
    next->items = reml_set_alloc_items(new_len);

    for (int64_t i = 0; i < set->len; i++) {
        next->items[i] = set->items[i];
        inc_ref(next->items[i]);
    }

    if (!exists) {
        next->items[set->len] = value_ptr;
        inc_ref(value_ptr);
    }

    return next;
}
