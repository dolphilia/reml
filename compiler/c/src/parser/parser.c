#include "reml/parser/parser.h"

#include <limits.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include <utarray.h>

#include "reml/parser/operator_table.h"

static void reml_parser_set_error(reml_parser *parser, const char *message, reml_span span) {
  if (parser->has_error) {
    return;
  }
  parser->has_error = true;
  parser->error.message = message;
  parser->error.span = span;
}

static void reml_parser_advance(reml_parser *parser) {
  parser->current = reml_lexer_next(&parser->lexer);
}

static bool reml_parser_expect(reml_parser *parser, reml_token_kind kind, const char *message) {
  if (parser->current.kind == kind) {
    reml_parser_advance(parser);
    return true;
  }
  reml_parser_set_error(parser, message, parser->current.span);
  return false;
}

static reml_expr *reml_parse_expression_prec(reml_parser *parser, int min_prec);
static reml_stmt *reml_parse_statement(reml_parser *parser);

static bool reml_token_is_underscore(const reml_token *token) {
  return token && token->kind == REML_TOKEN_IDENT && token->lexeme.length == 1 &&
         token->lexeme.data[0] == '_';
}

static void reml_free_stmt_array(UT_array *statements) {
  if (!statements) {
    return;
  }
  for (reml_stmt **it = (reml_stmt **)utarray_front(statements); it != NULL;
       it = (reml_stmt **)utarray_next(statements, it)) {
    reml_stmt_free(*it);
  }
  utarray_free(statements);
}

static void reml_free_pattern_array(UT_array *items) {
  if (!items) {
    return;
  }
  for (reml_pattern **it = (reml_pattern **)utarray_front(items); it != NULL;
       it = (reml_pattern **)utarray_next(items, it)) {
    reml_pattern_free(*it);
  }
  utarray_free(items);
}

static void reml_free_match_arms(UT_array *arms) {
  if (!arms) {
    return;
  }
  for (reml_match_arm *it = (reml_match_arm *)utarray_front(arms); it != NULL;
       it = (reml_match_arm *)utarray_next(arms, it)) {
    reml_pattern_free(it->pattern);
    reml_expr_free(it->body);
  }
  utarray_free(arms);
}

static void reml_free_record_fields(UT_array *fields) {
  if (!fields) {
    return;
  }
  for (reml_pattern_field *it = (reml_pattern_field *)utarray_front(fields); it != NULL;
       it = (reml_pattern_field *)utarray_next(fields, it)) {
    reml_pattern_free(it->pattern);
  }
  utarray_free(fields);
}

static bool reml_int_literal_fits_i64(reml_string_view view, bool *fits_i64) {
  if (!fits_i64) {
    return false;
  }
  *fits_i64 = true;

  size_t index = 0;
  int base = 10;
  if (view.length >= 2 && view.data[0] == '0') {
    char prefix = view.data[1];
    if (prefix == 'x' || prefix == 'X') {
      base = 16;
      index = 2;
    } else if (prefix == 'o' || prefix == 'O') {
      base = 8;
      index = 2;
    } else if (prefix == 'b' || prefix == 'B') {
      base = 2;
      index = 2;
    }
  }

  uint64_t value = 0;
  bool has_digit = false;

  for (; index < view.length; ++index) {
    char c = view.data[index];
    if (c == '_') {
      continue;
    }

    int digit = -1;
    if (c >= '0' && c <= '9') {
      digit = c - '0';
    } else if (base == 16 && c >= 'a' && c <= 'f') {
      digit = 10 + (c - 'a');
    } else if (base == 16 && c >= 'A' && c <= 'F') {
      digit = 10 + (c - 'A');
    }

    if (digit < 0 || digit >= base) {
      return false;
    }

    has_digit = true;
    if (*fits_i64) {
      if (value > (uint64_t)(INT64_MAX - digit) / (uint64_t)base) {
        *fits_i64 = false;
      } else {
        value = value * (uint64_t)base + (uint64_t)digit;
      }
    }
  }

  return has_digit;
}

static bool reml_literal_from_token(reml_parser *parser, reml_token token, reml_literal *out) {
  if (!out) {
    return false;
  }

  switch (token.kind) {
    case REML_TOKEN_INT: {
      bool fits_i64 = false;
      if (!reml_int_literal_fits_i64(token.lexeme, &fits_i64)) {
        reml_parser_set_error(parser, "invalid integer literal", token.span);
        return false;
      }
      out->kind = fits_i64 ? REML_LITERAL_INT : REML_LITERAL_BIGINT;
      break;
    }
    case REML_TOKEN_FLOAT:
      out->kind = REML_LITERAL_FLOAT;
      break;
    case REML_TOKEN_STRING:
      out->kind = REML_LITERAL_STRING;
      break;
    case REML_TOKEN_CHAR:
      out->kind = REML_LITERAL_CHAR;
      break;
    case REML_TOKEN_KW_TRUE:
    case REML_TOKEN_KW_FALSE:
      out->kind = REML_LITERAL_BOOL;
      break;
    default:
      out->kind = REML_LITERAL_INT;
      break;
  }
  out->text = token.lexeme;
  return true;
}

static reml_pattern *reml_parse_pattern(reml_parser *parser);

static reml_pattern *reml_parse_pattern_primary(reml_parser *parser) {
  reml_token token = parser->current;

  if (token.kind == REML_TOKEN_IDENT) {
    reml_parser_advance(parser);
    if (reml_token_is_underscore(&token)) {
      return reml_pattern_make_wildcard(token.span);
    }
    if (parser->current.kind == REML_TOKEN_LPAREN) {
      reml_parser_advance(parser);
      UT_icd item_icd = {sizeof(reml_pattern *), NULL, NULL, NULL};
      UT_array *items = NULL;
      utarray_new(items, &item_icd);

      if (parser->current.kind != REML_TOKEN_RPAREN) {
        reml_pattern *first = reml_parse_pattern(parser);
        if (!first) {
          reml_free_pattern_array(items);
          return NULL;
        }
        utarray_push_back(items, &first);

        while (parser->current.kind == REML_TOKEN_COMMA) {
          reml_parser_advance(parser);
          if (parser->current.kind == REML_TOKEN_RPAREN) {
            break;
          }
          reml_pattern *next = reml_parse_pattern(parser);
          if (!next) {
            reml_free_pattern_array(items);
            return NULL;
          }
          utarray_push_back(items, &next);
        }
      }

      reml_token end_token = parser->current;
      if (!reml_parser_expect(parser, REML_TOKEN_RPAREN, "expected ')'")) {
        reml_free_pattern_array(items);
        return NULL;
      }
      reml_span span = reml_span_combine(token.span, end_token.span);
      return reml_pattern_make_constructor(span, token.lexeme, items);
    }
    return reml_pattern_make_ident(token.span, token.lexeme);
  }

  if (token.kind == REML_TOKEN_INT || token.kind == REML_TOKEN_FLOAT || token.kind == REML_TOKEN_STRING ||
      token.kind == REML_TOKEN_CHAR || token.kind == REML_TOKEN_KW_TRUE ||
      token.kind == REML_TOKEN_KW_FALSE) {
    reml_parser_advance(parser);
    reml_literal literal;
    if (!reml_literal_from_token(parser, token, &literal)) {
      return NULL;
    }
    return reml_pattern_make_literal(token.span, literal);
  }

  if (token.kind == REML_TOKEN_LPAREN) {
    reml_parser_advance(parser);
    reml_pattern *first = reml_parse_pattern(parser);
    if (!first) {
      return NULL;
    }
    if (parser->current.kind != REML_TOKEN_COMMA) {
      reml_token end_token = parser->current;
      if (!reml_parser_expect(parser, REML_TOKEN_RPAREN, "expected ')'")) {
        reml_pattern_free(first);
        return NULL;
      }
      (void)end_token;
      return first;
    }

    UT_icd item_icd = {sizeof(reml_pattern *), NULL, NULL, NULL};
    UT_array *items = NULL;
    utarray_new(items, &item_icd);
    utarray_push_back(items, &first);

    while (parser->current.kind == REML_TOKEN_COMMA) {
      reml_parser_advance(parser);
      if (parser->current.kind == REML_TOKEN_RPAREN) {
        break;
      }
      reml_pattern *next = reml_parse_pattern(parser);
      if (!next) {
        reml_free_pattern_array(items);
        return NULL;
      }
      utarray_push_back(items, &next);
    }

    reml_token end_token = parser->current;
    if (!reml_parser_expect(parser, REML_TOKEN_RPAREN, "expected ')'")) {
      reml_free_pattern_array(items);
      return NULL;
    }

    reml_span span = reml_span_combine(token.span, end_token.span);
    return reml_pattern_make_tuple(span, items);
  }

  if (token.kind == REML_TOKEN_LBRACE) {
    reml_parser_advance(parser);
    UT_icd field_icd = {sizeof(reml_pattern_field), NULL, NULL, NULL};
    UT_array *fields = NULL;
    utarray_new(fields, &field_icd);

    while (parser->current.kind != REML_TOKEN_RBRACE &&
           parser->current.kind != REML_TOKEN_EOF) {
      if (parser->current.kind != REML_TOKEN_IDENT) {
        reml_parser_set_error(parser, "expected record field", parser->current.span);
        reml_free_record_fields(fields);
        return NULL;
      }
      reml_token field_name = parser->current;
      reml_parser_advance(parser);
      reml_pattern_field field;
      field.name = field_name.lexeme;
      field.pattern = NULL;

      if (parser->current.kind == REML_TOKEN_COLON) {
        reml_parser_advance(parser);
        field.pattern = reml_parse_pattern(parser);
        if (!field.pattern) {
          reml_free_record_fields(fields);
          return NULL;
        }
      }

      utarray_push_back(fields, &field);

      if (parser->current.kind == REML_TOKEN_COMMA) {
        reml_parser_advance(parser);
        continue;
      }
      break;
    }

    reml_token end_token = parser->current;
    if (!reml_parser_expect(parser, REML_TOKEN_RBRACE, "expected '}'")) {
      reml_free_record_fields(fields);
      return NULL;
    }

    reml_span span = reml_span_combine(token.span, end_token.span);
    return reml_pattern_make_record(span, fields);
  }

  reml_parser_set_error(parser, "expected pattern", token.span);
  return NULL;
}

static reml_pattern *reml_parse_pattern(reml_parser *parser) {
  return reml_parse_pattern_primary(parser);
}

static reml_expr *reml_parse_primary(reml_parser *parser) {
  reml_token token = parser->current;

  switch (token.kind) {
    case REML_TOKEN_IDENT: {
      reml_parser_advance(parser);
      return reml_expr_make_ident(token.span, token.lexeme);
    }
    case REML_TOKEN_INT:
    case REML_TOKEN_FLOAT:
    case REML_TOKEN_STRING:
    case REML_TOKEN_CHAR:
    case REML_TOKEN_KW_TRUE:
    case REML_TOKEN_KW_FALSE: {
      reml_parser_advance(parser);
      reml_literal literal;
      if (!reml_literal_from_token(parser, token, &literal)) {
        return NULL;
      }
      return reml_expr_make_literal(token.span, literal);
    }
    case REML_TOKEN_LBRACE: {
      reml_token start_token = token;
      reml_parser_advance(parser);
      UT_icd stmt_icd = {sizeof(reml_stmt *), NULL, NULL, NULL};
      UT_array *statements = NULL;
      utarray_new(statements, &stmt_icd);
      reml_expr *tail = NULL;

      while (parser->current.kind != REML_TOKEN_RBRACE &&
             parser->current.kind != REML_TOKEN_EOF) {
        if (parser->current.kind == REML_TOKEN_KW_RETURN ||
            parser->current.kind == REML_TOKEN_KW_LET ||
            parser->current.kind == REML_TOKEN_KW_VAR) {
          reml_stmt *stmt = reml_parse_statement(parser);
          if (!stmt) {
            reml_free_stmt_array(statements);
            return NULL;
          }
          utarray_push_back(statements, &stmt);
          continue;
        }

        reml_expr *expr = reml_parse_expression_prec(parser, 0);
        if (!expr) {
          reml_free_stmt_array(statements);
          return NULL;
        }

        if (parser->current.kind == REML_TOKEN_SEMI) {
          reml_parser_advance(parser);
          reml_stmt *stmt = reml_stmt_make_expr(expr->span, expr);
          if (!stmt) {
            reml_expr_free(expr);
            reml_free_stmt_array(statements);
            return NULL;
          }
          utarray_push_back(statements, &stmt);
          continue;
        }

        tail = expr;
        break;
      }

      reml_token end_token = parser->current;
      if (!reml_parser_expect(parser, REML_TOKEN_RBRACE, "expected '}'")) {
        if (tail) {
          reml_expr_free(tail);
        }
        reml_free_stmt_array(statements);
        return NULL;
      }

      reml_block_expr block;
      block.statements = statements;
      block.tail = tail;
      reml_span span = reml_span_combine(start_token.span, end_token.span);
      return reml_expr_make_block(span, block);
    }
    case REML_TOKEN_KW_IF: {
      reml_parser_advance(parser);
      reml_expr *condition = reml_parse_expression_prec(parser, 0);
      if (!condition) {
        return NULL;
      }
      if (!reml_parser_expect(parser, REML_TOKEN_KW_THEN, "expected 'then'")) {
        reml_expr_free(condition);
        return NULL;
      }
      reml_expr *then_branch = reml_parse_expression_prec(parser, 0);
      if (!then_branch) {
        reml_expr_free(condition);
        return NULL;
      }
      reml_expr *else_branch = NULL;
      if (parser->current.kind == REML_TOKEN_KW_ELSE) {
        reml_parser_advance(parser);
        else_branch = reml_parse_expression_prec(parser, 0);
        if (!else_branch) {
          reml_expr_free(condition);
          reml_expr_free(then_branch);
          return NULL;
        }
      }
      reml_span span = reml_span_combine(condition->span, then_branch->span);
      if (else_branch) {
        span = reml_span_combine(condition->span, else_branch->span);
      }
      return reml_expr_make_if(span, condition, then_branch, else_branch);
    }
    case REML_TOKEN_KW_WHILE: {
      reml_parser_advance(parser);
      reml_expr *condition = reml_parse_expression_prec(parser, 0);
      if (!condition) {
        return NULL;
      }
      if (parser->current.kind != REML_TOKEN_LBRACE) {
        reml_parser_set_error(parser, "expected block after while", parser->current.span);
        reml_expr_free(condition);
        return NULL;
      }
      reml_expr *body = reml_parse_primary(parser);
      if (!body) {
        reml_expr_free(condition);
        return NULL;
      }
      if (body->kind != REML_EXPR_BLOCK) {
        reml_parser_set_error(parser, "expected block after while", body->span);
        reml_expr_free(condition);
        reml_expr_free(body);
        return NULL;
      }
      reml_span span = reml_span_combine(condition->span, body->span);
      return reml_expr_make_while(span, condition, body);
    }
    case REML_TOKEN_KW_MATCH: {
      reml_parser_advance(parser);
      reml_expr *scrutinee = reml_parse_expression_prec(parser, 0);
      if (!scrutinee) {
        return NULL;
      }
      if (!reml_parser_expect(parser, REML_TOKEN_KW_WITH, "expected 'with'")) {
        reml_expr_free(scrutinee);
        return NULL;
      }

      UT_icd arm_icd = {sizeof(reml_match_arm), NULL, NULL, NULL};
      UT_array *arms = NULL;
      utarray_new(arms, &arm_icd);

      bool parsed_arm = false;
      reml_span last_span = scrutinee->span;
      while (parser->current.kind == REML_TOKEN_PIPE ||
             parser->current.kind == REML_TOKEN_IDENT ||
             parser->current.kind == REML_TOKEN_INT ||
             parser->current.kind == REML_TOKEN_FLOAT ||
             parser->current.kind == REML_TOKEN_STRING ||
             parser->current.kind == REML_TOKEN_CHAR ||
             parser->current.kind == REML_TOKEN_KW_TRUE ||
             parser->current.kind == REML_TOKEN_KW_FALSE ||
             parser->current.kind == REML_TOKEN_LPAREN ||
             parser->current.kind == REML_TOKEN_LBRACE) {
        if (parser->current.kind == REML_TOKEN_PIPE) {
          reml_parser_advance(parser);
        }
        reml_pattern *pattern = reml_parse_pattern(parser);
        if (!pattern) {
          reml_free_match_arms(arms);
          reml_expr_free(scrutinee);
          return NULL;
        }
        if (!reml_parser_expect(parser, REML_TOKEN_ARROW, "expected '->'")) {
          reml_pattern_free(pattern);
          reml_free_match_arms(arms);
          reml_expr_free(scrutinee);
          return NULL;
        }
        reml_expr *body = reml_parse_expression_prec(parser, 0);
        if (!body) {
          reml_pattern_free(pattern);
          reml_free_match_arms(arms);
          reml_expr_free(scrutinee);
          return NULL;
        }
        reml_match_arm arm;
        arm.pattern = pattern;
        arm.body = body;
        utarray_push_back(arms, &arm);
        parsed_arm = true;
        last_span = body->span;
      }

      if (!parsed_arm) {
        reml_free_match_arms(arms);
        reml_expr_free(scrutinee);
        reml_parser_set_error(parser, "expected match arm", parser->current.span);
        return NULL;
      }

      reml_span span = reml_span_combine(scrutinee->span, last_span);
      return reml_expr_make_match(span, scrutinee, arms);
    }
    case REML_TOKEN_LPAREN: {
      reml_parser_advance(parser);
      reml_expr *expr = reml_parse_expression_prec(parser, 0);
      if (!expr) {
        return NULL;
      }
      if (!reml_parser_expect(parser, REML_TOKEN_RPAREN, "expected ')'")) {
        return NULL;
      }
      return expr;
    }
    default:
      reml_parser_set_error(parser, "expected expression", token.span);
      return NULL;
  }
}

static bool reml_is_unary_operator(reml_token_kind kind) {
  return kind == REML_TOKEN_MINUS || kind == REML_TOKEN_BANG;
}

static reml_expr *reml_parse_prefix(reml_parser *parser) {
  if (reml_is_unary_operator(parser->current.kind)) {
    reml_token op = parser->current;
    reml_parser_advance(parser);
    reml_expr *operand = reml_parse_prefix(parser);
    if (!operand) {
      return NULL;
    }
    reml_span span = reml_span_combine(op.span, operand->span);
    return reml_expr_make_unary(span, op.kind, operand);
  }
  return reml_parse_primary(parser);
}

static reml_expr *reml_parse_expression_prec(reml_parser *parser, int min_prec) {
  reml_expr *left = reml_parse_prefix(parser);
  if (!left) {
    return NULL;
  }

  while (true) {
    reml_operator_entry entry = {0};
    if (!reml_operator_lookup(parser->current.kind, &entry)) {
      break;
    }
    if ((int)entry.precedence < min_prec) {
      break;
    }
    reml_token op = parser->current;
    reml_parser_advance(parser);

    int next_min = entry.assoc == REML_ASSOC_LEFT ? entry.precedence + 1 : entry.precedence;
    reml_expr *right = reml_parse_expression_prec(parser, next_min);
    if (!right) {
      return NULL;
    }
    reml_span span = reml_span_combine(left->span, right->span);
    left = reml_expr_make_binary(span, op.kind, left, right);
  }

  return left;
}

static reml_stmt *reml_parse_statement(reml_parser *parser) {
  if (parser->current.kind == REML_TOKEN_KW_LET || parser->current.kind == REML_TOKEN_KW_VAR) {
    reml_token keyword = parser->current;
    reml_parser_advance(parser);
    reml_pattern *pattern = reml_parse_pattern(parser);
    if (!pattern) {
      return NULL;
    }
    if (!reml_parser_expect(parser, REML_TOKEN_EQ, "expected '=' after pattern")) {
      reml_pattern_free(pattern);
      return NULL;
    }
    reml_expr *value = reml_parse_expression_prec(parser, 0);
    if (!value) {
      reml_pattern_free(pattern);
      return NULL;
    }
    if (!reml_parser_expect(parser, REML_TOKEN_SEMI, "expected ';' after let binding")) {
      reml_pattern_free(pattern);
      reml_expr_free(value);
      return NULL;
    }
    reml_span span = reml_span_combine(keyword.span, value->span);
    return reml_stmt_make_val_decl(span, pattern, value);
  }

  if (parser->current.kind == REML_TOKEN_KW_RETURN) {
    reml_token keyword = parser->current;
    reml_parser_advance(parser);
    reml_expr *expr = reml_parse_expression_prec(parser, 0);
    if (!expr) {
      return NULL;
    }
    reml_span span = reml_span_combine(keyword.span, expr->span);
    if (!reml_parser_expect(parser, REML_TOKEN_SEMI, "expected ';' after return")) {
      return NULL;
    }
    return reml_stmt_make_return(span, expr);
  }

  reml_expr *expr = reml_parse_expression_prec(parser, 0);
  if (!expr) {
    return NULL;
  }
  reml_span span = expr->span;
  if (!reml_parser_expect(parser, REML_TOKEN_SEMI, "expected ';' after expression")) {
    return NULL;
  }
  return reml_stmt_make_expr(span, expr);
}

void reml_parser_init(reml_parser *parser, const char *input, size_t length) {
  reml_lexer_init(&parser->lexer, input, length);
  parser->has_error = false;
  parser->error.message = NULL;
  parser->error.span = reml_span_make(0, 0, 1, 1, 1, 1);
  reml_parser_advance(parser);
}

reml_compilation_unit *reml_parse_compilation_unit(reml_parser *parser) {
  reml_compilation_unit *unit = reml_compilation_unit_new();
  if (!unit) {
    reml_parser_set_error(parser, "out of memory", parser->current.span);
    return NULL;
  }

  while (parser->current.kind != REML_TOKEN_EOF && !parser->has_error) {
    reml_stmt *stmt = reml_parse_statement(parser);
    if (!stmt) {
      reml_compilation_unit_free(unit);
      return NULL;
    }
    reml_compilation_unit_add_stmt(unit, stmt);
  }

  if (parser->has_error) {
    reml_compilation_unit_free(unit);
    return NULL;
  }

  return unit;
}

const reml_parse_error *reml_parser_error(const reml_parser *parser) {
  if (!parser || !parser->has_error) {
    return NULL;
  }
  return &parser->error;
}
