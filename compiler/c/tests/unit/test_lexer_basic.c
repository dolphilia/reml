#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <string.h>

#include <cmocka.h>

#include "reml/lexer/lexer.h"

static void test_lexer_basic_tokens(void **state) {
  (void)state;

  const char *source = "return 1 + 2 * 3;";
  reml_lexer lexer;
  reml_lexer_init(&lexer, source, strlen(source));

  reml_token_kind expected[] = {
      REML_TOKEN_KW_RETURN,
      REML_TOKEN_INT,
      REML_TOKEN_PLUS,
      REML_TOKEN_INT,
      REML_TOKEN_STAR,
      REML_TOKEN_INT,
      REML_TOKEN_SEMI,
      REML_TOKEN_EOF,
  };

  size_t count = sizeof(expected) / sizeof(expected[0]);
  for (size_t i = 0; i < count; ++i) {
    reml_token token = reml_lexer_next(&lexer);
    assert_int_equal(token.kind, expected[i]);
    if (token.kind == REML_TOKEN_INVALID) {
      break;
    }
  }

  assert_false(lexer.has_error);
}

static void test_lexer_string_literals(void **state) {
  (void)state;

  const char *source =
      "\"a\\n\\u{1F600}\" r\"raw\\n\" r#\"raw\"# \"\"\"line1\nline2\"\"\"";
  reml_lexer lexer;
  reml_lexer_init(&lexer, source, strlen(source));

  reml_token_kind expected[] = {
      REML_TOKEN_STRING,
      REML_TOKEN_STRING_RAW,
      REML_TOKEN_STRING_RAW,
      REML_TOKEN_STRING_MULTILINE,
      REML_TOKEN_EOF,
  };

  size_t count = sizeof(expected) / sizeof(expected[0]);
  for (size_t i = 0; i < count; ++i) {
    reml_token token = reml_lexer_next(&lexer);
    assert_int_equal(token.kind, expected[i]);
    if (token.kind == REML_TOKEN_INVALID) {
      break;
    }
  }

  assert_false(lexer.has_error);
}

void test_lexer_basic(void **state) {
  test_lexer_basic_tokens(state);
  test_lexer_string_literals(state);
}
