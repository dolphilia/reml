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
    default:
      break;
  }
  free(expr);
}

void reml_stmt_free(reml_stmt *stmt) {
  if (!stmt) {
    return;
  }
  reml_expr_free(stmt->data.expr);
  free(stmt);
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
