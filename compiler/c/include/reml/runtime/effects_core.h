#ifndef REML_RUNTIME_EFFECTS_CORE_H
#define REML_RUNTIME_EFFECTS_CORE_H

#include "reml/runtime/effects.h"

#ifdef __cplusplus
extern "C" {
#endif

#define REML_EFFECT_TAG_STATE_GET "state.get"
#define REML_EFFECT_TAG_STATE_PUT "state.put"
#define REML_EFFECT_TAG_EXCEPTION_RAISE "exception.raise"

typedef struct {
  void *value;
} reml_effects_state;

reml_effect_result reml_effects_state_get(reml_continuation *k);
reml_effect_result reml_effects_state_put(reml_effect_payload value, reml_continuation *k);
reml_effect_result reml_effects_exception_raise(reml_effect_payload value, reml_continuation *k);

reml_effect_result reml_effects_state_handler(reml_effect_tag tag,
                                              reml_effect_payload payload,
                                              reml_continuation *k,
                                              void *handler_env);
reml_effect_result reml_effects_exception_handler(reml_effect_tag tag,
                                                  reml_effect_payload payload,
                                                  reml_continuation *k,
                                                  void *handler_env);

#ifdef __cplusplus
}
#endif

#endif
