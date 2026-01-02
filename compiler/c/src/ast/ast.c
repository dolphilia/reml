#include "reml/ast/ast.h"

#include <stdlib.h>

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

reml_stmt *reml_stmt_make_expr(reml_span span, reml_expr *expr) {
  reml_stmt *stmt = (reml_stmt *)calloc(1, sizeof(reml_stmt));
  if (!stmt) {
    return NULL;
  }
  stmt->kind = REML_STMT_EXPR;
  stmt->span = span;
  stmt->data.expr = expr;
  return stmt;
}

reml_stmt *reml_stmt_make_return(reml_span span, reml_expr *expr) {
  reml_stmt *stmt = (reml_stmt *)calloc(1, sizeof(reml_stmt));
  if (!stmt) {
    return NULL;
  }
  stmt->kind = REML_STMT_RETURN;
  stmt->span = span;
  stmt->data.expr = expr;
  return stmt;
}

reml_stmt *reml_stmt_make_val_decl(reml_span span, reml_pattern *pattern, reml_expr *value) {
  reml_stmt *stmt = (reml_stmt *)calloc(1, sizeof(reml_stmt));
  if (!stmt) {
    return NULL;
  }
  stmt->kind = REML_STMT_VAL_DECL;
  stmt->span = span;
  stmt->data.val_decl.pattern = pattern;
  stmt->data.val_decl.value = value;
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
    case REML_EXPR_BINARY:
      reml_expr_free(expr->data.binary.left);
      reml_expr_free(expr->data.binary.right);
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
    case REML_EXPR_MATCH:
      reml_expr_free(expr->data.match_expr.scrutinee);
      if (expr->data.match_expr.arms) {
        for (reml_match_arm *it = (reml_match_arm *)utarray_front(expr->data.match_expr.arms);
             it != NULL;
             it = (reml_match_arm *)utarray_next(expr->data.match_expr.arms, it)) {
          reml_pattern_free(it->pattern);
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
