/**
 * record.c - Record リテラル生成
 *
 * Record リテラル用の最小 API を提供する。
 * フィールド値は RC 対象のヒープポインタとして扱い、生成時に inc_ref する。
 */

#include "../include/reml_runtime.h"
#include <stdarg.h>
#include <stdlib.h>

static void** reml_record_alloc_values(int64_t field_count) {
    if (field_count <= 0) {
        return NULL;
    }

    void** values = (void**)calloc((size_t)field_count, sizeof(void*));
    if (values == NULL) {
        panic("Record allocation failed");
    }
    return values;
}

void* reml_record_from(int64_t field_count, ...) {
    if (field_count < 0) {
        panic("record field_count is negative");
    }

    reml_record_t* record = (reml_record_t*)mem_alloc(sizeof(reml_record_t));
    reml_set_type_tag(record, REML_TAG_RECORD);
    record->field_count = field_count;
    record->values = reml_record_alloc_values(field_count);

    va_list args;
    va_start(args, field_count);
    for (int64_t i = 0; i < field_count; i++) {
        void* value = va_arg(args, void*);
        record->values[i] = value;
        if (value != NULL) {
            inc_ref(value);
        }
    }
    va_end(args);

    return record;
}
