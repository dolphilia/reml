#ifndef REML_UTIL_STRING_VIEW_H
#define REML_UTIL_STRING_VIEW_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
  const char *data;
  size_t length;
} reml_string_view;

static inline reml_string_view reml_string_view_make(const char *data, size_t length) {
  reml_string_view view;
  view.data = data;
  view.length = length;
  return view;
}

#ifdef __cplusplus
}
#endif

#endif
