#ifndef REML_RUNTIME_EFFECTS_H
#define REML_RUNTIME_EFFECTS_H

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef const char *reml_effect_tag;
typedef void *reml_effect_payload;

typedef enum {
  REML_EFFECT_RESULT_RETURN,
  REML_EFFECT_RESULT_PERFORM,
  REML_EFFECT_RESULT_PANIC
} reml_effect_result_kind;

typedef enum {
  REML_EFFECT_STATUS_OK,
  REML_EFFECT_STATUS_UNHANDLED,
  REML_EFFECT_STATUS_RESUME_TWICE,
  REML_EFFECT_STATUS_RESUME_OUT_OF_SCOPE
} reml_effect_status;

typedef struct reml_continuation reml_continuation;
typedef struct reml_effect_frame reml_effect_frame;

typedef struct {
  reml_effect_result_kind kind;
  reml_effect_status status;
  reml_effect_tag tag;
  reml_effect_payload payload;
  reml_continuation *cont;
} reml_effect_result;

typedef reml_effect_result (*reml_effect_fn)(void *env, reml_continuation *k);
typedef reml_effect_result (*reml_effect_handler_fn)(
  reml_effect_tag tag,
  reml_effect_payload payload,
  reml_continuation *k,
  void *handler_env
);

struct reml_continuation {
  reml_effect_fn fn;
  void *env;
  reml_effect_frame *handler;
  bool consumed;
};

struct reml_effect_frame {
  reml_effect_handler_fn handler;
  void *env;
  reml_effect_frame *parent;
  bool active;
};

void reml_continuation_init(reml_continuation *cont,
                            reml_effect_fn fn,
                            void *env,
                            reml_effect_frame *handler);

reml_effect_result reml_effect_result_return(reml_effect_payload payload);
reml_effect_result reml_effect_result_perform(reml_effect_tag tag,
                                              reml_effect_payload payload,
                                              reml_continuation *k);
reml_effect_result reml_effect_result_panic(reml_effect_status status);

reml_effect_frame *reml_effect_push_handler(reml_effect_handler_fn handler,
                                            void *handler_env,
                                            reml_effect_frame *parent);
reml_effect_frame *reml_effect_pop_handler(reml_effect_frame *frame);
void reml_effect_frame_destroy(reml_effect_frame *frame);

reml_effect_result reml_effect_perform(reml_effect_tag tag,
                                       reml_effect_payload payload,
                                       reml_continuation *k);
reml_effect_result reml_effect_resume(reml_continuation *k, reml_effect_payload value);
reml_effect_result reml_effect_trampoline(reml_effect_fn entry, void *env);

#ifdef __cplusplus
}
#endif

#endif
