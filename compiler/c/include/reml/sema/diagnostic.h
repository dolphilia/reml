#ifndef REML_SEMA_DIAGNOSTIC_H
#define REML_SEMA_DIAGNOSTIC_H

#include <stddef.h>

#include <utarray.h>

#include "reml/util/span.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
  REML_DIAG_UNDEFINED_SYMBOL,
  REML_DIAG_DUPLICATE_SYMBOL,
  REML_DIAG_TYPE_MISMATCH,
  REML_DIAG_UNSUPPORTED_FEATURE,
  REML_DIAG_EFFECT_VIOLATION,
  REML_DIAG_CODEGEN_UNSUPPORTED,
  REML_DIAG_CODEGEN_LLVM_FAILURE,
  REML_DIAG_CODEGEN_INTERNAL
} reml_diagnostic_code;

typedef struct {
  reml_diagnostic_code code;
  reml_span span;
  const char *message;
} reml_diagnostic;

typedef struct {
  UT_array *items;
} reml_diagnostic_list;

void reml_diagnostics_init(reml_diagnostic_list *list);
void reml_diagnostics_deinit(reml_diagnostic_list *list);
void reml_diagnostics_push(reml_diagnostic_list *list, reml_diagnostic diag);
size_t reml_diagnostics_count(const reml_diagnostic_list *list);
const reml_diagnostic *reml_diagnostics_at(const reml_diagnostic_list *list, size_t index);

#ifdef __cplusplus
}
#endif

#endif
