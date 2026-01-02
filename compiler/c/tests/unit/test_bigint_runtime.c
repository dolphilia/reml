#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdlib.h>

#include <cmocka.h>

#include "reml/numeric/core_numeric.h"

static char *reml_bigint_to_string10(const reml_bigint *value) {
  return reml_numeric_bigint_to_string(value, 10);
}

static void test_bigint_from_str_and_to_string(void **state) {
  (void)state;

  reml_bigint *value = reml_numeric_bigint_from_str("9_223_372_036_854_775_808", 0);
  assert_non_null(value);

  char *text = reml_bigint_to_string10(value);
  assert_non_null(text);
  assert_string_equal(text, "9223372036854775808");

  free(text);
  reml_numeric_bigint_free(value);
}

static void test_bigint_basic_arithmetic(void **state) {
  (void)state;

  reml_bigint *left = reml_numeric_bigint_from_str("9223372036854775808", 10);
  reml_bigint *right = reml_numeric_bigint_from_str("2", 10);
  assert_non_null(left);
  assert_non_null(right);

  reml_bigint *sum = reml_numeric_bigint_add(left, right);
  reml_bigint *diff = reml_numeric_bigint_sub(left, right);
  reml_bigint *prod = reml_numeric_bigint_mul(left, right);
  reml_bigint *quot = reml_numeric_bigint_div(left, right);
  reml_bigint *rem = reml_numeric_bigint_rem(left, right);

  assert_non_null(sum);
  assert_non_null(diff);
  assert_non_null(prod);
  assert_non_null(quot);
  assert_non_null(rem);

  char *sum_text = reml_bigint_to_string10(sum);
  char *diff_text = reml_bigint_to_string10(diff);
  char *prod_text = reml_bigint_to_string10(prod);
  char *quot_text = reml_bigint_to_string10(quot);
  char *rem_text = reml_bigint_to_string10(rem);

  assert_string_equal(sum_text, "9223372036854775810");
  assert_string_equal(diff_text, "9223372036854775806");
  assert_string_equal(prod_text, "18446744073709551616");
  assert_string_equal(quot_text, "4611686018427387904");
  assert_string_equal(rem_text, "0");

  free(sum_text);
  free(diff_text);
  free(prod_text);
  free(quot_text);
  free(rem_text);

  reml_numeric_bigint_free(sum);
  reml_numeric_bigint_free(diff);
  reml_numeric_bigint_free(prod);
  reml_numeric_bigint_free(quot);
  reml_numeric_bigint_free(rem);
  reml_numeric_bigint_free(left);
  reml_numeric_bigint_free(right);
}

static void test_bigint_comparison_and_sign(void **state) {
  (void)state;

  reml_bigint *pos = reml_numeric_bigint_from_str("10", 10);
  reml_bigint *neg = reml_numeric_bigint_from_str("-10", 10);
  assert_non_null(pos);
  assert_non_null(neg);

  assert_true(reml_numeric_bigint_cmp(pos, neg) > 0);
  assert_true(reml_numeric_bigint_cmp(neg, pos) < 0);
  assert_int_equal(reml_numeric_bigint_cmp(pos, pos), 0);

  assert_false(reml_numeric_bigint_is_negative(pos));
  assert_true(reml_numeric_bigint_is_negative(neg));

  reml_bigint *negated = reml_numeric_bigint_neg(pos);
  assert_non_null(negated);
  assert_true(reml_numeric_bigint_is_negative(negated));

  char *neg_text = reml_bigint_to_string10(negated);
  assert_non_null(neg_text);
  assert_string_equal(neg_text, "-10");
  free(neg_text);

  reml_numeric_bigint_free(negated);
  reml_numeric_bigint_free(pos);
  reml_numeric_bigint_free(neg);
}

void test_bigint_runtime(void **state) {
  test_bigint_from_str_and_to_string(state);
  test_bigint_basic_arithmetic(state);
  test_bigint_comparison_and_sign(state);
}
