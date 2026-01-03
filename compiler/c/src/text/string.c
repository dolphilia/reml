#include "reml/text/string.h"

#include <stdlib.h>
#include <string.h>

#include "reml/text/grapheme.h"

bool reml_string_init_from_utf8(reml_string *out, const char *input, size_t length,
                                reml_unicode_error *out_error) {
  if (!out) {
    return false;
  }

  char *normalized = NULL;
  size_t normalized_len = 0;
  if (!reml_unicode_normalize_nfc(input, length, &normalized, &normalized_len, out_error)) {
    return false;
  }

  out->ptr = normalized;
  out->len = normalized_len;
  return true;
}

void reml_string_deinit(reml_string *str) {
  if (!str || !str->ptr) {
    return;
  }
  free(str->ptr);
  str->ptr = NULL;
  str->len = 0;
}

reml_str reml_str_from_string(const reml_string *str) {
  reml_str view = {0};
  if (!str) {
    return view;
  }
  view.ptr = str->ptr;
  view.len = str->len;
  return view;
}

reml_str reml_str_make(const char *input, size_t length) {
  reml_str view;
  view.ptr = input;
  view.len = length;
  return view;
}

size_t reml_str_len_bytes(reml_str str) {
  return str.len;
}

size_t reml_str_len_graphemes(reml_str str) {
  reml_string_view view = reml_string_view_make(str.ptr, str.len);
  return reml_grapheme_len(view);
}

bool reml_str_is_codepoint_boundary(reml_str str, size_t offset) {
  if (offset > str.len) {
    return false;
  }
  if (offset == str.len) {
    return true;
  }
  unsigned char c = (unsigned char)str.ptr[offset];
  return (c & 0xC0) != 0x80;
}

bool reml_str_slice_codepoints(reml_str str, size_t start, size_t end, reml_str *out) {
  if (start > end || end > str.len) {
    return false;
  }
  if (!reml_str_is_codepoint_boundary(str, start) || !reml_str_is_codepoint_boundary(str, end)) {
    return false;
  }
  if (out) {
    out->ptr = str.ptr + start;
    out->len = end - start;
  }
  return true;
}

bool reml_str_slice_graphemes(reml_str str, size_t start, size_t end, reml_str *out) {
  reml_string_view view = reml_string_view_make(str.ptr, str.len);
  reml_string_view sliced = {0};
  if (!reml_grapheme_slice(view, start, end, &sliced)) {
    return false;
  }
  if (out) {
    out->ptr = sliced.data;
    out->len = sliced.length;
  }
  return true;
}
