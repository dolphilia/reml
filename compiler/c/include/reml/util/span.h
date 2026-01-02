#ifndef REML_UTIL_SPAN_H
#define REML_UTIL_SPAN_H

#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
  size_t start_offset;
  size_t end_offset;
  int start_line;
  int start_column;
  int end_line;
  int end_column;
} reml_span;

reml_span reml_span_make(size_t start_offset, size_t end_offset, int start_line, int start_column,
                         int end_line, int end_column);

reml_span reml_span_combine(reml_span left, reml_span right);

bool reml_span_is_valid(const reml_span *span);

#ifdef __cplusplus
}
#endif

#endif
