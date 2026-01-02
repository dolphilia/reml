#include "reml/numeric/core_numeric.h"

#include <stdlib.h>
#include <string.h>

static reml_bigint *reml_numeric_bigint_alloc(void) {
  reml_bigint *value = (reml_bigint *)malloc(sizeof(reml_bigint));
  if (!value) {
    return NULL;
  }
  if (!reml_bigint_init(value)) {
    free(value);
    return NULL;
  }
  return value;
}

static void reml_numeric_bigint_release(reml_bigint *value) {
  if (!value) {
    return;
  }
  reml_bigint_deinit(value);
  free(value);
}

static char *reml_numeric_strip_underscores(const char *text) {
  if (!text) {
    return NULL;
  }
  size_t length = strlen(text);
  char *buffer = (char *)malloc(length + 1);
  if (!buffer) {
    return NULL;
  }
  size_t out = 0;
  for (size_t i = 0; i < length; ++i) {
    if (text[i] != '_') {
      buffer[out++] = text[i];
    }
  }
  buffer[out] = '\0';
  return buffer;
}

static bool reml_numeric_bigint_prepare_str(const char *text, int base, char **out_text,
                                            int *out_base) {
  if (!text || !out_text || !out_base) {
    return false;
  }

  char *stripped = reml_numeric_strip_underscores(text);
  if (!stripped) {
    return false;
  }

  const char *cursor = stripped;
  char sign = '\0';
  if (*cursor == '+' || *cursor == '-') {
    sign = *cursor;
    cursor++;
  }

  int detected = base;
  if (detected == 0 && cursor[0] == '0') {
    if (cursor[1] == 'x' || cursor[1] == 'X') {
      detected = 16;
      cursor += 2;
    } else if (cursor[1] == 'o' || cursor[1] == 'O') {
      detected = 8;
      cursor += 2;
    } else if (cursor[1] == 'b' || cursor[1] == 'B') {
      detected = 2;
      cursor += 2;
    } else {
      detected = 10;
    }
  }

  if (detected == 0) {
    detected = 10;
  }

  size_t digits_len = strlen(cursor);
  size_t out_len = digits_len + (sign ? 1 : 0);
  char *normalized = (char *)malloc(out_len + 1);
  if (!normalized) {
    free(stripped);
    return false;
  }
  size_t offset = 0;
  if (sign) {
    normalized[offset++] = sign;
  }
  memcpy(normalized + offset, cursor, digits_len);
  normalized[out_len] = '\0';

  free(stripped);
  *out_text = normalized;
  *out_base = detected;
  return true;
}

reml_bigint *reml_numeric_bigint_new(void) {
  return reml_numeric_bigint_alloc();
}

void reml_numeric_bigint_free(reml_bigint *value) {
  reml_numeric_bigint_release(value);
}

reml_bigint *reml_numeric_bigint_from_i64(int64_t value) {
  reml_bigint *result = reml_numeric_bigint_alloc();
  if (!result) {
    return NULL;
  }
  if (!reml_bigint_set_i64(result, value)) {
    reml_numeric_bigint_release(result);
    return NULL;
  }
  return result;
}

reml_bigint *reml_numeric_bigint_from_str(const char *text, int base) {
  if (!text) {
    return NULL;
  }
  char *normalized = NULL;
  int resolved_base = 0;
  if (!reml_numeric_bigint_prepare_str(text, base, &normalized, &resolved_base)) {
    return NULL;
  }

  reml_bigint *result = reml_numeric_bigint_alloc();
  if (!result) {
    free(normalized);
    return NULL;
  }
  bool ok = reml_bigint_set_str(result, normalized, resolved_base);
  free(normalized);
  if (!ok) {
    reml_numeric_bigint_release(result);
    return NULL;
  }
  return result;
}

static reml_bigint *reml_numeric_bigint_binary(const reml_bigint *left, const reml_bigint *right,
                                               bool (*op)(reml_bigint *, const reml_bigint *,
                                                          const reml_bigint *)) {
  if (!left || !right || !op) {
    return NULL;
  }
  reml_bigint *result = reml_numeric_bigint_alloc();
  if (!result) {
    return NULL;
  }
  if (!op(result, left, right)) {
    reml_numeric_bigint_release(result);
    return NULL;
  }
  return result;
}

reml_bigint *reml_numeric_bigint_add(const reml_bigint *left, const reml_bigint *right) {
  return reml_numeric_bigint_binary(left, right, reml_bigint_add);
}

reml_bigint *reml_numeric_bigint_sub(const reml_bigint *left, const reml_bigint *right) {
  return reml_numeric_bigint_binary(left, right, reml_bigint_sub);
}

reml_bigint *reml_numeric_bigint_mul(const reml_bigint *left, const reml_bigint *right) {
  return reml_numeric_bigint_binary(left, right, reml_bigint_mul);
}

reml_bigint *reml_numeric_bigint_div(const reml_bigint *left, const reml_bigint *right) {
  return reml_numeric_bigint_binary(left, right, reml_bigint_div);
}

reml_bigint *reml_numeric_bigint_rem(const reml_bigint *left, const reml_bigint *right) {
  return reml_numeric_bigint_binary(left, right, reml_bigint_rem);
}

reml_bigint *reml_numeric_bigint_neg(const reml_bigint *value) {
  if (!value) {
    return NULL;
  }
  reml_bigint *result = reml_numeric_bigint_alloc();
  if (!result) {
    return NULL;
  }
  if (!reml_bigint_neg(result, value)) {
    reml_numeric_bigint_release(result);
    return NULL;
  }
  return result;
}

int reml_numeric_bigint_cmp(const reml_bigint *left, const reml_bigint *right) {
  return reml_bigint_cmp(left, right);
}

bool reml_numeric_bigint_is_zero(const reml_bigint *value) {
  return reml_bigint_is_zero(value);
}

bool reml_numeric_bigint_is_negative(const reml_bigint *value) {
  return reml_bigint_is_negative(value);
}

char *reml_numeric_bigint_to_string(const reml_bigint *value, int base) {
  char *text = NULL;
  if (!value) {
    return NULL;
  }
  if (!reml_bigint_to_string(value, base, &text)) {
    return NULL;
  }
  return text;
}
