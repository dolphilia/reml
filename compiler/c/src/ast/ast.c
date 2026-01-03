#include "reml/ast/ast.h"

#include <stdlib.h>

reml_attr_list reml_attr_list_empty(void) {
  reml_attr_list attrs;
  attrs.is_pure = false;
  attrs.is_no_panic = false;
  attrs.pure_span = reml_span_make(0, 0, 1, 1, 1, 1);
  attrs.no_panic_span = reml_span_make(0, 0, 1, 1, 1, 1);
  return attrs;
}

static reml_expr *reml_expr_alloc(reml_expr_kind kind, reml_span span) {
  reml_expr *expr = (reml_expr *)calloc(1, sizeof(reml_expr));
  if (!expr) {
    return NULL;
  }
  expr->kind = kind;
  expr->span = span;
  return expr;
}

reml_expr *reml_expr_make_literal(reml_span span, reml_literal literal) {
  reml_expr *expr = reml_expr_alloc(REML_EXPR_LITERAL, span);
  if (!expr) {
    return NULL;
  }
  expr->data.literal = literal;
  return expr;
}

reml_expr *reml_expr_make_ident(reml_span span, reml_string_view ident) {
  reml_expr *expr = reml_expr_alloc(REML_EXPR_IDENT, span);
  if (!expr) {
    return NULL;
  }
  expr->data.ident = ident;
  return expr;
}

reml_expr *reml_expr_make_unary(reml_span span, reml_token_kind op, reml_expr *operand) {
  reml_expr *expr = reml_expr_alloc(REML_EXPR_UNARY, span);
  if (!expr) {
    return NULL;
  }
  expr->data.unary.op = op;
  expr->data.unary.operand = operand;
  return expr;
}

reml_expr *reml_expr_make_ref(reml_span span, bool is_mutable, reml_expr *target) {
  reml_expr *expr = reml_expr_alloc(REML_EXPR_REF, span);
  if (!expr) {
    return NULL;
  }
  expr->data.ref.is_mutable = is_mutable;
  expr->data.ref.target = target;
  return expr;
}

reml_expr *reml_expr_make_binary(reml_span span, reml_token_kind op, reml_expr *left,
                                 reml_expr *right) {
  reml_expr *expr = reml_expr_alloc(REML_EXPR_BINARY, span);
  if (!expr) {
    return NULL;
  }
  expr->data.binary.op = op;
  expr->data.binary.left = left;
  expr->data.binary.right = right;
  return expr;
}

reml_expr *reml_expr_make_constructor(reml_span span, reml_string_view name, UT_array *args) {
  reml_expr *expr = reml_expr_alloc(REML_EXPR_CONSTRUCTOR, span);
  if (!expr) {
    return NULL;
  }
  expr->data.ctor.name = name;
  expr->data.ctor.args = args;
  expr->data.ctor.tag = -1;
  return expr;
}

reml_expr *reml_expr_make_tuple(reml_span span, UT_array *items) {
  reml_expr *expr = reml_expr_alloc(REML_EXPR_TUPLE, span);
  if (!expr) {
    return NULL;
  }
  expr->data.tuple = items;
  return expr;
}

reml_expr *reml_expr_make_record(reml_span span, UT_array *fields) {
  reml_expr *expr = reml_expr_alloc(REML_EXPR_RECORD, span);
  if (!expr) {
    return NULL;
  }
  expr->data.record = fields;
  return expr;
}

reml_expr *reml_expr_make_record_update(reml_span span, reml_expr *base, UT_array *fields) {
  reml_expr *expr = reml_expr_alloc(REML_EXPR_RECORD_UPDATE, span);
  if (!expr) {
    return NULL;
  }
  expr->data.record_update.base = base;
  expr->data.record_update.fields = fields;
  return expr;
}

reml_expr *reml_expr_make_block(reml_span span, reml_block_expr block) {
  reml_expr *expr = reml_expr_alloc(REML_EXPR_BLOCK, span);
  if (!expr) {
    return NULL;
  }
  expr->data.block = block;
  return expr;
}

reml_expr *reml_expr_make_if(reml_span span, reml_expr *condition, reml_expr *then_branch,
                             reml_expr *else_branch) {
  reml_expr *expr = reml_expr_alloc(REML_EXPR_IF, span);
  if (!expr) {
    return NULL;
  }
  expr->data.if_expr.condition = condition;
  expr->data.if_expr.then_branch = then_branch;
  expr->data.if_expr.else_branch = else_branch;
  return expr;
}

reml_expr *reml_expr_make_while(reml_span span, reml_expr *condition, reml_expr *body) {
  reml_expr *expr = reml_expr_alloc(REML_EXPR_WHILE, span);
  if (!expr) {
    return NULL;
  }
  expr->data.while_expr.condition = condition;
  expr->data.while_expr.body = body;
  return expr;
}

reml_expr *reml_expr_make_match(reml_span span, reml_expr *scrutinee, UT_array *arms) {
  reml_expr *expr = reml_expr_alloc(REML_EXPR_MATCH, span);
  if (!expr) {
    return NULL;
  }
  expr->data.match_expr.scrutinee = scrutinee;
  expr->data.match_expr.arms = arms;
  return expr;
}

static reml_pattern *reml_pattern_alloc(reml_pattern_kind kind, reml_span span) {
  reml_pattern *pattern = (reml_pattern *)calloc(1, sizeof(reml_pattern));
  if (!pattern) {
    return NULL;
  }
  pattern->kind = kind;
  pattern->span = span;
  return pattern;
}

reml_pattern *reml_pattern_make_wildcard(reml_span span) {
  return reml_pattern_alloc(REML_PATTERN_WILDCARD, span);
}

reml_pattern *reml_pattern_make_ident(reml_span span, reml_string_view ident) {
  reml_pattern *pattern = reml_pattern_alloc(REML_PATTERN_IDENT, span);
  if (!pattern) {
    return NULL;
  }
  pattern->data.ident = ident;
  return pattern;
}

reml_pattern *reml_pattern_make_literal(reml_span span, reml_literal literal) {
  reml_pattern *pattern = reml_pattern_alloc(REML_PATTERN_LITERAL, span);
  if (!pattern) {
    return NULL;
  }
  pattern->data.literal = literal;
  return pattern;
}

reml_pattern *reml_pattern_make_tuple(reml_span span, UT_array *items) {
  reml_pattern *pattern = reml_pattern_alloc(REML_PATTERN_TUPLE, span);
  if (!pattern) {
    return NULL;
  }
  pattern->data.items = items;
  return pattern;
}

reml_pattern *reml_pattern_make_record(reml_span span, UT_array *fields) {
  reml_pattern *pattern = reml_pattern_alloc(REML_PATTERN_RECORD, span);
  if (!pattern) {
    return NULL;
  }
  pattern->data.fields = fields;
  return pattern;
}

reml_pattern *reml_pattern_make_constructor(reml_span span, reml_string_view name,
                                            UT_array *items) {
  reml_pattern *pattern = reml_pattern_alloc(REML_PATTERN_CONSTRUCTOR, span);
  if (!pattern) {
    return NULL;
  }
  pattern->data.ctor.name = name;
  pattern->data.ctor.items = items;
  pattern->data.ctor.tag = -1;
  return pattern;
}

reml_pattern *reml_pattern_make_range(reml_span span, reml_literal start, reml_literal end,
                                      bool inclusive) {
  reml_pattern *pattern = reml_pattern_alloc(REML_PATTERN_RANGE, span);
  if (!pattern) {
    return NULL;
  }
  pattern->data.range.start = start;
  pattern->data.range.end = end;
  pattern->data.range.inclusive = inclusive;
  return pattern;
}

reml_stmt *reml_stmt_make_expr(reml_span span, reml_attr_list attrs, reml_expr *expr) {
  reml_stmt *stmt = (reml_stmt *)calloc(1, sizeof(reml_stmt));
  if (!stmt) {
    return NULL;
  }
  stmt->kind = REML_STMT_EXPR;
  stmt->span = span;
  stmt->attrs = attrs;
  stmt->data.expr = expr;
  return stmt;
}

reml_stmt *reml_stmt_make_return(reml_span span, reml_attr_list attrs, reml_expr *expr) {
  reml_stmt *stmt = (reml_stmt *)calloc(1, sizeof(reml_stmt));
  if (!stmt) {
    return NULL;
  }
  stmt->kind = REML_STMT_RETURN;
  stmt->span = span;
  stmt->attrs = attrs;
  stmt->data.expr = expr;
  return stmt;
}

reml_stmt *reml_stmt_make_val_decl(reml_span span, reml_attr_list attrs, reml_pattern *pattern,
                                   reml_expr *value, bool is_mutable) {
  reml_stmt *stmt = (reml_stmt *)calloc(1, sizeof(reml_stmt));
  if (!stmt) {
    return NULL;
  }
  stmt->kind = REML_STMT_VAL_DECL;
  stmt->span = span;
  stmt->attrs = attrs;
  stmt->data.val_decl.pattern = pattern;
  stmt->data.val_decl.value = value;
  stmt->data.val_decl.is_mutable = is_mutable;
  return stmt;
}

reml_stmt *reml_stmt_make_type_decl(reml_span span, reml_attr_list attrs, reml_string_view name,
                                    UT_array *variants) {
  reml_stmt *stmt = (reml_stmt *)calloc(1, sizeof(reml_stmt));
  if (!stmt) {
    return NULL;
  }
  stmt->kind = REML_STMT_TYPE_DECL;
  stmt->span = span;
  stmt->attrs = attrs;
  stmt->data.type_decl.name = name;
  stmt->data.type_decl.variants = variants;
  return stmt;
}

reml_compilation_unit *reml_compilation_unit_new(void) {
  reml_compilation_unit *unit = (reml_compilation_unit *)calloc(1, sizeof(reml_compilation_unit));
  if (!unit) {
    return NULL;
  }
  UT_icd stmt_icd = {sizeof(reml_stmt *), NULL, NULL, NULL};
  utarray_new(unit->statements, &stmt_icd);
  return unit;
}

void reml_compilation_unit_add_stmt(reml_compilation_unit *unit, reml_stmt *stmt) {
  if (!unit || !stmt) {
    return;
  }
  utarray_push_back(unit->statements, &stmt);
}

void reml_expr_free(reml_expr *expr) {
  if (!expr) {
    return;
  }
  switch (expr->kind) {
    case REML_EXPR_UNARY:
      reml_expr_free(expr->data.unary.operand);
      break;
    case REML_EXPR_REF:
      reml_expr_free(expr->data.ref.target);
      break;
    case REML_EXPR_BINARY:
      reml_expr_free(expr->data.binary.left);
      reml_expr_free(expr->data.binary.right);
      break;
    case REML_EXPR_CONSTRUCTOR:
      if (expr->data.ctor.args) {
        for (reml_expr **it = (reml_expr **)utarray_front(expr->data.ctor.args); it != NULL;
             it = (reml_expr **)utarray_next(expr->data.ctor.args, it)) {
          reml_expr_free(*it);
        }
        utarray_free(expr->data.ctor.args);
      }
      break;
    case REML_EXPR_TUPLE:
      if (expr->data.tuple) {
        for (reml_expr **it = (reml_expr **)utarray_front(expr->data.tuple); it != NULL;
             it = (reml_expr **)utarray_next(expr->data.tuple, it)) {
          reml_expr_free(*it);
        }
        utarray_free(expr->data.tuple);
      }
      break;
    case REML_EXPR_RECORD:
      if (expr->data.record) {
        for (reml_record_expr_field *it =
                 (reml_record_expr_field *)utarray_front(expr->data.record);
             it != NULL;
             it = (reml_record_expr_field *)utarray_next(expr->data.record, it)) {
          reml_expr_free(it->value);
        }
        utarray_free(expr->data.record);
      }
      break;
    case REML_EXPR_RECORD_UPDATE:
      reml_expr_free(expr->data.record_update.base);
      if (expr->data.record_update.fields) {
        for (reml_record_expr_field *it =
                 (reml_record_expr_field *)utarray_front(expr->data.record_update.fields);
             it != NULL;
             it = (reml_record_expr_field *)utarray_next(expr->data.record_update.fields, it)) {
          reml_expr_free(it->value);
        }
        utarray_free(expr->data.record_update.fields);
      }
      break;
    case REML_EXPR_BLOCK:
      if (expr->data.block.statements) {
        for (reml_stmt **it = (reml_stmt **)utarray_front(expr->data.block.statements);
             it != NULL;
             it = (reml_stmt **)utarray_next(expr->data.block.statements, it)) {
          reml_stmt_free(*it);
        }
        utarray_free(expr->data.block.statements);
      }
      reml_expr_free(expr->data.block.tail);
      break;
    case REML_EXPR_IF:
      reml_expr_free(expr->data.if_expr.condition);
      reml_expr_free(expr->data.if_expr.then_branch);
      reml_expr_free(expr->data.if_expr.else_branch);
      break;
    case REML_EXPR_WHILE:
      reml_expr_free(expr->data.while_expr.condition);
      reml_expr_free(expr->data.while_expr.body);
      break;
    case REML_EXPR_MATCH:
      reml_expr_free(expr->data.match_expr.scrutinee);
      if (expr->data.match_expr.arms) {
        for (reml_match_arm *it = (reml_match_arm *)utarray_front(expr->data.match_expr.arms);
             it != NULL;
             it = (reml_match_arm *)utarray_next(expr->data.match_expr.arms, it)) {
          reml_pattern_free(it->pattern);
          reml_expr_free(it->guard);
          reml_expr_free(it->body);
        }
        utarray_free(expr->data.match_expr.arms);
      }
      break;
    default:
      break;
  }
  free(expr);
}

void reml_stmt_free(reml_stmt *stmt) {
  if (!stmt) {
    return;
  }
  switch (stmt->kind) {
    case REML_STMT_VAL_DECL:
      reml_pattern_free(stmt->data.val_decl.pattern);
      reml_expr_free(stmt->data.val_decl.value);
      break;
    case REML_STMT_TYPE_DECL:
      if (stmt->data.type_decl.variants) {
        for (reml_type_decl_variant *it =
                 (reml_type_decl_variant *)utarray_front(stmt->data.type_decl.variants);
             it != NULL;
             it = (reml_type_decl_variant *)utarray_next(stmt->data.type_decl.variants, it)) {
          if (it->fields) {
            utarray_free(it->fields);
          }
        }
        utarray_free(stmt->data.type_decl.variants);
      }
      break;
    default:
      reml_expr_free(stmt->data.expr);
      break;
  }
  free(stmt);
}

void reml_pattern_free(reml_pattern *pattern) {
  if (!pattern) {
    return;
  }
  switch (pattern->kind) {
    case REML_PATTERN_TUPLE:
      if (pattern->data.items) {
        for (reml_pattern **it = (reml_pattern **)utarray_front(pattern->data.items); it != NULL;
             it = (reml_pattern **)utarray_next(pattern->data.items, it)) {
          reml_pattern_free(*it);
        }
        utarray_free(pattern->data.items);
      }
      break;
    case REML_PATTERN_RECORD:
      if (pattern->data.fields) {
        for (reml_pattern_field *it =
                 (reml_pattern_field *)utarray_front(pattern->data.fields);
             it != NULL;
             it = (reml_pattern_field *)utarray_next(pattern->data.fields, it)) {
          reml_pattern_free(it->pattern);
        }
        utarray_free(pattern->data.fields);
      }
      break;
    case REML_PATTERN_CONSTRUCTOR:
      if (pattern->data.ctor.items) {
        for (reml_pattern **it = (reml_pattern **)utarray_front(pattern->data.ctor.items);
             it != NULL;
             it = (reml_pattern **)utarray_next(pattern->data.ctor.items, it)) {
          reml_pattern_free(*it);
        }
        utarray_free(pattern->data.ctor.items);
      }
      break;
    default:
      break;
  }
  free(pattern);
}

void reml_compilation_unit_free(reml_compilation_unit *unit) {
  if (!unit) {
    return;
  }
  if (unit->statements) {
    for (reml_stmt **it = (reml_stmt **)utarray_front(unit->statements); it != NULL;
         it = (reml_stmt **)utarray_next(unit->statements, it)) {
      reml_stmt_free(*it);
    }
    utarray_free(unit->statements);
  }
  free(unit);
}
