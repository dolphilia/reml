#ifndef REML_PARSER_H
#define REML_PARSER_H

#include <stdbool.h>
#include <stddef.h>

#include "reml/ast/ast.h"
#include "reml/lexer/lexer.h"
#include "reml/util/span.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
  reml_span span;
  const char *message;
} reml_parse_error;

typedef struct {
  reml_lexer lexer;
  reml_token current;
  bool has_error;
  reml_parse_error error;
} reml_parser;

void reml_parser_init(reml_parser *parser, const char *input, size_t length);
reml_compilation_unit *reml_parse_compilation_unit(reml_parser *parser);
const reml_parse_error *reml_parser_error(const reml_parser *parser);

#ifdef __cplusplus
}
#endif

#endif
