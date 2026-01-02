#include "reml/typeck/type.h"

#include <stdlib.h>

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
