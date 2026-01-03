#include "reml/runtime/string.h"

#include <stdlib.h>
#include <string.h>

reml_string *reml_string_from_utf8(const char *data, size_t len) {
  if (!data && len != 0) {
    return NULL;
  }
  reml_string *str = (reml_string *)calloc(1, sizeof(reml_string));
  if (!str) {
    return NULL;
  }
  reml_unicode_error error = {0};
  if (!reml_string_init_from_utf8(str, data, len, &error)) {
    free(str);
    return NULL;
  }
  return str;
}

reml_string *reml_string_concat(const reml_string *left, const reml_string *right) {
  size_t left_len = left ? left->len : 0;
  size_t right_len = right ? right->len : 0;
  size_t total = left_len + right_len;
  char *buffer = NULL;
  if (total > 0) {
    buffer = (char *)malloc(total);
    if (!buffer) {
      return NULL;
    }
    if (left_len > 0 && left && left->ptr) {
      memcpy(buffer, left->ptr, left_len);
    }
    if (right_len > 0 && right && right->ptr) {
      memcpy(buffer + left_len, right->ptr, right_len);
    }
  }
  reml_string *result = reml_string_from_utf8(buffer ? buffer : "", total);
  free(buffer);
  return result;
}

int32_t reml_string_cmp(const reml_string *left, const reml_string *right) {
  const char *left_ptr = left ? left->ptr : NULL;
  const char *right_ptr = right ? right->ptr : NULL;
  size_t left_len = left ? left->len : 0;
  size_t right_len = right ? right->len : 0;
  size_t min_len = left_len < right_len ? left_len : right_len;
  if (min_len > 0 && left_ptr && right_ptr) {
    int cmp = memcmp(left_ptr, right_ptr, min_len);
    if (cmp != 0) {
      return cmp < 0 ? -1 : 1;
    }
  }
  if (left_len == right_len) {
    return 0;
  }
  return left_len < right_len ? -1 : 1;
}

void reml_string_free(reml_string *str) {
  if (!str) {
    return;
  }
  reml_string_deinit(str);
  free(str);
}
