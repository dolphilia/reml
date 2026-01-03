#ifndef REML_TEXT_STRING_H
#define REML_TEXT_STRING_H

#include <stdbool.h>
#include <stddef.h>

#include "reml/text/unicode.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
  char *ptr;
  size_t len;
} reml_string;

typedef struct {
  const char *ptr;
  size_t len;
} reml_str;

bool reml_string_init_from_utf8(reml_string *out, const char *input, size_t length,
                                reml_unicode_error *out_error);
void reml_string_deinit(reml_string *str);

reml_str reml_str_from_string(const reml_string *str);
reml_str reml_str_make(const char *input, size_t length);

size_t reml_str_len_bytes(reml_str str);
size_t reml_str_len_graphemes(reml_str str);
bool reml_str_is_codepoint_boundary(reml_str str, size_t offset);
bool reml_str_slice_codepoints(reml_str str, size_t start, size_t end, reml_str *out);
bool reml_str_slice_graphemes(reml_str str, size_t start, size_t end, reml_str *out);

#ifdef __cplusplus
}
#endif

#endif
