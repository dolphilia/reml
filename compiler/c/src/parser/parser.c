#include "reml/parser/parser.h"

#include <stdlib.h>

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
      switch (token.kind) {
        case REML_TOKEN_INT:
          literal.kind = REML_LITERAL_INT;
          break;
        case REML_TOKEN_FLOAT:
          literal.kind = REML_LITERAL_FLOAT;
          break;
        case REML_TOKEN_STRING:
          literal.kind = REML_LITERAL_STRING;
          break;
        case REML_TOKEN_CHAR:
          literal.kind = REML_LITERAL_CHAR;
          break;
        case REML_TOKEN_KW_TRUE:
        case REML_TOKEN_KW_FALSE:
          literal.kind = REML_LITERAL_BOOL;
          break;
        default:
          literal.kind = REML_LITERAL_INT;
          break;
      }
      literal.text = token.lexeme;
      return reml_expr_make_literal(token.span, literal);
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
