#include "reml/util/span.h"

#include "reml/text/grapheme.h"

#define REML_TAB_WIDTH 4

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

bool reml_span_location_from_utf8(const char *input, size_t length, size_t offset,
                                  reml_span_location *out) {
  if (!input || !out || offset > length) {
    return false;
  }

  size_t index = 0;
  int line = 1;
  int column = 1;

  while (index < offset) {
    unsigned char byte = (unsigned char)input[index];
    if (byte == '\n') {
      index += 1;
      line += 1;
      column = 1;
      continue;
    }
    if (byte == '\r') {
      index += 1;
      if (index < length && input[index] == '\n') {
        index += 1;
      }
      line += 1;
      column = 1;
      continue;
    }
    if (byte == '\t') {
      index += 1;
      column += REML_TAB_WIDTH;
      continue;
    }
    if (byte < 0x80) {
      index += 1;
      column += 1;
      continue;
    }

    reml_unicode_error error;
    size_t advance = reml_grapheme_advance(input, length, index, &error);
    if (advance == 0) {
      if (error.kind == REML_UNICODE_INVALID_SCALAR && error.length > 0 &&
          index + error.length <= offset) {
        index += error.length;
        column += 1;
        continue;
      }
      return false;
    }
    if (index + advance > offset) {
      return false;
    }
    index += advance;
    column += 1;
  }

  if (index != offset) {
    return false;
  }

  out->offset = offset;
  out->line = line;
  out->column = column;
  return true;
}

reml_span reml_span_from_offsets(const char *input, size_t length, size_t start_offset,
                                 size_t end_offset) {
  reml_span_location start = {0};
  reml_span_location end = {0};
  if (!reml_span_location_from_utf8(input, length, start_offset, &start) ||
      !reml_span_location_from_utf8(input, length, end_offset, &end)) {
    return reml_span_make(start_offset, end_offset, 1, 1, 1, 1);
  }
  return reml_span_make(start_offset, end_offset, start.line, start.column, end.line, end.column);
}
