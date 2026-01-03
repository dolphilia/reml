#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <cmocka.h>

#include "reml/ast/printer.h"
#include "reml/parser/parser.h"

static char *render_unit(reml_compilation_unit *unit) {
  char *buffer = NULL;
  size_t size = 0;
  FILE *stream = open_memstream(&buffer, &size);
  if (!stream) {
    return NULL;
  }
  reml_ast_write_compilation_unit(stream, unit);
  fclose(stream);
  return buffer;
}

static void test_parser_basic_return_expression(void **state) {
  (void)state;

  const char *source = "return 1 + 2 * 3;";
  reml_parser parser;
  reml_parser_init(&parser, source, strlen(source));

  reml_compilation_unit *unit = reml_parse_compilation_unit(&parser);
  assert_non_null(unit);

  char *rendered = render_unit(unit);
  assert_non_null(rendered);
  assert_string_equal(rendered, "(unit (return (+ (int 1) (* (int 2) (int 3)))))");

  free(rendered);
  reml_compilation_unit_free(unit);
}

void test_parser_basic(void **state) {
  test_parser_basic_return_expression(state);
  {
    const char *source = "let x = 1; match x with | 1 -> 2 | _ -> 3;";
    reml_parser parser;
    reml_parser_init(&parser, source, strlen(source));

    reml_compilation_unit *unit = reml_parse_compilation_unit(&parser);
    assert_non_null(unit);

    char *rendered = render_unit(unit);
    assert_non_null(rendered);
    assert_string_equal(
        rendered,
        "(unit (let (pident x) (int 1)) (expr (match (ident x) (arm (plit 1) (int 2)) (arm (_) (int 3)))))");

    free(rendered);
    reml_compilation_unit_free(unit);
  }
  {
    const char *source = "let x = 1; match x with | Some(1) -> 2 | _ -> 3;";
    reml_parser parser;
    reml_parser_init(&parser, source, strlen(source));

    reml_compilation_unit *unit = reml_parse_compilation_unit(&parser);
    assert_non_null(unit);

    char *rendered = render_unit(unit);
    assert_non_null(rendered);
    assert_string_equal(
        rendered,
        "(unit (let (pident x) (int 1)) (expr (match (ident x) (arm (pctor Some (plit 1)) (int 2)) (arm (_) (int 3)))))");

    free(rendered);
    reml_compilation_unit_free(unit);
  }
  {
    const char *source = "let x = 1; match x with | 1 when true -> 2 | _ -> 3;";
    reml_parser parser;
    reml_parser_init(&parser, source, strlen(source));

    reml_compilation_unit *unit = reml_parse_compilation_unit(&parser);
    assert_non_null(unit);

    char *rendered = render_unit(unit);
    assert_non_null(rendered);
    assert_string_equal(
        rendered,
        "(unit (let (pident x) (int 1)) (expr (match (ident x) (arm (plit 1) (guard (bool true)) (int 2)) (arm (_) (int 3)))))");

    free(rendered);
    reml_compilation_unit_free(unit);
  }
  {
    const char *source = "let x = 1; match x with | 1..=3 -> 2 | _ -> 3;";
    reml_parser parser;
    reml_parser_init(&parser, source, strlen(source));

    reml_compilation_unit *unit = reml_parse_compilation_unit(&parser);
    assert_non_null(unit);

    char *rendered = render_unit(unit);
    assert_non_null(rendered);
    assert_string_equal(
        rendered,
        "(unit (let (pident x) (int 1)) (expr (match (ident x) (arm (prange 1 ..= 3) (int 2)) (arm (_) (int 3)))))");

    free(rendered);
    reml_compilation_unit_free(unit);
  }
  {
    const char *source = "while true { let x = 1; };";
    reml_parser parser;
    reml_parser_init(&parser, source, strlen(source));

    reml_compilation_unit *unit = reml_parse_compilation_unit(&parser);
    assert_non_null(unit);

    char *rendered = render_unit(unit);
    assert_non_null(rendered);
    assert_string_equal(rendered,
                        "(unit (expr (while (bool true) (block (let (pident x) (int 1))))))");

    free(rendered);
    reml_compilation_unit_free(unit);
  }
  {
    const char *source = "let x = 9223372036854775808;";
    reml_parser parser;
    reml_parser_init(&parser, source, strlen(source));

    reml_compilation_unit *unit = reml_parse_compilation_unit(&parser);
    assert_non_null(unit);

    char *rendered = render_unit(unit);
    assert_non_null(rendered);
    assert_string_equal(rendered,
                        "(unit (let (pident x) (bigint 9223372036854775808)))");

    free(rendered);
    reml_compilation_unit_free(unit);
  }
  {
    const char *source = "let x = Some(1);";
    reml_parser parser;
    reml_parser_init(&parser, source, strlen(source));

    reml_compilation_unit *unit = reml_parse_compilation_unit(&parser);
    assert_non_null(unit);

    char *rendered = render_unit(unit);
    assert_non_null(rendered);
    assert_string_equal(rendered, "(unit (let (pident x) (ctor Some (int 1))))");

    free(rendered);
    reml_compilation_unit_free(unit);
  }
  {
    const char *source = "let x = (1, 2);";
    reml_parser parser;
    reml_parser_init(&parser, source, strlen(source));

    reml_compilation_unit *unit = reml_parse_compilation_unit(&parser);
    assert_non_null(unit);

    char *rendered = render_unit(unit);
    assert_non_null(rendered);
    assert_string_equal(rendered, "(unit (let (pident x) (tuple (int 1) (int 2))))");

    free(rendered);
    reml_compilation_unit_free(unit);
  }
  {
    const char *source = "let x = { a: 1, b: 2 };";
    reml_parser parser;
    reml_parser_init(&parser, source, strlen(source));

    reml_compilation_unit *unit = reml_parse_compilation_unit(&parser);
    assert_non_null(unit);

    char *rendered = render_unit(unit);
    assert_non_null(rendered);
    assert_string_equal(
        rendered,
        "(unit (let (pident x) (record (field a (int 1)) (field b (int 2)))))");

    free(rendered);
    reml_compilation_unit_free(unit);
  }
  {
    const char *source = "let { a, b: x } = { a: 1, b: 2 };";
    reml_parser parser;
    reml_parser_init(&parser, source, strlen(source));

    reml_compilation_unit *unit = reml_parse_compilation_unit(&parser);
    assert_non_null(unit);

    char *rendered = render_unit(unit);
    assert_non_null(rendered);
    assert_string_equal(
        rendered,
        "(unit (let (precord (field a (pident a)) (field b (pident x))) (record (field a (int 1)) (field b (int 2)))))");

    free(rendered);
    reml_compilation_unit_free(unit);
  }
}
