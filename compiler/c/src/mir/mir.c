#include "reml/mir/mir.h"

static const UT_icd reml_mir_op_icd = { sizeof(reml_mir_op), NULL, NULL, NULL };

void reml_mir_function_init(reml_mir_function *func) {
  if (!func) {
    return;
  }
  utarray_new(func->ops, &reml_mir_op_icd);
  func->requires_cps = false;
  func->cps_lowered = false;
}

void reml_mir_function_deinit(reml_mir_function *func) {
  if (!func) {
    return;
  }
  if (func->ops) {
    utarray_free(func->ops);
  }
  func->ops = NULL;
  func->requires_cps = false;
  func->cps_lowered = false;
}

void reml_mir_function_push_op(reml_mir_function *func, reml_mir_op op) {
  if (!func || !func->ops) {
    return;
  }
  utarray_push_back(func->ops, &op);
}

bool reml_mir_function_requires_cps(reml_mir_function *func) {
  if (!func || !func->ops) {
    return false;
  }
  func->requires_cps = false;
  for (reml_mir_op *op = (reml_mir_op *)utarray_front(func->ops); op;
       op = (reml_mir_op *)utarray_next(func->ops, op)) {
    switch (op->kind) {
      case REML_MIR_OP_PERFORM:
      case REML_MIR_OP_RESUME:
      case REML_MIR_OP_HANDLE_BEGIN:
        func->requires_cps = true;
        return true;
      case REML_MIR_OP_RETURN:
      case REML_MIR_OP_HANDLE_END:
        break;
    }
  }
  return false;
}

bool reml_mir_lower_to_cps(reml_mir_function *func) {
  if (!func) {
    return false;
  }
  if (reml_mir_function_requires_cps(func)) {
    func->cps_lowered = true;
  } else {
    func->cps_lowered = false;
  }
  return true;
}
