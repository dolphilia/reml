#include "reml/text/unicode.h"

#include <stdlib.h>
#include <string.h>

#include <utf8proc.h>

static void reml_unicode_set_error(reml_unicode_error *out_error, reml_unicode_error_kind kind,
                                   size_t offset, size_t length) {
  if (!out_error) {
    return;
  }
  out_error->kind = kind;
  out_error->offset = offset;
  out_error->length = length;
}

bool reml_unicode_validate_utf8(const char *input, size_t length, reml_unicode_error *out_error) {
  size_t index = 0;
  size_t last_valid = 0;

  while (index < length) {
    utf8proc_int32_t codepoint = 0;
    utf8proc_ssize_t len =
        utf8proc_iterate((const utf8proc_uint8_t *)input + index, (utf8proc_ssize_t)(length - index),
                         &codepoint);
    if (len < 0) {
      if (length - index >= 3) {
        unsigned char b0 = (unsigned char)input[index];
        unsigned char b1 = (unsigned char)input[index + 1];
        unsigned char b2 = (unsigned char)input[index + 2];
        if (b0 == 0xED && (b1 & 0xE0) == 0xA0 && (b2 & 0xC0) == 0x80) {
          reml_unicode_set_error(out_error, REML_UNICODE_INVALID_SCALAR, index, 3);
          return false;
        }
      }
      size_t error_offset = last_valid > 0 ? last_valid - 1 : 0;
      reml_unicode_set_error(out_error, REML_UNICODE_INVALID_UTF8, error_offset, 1);
      return false;
    }
    if (!utf8proc_codepoint_valid(codepoint)) {
      reml_unicode_set_error(out_error, REML_UNICODE_INVALID_SCALAR, index, (size_t)len);
      return false;
    }
    index += (size_t)len;
    last_valid = index;
  }

  reml_unicode_set_error(out_error, REML_UNICODE_OK, 0, 0);
  return true;
}

bool reml_unicode_normalize_nfc(const char *input, size_t length, char **out_data,
                                size_t *out_length, reml_unicode_error *out_error) {
  if (!reml_unicode_validate_utf8(input, length, out_error)) {
    return false;
  }

  utf8proc_uint8_t *normalized = NULL;
  utf8proc_ssize_t nlen =
      utf8proc_map((const utf8proc_uint8_t *)input, (utf8proc_ssize_t)length, &normalized,
                   UTF8PROC_STABLE | UTF8PROC_COMPOSE);
  if (nlen < 0) {
    reml_unicode_set_error(out_error, REML_UNICODE_INVALID_UTF8, 0, 1);
    return false;
  }

  *out_data = (char *)normalized;
  *out_length = (size_t)nlen;
  reml_unicode_set_error(out_error, REML_UNICODE_OK, 0, 0);
  return true;
}

bool reml_unicode_is_nfc(const char *input, size_t length, reml_unicode_error *out_error) {
  char *normalized = NULL;
  size_t normalized_len = 0;

  if (!reml_unicode_normalize_nfc(input, length, &normalized, &normalized_len, out_error)) {
    return false;
  }

  bool ok = normalized_len == length && memcmp(normalized, input, length) == 0;
  if (ok) {
    reml_unicode_set_error(out_error, REML_UNICODE_OK, 0, 0);
    free(normalized);
    return true;
  }

  size_t diff = 0;
  size_t min_len = normalized_len < length ? normalized_len : length;
  while (diff < min_len && normalized[diff] == input[diff]) {
    diff++;
  }
  reml_unicode_set_error(out_error, REML_UNICODE_NORMALIZE_REQUIRED, diff, 1);
  free(normalized);
  return false;
}
