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
  REML_TYPE_VAR
} reml_type_kind;

typedef struct reml_type reml_type;

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
      uint32_t effects;
    } function;
    struct {
      uint32_t id;
      reml_type *instance;
    } var;
  } data;
};

typedef struct {
  UT_array *arena;
  uint32_t next_var_id;
  reml_type *error_type;
  reml_type *prim_int;
  reml_type *prim_bigint;
  reml_type *prim_float;
  reml_type *prim_bool;
  reml_type *prim_char;
  reml_type *prim_string;
  reml_type *prim_unit;
} reml_type_ctx;

void reml_type_ctx_init(reml_type_ctx *ctx);
void reml_type_ctx_deinit(reml_type_ctx *ctx);

reml_type *reml_type_make_var(reml_type_ctx *ctx);
reml_type *reml_type_prune(reml_type *type);
bool reml_type_unify(reml_type_ctx *ctx, reml_type *left, reml_type *right);

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

#ifdef __cplusplus
}
#endif

#endif
