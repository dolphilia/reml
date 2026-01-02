#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <string.h>

#include <cmocka.h>

#include "reml/parser/parser.h"
#include "reml/sema/sema.h"

static reml_compilation_unit *parse_source(const char *source) {
  reml_parser parser;
  reml_parser_init(&parser, source, strlen(source));
  reml_compilation_unit *unit = reml_parse_compilation_unit(&parser);
  assert_non_null(unit);
  assert_null(reml_parser_error(&parser));
  return unit;
}

static void test_sema_basic_ok(void **state) {
  (void)state;

  const char *source = "let x = 1 + 2; x + 3;";
  reml_compilation_unit *unit = parse_source(source);

  reml_sema sema;
  reml_sema_init(&sema);
  bool ok = reml_sema_check(&sema, unit);

  const reml_diagnostic_list *diags = reml_sema_diagnostics(&sema);
  assert_true(ok);
  assert_int_equal(reml_diagnostics_count(diags), 0);

  reml_sema_deinit(&sema);
  reml_compilation_unit_free(unit);
}

static void test_sema_type_mismatch(void **state) {
  (void)state;

  const char *source = "let x = 1 + \"a\";";
  reml_compilation_unit *unit = parse_source(source);

  reml_sema sema;
  reml_sema_init(&sema);
  bool ok = reml_sema_check(&sema, unit);

  const reml_diagnostic_list *diags = reml_sema_diagnostics(&sema);
  assert_false(ok);
  assert_true(reml_diagnostics_count(diags) > 0);
  const reml_diagnostic *diag = reml_diagnostics_at(diags, 0);
  assert_non_null(diag);
  assert_int_equal(diag->code, REML_DIAG_TYPE_MISMATCH);

  reml_sema_deinit(&sema);
  reml_compilation_unit_free(unit);
}

static void test_sema_undefined_symbol(void **state) {
  (void)state;

  const char *source = "x + 1;";
  reml_compilation_unit *unit = parse_source(source);

  reml_sema sema;
  reml_sema_init(&sema);
  bool ok = reml_sema_check(&sema, unit);

  const reml_diagnostic_list *diags = reml_sema_diagnostics(&sema);
  assert_false(ok);
  assert_true(reml_diagnostics_count(diags) > 0);
  const reml_diagnostic *diag = reml_diagnostics_at(diags, 0);
  assert_non_null(diag);
  assert_int_equal(diag->code, REML_DIAG_UNDEFINED_SYMBOL);

  reml_sema_deinit(&sema);
  reml_compilation_unit_free(unit);
}

static void test_sema_bigint_literal_ok(void **state) {
  (void)state;

  const char *source = "let x = 9223372036854775808; x;";
  reml_compilation_unit *unit = parse_source(source);

  reml_sema sema;
  reml_sema_init(&sema);
  bool ok = reml_sema_check(&sema, unit);

  const reml_diagnostic_list *diags = reml_sema_diagnostics(&sema);
  assert_true(ok);
  assert_int_equal(reml_diagnostics_count(diags), 0);

  reml_sema_deinit(&sema);
  reml_compilation_unit_free(unit);
}

void test_sema(void **state) {
  test_sema_basic_ok(state);
  test_sema_type_mismatch(state);
  test_sema_undefined_symbol(state);
  test_sema_bigint_literal_ok(state);
}
