#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <string.h>

#include <cmocka.h>

#include "reml/codegen/codegen.h"
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

static void test_codegen_basic(void **state) {
  (void)state;

  const char *source = "let x = 1 + 2; x + 3;";
  reml_compilation_unit *unit = parse_source(source);

  reml_sema sema;
  reml_sema_init(&sema);
  bool ok = reml_sema_check(&sema, unit);
  assert_true(ok);
  assert_int_equal(reml_diagnostics_count(reml_sema_diagnostics(&sema)), 0);

  reml_codegen codegen;
  ok = reml_codegen_init(&codegen, "test_module");
  assert_true(ok);

  ok = reml_codegen_generate(&codegen, unit);
  assert_true(ok);
  assert_int_equal(reml_diagnostics_count(reml_codegen_diagnostics(&codegen)), 0);

  reml_codegen_deinit(&codegen);
  reml_sema_deinit(&sema);
  reml_compilation_unit_free(unit);
}

static void test_codegen_while(void **state) {
  (void)state;

  const char *source = "let x = 0; while x < 2 { let y = x + 1; }; x + 1;";
  reml_compilation_unit *unit = parse_source(source);

  reml_sema sema;
  reml_sema_init(&sema);
  bool ok = reml_sema_check(&sema, unit);
  assert_true(ok);
  assert_int_equal(reml_diagnostics_count(reml_sema_diagnostics(&sema)), 0);

  reml_codegen codegen;
  ok = reml_codegen_init(&codegen, "test_module");
  assert_true(ok);

  ok = reml_codegen_generate(&codegen, unit);
  assert_true(ok);
  assert_int_equal(reml_diagnostics_count(reml_codegen_diagnostics(&codegen)), 0);

  reml_codegen_deinit(&codegen);
  reml_sema_deinit(&sema);
  reml_compilation_unit_free(unit);
}

static void test_codegen_bigint(void **state) {
  (void)state;

  const char *source = "let x = 9223372036854775808; x + 9223372036854775808;";
  reml_compilation_unit *unit = parse_source(source);

  reml_sema sema;
  reml_sema_init(&sema);
  bool ok = reml_sema_check(&sema, unit);
  assert_true(ok);
  assert_int_equal(reml_diagnostics_count(reml_sema_diagnostics(&sema)), 0);

  reml_codegen codegen;
  ok = reml_codegen_init(&codegen, "test_module");
  assert_true(ok);

  ok = reml_codegen_generate(&codegen, unit);
  assert_true(ok);
  assert_int_equal(reml_diagnostics_count(reml_codegen_diagnostics(&codegen)), 0);

  reml_codegen_deinit(&codegen);
  reml_sema_deinit(&sema);
  reml_compilation_unit_free(unit);
}

static void test_codegen_match(void **state) {
  (void)state;

  const char *source = "let x = match 1 with | 1 -> 10 | _ -> 20; x;";
  reml_compilation_unit *unit = parse_source(source);

  reml_sema sema;
  reml_sema_init(&sema);
  bool ok = reml_sema_check(&sema, unit);
  assert_true(ok);
  assert_int_equal(reml_diagnostics_count(reml_sema_diagnostics(&sema)), 0);

  reml_codegen codegen;
  ok = reml_codegen_init(&codegen, "test_module");
  assert_true(ok);

  ok = reml_codegen_generate(&codegen, unit);
  assert_true(ok);
  assert_int_equal(reml_diagnostics_count(reml_codegen_diagnostics(&codegen)), 0);

  reml_codegen_deinit(&codegen);
  reml_sema_deinit(&sema);
  reml_compilation_unit_free(unit);
}

static void test_codegen_match_range(void **state) {
  (void)state;

  const char *source = "let x = match 2 with | 1..=3 -> 10 | _ -> 20; x;";
  reml_compilation_unit *unit = parse_source(source);

  reml_sema sema;
  reml_sema_init(&sema);
  bool ok = reml_sema_check(&sema, unit);
  assert_true(ok);
  assert_int_equal(reml_diagnostics_count(reml_sema_diagnostics(&sema)), 0);

  reml_codegen codegen;
  ok = reml_codegen_init(&codegen, "test_module");
  assert_true(ok);

  ok = reml_codegen_generate(&codegen, unit);
  assert_true(ok);
  assert_int_equal(reml_diagnostics_count(reml_codegen_diagnostics(&codegen)), 0);

  reml_codegen_deinit(&codegen);
  reml_sema_deinit(&sema);
  reml_compilation_unit_free(unit);
}

static void test_codegen_match_guard(void **state) {
  (void)state;

  const char *source = "let x = match 1 with | 1 when true -> 10 | _ -> 20; x;";
  reml_compilation_unit *unit = parse_source(source);

  reml_sema sema;
  reml_sema_init(&sema);
  bool ok = reml_sema_check(&sema, unit);
  assert_true(ok);
  assert_int_equal(reml_diagnostics_count(reml_sema_diagnostics(&sema)), 0);

  reml_codegen codegen;
  ok = reml_codegen_init(&codegen, "test_module");
  assert_true(ok);

  ok = reml_codegen_generate(&codegen, unit);
  assert_true(ok);
  assert_int_equal(reml_diagnostics_count(reml_codegen_diagnostics(&codegen)), 0);

  reml_codegen_deinit(&codegen);
  reml_sema_deinit(&sema);
  reml_compilation_unit_free(unit);
}

static void test_codegen_enum_constructor(void **state) {
  (void)state;

  const char *source =
      "type Option = | Some(Int) | None; let x = Some(1); match x with | Some(_) -> 1 | None() -> 2;";
  reml_compilation_unit *unit = parse_source(source);

  reml_sema sema;
  reml_sema_init(&sema);
  bool ok = reml_sema_check(&sema, unit);
  assert_true(ok);
  assert_int_equal(reml_diagnostics_count(reml_sema_diagnostics(&sema)), 0);

  reml_codegen codegen;
  ok = reml_codegen_init(&codegen, "test_module");
  assert_true(ok);

  ok = reml_codegen_generate(&codegen, unit);
  assert_true(ok);
  assert_int_equal(reml_diagnostics_count(reml_codegen_diagnostics(&codegen)), 0);

  reml_codegen_deinit(&codegen);
  reml_sema_deinit(&sema);
  reml_compilation_unit_free(unit);
}

static void test_codegen_match_tuple_record(void **state) {
  (void)state;

  {
    const char *source = "let x = (1, 2); match x with | (1, 2) -> 1 | _ -> 2;";
    reml_compilation_unit *unit = parse_source(source);

    reml_sema sema;
    reml_sema_init(&sema);
    bool ok = reml_sema_check(&sema, unit);
    assert_true(ok);
    assert_int_equal(reml_diagnostics_count(reml_sema_diagnostics(&sema)), 0);

    reml_codegen codegen;
    ok = reml_codegen_init(&codegen, "test_module");
    assert_true(ok);

    ok = reml_codegen_generate(&codegen, unit);
    assert_true(ok);
    assert_int_equal(reml_diagnostics_count(reml_codegen_diagnostics(&codegen)), 0);

    reml_codegen_deinit(&codegen);
    reml_sema_deinit(&sema);
    reml_compilation_unit_free(unit);
  }
  {
    const char *source =
        "let x = { a: 1, b: 2 }; match x with | { a: 1, b: 2 } -> 1 | _ -> 2;";
    reml_compilation_unit *unit = parse_source(source);

    reml_sema sema;
    reml_sema_init(&sema);
    bool ok = reml_sema_check(&sema, unit);
    assert_true(ok);
    assert_int_equal(reml_diagnostics_count(reml_sema_diagnostics(&sema)), 0);

    reml_codegen codegen;
    ok = reml_codegen_init(&codegen, "test_module");
    assert_true(ok);

    ok = reml_codegen_generate(&codegen, unit);
    assert_true(ok);
    assert_int_equal(reml_diagnostics_count(reml_codegen_diagnostics(&codegen)), 0);

    reml_codegen_deinit(&codegen);
    reml_sema_deinit(&sema);
    reml_compilation_unit_free(unit);
  }
}

static void test_codegen_enum_drop(void **state) {
  (void)state;

  const char *source = "type Option = | Some(Int) | None; let x = Some(1); x;";
  reml_compilation_unit *unit = parse_source(source);

  reml_sema sema;
  reml_sema_init(&sema);
  bool ok = reml_sema_check(&sema, unit);
  assert_true(ok);
  assert_int_equal(reml_diagnostics_count(reml_sema_diagnostics(&sema)), 0);

  reml_codegen codegen;
  ok = reml_codegen_init(&codegen, "test_module");
  assert_true(ok);

  ok = reml_codegen_generate(&codegen, unit);
  assert_true(ok);
  assert_int_equal(reml_diagnostics_count(reml_codegen_diagnostics(&codegen)), 0);

  char *ir = LLVMPrintModuleToString(codegen.module);
  assert_non_null(ir);
  assert_non_null(strstr(ir, "reml_enum_free"));
  LLVMDisposeMessage(ir);

  reml_codegen_deinit(&codegen);
  reml_sema_deinit(&sema);
  reml_compilation_unit_free(unit);
}

static void test_codegen_ref_mut_update(void **state) {
  (void)state;

  const char *source = "var x = 1; let r = &mut x; *r := 2; x;";
  reml_compilation_unit *unit = parse_source(source);

  reml_sema sema;
  reml_sema_init(&sema);
  bool ok = reml_sema_check(&sema, unit);
  assert_true(ok);
  assert_int_equal(reml_diagnostics_count(reml_sema_diagnostics(&sema)), 0);

  reml_codegen codegen;
  ok = reml_codegen_init(&codegen, "test_module");
  assert_true(ok);

  ok = reml_codegen_generate(&codegen, unit);
  assert_true(ok);
  assert_int_equal(reml_diagnostics_count(reml_codegen_diagnostics(&codegen)), 0);

  reml_codegen_deinit(&codegen);
  reml_sema_deinit(&sema);
  reml_compilation_unit_free(unit);
}

void test_codegen(void **state) {
  test_codegen_basic(state);
  test_codegen_while(state);
  test_codegen_bigint(state);
  test_codegen_match(state);
  test_codegen_match_range(state);
  test_codegen_match_guard(state);
  test_codegen_enum_constructor(state);
  test_codegen_match_tuple_record(state);
  test_codegen_ref_mut_update(state);
  {
    const char *source = "let x = { a: 1, b: 2 }; let y = { x with b: 3 }; y;";
    reml_compilation_unit *unit = parse_source(source);

    reml_sema sema;
    reml_sema_init(&sema);
    bool ok = reml_sema_check(&sema, unit);
    assert_true(ok);
    assert_int_equal(reml_diagnostics_count(reml_sema_diagnostics(&sema)), 0);

    reml_codegen codegen;
    ok = reml_codegen_init(&codegen, "test_module");
    assert_true(ok);

    ok = reml_codegen_generate(&codegen, unit);
    assert_true(ok);
    assert_int_equal(reml_diagnostics_count(reml_codegen_diagnostics(&codegen)), 0);

    reml_codegen_deinit(&codegen);
    reml_sema_deinit(&sema);
    reml_compilation_unit_free(unit);
  }
  test_codegen_enum_drop(state);
}
