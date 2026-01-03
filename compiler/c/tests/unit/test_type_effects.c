#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>

#include <cmocka.h>
#include <utarray.h>

#include "reml/typeck/type.h"

static UT_array *make_params(reml_type *param) {
  UT_icd param_icd = {sizeof(reml_type *), NULL, NULL, NULL};
  UT_array *params = NULL;
  utarray_new(params, &param_icd);
  if (param) {
    utarray_push_back(params, &param);
  }
  return params;
}

static void test_type_effect_union(void **state) {
  (void)state;

  reml_effect_set effects = reml_effect_union(REML_EFFECT_MUT, REML_EFFECT_IO);
  assert_true((effects & REML_EFFECT_MUT) != 0);
  assert_true((effects & REML_EFFECT_IO) != 0);
  assert_true((effects & REML_EFFECT_PANIC) == 0);
}

static void test_type_function_effects_unify(void **state) {
  (void)state;

  reml_type_ctx ctx;
  reml_type_ctx_init(&ctx);

  reml_type *param = reml_type_int(&ctx);
  reml_type *result = reml_type_int(&ctx);

  UT_array *left_params = make_params(param);
  UT_array *right_params = make_params(param);

  reml_effect_row left_row = reml_effect_row_closed(REML_EFFECT_MUT);
  reml_effect_row right_row = reml_effect_row_closed(REML_EFFECT_MUT);
  reml_type *left = reml_type_make_function(&ctx, left_params, result, left_row);
  reml_type *right = reml_type_make_function(&ctx, right_params, result, right_row);
  assert_true(reml_type_unify(&ctx, left, right));

  UT_array *mismatch_params = make_params(param);
  reml_effect_row mismatch_row = reml_effect_row_closed(REML_EFFECT_IO);
  reml_type *mismatch = reml_type_make_function(&ctx, mismatch_params, result, mismatch_row);
  assert_false(reml_type_unify(&ctx, left, mismatch));

  utarray_free(left_params);
  utarray_free(right_params);
  utarray_free(mismatch_params);
  reml_type_ctx_deinit(&ctx);
}

static void test_type_function_effect_row_open_closed(void **state) {
  (void)state;

  reml_type_ctx ctx;
  reml_type_ctx_init(&ctx);

  reml_type *param = reml_type_int(&ctx);
  reml_type *result = reml_type_int(&ctx);

  UT_array *left_params = make_params(param);
  UT_array *right_params = make_params(param);

  reml_effect_row_var *tail = reml_effect_row_var_make(&ctx);
  reml_effect_row left_row = reml_effect_row_make(REML_EFFECT_MUT, tail);
  reml_effect_row right_row = reml_effect_row_closed(REML_EFFECT_MUT | REML_EFFECT_IO);

  reml_type *left = reml_type_make_function(&ctx, left_params, result, left_row);
  reml_type *right = reml_type_make_function(&ctx, right_params, result, right_row);
  assert_true(reml_type_unify(&ctx, left, right));
  assert_non_null(tail->instance);
  assert_int_equal(tail->instance->effects, REML_EFFECT_IO);

  utarray_free(left_params);
  utarray_free(right_params);
  reml_type_ctx_deinit(&ctx);
}

static void test_type_function_effect_row_open_open(void **state) {
  (void)state;

  reml_type_ctx ctx;
  reml_type_ctx_init(&ctx);

  reml_type *param = reml_type_int(&ctx);
  reml_type *result = reml_type_int(&ctx);

  UT_array *left_params = make_params(param);
  UT_array *right_params = make_params(param);

  reml_effect_row_var *left_tail = reml_effect_row_var_make(&ctx);
  reml_effect_row_var *right_tail = reml_effect_row_var_make(&ctx);
  reml_effect_row left_row = reml_effect_row_make(REML_EFFECT_MUT, left_tail);
  reml_effect_row right_row = reml_effect_row_make(REML_EFFECT_IO, right_tail);

  reml_type *left = reml_type_make_function(&ctx, left_params, result, left_row);
  reml_type *right = reml_type_make_function(&ctx, right_params, result, right_row);
  assert_true(reml_type_unify(&ctx, left, right));
  assert_non_null(left_tail->instance);
  assert_non_null(right_tail->instance);

  utarray_free(left_params);
  utarray_free(right_params);
  reml_type_ctx_deinit(&ctx);
}

void test_type_effects(void **state) {
  test_type_effect_union(state);
  test_type_function_effects_unify(state);
  test_type_function_effect_row_open_closed(state);
  test_type_function_effect_row_open_open(state);
}
