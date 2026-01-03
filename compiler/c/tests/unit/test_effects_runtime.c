#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>

#include <cmocka.h>

#include "reml/runtime/effects.h"
#include "reml/runtime/effects_core.h"

static reml_effect_result test_continuation_fn(void *env, reml_continuation *k) {
  (void)k;
  return reml_effect_result_return(env);
}

static reml_effect_result test_handler_fn(reml_effect_tag tag,
                                          reml_effect_payload payload,
                                          reml_continuation *k,
                                          void *handler_env) {
  (void)tag;
  (void)payload;
  (void)k;
  (void)handler_env;
  return reml_effect_result_panic(REML_EFFECT_STATUS_UNHANDLED);
}

static void test_effects_one_shot_resume(void **state) {
  (void)state;

  reml_effect_frame *frame = reml_effect_push_handler(test_handler_fn, NULL, NULL);
  assert_non_null(frame);

  reml_continuation cont;
  int sentinel = 42;
  reml_continuation_init(&cont, test_continuation_fn, &sentinel, frame);

  reml_effect_result first = reml_effect_resume(&cont, NULL);
  assert_int_equal(first.kind, REML_EFFECT_RESULT_RETURN);
  assert_int_equal(first.status, REML_EFFECT_STATUS_OK);
  assert_ptr_equal(first.payload, &sentinel);

  reml_effect_result second = reml_effect_resume(&cont, NULL);
  assert_int_equal(second.kind, REML_EFFECT_RESULT_PANIC);
  assert_int_equal(second.status, REML_EFFECT_STATUS_RESUME_TWICE);

  reml_effect_pop_handler(frame);
  reml_effect_frame_destroy(frame);
}

static void test_effects_unhandled_perform(void **state) {
  (void)state;

  reml_continuation cont;
  reml_continuation_init(&cont, test_continuation_fn, NULL, NULL);

  reml_effect_result result = reml_effect_perform("state.get", NULL, &cont);
  assert_int_equal(result.kind, REML_EFFECT_RESULT_PANIC);
  assert_int_equal(result.status, REML_EFFECT_STATUS_UNHANDLED);
}

typedef struct {
  reml_effect_frame *frame;
  reml_effect_payload payload;
} reml_effects_exception_env;

static reml_effect_result test_exception_entry(void *env, reml_continuation *k) {
  (void)k;
  reml_effects_exception_env *ctx = (reml_effects_exception_env *)env;
  reml_continuation cont;
  reml_continuation_init(&cont, test_continuation_fn, ctx->payload, ctx->frame);
  return reml_effects_exception_raise(ctx->payload, &cont);
}

static void test_effects_exception_recovery(void **state) {
  (void)state;

  reml_effect_frame *frame = reml_effect_push_handler(reml_effects_exception_handler, NULL, NULL);
  assert_non_null(frame);

  int sentinel = 7;
  reml_effects_exception_env env = {.frame = frame, .payload = &sentinel};

  reml_effect_result result = reml_effect_trampoline(test_exception_entry, &env);
  assert_int_equal(result.kind, REML_EFFECT_RESULT_RETURN);
  assert_int_equal(result.status, REML_EFFECT_STATUS_OK);
  assert_ptr_equal(result.payload, &sentinel);

  reml_effect_pop_handler(frame);
  reml_effect_frame_destroy(frame);
}

void test_effects_runtime(void **state) {
  test_effects_one_shot_resume(state);
  test_effects_unhandled_perform(state);
  test_effects_exception_recovery(state);
}
