#include "reml/runtime/effects_core.h"

#include <string.h>

reml_effect_result reml_effects_state_get(reml_continuation *k) {
  return reml_effect_perform(REML_EFFECT_TAG_STATE_GET, NULL, k);
}

reml_effect_result reml_effects_state_put(reml_effect_payload value, reml_continuation *k) {
  return reml_effect_perform(REML_EFFECT_TAG_STATE_PUT, value, k);
}

reml_effect_result reml_effects_exception_raise(reml_effect_payload value, reml_continuation *k) {
  return reml_effect_perform(REML_EFFECT_TAG_EXCEPTION_RAISE, value, k);
}

reml_effect_result reml_effects_state_handler(reml_effect_tag tag,
                                              reml_effect_payload payload,
                                              reml_continuation *k,
                                              void *handler_env) {
  reml_effects_state *state = (reml_effects_state *)handler_env;
  if (!state) {
    return reml_effect_result_panic(REML_EFFECT_STATUS_UNHANDLED);
  }
  if (tag && strcmp(tag, REML_EFFECT_TAG_STATE_GET) == 0) {
    return reml_effect_resume(k, state->value);
  }
  if (tag && strcmp(tag, REML_EFFECT_TAG_STATE_PUT) == 0) {
    state->value = payload;
    return reml_effect_resume(k, NULL);
  }
  return reml_effect_result_panic(REML_EFFECT_STATUS_UNHANDLED);
}

reml_effect_result reml_effects_exception_handler(reml_effect_tag tag,
                                                  reml_effect_payload payload,
                                                  reml_continuation *k,
                                                  void *handler_env) {
  (void)k;
  (void)handler_env;
  if (tag && strcmp(tag, REML_EFFECT_TAG_EXCEPTION_RAISE) == 0) {
    return reml_effect_result_return(payload);
  }
  return reml_effect_result_panic(REML_EFFECT_STATUS_UNHANDLED);
}
