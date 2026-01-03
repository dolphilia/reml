#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>

#include <cmocka.h>

static void test_dummy(void **state) {
  (void)state;
  assert_true(1);
}

void test_operator_pow_and_or(void **state);
void test_lexer_basic(void **state);
void test_parser_basic(void **state);
void test_sema(void **state);
void test_bigint_runtime(void **state);
void test_effects_runtime(void **state);
void test_text_unicode(void **state);
void test_type_effects(void **state);

int main(void) {
  const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_dummy),
    cmocka_unit_test(test_operator_pow_and_or),
    cmocka_unit_test(test_lexer_basic),
    cmocka_unit_test(test_parser_basic),
    cmocka_unit_test(test_sema),
    cmocka_unit_test(test_bigint_runtime),
    cmocka_unit_test(test_effects_runtime),
    cmocka_unit_test(test_text_unicode),
    cmocka_unit_test(test_type_effects),
  };

  return cmocka_run_group_tests(tests, NULL, NULL);
}
