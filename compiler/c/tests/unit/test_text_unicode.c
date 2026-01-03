#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <string.h>

#include <cmocka.h>

#include "reml/runtime/string.h"
#include "reml/text/grapheme.h"
#include "reml/text/unicode.h"
#include "reml/util/string_view.h"

static void test_unicode_validation(void **state) {
  (void)state;
  const char invalid_utf8[] = {(char)0xC3, (char)0x28};
  reml_unicode_error error = {0};
  assert_false(reml_unicode_validate_utf8(invalid_utf8, sizeof(invalid_utf8), &error));
  assert_int_equal(error.kind, REML_UNICODE_INVALID_UTF8);

  const char invalid_scalar[] = {(char)0xED, (char)0xA0, (char)0x80};
  assert_false(reml_unicode_validate_utf8(invalid_scalar, sizeof(invalid_scalar), &error));
  assert_int_equal(error.kind, REML_UNICODE_INVALID_SCALAR);
}

static void test_unicode_nfc(void **state) {
  (void)state;
  const char nfd[] = "e\xCC\x81";
  const char nfc[] = "\xC3\xA9";
  reml_unicode_error error = {0};

  assert_false(reml_unicode_is_nfc(nfd, strlen(nfd), &error));
  assert_int_equal(error.kind, REML_UNICODE_NORMALIZE_REQUIRED);
  assert_true(reml_unicode_is_nfc(nfc, strlen(nfc), &error));
}

static void test_grapheme_len_and_width(void **state) {
  (void)state;
  const char combining[] = "e\xCC\x81";
  const char flag_jp[] = "\xF0\x9F\x87\xAF\xF0\x9F\x87\xB5";
  const char zwj[] = "\xF0\x9F\x91\xA8\xE2\x80\x8D\xF0\x9F\x92\xBB";

  reml_string_view view = reml_string_view_make(combining, strlen(combining));
  assert_int_equal(reml_grapheme_len(view), 1);
  assert_int_equal(reml_grapheme_display_width(view), 1);

  view = reml_string_view_make(flag_jp, strlen(flag_jp));
  assert_int_equal(reml_grapheme_len(view), 1);
  assert_int_equal(reml_grapheme_display_width(view), 2);

  view = reml_string_view_make(zwj, strlen(zwj));
  assert_int_equal(reml_grapheme_len(view), 1);
  assert_int_equal(reml_grapheme_display_width(view), 2);
}

static void test_runtime_string_concat_and_cmp(void **state) {
  (void)state;
  reml_string *left = reml_string_from_utf8("ab", 2);
  reml_string *right = reml_string_from_utf8("cd", 2);
  assert_non_null(left);
  assert_non_null(right);

  reml_string *joined = reml_string_concat(left, right);
  assert_non_null(joined);
  assert_int_equal(joined->len, 4);
  assert_memory_equal(joined->ptr, "abcd", 4);

  assert_int_equal(reml_string_cmp(left, right), -1);
  assert_int_equal(reml_string_cmp(right, left), 1);
  assert_int_equal(reml_string_cmp(left, left), 0);

  reml_string_free(joined);
  reml_string_free(right);
  reml_string_free(left);
}

void test_text_unicode(void **state) {
  test_unicode_validation(state);
  test_unicode_nfc(state);
  test_grapheme_len_and_width(state);
  test_runtime_string_concat_and_cmp(state);
}
