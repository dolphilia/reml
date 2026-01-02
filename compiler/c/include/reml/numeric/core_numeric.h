#ifndef REML_NUMERIC_CORE_NUMERIC_H
#define REML_NUMERIC_CORE_NUMERIC_H

#include <stdbool.h>
#include <stdint.h>

#include "reml/numeric/bigint.h"

#ifdef __cplusplus
extern "C" {
#endif

reml_bigint *reml_numeric_bigint_new(void);
void reml_numeric_bigint_free(reml_bigint *value);

reml_bigint *reml_numeric_bigint_from_i64(int64_t value);
reml_bigint *reml_numeric_bigint_from_str(const char *text, int base);

reml_bigint *reml_numeric_bigint_add(const reml_bigint *left, const reml_bigint *right);
reml_bigint *reml_numeric_bigint_sub(const reml_bigint *left, const reml_bigint *right);
reml_bigint *reml_numeric_bigint_mul(const reml_bigint *left, const reml_bigint *right);
reml_bigint *reml_numeric_bigint_div(const reml_bigint *left, const reml_bigint *right);
reml_bigint *reml_numeric_bigint_rem(const reml_bigint *left, const reml_bigint *right);
reml_bigint *reml_numeric_bigint_neg(const reml_bigint *value);

int reml_numeric_bigint_cmp(const reml_bigint *left, const reml_bigint *right);
bool reml_numeric_bigint_is_zero(const reml_bigint *value);
bool reml_numeric_bigint_is_negative(const reml_bigint *value);
char *reml_numeric_bigint_to_string(const reml_bigint *value, int base);

#ifdef __cplusplus
}
#endif

#endif
