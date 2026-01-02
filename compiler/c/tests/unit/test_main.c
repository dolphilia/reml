#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>

#include <cmocka.h>

static void test_dummy(void **state) {
  (void)state;
  assert_true(1);
}

int main(void) {
  const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_dummy),
  };

  return cmocka_run_group_tests(tests, NULL, NULL);
}
