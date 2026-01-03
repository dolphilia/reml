#ifndef REML_TYPECK_TYPE_H
#define REML_TYPECK_TYPE_H

#include <stdbool.h>
#include <stdint.h>

#include <utarray.h>

#include "reml/util/string_view.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
  REML_TYPE_ERROR,
  REML_TYPE_INT,
  REML_TYPE_BIGINT,
  REML_TYPE_FLOAT,
  REML_TYPE_BOOL,
  REML_TYPE_CHAR,
  REML_TYPE_STRING,
  REML_TYPE_UNIT,
  REML_TYPE_ENUM,
  REML_TYPE_TUPLE,
  REML_TYPE_RECORD,
  REML_TYPE_FUNCTION,
  REML_TYPE_REF,
  REML_TYPE_VAR
} reml_type_kind;

typedef struct reml_type reml_type;

typedef uint32_t reml_effect_set;

typedef struct reml_effect_row_var reml_effect_row_var;

typedef struct {
  reml_effect_set effects;
  reml_effect_row_var *tail;
} reml_effect_row;

struct reml_effect_row_var {
  uint32_t id;
  reml_effect_row *instance;
};

enum {
  REML_EFFECT_NONE = 0,
  REML_EFFECT_MUT = 1u << 0,
  REML_EFFECT_IO = 1u << 1,
  REML_EFFECT_PANIC = 1u << 2,
  REML_EFFECT_FFI = 1u << 3,
  REML_EFFECT_UNSAFE = 1u << 4
};

typedef struct {
  reml_string_view name;
  UT_array *fields;
  int32_t tag;
} reml_enum_variant;

typedef struct {
  reml_string_view name;
  reml_type *type;
} reml_record_field;

struct reml_type {
  reml_type_kind kind;
  union {
    struct {
      UT_array *variants;
    } enum_type;
    struct {
      UT_array *items;
    } tuple;
    struct {
      UT_array *fields;
    } record;
    struct {
      UT_array *params;
      reml_type *result;
      reml_effect_row effects;
    } function;
    struct {
      reml_type *target;
      bool is_mutable;
    } ref;
    struct {
      uint32_t id;
      reml_type *instance;
    } var;
  } data;
};

typedef struct {
  UT_array *arena;
  UT_array *effect_rows;
  UT_array *effect_row_vars;
  UT_array *numeric_vars;
  uint32_t next_var_id;
  uint32_t next_effect_row_id;
  reml_type *error_type;
  reml_type *prim_int;
  reml_type *prim_bigint;
  reml_type *prim_float;
  reml_type *prim_bool;
  reml_type *prim_char;
  reml_type *prim_string;
  reml_type *prim_unit;
} reml_type_ctx;

reml_effect_set reml_effect_union(reml_effect_set left, reml_effect_set right);
reml_effect_row reml_effect_row_make(reml_effect_set effects, reml_effect_row_var *tail);
reml_effect_row reml_effect_row_closed(reml_effect_set effects);
reml_effect_row_var *reml_effect_row_var_make(reml_type_ctx *ctx);

void reml_type_ctx_init(reml_type_ctx *ctx);
void reml_type_ctx_deinit(reml_type_ctx *ctx);

reml_type *reml_type_make_var(reml_type_ctx *ctx);
reml_type *reml_type_prune(reml_type *type);
bool reml_type_unify(reml_type_ctx *ctx, reml_type *left, reml_type *right);
void reml_type_mark_numeric(reml_type_ctx *ctx, reml_type *type);
void reml_type_apply_numeric_defaults(reml_type_ctx *ctx, reml_type *default_type);

reml_type *reml_type_error(reml_type_ctx *ctx);
reml_type *reml_type_int(reml_type_ctx *ctx);
reml_type *reml_type_bigint(reml_type_ctx *ctx);
reml_type *reml_type_float(reml_type_ctx *ctx);
reml_type *reml_type_bool(reml_type_ctx *ctx);
reml_type *reml_type_char(reml_type_ctx *ctx);
reml_type *reml_type_string(reml_type_ctx *ctx);
reml_type *reml_type_unit(reml_type_ctx *ctx);
reml_type *reml_type_make_enum(reml_type_ctx *ctx);
reml_type *reml_type_make_tuple(reml_type_ctx *ctx, UT_array *items);
reml_type *reml_type_make_record(reml_type_ctx *ctx, UT_array *fields);
reml_type *reml_type_make_function(reml_type_ctx *ctx, UT_array *params, reml_type *result,
                                   reml_effect_row effects);
reml_type *reml_type_make_ref(reml_type_ctx *ctx, reml_type *target, bool is_mutable);

#ifdef __cplusplus
}
#endif

#endif
