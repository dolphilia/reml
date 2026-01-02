#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>

#include <cmocka.h>

void test_codegen(void **state);

int main(void) {
  const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_codegen),
  };

  return cmocka_run_group_tests(tests, NULL, NULL);
}
