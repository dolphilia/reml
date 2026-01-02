#ifndef REML_AST_PRINTER_H
#define REML_AST_PRINTER_H

#include <stdio.h>

#include "reml/ast/ast.h"

#ifdef __cplusplus
extern "C" {
#endif

void reml_ast_write_expr(FILE *out, const reml_expr *expr);
void reml_ast_write_stmt(FILE *out, const reml_stmt *stmt);
void reml_ast_write_compilation_unit(FILE *out, const reml_compilation_unit *unit);

#ifdef __cplusplus
}
#endif

#endif
