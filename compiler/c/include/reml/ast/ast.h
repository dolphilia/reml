#ifndef REML_AST_AST_H
#define REML_AST_AST_H

#include <stdbool.h>
#include <stdint.h>

#include <utarray.h>

#include "reml/lexer/token.h"
#include "reml/util/span.h"
#include "reml/util/string_view.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
  REML_LITERAL_INT,
  REML_LITERAL_BIGINT,
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
  REML_EXPR_BINARY,
  REML_EXPR_BLOCK,
  REML_EXPR_IF,
  REML_EXPR_WHILE,
  REML_EXPR_MATCH
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

typedef struct {
  UT_array *statements;
  reml_expr *tail;
} reml_block_expr;

typedef struct {
  reml_expr *condition;
  reml_expr *then_branch;
  reml_expr *else_branch;
} reml_if_expr;

typedef struct {
  reml_expr *condition;
  reml_expr *body;
} reml_while_expr;

typedef enum {
  REML_PATTERN_WILDCARD,
  REML_PATTERN_IDENT,
  REML_PATTERN_LITERAL,
  REML_PATTERN_TUPLE,
  REML_PATTERN_RECORD,
  REML_PATTERN_CONSTRUCTOR
} reml_pattern_kind;

typedef struct reml_pattern reml_pattern;
typedef struct reml_type reml_type;

typedef uint32_t reml_symbol_id;
#define REML_SYMBOL_ID_INVALID 0u

typedef struct {
  reml_string_view name;
  reml_pattern *pattern;
} reml_pattern_field;

typedef struct {
  reml_string_view name;
  UT_array *items;
  int32_t tag;
} reml_pattern_constructor;

typedef struct {
  reml_literal start;
  reml_literal end;
  bool inclusive;
} reml_pattern_range;

struct reml_pattern {
  reml_pattern_kind kind;
  reml_span span;
  reml_symbol_id symbol_id;
  reml_type *type;
  union {
    reml_string_view ident;
    reml_literal literal;
    UT_array *items;
    UT_array *fields;
    reml_pattern_constructor ctor;
    reml_pattern_range range;
  } data;
};

typedef struct {
  reml_pattern *pattern;
  reml_expr *guard;
  reml_expr *body;
} reml_match_arm;

typedef struct {
  reml_expr *scrutinee;
  UT_array *arms;
} reml_match_expr;

struct reml_expr {
  reml_expr_kind kind;
  reml_span span;
  reml_symbol_id symbol_id;
  reml_type *type;
  union {
    reml_literal literal;
    reml_string_view ident;
    reml_unary_expr unary;
    reml_binary_expr binary;
    reml_block_expr block;
    reml_if_expr if_expr;
    reml_while_expr while_expr;
    reml_match_expr match_expr;
  } data;
};

typedef enum {
  REML_STMT_EXPR,
  REML_STMT_RETURN,
  REML_STMT_VAL_DECL
} reml_stmt_kind;

typedef struct reml_stmt {
  reml_stmt_kind kind;
  reml_span span;
  union {
    reml_expr *expr;
    struct {
      reml_pattern *pattern;
      reml_expr *value;
    } val_decl;
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
reml_expr *reml_expr_make_block(reml_span span, reml_block_expr block);
reml_expr *reml_expr_make_if(reml_span span, reml_expr *condition, reml_expr *then_branch,
                             reml_expr *else_branch);
reml_expr *reml_expr_make_while(reml_span span, reml_expr *condition, reml_expr *body);
reml_expr *reml_expr_make_match(reml_span span, reml_expr *scrutinee, UT_array *arms);

reml_pattern *reml_pattern_make_wildcard(reml_span span);
reml_pattern *reml_pattern_make_ident(reml_span span, reml_string_view ident);
reml_pattern *reml_pattern_make_literal(reml_span span, reml_literal literal);
reml_pattern *reml_pattern_make_tuple(reml_span span, UT_array *items);
reml_pattern *reml_pattern_make_record(reml_span span, UT_array *fields);
reml_pattern *reml_pattern_make_constructor(reml_span span, reml_string_view name, UT_array *items);
reml_pattern *reml_pattern_make_range(reml_span span, reml_literal start, reml_literal end,
                                      bool inclusive);

reml_stmt *reml_stmt_make_expr(reml_span span, reml_expr *expr);
reml_stmt *reml_stmt_make_return(reml_span span, reml_expr *expr);
reml_stmt *reml_stmt_make_val_decl(reml_span span, reml_pattern *pattern, reml_expr *value);

reml_compilation_unit *reml_compilation_unit_new(void);
void reml_compilation_unit_add_stmt(reml_compilation_unit *unit, reml_stmt *stmt);
void reml_compilation_unit_free(reml_compilation_unit *unit);

void reml_expr_free(reml_expr *expr);
void reml_stmt_free(reml_stmt *stmt);
void reml_pattern_free(reml_pattern *pattern);

#ifdef __cplusplus
}
#endif

#endif
