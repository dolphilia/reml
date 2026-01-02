#include "reml/util/span.h"

reml_span reml_span_make(size_t start_offset, size_t end_offset, int start_line, int start_column,
                         int end_line, int end_column) {
  reml_span span;
  span.start_offset = start_offset;
  span.end_offset = end_offset;
  span.start_line = start_line;
  span.start_column = start_column;
  span.end_line = end_line;
  span.end_column = end_column;
  return span;
}

reml_span reml_span_combine(reml_span left, reml_span right) {
  return reml_span_make(left.start_offset, right.end_offset, left.start_line, left.start_column,
                        right.end_line, right.end_column);
}

bool reml_span_is_valid(const reml_span *span) {
  if (!span) {
    return false;
  }
  if (span->start_offset > span->end_offset) {
    return false;
  }
  if (span->start_line < 1 || span->start_column < 1 || span->end_line < 1 || span->end_column < 1) {
    return false;
  }
  return true;
}
