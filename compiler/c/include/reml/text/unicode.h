#ifndef REML_TEXT_UNICODE_H
#define REML_TEXT_UNICODE_H

#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
  REML_UNICODE_OK = 0,
  REML_UNICODE_INVALID_UTF8,
  REML_UNICODE_INVALID_SCALAR,
  REML_UNICODE_NORMALIZE_REQUIRED
} reml_unicode_error_kind;

typedef struct {
  reml_unicode_error_kind kind;
  size_t offset;
  size_t length;
} reml_unicode_error;

bool reml_unicode_validate_utf8(const char *input, size_t length, reml_unicode_error *out_error);
bool reml_unicode_is_nfc(const char *input, size_t length, reml_unicode_error *out_error);
bool reml_unicode_normalize_nfc(const char *input, size_t length, char **out_data,
                                size_t *out_length, reml_unicode_error *out_error);

#ifdef __cplusplus
}
#endif

#endif
