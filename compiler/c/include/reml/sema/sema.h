#ifndef REML_SEMA_SEMA_H
#define REML_SEMA_SEMA_H

#include <stdbool.h>

#include <utarray.h>

#include "reml/ast/ast.h"
#include "reml/sema/diagnostic.h"
#include "reml/typeck/type.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct reml_symbol_table reml_symbol_table;
typedef struct reml_constructor_entry reml_constructor_entry;
typedef struct reml_enum_decl_entry reml_enum_decl_entry;

typedef struct {
  reml_symbol_table *symbols;
  reml_constructor_entry *constructors;
  reml_enum_decl_entry *enum_decls;
  reml_type_ctx types;
  reml_diagnostic_list diagnostics;
  UT_array *trait_constraints;
} reml_sema;

void reml_sema_init(reml_sema *sema);
void reml_sema_deinit(reml_sema *sema);
bool reml_sema_check(reml_sema *sema, reml_compilation_unit *unit);
const reml_diagnostic_list *reml_sema_diagnostics(const reml_sema *sema);

#ifdef __cplusplus
}
#endif

#endif
