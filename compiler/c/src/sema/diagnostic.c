#include "reml/sema/diagnostic.h"

#include <stdlib.h>

#include <utarray.h>

void reml_diagnostics_init(reml_diagnostic_list *list) {
  if (!list) {
    return;
  }
  UT_icd diag_icd = {sizeof(reml_diagnostic), NULL, NULL, NULL};
  utarray_new(list->items, &diag_icd);
}

void reml_diagnostics_deinit(reml_diagnostic_list *list) {
  if (!list || !list->items) {
    return;
  }
  for (reml_diagnostic *it = (reml_diagnostic *)utarray_front(list->items); it != NULL;
       it = (reml_diagnostic *)utarray_next(list->items, it)) {
    if (!it->pattern) {
      if (it->effect) {
        free(it->effect);
        it->effect = NULL;
      }
      continue;
    }
    if (it->pattern->missing_variants) {
      utarray_free(it->pattern->missing_variants);
    }
    if (it->pattern->missing_ranges) {
      utarray_free(it->pattern->missing_ranges);
    }
    free(it->pattern);
    it->pattern = NULL;
    if (it->effect) {
      free(it->effect);
      it->effect = NULL;
    }
  }
  utarray_free(list->items);
  list->items = NULL;
}

void reml_diagnostics_push(reml_diagnostic_list *list, reml_diagnostic diag) {
  if (!list || !list->items) {
    return;
  }
  utarray_push_back(list->items, &diag);
}

size_t reml_diagnostics_count(const reml_diagnostic_list *list) {
  if (!list || !list->items) {
    return 0;
  }
  return utarray_len(list->items);
}

const reml_diagnostic *reml_diagnostics_at(const reml_diagnostic_list *list, size_t index) {
  if (!list || !list->items) {
    return NULL;
  }
  return (const reml_diagnostic *)utarray_eltptr(list->items, index);
}
