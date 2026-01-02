#ifndef REML_LEXER_H
#define REML_LEXER_H

#include <stdbool.h>
#include <stddef.h>

#include "reml/lexer/token.h"
#include "reml/util/span.h"
#include "reml/util/string_view.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
  reml_token_kind kind;
  reml_string_view lexeme;
  reml_span span;
} reml_token;

typedef struct {
  reml_span span;
  const char *message;
} reml_lex_error;

typedef struct {
  const char *input;
  size_t length;
  size_t index;
  int line;
  int column;
  bool has_error;
  reml_lex_error error;
} reml_lexer;

void reml_lexer_init(reml_lexer *lexer, const char *input, size_t length);
reml_token reml_lexer_next(reml_lexer *lexer);
const char *reml_token_kind_name(reml_token_kind kind);

#ifdef __cplusplus
}
#endif

#endif
