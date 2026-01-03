#include "reml/typeck/type.h"

#include <stdlib.h>
#include <string.h>

reml_effect_set reml_effect_union(reml_effect_set left, reml_effect_set right) {
  return (reml_effect_set)(left | right);
}

reml_effect_row reml_effect_row_make(reml_effect_set effects, reml_effect_row_var *tail) {
  reml_effect_row row = {.effects = effects, .tail = tail};
  return row;
}

reml_effect_row reml_effect_row_closed(reml_effect_set effects) {
  return reml_effect_row_make(effects, NULL);
}

static bool reml_type_is_numeric_kind(reml_type_kind kind) {
  return kind == REML_TYPE_INT || kind == REML_TYPE_BIGINT || kind == REML_TYPE_FLOAT;
}

static reml_effect_row *reml_effect_row_new(reml_type_ctx *ctx, reml_effect_set effects,
                                            reml_effect_row_var *tail) {
  reml_effect_row *row = (reml_effect_row *)calloc(1, sizeof(reml_effect_row));
  if (!row) {
    return NULL;
  }
  row->effects = effects;
  row->tail = tail;
  if (ctx) {
    if (!ctx->effect_rows) {
      UT_icd row_icd = {sizeof(reml_effect_row *), NULL, NULL, NULL};
      utarray_new(ctx->effect_rows, &row_icd);
    }
    utarray_push_back(ctx->effect_rows, &row);
  }
  return row;
}

reml_effect_row_var *reml_effect_row_var_make(reml_type_ctx *ctx) {
  if (!ctx) {
    return NULL;
  }
  reml_effect_row_var *var = (reml_effect_row_var *)calloc(1, sizeof(reml_effect_row_var));
  if (!var) {
    return NULL;
  }
  var->id = ctx->next_effect_row_id++;
  var->instance = NULL;
  if (!ctx->effect_row_vars) {
    UT_icd var_icd = {sizeof(reml_effect_row_var *), NULL, NULL, NULL};
    utarray_new(ctx->effect_row_vars, &var_icd);
  }
  utarray_push_back(ctx->effect_row_vars, &var);
  return var;
}

static bool reml_string_view_equal(reml_string_view left, reml_string_view right) {
  if (left.length != right.length) {
    return false;
  }
  if (left.length == 0) {
    return true;
  }
  return memcmp(left.data, right.data, left.length) == 0;
}

static bool reml_type_ctx_has_numeric(reml_type_ctx *ctx, reml_type *type) {
  if (!ctx || !ctx->numeric_vars || !type || type->kind != REML_TYPE_VAR) {
    return false;
  }
  for (reml_type **it = (reml_type **)utarray_front(ctx->numeric_vars); it != NULL;
       it = (reml_type **)utarray_next(ctx->numeric_vars, it)) {
    if (*it == type) {
      return true;
    }
  }
  return false;
}

static void reml_type_ctx_mark_numeric_var(reml_type_ctx *ctx, reml_type *type) {
  if (!ctx || !type || type->kind != REML_TYPE_VAR) {
    return;
  }
  if (!ctx->numeric_vars) {
    UT_icd var_icd = {sizeof(reml_type *), NULL, NULL, NULL};
    utarray_new(ctx->numeric_vars, &var_icd);
  }
  if (reml_type_ctx_has_numeric(ctx, type)) {
    return;
  }
  utarray_push_back(ctx->numeric_vars, &type);
}

static reml_enum_variant *reml_enum_variant_find(reml_type *enum_type, reml_string_view name) {
  if (!enum_type || enum_type->kind != REML_TYPE_ENUM || !enum_type->data.enum_type.variants) {
    return NULL;
  }
  for (reml_enum_variant *it =
           (reml_enum_variant *)utarray_front(enum_type->data.enum_type.variants);
       it != NULL;
       it = (reml_enum_variant *)utarray_next(enum_type->data.enum_type.variants, it)) {
    if (reml_string_view_equal(it->name, name)) {
      return it;
    }
  }
  return NULL;
}

static bool reml_enum_variant_merge(reml_type_ctx *ctx, reml_enum_variant *dst,
                                    const reml_enum_variant *src) {
  if (!dst || !src) {
    return false;
  }
  size_t dst_count = dst->fields ? utarray_len(dst->fields) : 0;
  size_t src_count = src->fields ? utarray_len(src->fields) : 0;
  if (dst_count != src_count) {
    return false;
  }
  if (dst_count == 0) {
    return true;
  }
  for (size_t i = 0; i < dst_count; ++i) {
    reml_type **dst_field = (reml_type **)utarray_eltptr(dst->fields, i);
    reml_type **src_field = (reml_type **)utarray_eltptr(src->fields, i);
    if (dst_field && src_field) {
      if (!reml_type_unify(ctx, *dst_field, *src_field)) {
        return false;
      }
    }
  }
  return true;
}

static bool reml_enum_variant_clone(reml_type_ctx *ctx, reml_type *enum_type,
                                    const reml_enum_variant *src) {
  if (!ctx || !enum_type || !src) {
    return false;
  }
  if (!enum_type->data.enum_type.variants) {
    UT_icd variant_icd = {sizeof(reml_enum_variant), NULL, NULL, NULL};
    utarray_new(enum_type->data.enum_type.variants, &variant_icd);
  }
  reml_enum_variant clone;
  clone.name = src->name;
  clone.tag = src->tag;
  clone.fields = NULL;
  if (src->fields) {
    UT_icd field_icd = {sizeof(reml_type *), NULL, NULL, NULL};
    utarray_new(clone.fields, &field_icd);
    for (reml_type **it = (reml_type **)utarray_front(src->fields); it != NULL;
         it = (reml_type **)utarray_next(src->fields, it)) {
      reml_type *field_type = *it;
      utarray_push_back(clone.fields, &field_type);
    }
  }
  utarray_push_back(enum_type->data.enum_type.variants, &clone);
  return true;
}

static reml_effect_row reml_effect_row_prune(reml_effect_row row) {
  if (row.tail && row.tail->instance) {
    reml_effect_row resolved = reml_effect_row_prune(*row.tail->instance);
    row.effects = reml_effect_union(row.effects, resolved.effects);
    row.tail = resolved.tail;
  }
  return row;
}

static bool reml_effect_row_occurs(reml_effect_row_var *var, reml_effect_row row) {
  if (!var) {
    return false;
  }
  row = reml_effect_row_prune(row);
  if (!row.tail) {
    return false;
  }
  if (row.tail == var) {
    return true;
  }
  if (row.tail->instance) {
    return reml_effect_row_occurs(var, *row.tail->instance);
  }
  return false;
}

static bool reml_effect_row_unify(reml_type_ctx *ctx, reml_effect_row left, reml_effect_row right);

static bool reml_effect_row_bind(reml_type_ctx *ctx, reml_effect_row_var *var,
                                 reml_effect_row row) {
  if (!var) {
    return false;
  }
  if (var->instance) {
    return reml_effect_row_unify(ctx, *var->instance, row);
  }
  row = reml_effect_row_prune(row);
  if (reml_effect_row_occurs(var, row)) {
    return false;
  }
  reml_effect_row *instance = reml_effect_row_new(ctx, row.effects, row.tail);
  if (!instance) {
    return false;
  }
  var->instance = instance;
  return true;
}

static bool reml_effect_row_unify(reml_type_ctx *ctx, reml_effect_row left, reml_effect_row right) {
  left = reml_effect_row_prune(left);
  right = reml_effect_row_prune(right);

  if (!left.tail && !right.tail) {
    return left.effects == right.effects;
  }

  reml_effect_set left_only = (reml_effect_set)(left.effects & ~right.effects);
  reml_effect_set right_only = (reml_effect_set)(right.effects & ~left.effects);

  if (left_only != REML_EFFECT_NONE && right_only != REML_EFFECT_NONE) {
    if (!left.tail || !right.tail) {
      return false;
    }
    reml_effect_row_var *shared = reml_effect_row_var_make(ctx);
    if (!shared) {
      return false;
    }
    reml_effect_row left_bind = reml_effect_row_make(right_only, shared);
    reml_effect_row right_bind = reml_effect_row_make(left_only, shared);
    if (!reml_effect_row_bind(ctx, left.tail, left_bind)) {
      return false;
    }
    if (!reml_effect_row_bind(ctx, right.tail, right_bind)) {
      return false;
    }
  } else if (left_only != REML_EFFECT_NONE) {
    if (!right.tail) {
      return false;
    }
    reml_effect_row bind = reml_effect_row_make(left_only, left.tail);
    if (!reml_effect_row_bind(ctx, right.tail, bind)) {
      return false;
    }
  } else if (right_only != REML_EFFECT_NONE) {
    if (!left.tail) {
      return false;
    }
    reml_effect_row bind = reml_effect_row_make(right_only, right.tail);
    if (!reml_effect_row_bind(ctx, left.tail, bind)) {
      return false;
    }
  }

  left = reml_effect_row_prune(left);
  right = reml_effect_row_prune(right);

  if (left.effects != right.effects) {
    return false;
  }

  if (left.tail && right.tail) {
    if (left.tail == right.tail) {
      return true;
    }
    reml_effect_row bind = reml_effect_row_make(REML_EFFECT_NONE, right.tail);
    return reml_effect_row_bind(ctx, left.tail, bind);
  }
  if (left.tail) {
    reml_effect_row bind = reml_effect_row_make(REML_EFFECT_NONE, NULL);
    return reml_effect_row_bind(ctx, left.tail, bind);
  }
  if (right.tail) {
    reml_effect_row bind = reml_effect_row_make(REML_EFFECT_NONE, NULL);
    return reml_effect_row_bind(ctx, right.tail, bind);
  }
  return true;
}

static bool reml_type_unify_enum(reml_type_ctx *ctx, reml_type *left, reml_type *right) {
  if (!left || !right || left->kind != REML_TYPE_ENUM || right->kind != REML_TYPE_ENUM) {
    return false;
  }
  if (left == right) {
    return true;
  }
  if (!left->data.enum_type.variants) {
    UT_icd variant_icd = {sizeof(reml_enum_variant), NULL, NULL, NULL};
    utarray_new(left->data.enum_type.variants, &variant_icd);
  }
  if (!right->data.enum_type.variants) {
    UT_icd variant_icd = {sizeof(reml_enum_variant), NULL, NULL, NULL};
    utarray_new(right->data.enum_type.variants, &variant_icd);
  }

  for (reml_enum_variant *it =
           (reml_enum_variant *)utarray_front(right->data.enum_type.variants);
       it != NULL;
       it = (reml_enum_variant *)utarray_next(right->data.enum_type.variants, it)) {
    reml_enum_variant *existing = reml_enum_variant_find(left, it->name);
    if (existing) {
      if (!reml_enum_variant_merge(ctx, existing, it)) {
        return false;
      }
      it->tag = existing->tag;
    } else {
      if (!reml_enum_variant_clone(ctx, left, it)) {
        return false;
      }
    }
  }

  for (reml_enum_variant *it =
           (reml_enum_variant *)utarray_front(left->data.enum_type.variants);
       it != NULL;
       it = (reml_enum_variant *)utarray_next(left->data.enum_type.variants, it)) {
    reml_enum_variant *existing = reml_enum_variant_find(right, it->name);
    if (existing) {
      if (!reml_enum_variant_merge(ctx, existing, it)) {
        return false;
      }
      existing->tag = it->tag;
    } else {
      if (!reml_enum_variant_clone(ctx, right, it)) {
        return false;
      }
    }
  }
  return true;
}

static reml_type *reml_type_new(reml_type_ctx *ctx, reml_type_kind kind) {
  reml_type *type = (reml_type *)calloc(1, sizeof(reml_type));
  if (!type) {
    return NULL;
  }
  type->kind = kind;
  if (ctx) {
    if (!ctx->arena) {
      UT_icd arena_icd = {sizeof(reml_type *), NULL, NULL, NULL};
      utarray_new(ctx->arena, &arena_icd);
    }
    utarray_push_back(ctx->arena, &type);
  }
  return type;
}

void reml_type_ctx_init(reml_type_ctx *ctx) {
  if (!ctx) {
    return;
  }
  ctx->arena = NULL;
  ctx->effect_rows = NULL;
  ctx->effect_row_vars = NULL;
  ctx->numeric_vars = NULL;
  ctx->next_var_id = 1;
  ctx->next_effect_row_id = 1;
  ctx->error_type = reml_type_new(ctx, REML_TYPE_ERROR);
  ctx->prim_int = reml_type_new(ctx, REML_TYPE_INT);
  ctx->prim_bigint = reml_type_new(ctx, REML_TYPE_BIGINT);
  ctx->prim_float = reml_type_new(ctx, REML_TYPE_FLOAT);
  ctx->prim_bool = reml_type_new(ctx, REML_TYPE_BOOL);
  ctx->prim_char = reml_type_new(ctx, REML_TYPE_CHAR);
  ctx->prim_string = reml_type_new(ctx, REML_TYPE_STRING);
  ctx->prim_unit = reml_type_new(ctx, REML_TYPE_UNIT);
}

void reml_type_ctx_deinit(reml_type_ctx *ctx) {
  if (!ctx) {
    return;
  }
  if (ctx->effect_rows) {
    for (reml_effect_row **it = (reml_effect_row **)utarray_front(ctx->effect_rows);
         it != NULL; it = (reml_effect_row **)utarray_next(ctx->effect_rows, it)) {
      free(*it);
    }
    utarray_free(ctx->effect_rows);
    ctx->effect_rows = NULL;
  }
  if (ctx->effect_row_vars) {
    for (reml_effect_row_var **it =
             (reml_effect_row_var **)utarray_front(ctx->effect_row_vars);
         it != NULL;
         it = (reml_effect_row_var **)utarray_next(ctx->effect_row_vars, it)) {
      free(*it);
    }
    utarray_free(ctx->effect_row_vars);
    ctx->effect_row_vars = NULL;
  }
  if (ctx->numeric_vars) {
    utarray_free(ctx->numeric_vars);
    ctx->numeric_vars = NULL;
  }
  if (ctx->arena) {
    for (reml_type **it = (reml_type **)utarray_front(ctx->arena); it != NULL;
         it = (reml_type **)utarray_next(ctx->arena, it)) {
      free(*it);
    }
    utarray_free(ctx->arena);
    ctx->arena = NULL;
  }
}

reml_type *reml_type_make_var(reml_type_ctx *ctx) {
  if (!ctx) {
    return NULL;
  }
  reml_type *type = reml_type_new(ctx, REML_TYPE_VAR);
  if (!type) {
    return NULL;
  }
  type->data.var.id = ctx->next_var_id++;
  type->data.var.instance = NULL;
  return type;
}

reml_type *reml_type_prune(reml_type *type) {
  if (!type) {
    return NULL;
  }
  if (type->kind == REML_TYPE_VAR && type->data.var.instance) {
    type->data.var.instance = reml_type_prune(type->data.var.instance);
    return type->data.var.instance;
  }
  return type;
}

static bool reml_type_occurs_in(reml_type *var, reml_type *type) {
  if (!var || !type) {
    return false;
  }
  type = reml_type_prune(type);
  if (type == var) {
    return true;
  }
  if (type->kind == REML_TYPE_TUPLE && type->data.tuple.items) {
    for (reml_type **it = (reml_type **)utarray_front(type->data.tuple.items); it != NULL;
         it = (reml_type **)utarray_next(type->data.tuple.items, it)) {
      if (reml_type_occurs_in(var, *it)) {
        return true;
      }
    }
  }
  if (type->kind == REML_TYPE_RECORD && type->data.record.fields) {
    for (reml_record_field *it =
             (reml_record_field *)utarray_front(type->data.record.fields);
         it != NULL;
         it = (reml_record_field *)utarray_next(type->data.record.fields, it)) {
      if (reml_type_occurs_in(var, it->type)) {
        return true;
      }
    }
  }
  if (type->kind == REML_TYPE_ENUM && type->data.enum_type.variants) {
    for (reml_enum_variant *it =
             (reml_enum_variant *)utarray_front(type->data.enum_type.variants);
         it != NULL; it = (reml_enum_variant *)utarray_next(type->data.enum_type.variants, it)) {
      if (it->fields) {
        for (reml_type **field = (reml_type **)utarray_front(it->fields); field != NULL;
             field = (reml_type **)utarray_next(it->fields, field)) {
          if (reml_type_occurs_in(var, *field)) {
            return true;
          }
        }
      }
    }
  }
  if (type->kind == REML_TYPE_FUNCTION) {
    if (type->data.function.params) {
      for (reml_type **it = (reml_type **)utarray_front(type->data.function.params); it != NULL;
           it = (reml_type **)utarray_next(type->data.function.params, it)) {
        if (reml_type_occurs_in(var, *it)) {
          return true;
        }
      }
    }
    if (reml_type_occurs_in(var, type->data.function.result)) {
      return true;
    }
  }
  if (type->kind == REML_TYPE_REF) {
    if (reml_type_occurs_in(var, type->data.ref.target)) {
      return true;
    }
  }
  return false;
}

static reml_record_field *reml_record_field_find(reml_type *record, reml_string_view name) {
  if (!record || record->kind != REML_TYPE_RECORD || !record->data.record.fields) {
    return NULL;
  }
  for (reml_record_field *it =
           (reml_record_field *)utarray_front(record->data.record.fields);
       it != NULL; it = (reml_record_field *)utarray_next(record->data.record.fields, it)) {
    if (reml_string_view_equal(it->name, name)) {
      return it;
    }
  }
  return NULL;
}

static bool reml_type_unify_composite(reml_type_ctx *ctx, reml_type *left, reml_type *right) {
  if (!left || !right) {
    return false;
  }
  if (left->kind != right->kind) {
    return false;
  }
  if (left->kind == REML_TYPE_TUPLE) {
    size_t left_count = left->data.tuple.items ? utarray_len(left->data.tuple.items) : 0;
    size_t right_count = right->data.tuple.items ? utarray_len(right->data.tuple.items) : 0;
    if (left_count != right_count) {
      return false;
    }
    for (size_t i = 0; i < left_count; ++i) {
      reml_type **left_item = (reml_type **)utarray_eltptr(left->data.tuple.items, i);
      reml_type **right_item = (reml_type **)utarray_eltptr(right->data.tuple.items, i);
      if (!reml_type_unify(ctx, *left_item, *right_item)) {
        return false;
      }
    }
    return true;
  }
  if (left->kind == REML_TYPE_RECORD) {
    size_t left_count = left->data.record.fields ? utarray_len(left->data.record.fields) : 0;
    size_t right_count = right->data.record.fields ? utarray_len(right->data.record.fields) : 0;
    if (left_count != right_count) {
      return false;
    }
    for (reml_record_field *it =
             (reml_record_field *)utarray_front(left->data.record.fields);
         it != NULL; it = (reml_record_field *)utarray_next(left->data.record.fields, it)) {
      reml_record_field *match = reml_record_field_find(right, it->name);
      if (!match) {
        return false;
      }
      if (!reml_type_unify(ctx, it->type, match->type)) {
        return false;
      }
    }
    return true;
  }
  if (left->kind == REML_TYPE_FUNCTION) {
    size_t left_count = left->data.function.params ? utarray_len(left->data.function.params) : 0;
    size_t right_count =
        right->data.function.params ? utarray_len(right->data.function.params) : 0;
    if (left_count != right_count) {
      return false;
    }
    if (!reml_effect_row_unify(ctx, left->data.function.effects, right->data.function.effects)) {
      return false;
    }
    for (size_t i = 0; i < left_count; ++i) {
      reml_type **left_item = (reml_type **)utarray_eltptr(left->data.function.params, i);
      reml_type **right_item = (reml_type **)utarray_eltptr(right->data.function.params, i);
      if (!reml_type_unify(ctx, *left_item, *right_item)) {
        return false;
      }
    }
    return reml_type_unify(ctx, left->data.function.result, right->data.function.result);
  }
  return false;
}

bool reml_type_unify(reml_type_ctx *ctx, reml_type *left, reml_type *right) {
  if (!left || !right) {
    return false;
  }

  left = reml_type_prune(left);
  right = reml_type_prune(right);

  if (left == right) {
    return true;
  }

  if (left->kind == REML_TYPE_ERROR || right->kind == REML_TYPE_ERROR) {
    return true;
  }

  if (left->kind == REML_TYPE_VAR) {
    if (reml_type_ctx_has_numeric(ctx, left)) {
      if (right->kind == REML_TYPE_VAR) {
        reml_type_ctx_mark_numeric_var(ctx, right);
      } else if (!reml_type_is_numeric_kind(right->kind)) {
        return false;
      }
    }
    if (reml_type_occurs_in(left, right)) {
      return false;
    }
    left->data.var.instance = right;
    return true;
  }

  if (right->kind == REML_TYPE_VAR) {
    if (reml_type_ctx_has_numeric(ctx, right)) {
      if (left->kind == REML_TYPE_VAR) {
        reml_type_ctx_mark_numeric_var(ctx, left);
      } else if (!reml_type_is_numeric_kind(left->kind)) {
        return false;
      }
    }
    if (reml_type_occurs_in(right, left)) {
      return false;
    }
    right->data.var.instance = left;
    return true;
  }

  if (left->kind == right->kind) {
    if (left->kind == REML_TYPE_ENUM) {
      return reml_type_unify_enum(ctx, left, right);
    }
    if (left->kind == REML_TYPE_TUPLE || left->kind == REML_TYPE_RECORD ||
        left->kind == REML_TYPE_FUNCTION) {
      return reml_type_unify_composite(ctx, left, right);
    }
    if (left->kind == REML_TYPE_REF) {
      if (left->data.ref.is_mutable != right->data.ref.is_mutable) {
        return false;
      }
      return reml_type_unify(ctx, left->data.ref.target, right->data.ref.target);
    }
    return true;
  }

  return false;
}

void reml_type_mark_numeric(reml_type_ctx *ctx, reml_type *type) {
  if (!ctx || !type) {
    return;
  }
  type = reml_type_prune(type);
  if (!type || type->kind != REML_TYPE_VAR) {
    return;
  }
  reml_type_ctx_mark_numeric_var(ctx, type);
}

bool reml_type_is_numeric_var(reml_type_ctx *ctx, reml_type *type) {
  return reml_type_ctx_has_numeric(ctx, type);
}

void reml_type_apply_numeric_defaults(reml_type_ctx *ctx, reml_type *default_type) {
  if (!ctx || !ctx->numeric_vars || !default_type) {
    return;
  }
  for (reml_type **it = (reml_type **)utarray_front(ctx->numeric_vars); it != NULL;
       it = (reml_type **)utarray_next(ctx->numeric_vars, it)) {
    reml_type *var = *it;
    if (!var) {
      continue;
    }
    var = reml_type_prune(var);
    if (!var || var->kind != REML_TYPE_VAR || var->data.var.instance) {
      continue;
    }
    var->data.var.instance = default_type;
  }
}

reml_type *reml_type_error(reml_type_ctx *ctx) {
  return ctx ? ctx->error_type : NULL;
}

reml_type *reml_type_int(reml_type_ctx *ctx) {
  return ctx ? ctx->prim_int : NULL;
}

reml_type *reml_type_bigint(reml_type_ctx *ctx) {
  return ctx ? ctx->prim_bigint : NULL;
}

reml_type *reml_type_float(reml_type_ctx *ctx) {
  return ctx ? ctx->prim_float : NULL;
}

reml_type *reml_type_bool(reml_type_ctx *ctx) {
  return ctx ? ctx->prim_bool : NULL;
}

reml_type *reml_type_char(reml_type_ctx *ctx) {
  return ctx ? ctx->prim_char : NULL;
}

reml_type *reml_type_string(reml_type_ctx *ctx) {
  return ctx ? ctx->prim_string : NULL;
}

reml_type *reml_type_unit(reml_type_ctx *ctx) {
  return ctx ? ctx->prim_unit : NULL;
}

reml_type *reml_type_make_enum(reml_type_ctx *ctx) {
  reml_type *type = reml_type_new(ctx, REML_TYPE_ENUM);
  if (!type) {
    return NULL;
  }
  UT_icd variant_icd = {sizeof(reml_enum_variant), NULL, NULL, NULL};
  utarray_new(type->data.enum_type.variants, &variant_icd);
  return type;
}

reml_type *reml_type_make_tuple(reml_type_ctx *ctx, UT_array *items) {
  reml_type *type = reml_type_new(ctx, REML_TYPE_TUPLE);
  if (!type) {
    return NULL;
  }
  type->data.tuple.items = items;
  return type;
}

reml_type *reml_type_make_record(reml_type_ctx *ctx, UT_array *fields) {
  reml_type *type = reml_type_new(ctx, REML_TYPE_RECORD);
  if (!type) {
    return NULL;
  }
  type->data.record.fields = fields;
  return type;
}

reml_type *reml_type_make_function(reml_type_ctx *ctx, UT_array *params, reml_type *result,
                                   reml_effect_row effects) {
  reml_type *type = reml_type_new(ctx, REML_TYPE_FUNCTION);
  if (!type) {
    return NULL;
  }
  type->data.function.params = params;
  type->data.function.result = result;
  type->data.function.effects = effects;
  return type;
}

reml_type *reml_type_make_ref(reml_type_ctx *ctx, reml_type *target, bool is_mutable) {
  reml_type *type = reml_type_new(ctx, REML_TYPE_REF);
  if (!type) {
    return NULL;
  }
  type->data.ref.target = target;
  type->data.ref.is_mutable = is_mutable;
  return type;
}
