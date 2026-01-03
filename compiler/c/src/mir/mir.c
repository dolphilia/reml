#include "reml/mir/mir.h"

static const UT_icd reml_mir_op_icd = { sizeof(reml_mir_op), NULL, NULL, NULL };

static void reml_mir_collect_expr(reml_mir_function *func, const reml_expr *expr);
static void reml_mir_collect_stmt(reml_mir_function *func, const reml_stmt *stmt);

static void reml_mir_collect_expr(reml_mir_function *func, const reml_expr *expr) {
  if (!func || !expr) {
    return;
  }
  switch (expr->kind) {
    case REML_EXPR_UNARY:
      reml_mir_collect_expr(func, expr->data.unary.operand);
      break;
    case REML_EXPR_REF:
      reml_mir_collect_expr(func, expr->data.ref.target);
      break;
    case REML_EXPR_BINARY:
      reml_mir_collect_expr(func, expr->data.binary.left);
      reml_mir_collect_expr(func, expr->data.binary.right);
      break;
    case REML_EXPR_CONSTRUCTOR:
      if (expr->data.ctor.args) {
        for (reml_expr **it = (reml_expr **)utarray_front(expr->data.ctor.args); it != NULL;
             it = (reml_expr **)utarray_next(expr->data.ctor.args, it)) {
          reml_mir_collect_expr(func, *it);
        }
      }
      break;
    case REML_EXPR_PERFORM: {
      reml_mir_op op = {.kind = REML_MIR_OP_PERFORM, .payload = NULL};
      reml_mir_function_push_op(func, op);
      if (expr->data.perform.args) {
        for (reml_expr **it = (reml_expr **)utarray_front(expr->data.perform.args); it != NULL;
             it = (reml_expr **)utarray_next(expr->data.perform.args, it)) {
          reml_mir_collect_expr(func, *it);
        }
      }
      break;
    }
    case REML_EXPR_HANDLE: {
      reml_mir_op begin = {.kind = REML_MIR_OP_HANDLE_BEGIN, .payload = NULL};
      reml_mir_function_push_op(func, begin);
      reml_mir_collect_expr(func, expr->data.handle.target);
      if (expr->data.handle.handler.entries) {
        for (reml_handler_entry *it =
                 (reml_handler_entry *)utarray_front(expr->data.handle.handler.entries);
             it != NULL;
             it = (reml_handler_entry *)utarray_next(expr->data.handle.handler.entries, it)) {
          if (it->kind == REML_HANDLER_ENTRY_OPERATION) {
            reml_mir_collect_expr(func, it->data.operation.body);
          } else {
            reml_mir_collect_expr(func, it->data.ret.body);
          }
        }
      }
      reml_mir_op end = {.kind = REML_MIR_OP_HANDLE_END, .payload = NULL};
      reml_mir_function_push_op(func, end);
      break;
    }
    case REML_EXPR_RESUME: {
      reml_mir_op op = {.kind = REML_MIR_OP_RESUME, .payload = NULL};
      reml_mir_function_push_op(func, op);
      reml_mir_collect_expr(func, expr->data.resume.value);
      break;
    }
    case REML_EXPR_TUPLE:
      if (expr->data.tuple) {
        for (reml_expr **it = (reml_expr **)utarray_front(expr->data.tuple); it != NULL;
             it = (reml_expr **)utarray_next(expr->data.tuple, it)) {
          reml_mir_collect_expr(func, *it);
        }
      }
      break;
    case REML_EXPR_RECORD:
      if (expr->data.record) {
        for (reml_record_expr_field *it =
                 (reml_record_expr_field *)utarray_front(expr->data.record);
             it != NULL;
             it = (reml_record_expr_field *)utarray_next(expr->data.record, it)) {
          reml_mir_collect_expr(func, it->value);
        }
      }
      break;
    case REML_EXPR_RECORD_UPDATE:
      reml_mir_collect_expr(func, expr->data.record_update.base);
      if (expr->data.record_update.fields) {
        for (reml_record_expr_field *it =
                 (reml_record_expr_field *)utarray_front(expr->data.record_update.fields);
             it != NULL;
             it = (reml_record_expr_field *)utarray_next(expr->data.record_update.fields, it)) {
          reml_mir_collect_expr(func, it->value);
        }
      }
      break;
    case REML_EXPR_BLOCK:
      if (expr->data.block.statements) {
        for (reml_stmt **it = (reml_stmt **)utarray_front(expr->data.block.statements);
             it != NULL;
             it = (reml_stmt **)utarray_next(expr->data.block.statements, it)) {
          reml_mir_collect_stmt(func, *it);
        }
      }
      reml_mir_collect_expr(func, expr->data.block.tail);
      break;
    case REML_EXPR_IF:
      reml_mir_collect_expr(func, expr->data.if_expr.condition);
      reml_mir_collect_expr(func, expr->data.if_expr.then_branch);
      reml_mir_collect_expr(func, expr->data.if_expr.else_branch);
      break;
    case REML_EXPR_WHILE:
      reml_mir_collect_expr(func, expr->data.while_expr.condition);
      reml_mir_collect_expr(func, expr->data.while_expr.body);
      break;
    case REML_EXPR_MATCH:
      reml_mir_collect_expr(func, expr->data.match_expr.scrutinee);
      if (expr->data.match_expr.arms) {
        for (reml_match_arm *it =
                 (reml_match_arm *)utarray_front(expr->data.match_expr.arms);
             it != NULL;
             it = (reml_match_arm *)utarray_next(expr->data.match_expr.arms, it)) {
          reml_mir_collect_expr(func, it->guard);
          reml_mir_collect_expr(func, it->body);
        }
      }
      break;
    case REML_EXPR_LITERAL:
    case REML_EXPR_IDENT:
      break;
    default:
      break;
  }
}

static void reml_mir_collect_stmt(reml_mir_function *func, const reml_stmt *stmt) {
  if (!func || !stmt) {
    return;
  }
  switch (stmt->kind) {
    case REML_STMT_EXPR:
      reml_mir_collect_expr(func, stmt->data.expr);
      break;
    case REML_STMT_RETURN:
      reml_mir_collect_expr(func, stmt->data.expr);
      break;
    case REML_STMT_VAL_DECL:
      reml_mir_collect_expr(func, stmt->data.val_decl.value);
      break;
    case REML_STMT_TYPE_DECL:
      break;
    default:
      break;
  }
}

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

void reml_mir_collect_unit(reml_mir_function *func, const reml_compilation_unit *unit) {
  if (!func || !unit || !unit->statements) {
    return;
  }
  for (reml_stmt **it = (reml_stmt **)utarray_front(unit->statements); it != NULL;
       it = (reml_stmt **)utarray_next(unit->statements, it)) {
    reml_mir_collect_stmt(func, *it);
  }
}
