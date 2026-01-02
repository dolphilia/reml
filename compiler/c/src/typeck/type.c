#include "reml/typeck/type.h"

#include <stdlib.h>
#include <string.h>

static bool reml_string_view_equal(reml_string_view left, reml_string_view right) {
  if (left.length != right.length) {
    return false;
  }
  if (left.length == 0) {
    return true;
  }
  return memcmp(left.data, right.data, left.length) == 0;
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
  ctx->next_var_id = 1;
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
  if (!ctx || !ctx->arena) {
    return;
  }
  for (reml_type **it = (reml_type **)utarray_front(ctx->arena); it != NULL;
       it = (reml_type **)utarray_next(ctx->arena, it)) {
    free(*it);
  }
  utarray_free(ctx->arena);
  ctx->arena = NULL;
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
  return false;
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
  if (left->kind == REML_TYPE_FUNCTION) {
    size_t left_count = left->data.function.params ? utarray_len(left->data.function.params) : 0;
    size_t right_count =
        right->data.function.params ? utarray_len(right->data.function.params) : 0;
    if (left_count != right_count) {
      return false;
    }
    if (left->data.function.effects != right->data.function.effects) {
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
    if (reml_type_occurs_in(left, right)) {
      return false;
    }
    left->data.var.instance = right;
    return true;
  }

  if (right->kind == REML_TYPE_VAR) {
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
    if (left->kind == REML_TYPE_TUPLE || left->kind == REML_TYPE_FUNCTION) {
      return reml_type_unify_composite(ctx, left, right);
    }
    return true;
  }

  return false;
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
