#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <string.h>

#include <utarray.h>

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

static bool has_diag(const reml_diagnostic_list *diags, reml_diagnostic_code code) {
  if (!diags) {
    return false;
  }
  size_t count = reml_diagnostics_count(diags);
  for (size_t i = 0; i < count; ++i) {
    const reml_diagnostic *diag = reml_diagnostics_at(diags, i);
    if (diag && diag->code == code) {
      return true;
    }
  }
  return false;
}

static const reml_diagnostic *find_diag(const reml_diagnostic_list *diags,
                                        reml_diagnostic_code code) {
  if (!diags) {
    return NULL;
  }
  size_t count = reml_diagnostics_count(diags);
  for (size_t i = 0; i < count; ++i) {
    const reml_diagnostic *diag = reml_diagnostics_at(diags, i);
    if (diag && diag->code == code) {
      return diag;
    }
  }
  return NULL;
}

static bool pattern_missing_variant(const reml_diagnostic *diag, const char *name) {
  if (!diag || !diag->pattern || !diag->pattern->missing_variants || !name) {
    return false;
  }
  reml_string_view target = reml_string_view_make(name, strlen(name));
  for (reml_string_view *it =
           (reml_string_view *)utarray_front(diag->pattern->missing_variants);
       it != NULL;
       it = (reml_string_view *)utarray_next(diag->pattern->missing_variants, it)) {
    if (it->length == target.length && memcmp(it->data, target.data, target.length) == 0) {
      return true;
    }
  }
  return false;
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

static void test_sema_trait_float_add_ok(void **state) {
  (void)state;

  const char *source = "let x = 1.0 + 2.5; x;";
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

static void test_sema_trait_string_add_ok(void **state) {
  (void)state;

  const char *source = "let x = \"a\" + \"b\"; x;";
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
  assert_int_equal(diag->code, REML_DIAG_TRAIT_UNRESOLVED);

  reml_sema_deinit(&sema);
  reml_compilation_unit_free(unit);
}

static void test_sema_trait_unresolved(void **state) {
  (void)state;

  const char *source = "let x = \"a\" - \"b\";";
  reml_compilation_unit *unit = parse_source(source);

  reml_sema sema;
  reml_sema_init(&sema);
  bool ok = reml_sema_check(&sema, unit);

  const reml_diagnostic_list *diags = reml_sema_diagnostics(&sema);
  assert_false(ok);
  assert_true(reml_diagnostics_count(diags) > 0);
  const reml_diagnostic *diag = reml_diagnostics_at(diags, 0);
  assert_non_null(diag);
  assert_int_equal(diag->code, REML_DIAG_TRAIT_UNRESOLVED);

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

static void test_sema_attr_pure_violation(void **state) {
  (void)state;

  const char *source = "@pure let x = { var y = 0; y := y + 1; y };";
  reml_compilation_unit *unit = parse_source(source);

  reml_sema sema;
  reml_sema_init(&sema);
  bool ok = reml_sema_check(&sema, unit);

  const reml_diagnostic_list *diags = reml_sema_diagnostics(&sema);
  assert_false(ok);
  assert_true(reml_diagnostics_count(diags) > 0);
  assert_true(has_diag(diags, REML_DIAG_EFFECT_VIOLATION));

  reml_sema_deinit(&sema);
  reml_compilation_unit_free(unit);
}

static void test_sema_attr_no_panic_ok(void **state) {
  (void)state;

  const char *source = "@no_panic let x = 1 + 2; x;";
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

static void test_sema_match_non_exhaustive(void **state) {
  (void)state;

  const char *source = "let x = true; match x with | true -> 1;";
  reml_compilation_unit *unit = parse_source(source);

  reml_sema sema;
  reml_sema_init(&sema);
  bool ok = reml_sema_check(&sema, unit);

  const reml_diagnostic_list *diags = reml_sema_diagnostics(&sema);
  assert_false(ok);
  assert_true(reml_diagnostics_count(diags) > 0);
  assert_true(has_diag(diags, REML_DIAG_PATTERN_EXHAUSTIVENESS_MISSING));
  const reml_diagnostic *diag =
      find_diag(diags, REML_DIAG_PATTERN_EXHAUSTIVENESS_MISSING);
  assert_non_null(diag);
  assert_non_null(diag->pattern);
  assert_true(pattern_missing_variant(diag, "false"));

  reml_sema_deinit(&sema);
  reml_compilation_unit_free(unit);
}

static void test_sema_match_unreachable(void **state) {
  (void)state;

  const char *source = "let x = true; match x with | _ -> 1 | false -> 2;";
  reml_compilation_unit *unit = parse_source(source);

  reml_sema sema;
  reml_sema_init(&sema);
  bool ok = reml_sema_check(&sema, unit);

  const reml_diagnostic_list *diags = reml_sema_diagnostics(&sema);
  assert_false(ok);
  assert_true(reml_diagnostics_count(diags) > 0);
  assert_true(has_diag(diags, REML_DIAG_PATTERN_UNREACHABLE_ARM));

  reml_sema_deinit(&sema);
  reml_compilation_unit_free(unit);
}

static void test_sema_match_guard_ok(void **state) {
  (void)state;

  const char *source = "let x = 1; match x with | 1 when true -> 2 | _ -> 3;";
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

static void test_sema_ref_read_ok(void **state) {
  (void)state;

  const char *source = "let x = 1; let r = &x; *r;";
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

static void test_sema_ref_mut_update_ok(void **state) {
  (void)state;

  const char *source = "var x = 1; let r = &mut x; *r := 2; x;";
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

static void test_sema_ref_alias_conflict(void **state) {
  (void)state;

  const char *source = "var x = 1; let a = &x; let b = &mut x;";
  reml_compilation_unit *unit = parse_source(source);

  reml_sema sema;
  reml_sema_init(&sema);
  bool ok = reml_sema_check(&sema, unit);

  const reml_diagnostic_list *diags = reml_sema_diagnostics(&sema);
  assert_false(ok);
  assert_true(has_diag(diags, REML_DIAG_REF_ALIAS_CONFLICT));

  reml_sema_deinit(&sema);
  reml_compilation_unit_free(unit);
}

static void test_sema_ref_mut_requires_var(void **state) {
  (void)state;

  const char *source = "let x = 1; let r = &mut x;";
  reml_compilation_unit *unit = parse_source(source);

  reml_sema sema;
  reml_sema_init(&sema);
  bool ok = reml_sema_check(&sema, unit);

  const reml_diagnostic_list *diags = reml_sema_diagnostics(&sema);
  assert_false(ok);
  assert_true(has_diag(diags, REML_DIAG_REF_NOT_MUTABLE));

  reml_sema_deinit(&sema);
  reml_compilation_unit_free(unit);
}

static void test_sema_match_range_ok(void **state) {
  (void)state;

  const char *source = "let x = 1; match x with | 1..=3 -> 2 | _ -> 3;";
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

static void test_sema_enum_constructor_ok(void **state) {
  (void)state;

  const char *source =
      "type Option = | Some(Int) | None; let x = Some(1); match x with | Some(_) -> 1 | None() -> 2;";
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

static void test_sema_match_enum_payload_non_exhaustive(void **state) {
  (void)state;

  const char *source =
      "type Option = | Some(Int) | None; let x = Some(1); match x with | Some(1) -> 1 | None() -> 2;";
  reml_compilation_unit *unit = parse_source(source);

  reml_sema sema;
  reml_sema_init(&sema);
  bool ok = reml_sema_check(&sema, unit);

  const reml_diagnostic_list *diags = reml_sema_diagnostics(&sema);
  assert_false(ok);
  assert_true(reml_diagnostics_count(diags) > 0);
  const reml_diagnostic *diag =
      find_diag(diags, REML_DIAG_PATTERN_EXHAUSTIVENESS_MISSING);
  assert_non_null(diag);
  assert_true(pattern_missing_variant(diag, "Some"));

  reml_sema_deinit(&sema);
  reml_compilation_unit_free(unit);
}

static void test_sema_match_tuple_record_ok(void **state) {
  (void)state;

  {
    const char *source = "let x = (1, 2); match x with | (1, 2) -> 1 | _ -> 2;";
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
  {
    const char *source =
        "let x = { a: 1, b: 2 }; match x with | { a: 1, b: 2 } -> 1 | _ -> 2;";
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
  {
    const char *source = "let x = { a: 1, b: 2 }; let y = { x with b: 3 }; y;";
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
  {
    const char *source = "type Option = | Some(Int) | None; let x = Foo(1);";
    reml_compilation_unit *unit = parse_source(source);

    reml_sema sema;
    reml_sema_init(&sema);
    bool ok = reml_sema_check(&sema, unit);

    const reml_diagnostic_list *diags = reml_sema_diagnostics(&sema);
    assert_false(ok);
    assert_true(reml_diagnostics_count(diags) > 0);
    assert_true(has_diag(diags, REML_DIAG_CONSTRUCTOR_UNKNOWN));

    reml_sema_deinit(&sema);
    reml_compilation_unit_free(unit);
  }
  {
    const char *source = "let x = { a: 1, b: 2 }; match x with | { a: 1 } -> 1 | _ -> 2;";
    reml_compilation_unit *unit = parse_source(source);

    reml_sema sema;
    reml_sema_init(&sema);
    bool ok = reml_sema_check(&sema, unit);

    const reml_diagnostic_list *diags = reml_sema_diagnostics(&sema);
    assert_false(ok);
    assert_true(reml_diagnostics_count(diags) > 0);
    assert_true(has_diag(diags, REML_DIAG_RECORD_FIELD_MISSING));

    reml_sema_deinit(&sema);
    reml_compilation_unit_free(unit);
  }
}

void test_sema(void **state) {
  test_sema_basic_ok(state);
  test_sema_trait_float_add_ok(state);
  test_sema_trait_string_add_ok(state);
  test_sema_type_mismatch(state);
  test_sema_trait_unresolved(state);
  test_sema_undefined_symbol(state);
  test_sema_bigint_literal_ok(state);
  test_sema_attr_pure_violation(state);
  test_sema_attr_no_panic_ok(state);
  test_sema_match_non_exhaustive(state);
  test_sema_match_unreachable(state);
  test_sema_match_guard_ok(state);
  test_sema_ref_read_ok(state);
  test_sema_ref_mut_update_ok(state);
  test_sema_ref_alias_conflict(state);
  test_sema_ref_mut_requires_var(state);
  test_sema_match_range_ok(state);
  test_sema_enum_constructor_ok(state);
  test_sema_match_enum_payload_non_exhaustive(state);
  test_sema_match_tuple_record_ok(state);
}
