#include "reml/runtime/effects.h"

#include <stdlib.h>

void reml_continuation_init(reml_continuation *cont,
                            reml_effect_fn fn,
                            void *env,
                            reml_effect_frame *handler) {
  if (!cont) {
    return;
  }
  cont->fn = fn;
  cont->env = env;
  cont->handler = handler;
  cont->consumed = false;
}

reml_effect_result reml_effect_result_return(reml_effect_payload payload) {
  reml_effect_result result;
  result.kind = REML_EFFECT_RESULT_RETURN;
  result.status = REML_EFFECT_STATUS_OK;
  result.tag = NULL;
  result.payload = payload;
  result.cont = NULL;
  return result;
}

reml_effect_result reml_effect_result_perform(reml_effect_tag tag,
                                              reml_effect_payload payload,
                                              reml_continuation *k) {
  reml_effect_result result;
  result.kind = REML_EFFECT_RESULT_PERFORM;
  result.status = REML_EFFECT_STATUS_OK;
  result.tag = tag;
  result.payload = payload;
  result.cont = k;
  return result;
}

reml_effect_result reml_effect_result_panic(reml_effect_status status) {
  reml_effect_result result;
  result.kind = REML_EFFECT_RESULT_PANIC;
  result.status = status;
  result.tag = NULL;
  result.payload = NULL;
  result.cont = NULL;
  return result;
}

reml_effect_frame *reml_effect_push_handler(reml_effect_handler_fn handler,
                                            void *handler_env,
                                            reml_effect_frame *parent) {
  reml_effect_frame *frame = (reml_effect_frame *)malloc(sizeof(reml_effect_frame));
  if (!frame) {
    return NULL;
  }
  frame->handler = handler;
  frame->env = handler_env;
  frame->parent = parent;
  frame->active = true;
  return frame;
}

reml_effect_frame *reml_effect_pop_handler(reml_effect_frame *frame) {
  if (!frame) {
    return NULL;
  }
  frame->active = false;
  return frame->parent;
}

void reml_effect_frame_destroy(reml_effect_frame *frame) {
  free(frame);
}

reml_effect_result reml_effect_perform(reml_effect_tag tag,
                                       reml_effect_payload payload,
                                       reml_continuation *k) {
  if (!k || !k->handler || !k->handler->active) {
    return reml_effect_result_panic(REML_EFFECT_STATUS_UNHANDLED);
  }
  reml_effect_frame *frame = k->handler;
  return frame->handler(tag, payload, k, frame->env);
}

reml_effect_result reml_effect_resume(reml_continuation *k, reml_effect_payload value) {
  (void)value;
  if (!k) {
    return reml_effect_result_panic(REML_EFFECT_STATUS_RESUME_OUT_OF_SCOPE);
  }
  if (k->consumed) {
    return reml_effect_result_panic(REML_EFFECT_STATUS_RESUME_TWICE);
  }
  if (!k->handler || !k->handler->active || !k->fn) {
    return reml_effect_result_panic(REML_EFFECT_STATUS_RESUME_OUT_OF_SCOPE);
  }
  k->consumed = true;
  return k->fn(k->env, k);
}

reml_effect_result reml_effect_trampoline(reml_effect_fn entry, void *env) {
  if (!entry) {
    return reml_effect_result_panic(REML_EFFECT_STATUS_UNHANDLED);
  }
  reml_effect_result result = entry(env, NULL);
  while (result.kind == REML_EFFECT_RESULT_PERFORM) {
    result = reml_effect_perform(result.tag, result.payload, result.cont);
  }
  return result;
}
