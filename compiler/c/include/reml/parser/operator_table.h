#ifndef REML_PARSER_OPERATOR_TABLE_H
#define REML_PARSER_OPERATOR_TABLE_H

#include <stdbool.h>

#include "reml/lexer/token.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
  REML_ASSOC_LEFT = 0,
  REML_ASSOC_RIGHT,
  REML_ASSOC_NONE
} reml_assoc;

typedef enum {
  REML_PREC_ASSIGN = 0,
  REML_PREC_PIPE = 1,
  REML_PREC_OR = 2,
  REML_PREC_AND = 3,
  REML_PREC_EQ = 4,
  REML_PREC_REL = 5,
  REML_PREC_RANGE = 6,
  REML_PREC_ADD = 7,
  REML_PREC_MUL = 8,
  REML_PREC_POW = 9
} reml_precedence;

typedef struct {
  reml_token_kind kind;
  reml_precedence precedence;
  reml_assoc assoc;
  const char *symbol;
} reml_operator_entry;

bool reml_operator_lookup(reml_token_kind kind, reml_operator_entry *out);

#ifdef __cplusplus
}
#endif

#endif
