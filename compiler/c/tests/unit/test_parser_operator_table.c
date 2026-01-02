#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>

#include <cmocka.h>

#include "reml/parser/operator_table.h"

void test_operator_pow_and_or(void **state) {
  (void)state;

  reml_operator_entry entry = {0};

  assert_true(reml_operator_lookup(REML_TOKEN_CARET, &entry));
  assert_int_equal(entry.precedence, REML_PREC_POW);
  assert_int_equal(entry.assoc, REML_ASSOC_LEFT);
  assert_string_equal(entry.symbol, "^");

  assert_true(reml_operator_lookup(REML_TOKEN_LOGICAL_AND, &entry));
  assert_int_equal(entry.precedence, REML_PREC_AND);
  assert_string_equal(entry.symbol, "&&");

  assert_true(reml_operator_lookup(REML_TOKEN_LOGICAL_OR, &entry));
  assert_int_equal(entry.precedence, REML_PREC_OR);
  assert_string_equal(entry.symbol, "||");

  reml_operator_entry eq_entry = {0};
  assert_true(reml_operator_lookup(REML_TOKEN_EQEQ, &eq_entry));
  assert_true(eq_entry.precedence > REML_PREC_AND);
  assert_true(REML_PREC_AND > REML_PREC_OR);

  reml_operator_entry range_entry = {0};
  assert_true(reml_operator_lookup(REML_TOKEN_DOTDOT, &range_entry));
  assert_int_equal(range_entry.precedence, REML_PREC_RANGE);
  assert_string_equal(range_entry.symbol, "..");
  assert_true(REML_PREC_ADD > REML_PREC_RANGE);
  assert_true(REML_PREC_RANGE > REML_PREC_REL);
}
