#include "reml/numeric/bigint.h"

#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>

bool reml_bigint_init(reml_bigint *value) {
  if (!value) {
    return false;
  }
  return mp_init(&value->value) == MP_OKAY;
}

void reml_bigint_deinit(reml_bigint *value) {
  if (!value) {
    return;
  }
  mp_clear(&value->value);
}

bool reml_bigint_copy(reml_bigint *dest, const reml_bigint *src) {
  if (!dest || !src) {
    return false;
  }
  return mp_copy(&src->value, &dest->value) == MP_OKAY;
}

bool reml_bigint_set_i64(reml_bigint *value, int64_t input) {
  if (!value) {
    return false;
  }
  char buffer[32];
  int written = snprintf(buffer, sizeof(buffer), "%" PRId64, input);
  if (written <= 0 || (size_t)written >= sizeof(buffer)) {
    return false;
  }
  return mp_read_radix(&value->value, buffer, 10) == MP_OKAY;
}

bool reml_bigint_set_str(reml_bigint *value, const char *text, int base) {
  if (!value || !text) {
    return false;
  }
  return mp_read_radix(&value->value, text, base) == MP_OKAY;
}

bool reml_bigint_add(reml_bigint *out, const reml_bigint *left, const reml_bigint *right) {
  if (!out || !left || !right) {
    return false;
  }
  return mp_add(&left->value, &right->value, &out->value) == MP_OKAY;
}

bool reml_bigint_sub(reml_bigint *out, const reml_bigint *left, const reml_bigint *right) {
  if (!out || !left || !right) {
    return false;
  }
  return mp_sub(&left->value, &right->value, &out->value) == MP_OKAY;
}

bool reml_bigint_mul(reml_bigint *out, const reml_bigint *left, const reml_bigint *right) {
  if (!out || !left || !right) {
    return false;
  }
  return mp_mul(&left->value, &right->value, &out->value) == MP_OKAY;
}

bool reml_bigint_div(reml_bigint *out, const reml_bigint *left, const reml_bigint *right) {
  if (!out || !left || !right) {
    return false;
  }
  return mp_div(&left->value, &right->value, &out->value, NULL) == MP_OKAY;
}

bool reml_bigint_rem(reml_bigint *out, const reml_bigint *left, const reml_bigint *right) {
  if (!out || !left || !right) {
    return false;
  }
  return mp_mod(&left->value, &right->value, &out->value) == MP_OKAY;
}

int reml_bigint_cmp(const reml_bigint *left, const reml_bigint *right) {
  if (!left || !right) {
    return 0;
  }
  return mp_cmp(&left->value, &right->value);
}

bool reml_bigint_is_zero(const reml_bigint *value) {
  if (!value) {
    return false;
  }
  return mp_iszero(&value->value) == MP_YES;
}

bool reml_bigint_is_negative(const reml_bigint *value) {
  if (!value) {
    return false;
  }
  return mp_isneg(&value->value) == MP_YES;
}

bool reml_bigint_to_string(const reml_bigint *value, int base, char **out_str) {
  if (!value || !out_str) {
    return false;
  }
  int size = 0;
  if (mp_radix_size(&value->value, base, &size) != MP_OKAY || size <= 0) {
    return false;
  }
  char *buffer = (char *)malloc((size_t)size);
  if (!buffer) {
    return false;
  }
  if (mp_to_radix(&value->value, buffer, base) != MP_OKAY) {
    free(buffer);
    return false;
  }
  *out_str = buffer;
  return true;
}
