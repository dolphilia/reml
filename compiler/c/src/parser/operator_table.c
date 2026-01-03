#include "reml/parser/operator_table.h"

#include <stddef.h>

static const reml_operator_entry kOperators[] = {
  {REML_TOKEN_COLONEQ, REML_PREC_ASSIGN, REML_ASSOC_RIGHT, ":="},
  {REML_TOKEN_CARET, REML_PREC_POW, REML_ASSOC_LEFT, "^"},
  {REML_TOKEN_STAR, REML_PREC_MUL, REML_ASSOC_LEFT, "*"},
  {REML_TOKEN_SLASH, REML_PREC_MUL, REML_ASSOC_LEFT, "/"},
  {REML_TOKEN_PERCENT, REML_PREC_MUL, REML_ASSOC_LEFT, "%"},
  {REML_TOKEN_PLUS, REML_PREC_ADD, REML_ASSOC_LEFT, "+"},
  {REML_TOKEN_MINUS, REML_PREC_ADD, REML_ASSOC_LEFT, "-"},
  {REML_TOKEN_DOTDOT, REML_PREC_RANGE, REML_ASSOC_LEFT, ".."},
  {REML_TOKEN_LT, REML_PREC_REL, REML_ASSOC_LEFT, "<"},
  {REML_TOKEN_LE, REML_PREC_REL, REML_ASSOC_LEFT, "<="},
  {REML_TOKEN_GT, REML_PREC_REL, REML_ASSOC_LEFT, ">"},
  {REML_TOKEN_GE, REML_PREC_REL, REML_ASSOC_LEFT, ">="},
  {REML_TOKEN_EQEQ, REML_PREC_EQ, REML_ASSOC_LEFT, "=="},
  {REML_TOKEN_NOTEQ, REML_PREC_EQ, REML_ASSOC_LEFT, "!="},
  {REML_TOKEN_LOGICAL_AND, REML_PREC_AND, REML_ASSOC_LEFT, "&&"},
  {REML_TOKEN_LOGICAL_OR, REML_PREC_OR, REML_ASSOC_LEFT, "||"},
  {REML_TOKEN_PIPE_FORWARD, REML_PREC_PIPE, REML_ASSOC_LEFT, "|>"},
};

bool reml_operator_lookup(reml_token_kind kind, reml_operator_entry *out) {
  if (!out) {
    return false;
  }

  for (size_t i = 0; i < sizeof(kOperators) / sizeof(kOperators[0]); ++i) {
    if (kOperators[i].kind == kind) {
      *out = kOperators[i];
      return true;
    }
  }

  return false;
}
