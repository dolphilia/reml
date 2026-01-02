#ifndef REML_AST_AST_H
#define REML_AST_AST_H

#include <stdbool.h>

#include <utarray.h>

#include "reml/lexer/token.h"
#include "reml/util/span.h"
#include "reml/util/string_view.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
  REML_LITERAL_INT,
  REML_LITERAL_FLOAT,
  REML_LITERAL_STRING,
  REML_LITERAL_CHAR,
  REML_LITERAL_BOOL
} reml_literal_kind;

typedef struct {
  reml_literal_kind kind;
  reml_string_view text;
} reml_literal;

typedef enum {
  REML_EXPR_LITERAL,
  REML_EXPR_IDENT,
  REML_EXPR_UNARY,
  REML_EXPR_BINARY
} reml_expr_kind;

typedef struct reml_expr reml_expr;

typedef struct {
  reml_token_kind op;
  reml_expr *operand;
} reml_unary_expr;

typedef struct {
  reml_token_kind op;
  reml_expr *left;
  reml_expr *right;
} reml_binary_expr;

struct reml_expr {
  reml_expr_kind kind;
  reml_span span;
  union {
    reml_literal literal;
    reml_string_view ident;
    reml_unary_expr unary;
    reml_binary_expr binary;
  } data;
};

typedef enum {
  REML_STMT_EXPR,
  REML_STMT_RETURN
} reml_stmt_kind;

typedef struct reml_stmt {
  reml_stmt_kind kind;
  reml_span span;
  union {
    reml_expr *expr;
  } data;
} reml_stmt;

typedef struct {
  UT_array *statements;
} reml_compilation_unit;

reml_expr *reml_expr_make_literal(reml_span span, reml_literal literal);
reml_expr *reml_expr_make_ident(reml_span span, reml_string_view ident);
reml_expr *reml_expr_make_unary(reml_span span, reml_token_kind op, reml_expr *operand);
reml_expr *reml_expr_make_binary(reml_span span, reml_token_kind op, reml_expr *left,
                                 reml_expr *right);

reml_stmt *reml_stmt_make_expr(reml_span span, reml_expr *expr);
reml_stmt *reml_stmt_make_return(reml_span span, reml_expr *expr);

reml_compilation_unit *reml_compilation_unit_new(void);
void reml_compilation_unit_add_stmt(reml_compilation_unit *unit, reml_stmt *stmt);
void reml_compilation_unit_free(reml_compilation_unit *unit);

void reml_expr_free(reml_expr *expr);
void reml_stmt_free(reml_stmt *stmt);

#ifdef __cplusplus
}
#endif

#endif
