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
  REML_PREC_PIPE = 0,
  REML_PREC_OR = 1,
  REML_PREC_AND = 2,
  REML_PREC_EQ = 3,
  REML_PREC_REL = 4,
  REML_PREC_RANGE = 5,
  REML_PREC_ADD = 6,
  REML_PREC_MUL = 7,
  REML_PREC_POW = 8
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
