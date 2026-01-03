#ifndef REML_MIR_MIR_H
#define REML_MIR_MIR_H

#include <stdbool.h>

#include <utarray.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
  REML_MIR_OP_RETURN,
  REML_MIR_OP_PERFORM,
  REML_MIR_OP_RESUME,
  REML_MIR_OP_HANDLE_BEGIN,
  REML_MIR_OP_HANDLE_END
} reml_mir_op_kind;

typedef struct {
  reml_mir_op_kind kind;
  void *payload;
} reml_mir_op;

typedef struct {
  UT_array *ops;
  bool requires_cps;
  bool cps_lowered;
} reml_mir_function;

void reml_mir_function_init(reml_mir_function *func);
void reml_mir_function_deinit(reml_mir_function *func);
void reml_mir_function_push_op(reml_mir_function *func, reml_mir_op op);
bool reml_mir_function_requires_cps(reml_mir_function *func);
bool reml_mir_lower_to_cps(reml_mir_function *func);

#ifdef __cplusplus
}
#endif

#endif
