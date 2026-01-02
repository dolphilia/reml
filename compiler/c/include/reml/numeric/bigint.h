#ifndef REML_NUMERIC_BIGINT_H
#define REML_NUMERIC_BIGINT_H

#include <stdbool.h>
#include <stdint.h>

#include <tommath.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
  mp_int value;
} reml_bigint;

bool reml_bigint_init(reml_bigint *value);
void reml_bigint_deinit(reml_bigint *value);

bool reml_bigint_copy(reml_bigint *dest, const reml_bigint *src);
bool reml_bigint_set_i64(reml_bigint *value, int64_t input);
bool reml_bigint_set_str(reml_bigint *value, const char *text, int base);

bool reml_bigint_add(reml_bigint *out, const reml_bigint *left, const reml_bigint *right);
bool reml_bigint_sub(reml_bigint *out, const reml_bigint *left, const reml_bigint *right);
bool reml_bigint_mul(reml_bigint *out, const reml_bigint *left, const reml_bigint *right);
bool reml_bigint_div(reml_bigint *out, const reml_bigint *left, const reml_bigint *right);
bool reml_bigint_rem(reml_bigint *out, const reml_bigint *left, const reml_bigint *right);
bool reml_bigint_neg(reml_bigint *out, const reml_bigint *value);

int reml_bigint_cmp(const reml_bigint *left, const reml_bigint *right);
bool reml_bigint_is_zero(const reml_bigint *value);
bool reml_bigint_is_negative(const reml_bigint *value);
bool reml_bigint_to_string(const reml_bigint *value, int base, char **out_str);

#ifdef __cplusplus
}
#endif

#endif
