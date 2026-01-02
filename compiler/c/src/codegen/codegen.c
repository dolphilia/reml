#include "reml/codegen/codegen.h"

#include <errno.h>
#include <stdlib.h>
#include <string.h>

#include <utarray.h>

#include "reml/typeck/type.h"
#include "reml/util/span.h"

typedef struct {
  reml_symbol_id id;
  LLVMValueRef value;
  LLVMTypeRef type;
} reml_codegen_binding;

typedef struct {
  UT_array *bindings;
} reml_codegen_scope;

typedef struct {
  UT_array *scopes;
} reml_codegen_scope_stack;

typedef struct {
  LLVMValueRef value;
  reml_type *type;
  bool terminated;
} reml_codegen_value;

typedef struct {
  LLVMValueRef value;
  LLVMBasicBlockRef block;
} reml_codegen_phi_incoming;

typedef struct {
  reml_match_arm *arm;
  LLVMValueRef value;
} reml_codegen_switch_case;

static bool reml_type_is_int(reml_type *type);
static bool reml_type_is_bigint(reml_type *type);
static bool reml_type_is_float(reml_type *type);
static bool reml_type_is_bool(reml_type *type);
static bool reml_type_is_unit(reml_type *type);

static void reml_codegen_report(reml_codegen *codegen, reml_diagnostic_code code, reml_span span,
                                const char *message) {
  if (!codegen) {
    return;
  }
  reml_diagnostic diag = {.code = code, .span = span, .message = message};
  reml_diagnostics_push(&codegen->diagnostics, diag);
}

static reml_codegen_scope *reml_codegen_scope_new(void) {
  reml_codegen_scope *scope = (reml_codegen_scope *)calloc(1, sizeof(reml_codegen_scope));
  if (!scope) {
    return NULL;
  }
  UT_icd binding_icd = {sizeof(reml_codegen_binding), NULL, NULL, NULL};
  utarray_new(scope->bindings, &binding_icd);
  return scope;
}

static void reml_codegen_scope_free(reml_codegen_scope *scope) {
  if (!scope) {
    return;
  }
  if (scope->bindings) {
    utarray_free(scope->bindings);
  }
  free(scope);
}

static void reml_codegen_scope_stack_init(reml_codegen_scope_stack *stack) {
  if (!stack) {
    return;
  }
  UT_icd scope_icd = {sizeof(reml_codegen_scope *), NULL, NULL, NULL};
  utarray_new(stack->scopes, &scope_icd);
}

static void reml_codegen_scope_stack_deinit(reml_codegen_scope_stack *stack) {
  if (!stack || !stack->scopes) {
    return;
  }
  for (reml_codegen_scope **it = (reml_codegen_scope **)utarray_front(stack->scopes); it != NULL;
       it = (reml_codegen_scope **)utarray_next(stack->scopes, it)) {
    reml_codegen_scope_free(*it);
  }
  utarray_free(stack->scopes);
  stack->scopes = NULL;
}

static reml_codegen_scope *reml_codegen_scope_stack_current(reml_codegen_scope_stack *stack) {
  if (!stack || !stack->scopes || utarray_len(stack->scopes) == 0) {
    return NULL;
  }
  return *(reml_codegen_scope **)utarray_back(stack->scopes);
}

static void reml_codegen_scope_stack_push(reml_codegen_scope_stack *stack) {
  if (!stack || !stack->scopes) {
    return;
  }
  reml_codegen_scope *scope = reml_codegen_scope_new();
  if (!scope) {
    return;
  }
  utarray_push_back(stack->scopes, &scope);
}

static void reml_codegen_scope_stack_pop(reml_codegen_scope_stack *stack) {
  if (!stack || !stack->scopes || utarray_len(stack->scopes) == 0) {
    return;
  }
  reml_codegen_scope **scope_ptr = (reml_codegen_scope **)utarray_back(stack->scopes);
  reml_codegen_scope_free(*scope_ptr);
  utarray_pop_back(stack->scopes);
}

static void reml_codegen_scope_define(reml_codegen_scope_stack *stack, reml_symbol_id id,
                                     LLVMValueRef value, LLVMTypeRef type) {
  if (!stack || id == REML_SYMBOL_ID_INVALID) {
    return;
  }
  reml_codegen_scope *scope = reml_codegen_scope_stack_current(stack);
  if (!scope) {
    return;
  }
  reml_codegen_binding binding = {.id = id, .value = value, .type = type};
  utarray_push_back(scope->bindings, &binding);
}

static reml_codegen_binding *reml_codegen_scope_lookup(reml_codegen_scope_stack *stack,
                                                       reml_symbol_id id) {
  if (!stack || !stack->scopes || id == REML_SYMBOL_ID_INVALID) {
    return NULL;
  }
  for (reml_codegen_scope **it = (reml_codegen_scope **)utarray_back(stack->scopes); it != NULL;
       it = (reml_codegen_scope **)utarray_prev(stack->scopes, it)) {
    reml_codegen_scope *scope = *it;
    for (reml_codegen_binding *binding = (reml_codegen_binding *)utarray_front(scope->bindings);
         binding != NULL;
         binding = (reml_codegen_binding *)utarray_next(scope->bindings, binding)) {
      if (binding->id == id) {
        return binding;
      }
    }
  }
  return NULL;
}

static char *reml_string_view_to_cstr(reml_string_view view) {
  char *buffer = (char *)malloc(view.length + 1);
  if (!buffer) {
    return NULL;
  }
  memcpy(buffer, view.data, view.length);
  buffer[view.length] = '\0';
  return buffer;
}

static char *reml_strip_numeric_literal(reml_string_view view) {
  char *buffer = (char *)malloc(view.length + 1);
  if (!buffer) {
    return NULL;
  }
  size_t out = 0;
  for (size_t i = 0; i < view.length; ++i) {
    if (view.data[i] != '_') {
      buffer[out++] = view.data[i];
    }
  }
  buffer[out] = '\0';
  return buffer;
}

static LLVMTypeRef reml_codegen_lower_type(reml_codegen *codegen, reml_type *type) {
  if (!codegen || !type) {
    return NULL;
  }
  type = reml_type_prune(type);
  switch (type->kind) {
    case REML_TYPE_INT:
      return LLVMInt64TypeInContext(codegen->context);
    case REML_TYPE_BIGINT:
      return LLVMPointerType(LLVMInt8TypeInContext(codegen->context), 0);
    case REML_TYPE_FLOAT:
      return LLVMDoubleTypeInContext(codegen->context);
    case REML_TYPE_BOOL:
      return LLVMInt1TypeInContext(codegen->context);
    case REML_TYPE_UNIT:
      return LLVMVoidTypeInContext(codegen->context);
    default:
      return NULL;
  }
}

static LLVMValueRef reml_codegen_create_entry_alloca(reml_codegen *codegen, LLVMTypeRef type,
                                                     const char *name) {
  if (!codegen || !codegen->current_function) {
    return NULL;
  }
  LLVMBasicBlockRef entry = LLVMGetEntryBasicBlock(codegen->current_function);
  LLVMValueRef first = LLVMGetFirstInstruction(entry);
  if (first) {
    LLVMPositionBuilderBefore(codegen->alloca_builder, first);
  } else {
    LLVMPositionBuilderAtEnd(codegen->alloca_builder, entry);
  }
  return LLVMBuildAlloca(codegen->alloca_builder, type, name);
}

static LLVMValueRef reml_codegen_get_runtime_fn(reml_codegen *codegen, const char *name,
                                                LLVMTypeRef fn_type) {
  LLVMValueRef fn = LLVMGetNamedFunction(codegen->module, name);
  if (!fn) {
    fn = LLVMAddFunction(codegen->module, name, fn_type);
  }
  return fn;
}

static LLVMTypeRef reml_codegen_bigint_type(reml_codegen *codegen) {
  return LLVMPointerType(LLVMInt8TypeInContext(codegen->context), 0);
}

static LLVMValueRef reml_codegen_call_bigint_binary(reml_codegen *codegen, const char *name,
                                                    LLVMValueRef left, LLVMValueRef right) {
  LLVMTypeRef bigint_ptr = reml_codegen_bigint_type(codegen);
  LLVMTypeRef params[2] = {bigint_ptr, bigint_ptr};
  LLVMTypeRef fn_type = LLVMFunctionType(bigint_ptr, params, 2, 0);
  LLVMValueRef fn = reml_codegen_get_runtime_fn(codegen, name, fn_type);
  LLVMValueRef args[2] = {left, right};
  return LLVMBuildCall2(codegen->builder, fn_type, fn, args, 2, "bigint.op");
}

static LLVMValueRef reml_codegen_call_bigint_cmp(reml_codegen *codegen, const char *name,
                                                 LLVMValueRef left, LLVMValueRef right) {
  LLVMTypeRef bigint_ptr = reml_codegen_bigint_type(codegen);
  LLVMTypeRef params[2] = {bigint_ptr, bigint_ptr};
  LLVMTypeRef fn_type = LLVMFunctionType(LLVMInt32TypeInContext(codegen->context), params, 2, 0);
  LLVMValueRef fn = reml_codegen_get_runtime_fn(codegen, name, fn_type);
  LLVMValueRef args[2] = {left, right};
  return LLVMBuildCall2(codegen->builder, fn_type, fn, args, 2, "bigint.cmp");
}

static bool reml_pattern_is_catch_all(const reml_pattern *pattern) {
  if (!pattern) {
    return false;
  }
  return pattern->kind == REML_PATTERN_WILDCARD || pattern->kind == REML_PATTERN_IDENT;
}

static bool reml_pattern_is_switch_literal(const reml_pattern *pattern, reml_type *type) {
  if (!pattern || pattern->kind != REML_PATTERN_LITERAL) {
    return false;
  }
  if (reml_type_is_bool(type)) {
    return pattern->data.literal.kind == REML_LITERAL_BOOL;
  }
  if (reml_type_is_int(type)) {
    return pattern->data.literal.kind == REML_LITERAL_INT;
  }
  return false;
}

typedef enum {
  REML_NUMERIC_OK,
  REML_NUMERIC_INVALID,
  REML_NUMERIC_OVERFLOW
} reml_numeric_parse_result;

static reml_numeric_parse_result reml_parse_int_literal(reml_literal literal, int64_t *out_value) {
  if (!out_value) {
    return REML_NUMERIC_INVALID;
  }
  char *text = reml_strip_numeric_literal(literal.text);
  if (!text) {
    return REML_NUMERIC_INVALID;
  }
  errno = 0;
  char *end = NULL;
  long long value = strtoll(text, &end, 0);
  bool ok = (errno == 0 && end != NULL && *end == '\0');
  bool overflow = (errno == ERANGE);
  free(text);
  if (!ok) {
    return overflow ? REML_NUMERIC_OVERFLOW : REML_NUMERIC_INVALID;
  }
  *out_value = (int64_t)value;
  return REML_NUMERIC_OK;
}

static bool reml_parse_float_literal(reml_literal literal, double *out_value) {
  if (!out_value) {
    return false;
  }
  char *text = reml_strip_numeric_literal(literal.text);
  if (!text) {
    return false;
  }
  errno = 0;
  char *end = NULL;
  double value = strtod(text, &end);
  bool ok = (errno == 0 && end != NULL && *end == '\0');
  free(text);
  if (!ok) {
    return false;
  }
  *out_value = value;
  return true;
}

static LLVMValueRef reml_codegen_emit_literal_value(reml_codegen *codegen, reml_literal literal,
                                                    reml_span span) {
  switch (literal.kind) {
    case REML_LITERAL_INT: {
      int64_t value = 0;
      reml_numeric_parse_result status = reml_parse_int_literal(literal, &value);
      if (status != REML_NUMERIC_OK) {
        reml_codegen_report(codegen,
                            status == REML_NUMERIC_OVERFLOW ? REML_DIAG_NUMERIC_OVERFLOW
                                                            : REML_DIAG_NUMERIC_INVALID,
                            span,
                            status == REML_NUMERIC_OVERFLOW ? "integer literal overflows i64"
                                                            : "invalid integer literal");
        return NULL;
      }
      return LLVMConstInt(LLVMInt64TypeInContext(codegen->context),
                          (unsigned long long)value, 1);
    }
    case REML_LITERAL_BIGINT: {
      char *text = reml_strip_numeric_literal(literal.text);
      if (!text) {
        reml_codegen_report(codegen, REML_DIAG_NUMERIC_INVALID, span,
                            "invalid bigint literal");
        return NULL;
      }
      LLVMValueRef literal_ptr = LLVMBuildGlobalStringPtr(codegen->builder, text, "bigint.lit");
      free(text);

      LLVMTypeRef bigint_ptr = reml_codegen_bigint_type(codegen);
      LLVMTypeRef params[2] = {LLVMPointerType(LLVMInt8TypeInContext(codegen->context), 0),
                               LLVMInt32TypeInContext(codegen->context)};
      LLVMTypeRef fn_type = LLVMFunctionType(bigint_ptr, params, 2, 0);
      LLVMValueRef fn = reml_codegen_get_runtime_fn(codegen, "reml_numeric_bigint_from_str",
                                                    fn_type);
      LLVMValueRef args[2] = {literal_ptr, LLVMConstInt(LLVMInt32TypeInContext(codegen->context), 0,
                                                       1)};
      return LLVMBuildCall2(codegen->builder, fn_type, fn, args, 2, "bigint.lit");
    }
    case REML_LITERAL_FLOAT: {
      double value = 0.0;
      if (!reml_parse_float_literal(literal, &value)) {
        reml_codegen_report(codegen, REML_DIAG_NUMERIC_INVALID, span,
                            "invalid float literal");
        return NULL;
      }
      return LLVMConstReal(LLVMDoubleTypeInContext(codegen->context), value);
    }
    case REML_LITERAL_BOOL: {
      bool is_true = literal.text.length > 0 && literal.text.data[0] == 't';
      return LLVMConstInt(LLVMInt1TypeInContext(codegen->context), is_true ? 1 : 0, 0);
    }
    default:
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, span,
                          "unsupported literal in pattern");
      return NULL;
  }
}

static bool reml_codegen_match_switch_value(reml_codegen *codegen, reml_pattern *pattern,
                                            reml_type *type, int64_t *out_value,
                                            LLVMValueRef *out_const) {
  if (!pattern || !out_value || !out_const) {
    return false;
  }
  if (!reml_pattern_is_switch_literal(pattern, type)) {
    return false;
  }
  if (pattern->data.literal.kind == REML_LITERAL_BOOL) {
    bool is_true = pattern->data.literal.text.length > 0 &&
                   pattern->data.literal.text.data[0] == 't';
    *out_value = is_true ? 1 : 0;
    *out_const = LLVMConstInt(LLVMInt1TypeInContext(codegen->context), *out_value, 0);
    return true;
  }
  if (pattern->data.literal.kind == REML_LITERAL_INT) {
    int64_t value = 0;
    reml_numeric_parse_result status = reml_parse_int_literal(pattern->data.literal, &value);
    if (status != REML_NUMERIC_OK) {
      reml_codegen_report(codegen,
                          status == REML_NUMERIC_OVERFLOW ? REML_DIAG_NUMERIC_OVERFLOW
                                                          : REML_DIAG_NUMERIC_INVALID,
                          pattern->span,
                          status == REML_NUMERIC_OVERFLOW ? "integer literal overflows i64"
                                                          : "invalid integer literal");
      return false;
    }
    *out_value = value;
    *out_const = LLVMConstInt(LLVMInt64TypeInContext(codegen->context),
                              (unsigned long long)value, 1);
    return true;
  }
  return false;
}

static reml_codegen_value reml_codegen_make_value(LLVMValueRef value, reml_type *type,
                                                  bool terminated) {
  reml_codegen_value result;
  result.value = value;
  result.type = type;
  result.terminated = terminated;
  return result;
}

static reml_codegen_value reml_codegen_emit_expr(reml_codegen *codegen,
                                                 reml_codegen_scope_stack *scopes,
                                                 reml_expr *expr);

static bool reml_codegen_emit_statement(reml_codegen *codegen, reml_codegen_scope_stack *scopes,
                                        reml_stmt *stmt) {
  if (!codegen || !stmt) {
    return false;
  }
  switch (stmt->kind) {
    case REML_STMT_EXPR: {
      reml_codegen_value value = reml_codegen_emit_expr(codegen, scopes, stmt->data.expr);
      return value.terminated;
    }
    case REML_STMT_RETURN: {
      reml_codegen_value value = reml_codegen_emit_expr(codegen, scopes, stmt->data.expr);
      if (value.terminated) {
        return true;
      }
      if (!value.value) {
        reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, stmt->span,
                            "missing return value");
        LLVMBuildRet(codegen->builder,
                     LLVMConstInt(LLVMInt64TypeInContext(codegen->context), 0, 1));
        return true;
      }
      reml_type *result_type = value.type ? reml_type_prune(value.type) : NULL;
      LLVMValueRef result = NULL;
      if (result_type && result_type->kind == REML_TYPE_INT) {
        result = value.value;
      } else if (result_type && result_type->kind == REML_TYPE_BOOL) {
        result = LLVMBuildZExt(codegen->builder, value.value,
                               LLVMInt64TypeInContext(codegen->context), "ret.bool");
      } else if (result_type && result_type->kind == REML_TYPE_FLOAT) {
        result = LLVMBuildFPToSI(codegen->builder, value.value,
                                 LLVMInt64TypeInContext(codegen->context), "ret.float");
      } else {
        reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, stmt->span,
                            "unsupported return type");
        result = LLVMConstInt(LLVMInt64TypeInContext(codegen->context), 0, 1);
      }
      LLVMBuildRet(codegen->builder, result);
      return true;
    }
    case REML_STMT_VAL_DECL: {
      reml_pattern *pattern = stmt->data.val_decl.pattern;
      if (!pattern) {
        return false;
      }
      reml_codegen_value value =
          reml_codegen_emit_expr(codegen, scopes, stmt->data.val_decl.value);
      if (value.terminated) {
        return true;
      }
      if (!value.value) {
        reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, stmt->span,
                            "missing value for let binding");
        return false;
      }
      if (pattern->kind == REML_PATTERN_WILDCARD) {
        return false;
      }
      if (pattern->kind != REML_PATTERN_IDENT) {
        reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, pattern->span,
                            "only identifier patterns are supported in codegen");
        return false;
      }
      LLVMTypeRef llvm_type = reml_codegen_lower_type(codegen, value.type);
      if (!llvm_type) {
        reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, pattern->span,
                            "unsupported value type in let binding");
        return false;
      }
      char *name = reml_string_view_to_cstr(pattern->data.ident);
      LLVMValueRef alloca = reml_codegen_create_entry_alloca(
          codegen, llvm_type, name ? name : "tmp");
      free(name);
      if (!alloca) {
        reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, pattern->span,
                            "failed to allocate local variable");
        return false;
      }
      LLVMBuildStore(codegen->builder, value.value, alloca);
      reml_codegen_scope_define(scopes, pattern->symbol_id, alloca, llvm_type);
      return false;
    }
    default:
      return false;
  }
}

static reml_codegen_value reml_codegen_emit_block(reml_codegen *codegen,
                                                  reml_codegen_scope_stack *scopes,
                                                  reml_block_expr *block, reml_type *type) {
  reml_codegen_scope_stack_push(scopes);
  if (block && block->statements) {
    for (reml_stmt **it = (reml_stmt **)utarray_front(block->statements); it != NULL;
         it = (reml_stmt **)utarray_next(block->statements, it)) {
      if (reml_codegen_emit_statement(codegen, scopes, *it)) {
        reml_codegen_scope_stack_pop(scopes);
        return reml_codegen_make_value(NULL, type, true);
      }
    }
  }

  if (block && block->tail) {
    reml_codegen_value value = reml_codegen_emit_expr(codegen, scopes, block->tail);
    reml_codegen_scope_stack_pop(scopes);
    return value;
  }

  reml_codegen_scope_stack_pop(scopes);
  return reml_codegen_make_value(NULL, type, false);
}

static reml_codegen_value reml_codegen_emit_literal(reml_codegen *codegen, reml_expr *expr) {
  reml_literal literal = expr->data.literal;
  switch (literal.kind) {
    case REML_LITERAL_INT: {
      int64_t value = 0;
      reml_numeric_parse_result status = reml_parse_int_literal(literal, &value);
      if (status != REML_NUMERIC_OK) {
        reml_codegen_report(codegen,
                            status == REML_NUMERIC_OVERFLOW ? REML_DIAG_NUMERIC_OVERFLOW
                                                            : REML_DIAG_NUMERIC_INVALID,
                            expr->span,
                            status == REML_NUMERIC_OVERFLOW ? "integer literal overflows i64"
                                                            : "invalid integer literal");
        return reml_codegen_make_value(NULL, expr->type, false);
      }
      LLVMValueRef llvm_value = LLVMConstInt(LLVMInt64TypeInContext(codegen->context),
                                             (unsigned long long)value, 1);
      return reml_codegen_make_value(llvm_value, expr->type, false);
    }
    case REML_LITERAL_BIGINT: {
      char *text = reml_strip_numeric_literal(literal.text);
      if (!text) {
        reml_codegen_report(codegen, REML_DIAG_NUMERIC_INVALID, expr->span,
                            "invalid bigint literal");
        return reml_codegen_make_value(NULL, expr->type, false);
      }
      LLVMValueRef literal_ptr = LLVMBuildGlobalStringPtr(codegen->builder, text, "bigint.lit");
      free(text);

      LLVMTypeRef bigint_ptr = reml_codegen_bigint_type(codegen);
      LLVMTypeRef params[2] = {LLVMPointerType(LLVMInt8TypeInContext(codegen->context), 0),
                               LLVMInt32TypeInContext(codegen->context)};
      LLVMTypeRef fn_type = LLVMFunctionType(bigint_ptr, params, 2, 0);
      LLVMValueRef fn = reml_codegen_get_runtime_fn(codegen, "reml_numeric_bigint_from_str",
                                                    fn_type);
      LLVMValueRef args[2] = {literal_ptr, LLVMConstInt(LLVMInt32TypeInContext(codegen->context), 0,
                                                       1)};
      LLVMValueRef value = LLVMBuildCall2(codegen->builder, fn_type, fn, args, 2, "bigint.lit");
      return reml_codegen_make_value(value, expr->type, false);
    }
    case REML_LITERAL_FLOAT: {
      double value = 0.0;
      if (!reml_parse_float_literal(literal, &value)) {
        reml_codegen_report(codegen, REML_DIAG_NUMERIC_INVALID, expr->span,
                            "invalid float literal");
        return reml_codegen_make_value(NULL, expr->type, false);
      }
      LLVMValueRef llvm_value = LLVMConstReal(LLVMDoubleTypeInContext(codegen->context), value);
      return reml_codegen_make_value(llvm_value, expr->type, false);
    }
    case REML_LITERAL_BOOL: {
      bool is_true = literal.text.length > 0 && literal.text.data[0] == 't';
      LLVMValueRef llvm_value =
          LLVMConstInt(LLVMInt1TypeInContext(codegen->context), is_true ? 1 : 0, 0);
      return reml_codegen_make_value(llvm_value, expr->type, false);
    }
    case REML_LITERAL_STRING:
    case REML_LITERAL_CHAR:
    default:
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, expr->span,
                          "unsupported literal in codegen");
      return reml_codegen_make_value(NULL, expr->type, false);
  }
}

static reml_codegen_value reml_codegen_emit_ident(reml_codegen *codegen,
                                                  reml_codegen_scope_stack *scopes,
                                                  reml_expr *expr) {
  reml_codegen_binding *binding = reml_codegen_scope_lookup(scopes, expr->symbol_id);
  if (!binding) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, expr->span,
                        "unknown local binding");
    return reml_codegen_make_value(NULL, expr->type, false);
  }
  LLVMValueRef loaded = LLVMBuildLoad2(codegen->builder, binding->type, binding->value, "load");
  return reml_codegen_make_value(loaded, expr->type, false);
}

static LLVMValueRef reml_codegen_emit_pattern_check(reml_codegen *codegen, reml_pattern *pattern,
                                                    LLVMValueRef scrutinee, reml_type *type) {
  if (!pattern) {
    return NULL;
  }
  if (pattern->kind == REML_PATTERN_LITERAL) {
    if (reml_type_is_bool(type) && pattern->data.literal.kind != REML_LITERAL_BOOL) {
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, pattern->span,
                          "boolean match expects a boolean literal");
      return LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 0, 0);
    }
    if (reml_type_is_int(type) && pattern->data.literal.kind != REML_LITERAL_INT) {
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, pattern->span,
                          "integer match expects an integer literal");
      return LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 0, 0);
    }
    if (reml_type_is_bigint(type) && pattern->data.literal.kind != REML_LITERAL_BIGINT) {
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, pattern->span,
                          "bigint match expects a bigint literal");
      return LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 0, 0);
    }
    if (reml_type_is_float(type) && pattern->data.literal.kind != REML_LITERAL_FLOAT) {
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, pattern->span,
                          "float match expects a float literal");
      return LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 0, 0);
    }
    LLVMValueRef literal_value =
        reml_codegen_emit_literal_value(codegen, pattern->data.literal, pattern->span);
    if (!literal_value) {
      return LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 0, 0);
    }

    if (reml_type_is_bool(type) || reml_type_is_int(type)) {
      return LLVMBuildICmp(codegen->builder, LLVMIntEQ, scrutinee, literal_value, "match.cmp");
    }
    if (reml_type_is_float(type)) {
      return LLVMBuildFCmp(codegen->builder, LLVMRealOEQ, scrutinee, literal_value, "match.cmp");
    }
    if (reml_type_is_bigint(type)) {
      LLVMValueRef cmp =
          reml_codegen_call_bigint_cmp(codegen, "reml_numeric_bigint_cmp", scrutinee,
                                       literal_value);
      LLVMValueRef zero = LLVMConstInt(LLVMInt32TypeInContext(codegen->context), 0, 1);
      return LLVMBuildICmp(codegen->builder, LLVMIntEQ, cmp, zero, "match.cmp");
    }
  }

  reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, pattern->span,
                      "unsupported pattern in codegen");
  return LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 0, 0);
}

static void reml_codegen_bind_pattern(reml_codegen *codegen, reml_codegen_scope_stack *scopes,
                                      reml_pattern *pattern, LLVMValueRef scrutinee,
                                      reml_type *type) {
  if (!pattern) {
    return;
  }
  if (pattern->kind == REML_PATTERN_IDENT) {
    if (pattern->symbol_id == REML_SYMBOL_ID_INVALID) {
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, pattern->span,
                          "match binding is missing symbol id");
      return;
    }
    LLVMTypeRef llvm_type = reml_codegen_lower_type(codegen, type);
    if (!llvm_type) {
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, pattern->span,
                          "unsupported match binding type");
      return;
    }
    char *name = reml_string_view_to_cstr(pattern->data.ident);
    LLVMValueRef alloca = reml_codegen_create_entry_alloca(
        codegen, llvm_type, name ? name : "match.bind");
    free(name);
    if (!alloca) {
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, pattern->span,
                          "failed to allocate match binding");
      return;
    }
    LLVMBuildStore(codegen->builder, scrutinee, alloca);
    reml_codegen_scope_define(scopes, pattern->symbol_id, alloca, llvm_type);
    return;
  }
  if (pattern->kind == REML_PATTERN_TUPLE || pattern->kind == REML_PATTERN_RECORD) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, pattern->span,
                        "complex patterns are not supported in codegen");
    return;
  }
  if (pattern->kind == REML_PATTERN_CONSTRUCTOR) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, pattern->span,
                        "constructor patterns are not supported in codegen");
  }
}

static bool reml_type_is_int(reml_type *type) {
  type = type ? reml_type_prune(type) : NULL;
  return type && type->kind == REML_TYPE_INT;
}

static bool reml_type_is_bigint(reml_type *type) {
  type = type ? reml_type_prune(type) : NULL;
  return type && type->kind == REML_TYPE_BIGINT;
}

static bool reml_type_is_float(reml_type *type) {
  type = type ? reml_type_prune(type) : NULL;
  return type && type->kind == REML_TYPE_FLOAT;
}

static bool reml_type_is_bool(reml_type *type) {
  type = type ? reml_type_prune(type) : NULL;
  return type && type->kind == REML_TYPE_BOOL;
}

static bool reml_type_is_unit(reml_type *type) {
  type = type ? reml_type_prune(type) : NULL;
  return type && type->kind == REML_TYPE_UNIT;
}

static reml_codegen_value reml_codegen_emit_unary(reml_codegen *codegen,
                                                  reml_codegen_scope_stack *scopes,
                                                  reml_expr *expr) {
  reml_codegen_value operand = reml_codegen_emit_expr(codegen, scopes, expr->data.unary.operand);
  if (operand.terminated) {
    return operand;
  }
  if (!operand.value) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, expr->span,
                        "missing operand value");
    return reml_codegen_make_value(NULL, expr->type, false);
  }
  if (expr->data.unary.op == REML_TOKEN_MINUS) {
    if (reml_type_is_float(expr->type)) {
      LLVMValueRef value = LLVMBuildFNeg(codegen->builder, operand.value, "neg");
      return reml_codegen_make_value(value, expr->type, false);
    }
    if (reml_type_is_int(expr->type)) {
      LLVMValueRef value = LLVMBuildNeg(codegen->builder, operand.value, "neg");
      return reml_codegen_make_value(value, expr->type, false);
    }
    if (reml_type_is_bigint(expr->type)) {
      LLVMTypeRef bigint_ptr = reml_codegen_bigint_type(codegen);
      LLVMTypeRef params[1] = {bigint_ptr};
      LLVMTypeRef fn_type = LLVMFunctionType(bigint_ptr, params, 1, 0);
      LLVMValueRef fn = reml_codegen_get_runtime_fn(codegen, "reml_numeric_bigint_neg", fn_type);
      LLVMValueRef args[1] = {operand.value};
      LLVMValueRef value = LLVMBuildCall2(codegen->builder, fn_type, fn, args, 1, "bigint.neg");
      return reml_codegen_make_value(value, expr->type, false);
    }
  }
  if (expr->data.unary.op == REML_TOKEN_BANG) {
    if (reml_type_is_bool(operand.type)) {
      LLVMValueRef value = LLVMBuildNot(codegen->builder, operand.value, "not");
      return reml_codegen_make_value(value, expr->type, false);
    }
  }
  reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, expr->span,
                      "unsupported unary operator in codegen");
  return reml_codegen_make_value(NULL, expr->type, false);
}

static reml_codegen_value reml_codegen_emit_binary(reml_codegen *codegen,
                                                   reml_codegen_scope_stack *scopes,
                                                   reml_expr *expr) {
  reml_codegen_value left = reml_codegen_emit_expr(codegen, scopes, expr->data.binary.left);
  if (left.terminated) {
    return left;
  }
  reml_codegen_value right = reml_codegen_emit_expr(codegen, scopes, expr->data.binary.right);
  if (right.terminated) {
    return right;
  }
  if (!left.value || !right.value) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, expr->span,
                        "missing operand values");
    return reml_codegen_make_value(NULL, expr->type, false);
  }

  bool is_float = reml_type_is_float(left.type);
  bool is_int = reml_type_is_int(left.type);
  bool is_bigint = reml_type_is_bigint(left.type);
  bool is_bool = reml_type_is_bool(left.type);

  switch (expr->data.binary.op) {
    case REML_TOKEN_PLUS:
      if (is_float) {
        return reml_codegen_make_value(
            LLVMBuildFAdd(codegen->builder, left.value, right.value, "add"), expr->type, false);
      }
      if (is_int) {
        return reml_codegen_make_value(
            LLVMBuildAdd(codegen->builder, left.value, right.value, "add"), expr->type, false);
      }
      if (is_bigint) {
        LLVMValueRef value = reml_codegen_call_bigint_binary(codegen, "reml_numeric_bigint_add",
                                                             left.value, right.value);
        return reml_codegen_make_value(value, expr->type, false);
      }
      break;
    case REML_TOKEN_MINUS:
      if (is_float) {
        return reml_codegen_make_value(
            LLVMBuildFSub(codegen->builder, left.value, right.value, "sub"), expr->type, false);
      }
      if (is_int) {
        return reml_codegen_make_value(
            LLVMBuildSub(codegen->builder, left.value, right.value, "sub"), expr->type, false);
      }
      if (is_bigint) {
        LLVMValueRef value = reml_codegen_call_bigint_binary(codegen, "reml_numeric_bigint_sub",
                                                             left.value, right.value);
        return reml_codegen_make_value(value, expr->type, false);
      }
      break;
    case REML_TOKEN_STAR:
      if (is_float) {
        return reml_codegen_make_value(
            LLVMBuildFMul(codegen->builder, left.value, right.value, "mul"), expr->type, false);
      }
      if (is_int) {
        return reml_codegen_make_value(
            LLVMBuildMul(codegen->builder, left.value, right.value, "mul"), expr->type, false);
      }
      if (is_bigint) {
        LLVMValueRef value = reml_codegen_call_bigint_binary(codegen, "reml_numeric_bigint_mul",
                                                             left.value, right.value);
        return reml_codegen_make_value(value, expr->type, false);
      }
      break;
    case REML_TOKEN_SLASH:
      if (is_float) {
        return reml_codegen_make_value(
            LLVMBuildFDiv(codegen->builder, left.value, right.value, "div"), expr->type, false);
      }
      if (is_int) {
        return reml_codegen_make_value(
            LLVMBuildSDiv(codegen->builder, left.value, right.value, "div"), expr->type, false);
      }
      if (is_bigint) {
        LLVMValueRef value = reml_codegen_call_bigint_binary(codegen, "reml_numeric_bigint_div",
                                                             left.value, right.value);
        return reml_codegen_make_value(value, expr->type, false);
      }
      break;
    case REML_TOKEN_PERCENT:
      if (is_float) {
        return reml_codegen_make_value(
            LLVMBuildFRem(codegen->builder, left.value, right.value, "rem"), expr->type, false);
      }
      if (is_int) {
        return reml_codegen_make_value(
            LLVMBuildSRem(codegen->builder, left.value, right.value, "rem"), expr->type, false);
      }
      if (is_bigint) {
        LLVMValueRef value = reml_codegen_call_bigint_binary(codegen, "reml_numeric_bigint_rem",
                                                             left.value, right.value);
        return reml_codegen_make_value(value, expr->type, false);
      }
      break;
    case REML_TOKEN_CARET:
      if (is_int) {
        return reml_codegen_make_value(
            LLVMBuildXor(codegen->builder, left.value, right.value, "xor"), expr->type, false);
      }
      break;
    case REML_TOKEN_LT:
      if (is_float) {
        return reml_codegen_make_value(
            LLVMBuildFCmp(codegen->builder, LLVMRealOLT, left.value, right.value, "cmp"),
            expr->type, false);
      }
      if (is_int) {
        return reml_codegen_make_value(
            LLVMBuildICmp(codegen->builder, LLVMIntSLT, left.value, right.value, "cmp"),
            expr->type, false);
      }
      if (is_bigint) {
        LLVMValueRef cmp = reml_codegen_call_bigint_cmp(codegen, "reml_numeric_bigint_cmp",
                                                        left.value, right.value);
        LLVMValueRef zero = LLVMConstInt(LLVMInt32TypeInContext(codegen->context), 0, 1);
        return reml_codegen_make_value(
            LLVMBuildICmp(codegen->builder, LLVMIntSLT, cmp, zero, "cmp"), expr->type, false);
      }
      break;
    case REML_TOKEN_LE:
      if (is_float) {
        return reml_codegen_make_value(
            LLVMBuildFCmp(codegen->builder, LLVMRealOLE, left.value, right.value, "cmp"),
            expr->type, false);
      }
      if (is_int) {
        return reml_codegen_make_value(
            LLVMBuildICmp(codegen->builder, LLVMIntSLE, left.value, right.value, "cmp"),
            expr->type, false);
      }
      if (is_bigint) {
        LLVMValueRef cmp = reml_codegen_call_bigint_cmp(codegen, "reml_numeric_bigint_cmp",
                                                        left.value, right.value);
        LLVMValueRef zero = LLVMConstInt(LLVMInt32TypeInContext(codegen->context), 0, 1);
        return reml_codegen_make_value(
            LLVMBuildICmp(codegen->builder, LLVMIntSLE, cmp, zero, "cmp"), expr->type, false);
      }
      break;
    case REML_TOKEN_GT:
      if (is_float) {
        return reml_codegen_make_value(
            LLVMBuildFCmp(codegen->builder, LLVMRealOGT, left.value, right.value, "cmp"),
            expr->type, false);
      }
      if (is_int) {
        return reml_codegen_make_value(
            LLVMBuildICmp(codegen->builder, LLVMIntSGT, left.value, right.value, "cmp"),
            expr->type, false);
      }
      if (is_bigint) {
        LLVMValueRef cmp = reml_codegen_call_bigint_cmp(codegen, "reml_numeric_bigint_cmp",
                                                        left.value, right.value);
        LLVMValueRef zero = LLVMConstInt(LLVMInt32TypeInContext(codegen->context), 0, 1);
        return reml_codegen_make_value(
            LLVMBuildICmp(codegen->builder, LLVMIntSGT, cmp, zero, "cmp"), expr->type, false);
      }
      break;
    case REML_TOKEN_GE:
      if (is_float) {
        return reml_codegen_make_value(
            LLVMBuildFCmp(codegen->builder, LLVMRealOGE, left.value, right.value, "cmp"),
            expr->type, false);
      }
      if (is_int) {
        return reml_codegen_make_value(
            LLVMBuildICmp(codegen->builder, LLVMIntSGE, left.value, right.value, "cmp"),
            expr->type, false);
      }
      if (is_bigint) {
        LLVMValueRef cmp = reml_codegen_call_bigint_cmp(codegen, "reml_numeric_bigint_cmp",
                                                        left.value, right.value);
        LLVMValueRef zero = LLVMConstInt(LLVMInt32TypeInContext(codegen->context), 0, 1);
        return reml_codegen_make_value(
            LLVMBuildICmp(codegen->builder, LLVMIntSGE, cmp, zero, "cmp"), expr->type, false);
      }
      break;
    case REML_TOKEN_EQEQ:
      if (is_float) {
        return reml_codegen_make_value(
            LLVMBuildFCmp(codegen->builder, LLVMRealOEQ, left.value, right.value, "cmp"),
            expr->type, false);
      }
      if (is_bigint) {
        LLVMValueRef cmp = reml_codegen_call_bigint_cmp(codegen, "reml_numeric_bigint_cmp",
                                                        left.value, right.value);
        LLVMValueRef zero = LLVMConstInt(LLVMInt32TypeInContext(codegen->context), 0, 1);
        return reml_codegen_make_value(
            LLVMBuildICmp(codegen->builder, LLVMIntEQ, cmp, zero, "cmp"), expr->type, false);
      }
      return reml_codegen_make_value(
          LLVMBuildICmp(codegen->builder, LLVMIntEQ, left.value, right.value, "cmp"),
          expr->type, false);
    case REML_TOKEN_NOTEQ:
      if (is_float) {
        return reml_codegen_make_value(
            LLVMBuildFCmp(codegen->builder, LLVMRealONE, left.value, right.value, "cmp"),
            expr->type, false);
      }
      if (is_bigint) {
        LLVMValueRef cmp = reml_codegen_call_bigint_cmp(codegen, "reml_numeric_bigint_cmp",
                                                        left.value, right.value);
        LLVMValueRef zero = LLVMConstInt(LLVMInt32TypeInContext(codegen->context), 0, 1);
        return reml_codegen_make_value(
            LLVMBuildICmp(codegen->builder, LLVMIntNE, cmp, zero, "cmp"), expr->type, false);
      }
      return reml_codegen_make_value(
          LLVMBuildICmp(codegen->builder, LLVMIntNE, left.value, right.value, "cmp"),
          expr->type, false);
    case REML_TOKEN_LOGICAL_AND:
      if (is_bool) {
        return reml_codegen_make_value(
            LLVMBuildAnd(codegen->builder, left.value, right.value, "and"), expr->type, false);
      }
      break;
    case REML_TOKEN_LOGICAL_OR:
      if (is_bool) {
        return reml_codegen_make_value(
            LLVMBuildOr(codegen->builder, left.value, right.value, "or"), expr->type, false);
      }
      break;
    default:
      break;
  }

  reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, expr->span,
                      "unsupported binary operator in codegen");
  return reml_codegen_make_value(NULL, expr->type, false);
}

static reml_codegen_value reml_codegen_emit_if(reml_codegen *codegen,
                                               reml_codegen_scope_stack *scopes, reml_expr *expr) {
  reml_codegen_value cond = reml_codegen_emit_expr(codegen, scopes, expr->data.if_expr.condition);
  if (cond.terminated) {
    return cond;
  }

  LLVMBasicBlockRef then_bb =
      LLVMAppendBasicBlockInContext(codegen->context, codegen->current_function, "if.then");
  LLVMBasicBlockRef else_bb =
      LLVMAppendBasicBlockInContext(codegen->context, codegen->current_function, "if.else");

  LLVMBuildCondBr(codegen->builder, cond.value, then_bb, else_bb);

  LLVMPositionBuilderAtEnd(codegen->builder, then_bb);
  reml_codegen_value then_value =
      reml_codegen_emit_expr(codegen, scopes, expr->data.if_expr.then_branch);
  bool then_terminated = then_value.terminated;

  LLVMPositionBuilderAtEnd(codegen->builder, else_bb);
  reml_codegen_value else_value = {0};
  bool else_terminated = false;
  if (expr->data.if_expr.else_branch) {
    else_value = reml_codegen_emit_expr(codegen, scopes, expr->data.if_expr.else_branch);
    else_terminated = else_value.terminated;
  }

  bool is_unit = reml_type_is_unit(expr->type);
  if (then_terminated && else_terminated) {
    return reml_codegen_make_value(NULL, expr->type, true);
  }

  LLVMBasicBlockRef merge_bb =
      LLVMAppendBasicBlockInContext(codegen->context, codegen->current_function, "if.merge");

  if (!then_terminated) {
    LLVMPositionBuilderAtEnd(codegen->builder, then_bb);
    LLVMBuildBr(codegen->builder, merge_bb);
  }
  if (!else_terminated) {
    LLVMPositionBuilderAtEnd(codegen->builder, else_bb);
    LLVMBuildBr(codegen->builder, merge_bb);
  }

  LLVMPositionBuilderAtEnd(codegen->builder, merge_bb);

  if (!is_unit && (then_terminated || else_terminated)) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, expr->span,
                        "if expression must yield a value on all branches");
    return reml_codegen_make_value(NULL, expr->type, false);
  }

  if (!expr->data.if_expr.else_branch) {
    if (!is_unit) {
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, expr->span,
                          "if expression without else must be unit");
    }
    return reml_codegen_make_value(NULL, expr->type, false);
  }

  if (is_unit) {
    return reml_codegen_make_value(NULL, expr->type, false);
  }

  LLVMTypeRef phi_type = reml_codegen_lower_type(codegen, expr->type);
  if (!phi_type) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, expr->span,
                        "unsupported if expression type");
    return reml_codegen_make_value(NULL, expr->type, false);
  }

  LLVMValueRef phi = LLVMBuildPhi(codegen->builder, phi_type, "if.result");
  LLVMAddIncoming(phi, &then_value.value, &then_bb, 1);
  LLVMAddIncoming(phi, &else_value.value, &else_bb, 1);
  return reml_codegen_make_value(phi, expr->type, false);
}

static reml_codegen_value reml_codegen_emit_while(reml_codegen *codegen,
                                                  reml_codegen_scope_stack *scopes,
                                                  reml_expr *expr) {
  LLVMBasicBlockRef cond_bb =
      LLVMAppendBasicBlockInContext(codegen->context, codegen->current_function, "while.cond");
  LLVMBasicBlockRef body_bb =
      LLVMAppendBasicBlockInContext(codegen->context, codegen->current_function, "while.body");
  LLVMBasicBlockRef exit_bb =
      LLVMAppendBasicBlockInContext(codegen->context, codegen->current_function, "while.exit");

  LLVMBuildBr(codegen->builder, cond_bb);

  LLVMPositionBuilderAtEnd(codegen->builder, cond_bb);
  reml_codegen_value cond = reml_codegen_emit_expr(codegen, scopes, expr->data.while_expr.condition);
  if (cond.terminated || !cond.value) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, expr->span,
                        "missing while condition value");
    LLVMBuildBr(codegen->builder, exit_bb);
    LLVMPositionBuilderAtEnd(codegen->builder, exit_bb);
    return reml_codegen_make_value(NULL, expr->type, false);
  }
  LLVMBuildCondBr(codegen->builder, cond.value, body_bb, exit_bb);

  LLVMPositionBuilderAtEnd(codegen->builder, body_bb);
  reml_codegen_value body = reml_codegen_emit_expr(codegen, scopes, expr->data.while_expr.body);
  if (!body.terminated) {
    LLVMBuildBr(codegen->builder, cond_bb);
  }

  LLVMPositionBuilderAtEnd(codegen->builder, exit_bb);
  return reml_codegen_make_value(NULL, expr->type, false);
}

static reml_codegen_value reml_codegen_emit_match(reml_codegen *codegen,
                                                  reml_codegen_scope_stack *scopes,
                                                  reml_expr *expr) {
  reml_codegen_value scrutinee =
      reml_codegen_emit_expr(codegen, scopes, expr->data.match_expr.scrutinee);
  if (scrutinee.terminated) {
    return scrutinee;
  }
  if (!scrutinee.value) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, expr->span,
                        "missing match scrutinee value");
    return reml_codegen_make_value(NULL, expr->type, false);
  }

  bool is_unit = reml_type_is_unit(expr->type);
  LLVMTypeRef phi_type = NULL;
  if (!is_unit) {
    phi_type = reml_codegen_lower_type(codegen, expr->type);
    if (!phi_type) {
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, expr->span,
                          "unsupported match expression type");
      return reml_codegen_make_value(NULL, expr->type, false);
    }
  }

  LLVMBasicBlockRef merge_bb =
      LLVMAppendBasicBlockInContext(codegen->context, codegen->current_function, "match.merge");

  UT_array *incoming = NULL;
  if (!is_unit) {
    UT_icd incoming_icd = {sizeof(reml_codegen_phi_incoming), NULL, NULL, NULL};
    utarray_new(incoming, &incoming_icd);
  }

  bool has_fallthrough = true;
  bool any_non_terminated = false;
  LLVMBasicBlockRef current_bb = LLVMGetInsertBlock(codegen->builder);
  bool use_switch = false;

  if (expr->data.match_expr.arms && (reml_type_is_bool(scrutinee.type) ||
                                     reml_type_is_int(scrutinee.type))) {
    UT_icd case_icd = {sizeof(reml_codegen_switch_case), NULL, NULL, NULL};
    UT_array *cases = NULL;
    utarray_new(cases, &case_icd);
    UT_icd seen_icd = {sizeof(int64_t), NULL, NULL, NULL};
    UT_array *seen_values = NULL;
    utarray_new(seen_values, &seen_icd);

    reml_match_arm *default_arm = NULL;
    size_t arm_count = utarray_len(expr->data.match_expr.arms);
    size_t index = 0;
    bool valid = true;
    for (reml_match_arm *it = (reml_match_arm *)utarray_front(expr->data.match_expr.arms);
         it != NULL; it = (reml_match_arm *)utarray_next(expr->data.match_expr.arms, it)) {
      bool catch_all = reml_pattern_is_catch_all(it->pattern);
      bool is_last = index + 1 == arm_count;
      if (catch_all) {
        if (!is_last || default_arm) {
          valid = false;
          break;
        }
        default_arm = it;
      } else if (reml_pattern_is_switch_literal(it->pattern, scrutinee.type)) {
        int64_t value = 0;
        LLVMValueRef literal_value = NULL;
        if (!reml_codegen_match_switch_value(codegen, it->pattern, scrutinee.type, &value,
                                              &literal_value)) {
          valid = false;
          break;
        }
        bool seen = false;
        for (int64_t *it_val = (int64_t *)utarray_front(seen_values); it_val != NULL;
             it_val = (int64_t *)utarray_next(seen_values, it_val)) {
          if (*it_val == value) {
            seen = true;
            break;
          }
        }
        if (seen) {
          valid = false;
          break;
        }
        utarray_push_back(seen_values, &value);
        reml_codegen_switch_case entry = {.arm = it, .value = literal_value};
        utarray_push_back(cases, &entry);
      } else {
        valid = false;
        break;
      }
      index++;
    }

    if (valid && utarray_len(cases) > 0) {
      use_switch = true;
      LLVMBasicBlockRef default_bb = NULL;
      if (default_arm) {
        default_bb = LLVMAppendBasicBlockInContext(codegen->context, codegen->current_function,
                                                   "match.default");
        has_fallthrough = false;
      } else {
        default_bb = LLVMAppendBasicBlockInContext(codegen->context, codegen->current_function,
                                                   "match.default");
      }

      LLVMValueRef switch_inst =
          LLVMBuildSwitch(codegen->builder, scrutinee.value, default_bb, (unsigned)utarray_len(cases));

      for (reml_codegen_switch_case *it = (reml_codegen_switch_case *)utarray_front(cases);
           it != NULL; it = (reml_codegen_switch_case *)utarray_next(cases, it)) {
        LLVMBasicBlockRef arm_bb =
            LLVMAppendBasicBlockInContext(codegen->context, codegen->current_function, "match.arm");
        LLVMAddCase(switch_inst, it->value, arm_bb);

        LLVMPositionBuilderAtEnd(codegen->builder, arm_bb);
        reml_codegen_scope_stack_push(scopes);
        reml_codegen_bind_pattern(codegen, scopes, it->arm->pattern, scrutinee.value,
                                  scrutinee.type);
        reml_codegen_value body = reml_codegen_emit_expr(codegen, scopes, it->arm->body);
        reml_codegen_scope_stack_pop(scopes);

        if (!body.terminated) {
          LLVMBuildBr(codegen->builder, merge_bb);
          any_non_terminated = true;
          if (!is_unit) {
            if (!body.value) {
              reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, it->arm->body->span,
                                  "missing match arm value");
            } else if (incoming) {
              reml_codegen_phi_incoming entry = {.value = body.value, .block = arm_bb};
              utarray_push_back(incoming, &entry);
            }
          }
        }
      }

      LLVMPositionBuilderAtEnd(codegen->builder, default_bb);
      if (default_arm) {
        reml_codegen_scope_stack_push(scopes);
        reml_codegen_bind_pattern(codegen, scopes, default_arm->pattern, scrutinee.value,
                                  scrutinee.type);
        reml_codegen_value body = reml_codegen_emit_expr(codegen, scopes, default_arm->body);
        reml_codegen_scope_stack_pop(scopes);

        if (!body.terminated) {
          LLVMBuildBr(codegen->builder, merge_bb);
          any_non_terminated = true;
          if (!is_unit) {
            if (!body.value) {
              reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, default_arm->body->span,
                                  "missing match arm value");
            } else if (incoming) {
              reml_codegen_phi_incoming entry = {.value = body.value, .block = default_bb};
              utarray_push_back(incoming, &entry);
            }
          }
        }
      } else {
        bool exhaustive = reml_type_is_bool(scrutinee.type) && utarray_len(cases) == 2;
        if (!exhaustive) {
          reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, expr->span,
                              "non-exhaustive match expression");
        } else {
          has_fallthrough = false;
        }
        if (!is_unit) {
          LLVMValueRef undef = LLVMGetUndef(phi_type);
          LLVMBuildBr(codegen->builder, merge_bb);
          if (incoming) {
            reml_codegen_phi_incoming entry = {.value = undef, .block = default_bb};
            utarray_push_back(incoming, &entry);
          }
          any_non_terminated = true;
        } else {
          LLVMBuildBr(codegen->builder, merge_bb);
        }
      }
    }

    if (seen_values) {
      utarray_free(seen_values);
    }
    if (cases) {
      utarray_free(cases);
    }
  }

  if (!use_switch) {
    if (expr->data.match_expr.arms) {
      for (reml_match_arm *it = (reml_match_arm *)utarray_front(expr->data.match_expr.arms);
           it != NULL && current_bb != NULL; it = (reml_match_arm *)utarray_next(
                                                   expr->data.match_expr.arms, it)) {
        bool catch_all = reml_pattern_is_catch_all(it->pattern);
        LLVMBasicBlockRef arm_bb =
            LLVMAppendBasicBlockInContext(codegen->context, codegen->current_function, "match.arm");
        LLVMBasicBlockRef next_bb = NULL;

        if (catch_all) {
          LLVMBuildBr(codegen->builder, arm_bb);
          has_fallthrough = false;
        } else {
          next_bb = LLVMAppendBasicBlockInContext(codegen->context, codegen->current_function,
                                                  "match.next");
          LLVMValueRef cond = reml_codegen_emit_pattern_check(codegen, it->pattern,
                                                              scrutinee.value, scrutinee.type);
          if (!cond) {
            cond = LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 0, 0);
          }
          LLVMBuildCondBr(codegen->builder, cond, arm_bb, next_bb);
        }

        LLVMPositionBuilderAtEnd(codegen->builder, arm_bb);
        reml_codegen_scope_stack_push(scopes);
        reml_codegen_bind_pattern(codegen, scopes, it->pattern, scrutinee.value, scrutinee.type);
        reml_codegen_value body = reml_codegen_emit_expr(codegen, scopes, it->body);
        reml_codegen_scope_stack_pop(scopes);

        if (!body.terminated) {
          LLVMBuildBr(codegen->builder, merge_bb);
          any_non_terminated = true;
          if (!is_unit) {
            if (!body.value) {
              reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, it->body->span,
                                  "missing match arm value");
            } else if (incoming) {
              reml_codegen_phi_incoming entry = {.value = body.value, .block = arm_bb};
              utarray_push_back(incoming, &entry);
            }
          }
        }

        if (catch_all) {
          current_bb = NULL;
          break;
        }

        LLVMPositionBuilderAtEnd(codegen->builder, next_bb);
        current_bb = next_bb;
      }
    }

    if (current_bb != NULL) {
      if (has_fallthrough) {
        reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, expr->span,
                            "non-exhaustive match expression");
      }
      if (!is_unit) {
        LLVMValueRef undef = LLVMGetUndef(phi_type);
        LLVMBuildBr(codegen->builder, merge_bb);
        if (incoming) {
          reml_codegen_phi_incoming entry = {.value = undef, .block = current_bb};
          utarray_push_back(incoming, &entry);
        }
        any_non_terminated = true;
      } else {
        LLVMBuildBr(codegen->builder, merge_bb);
      }
    }
  }

  LLVMPositionBuilderAtEnd(codegen->builder, merge_bb);

  if (!any_non_terminated) {
    if (incoming) {
      utarray_free(incoming);
    }
    return reml_codegen_make_value(NULL, expr->type, true);
  }

  if (is_unit) {
    if (incoming) {
      utarray_free(incoming);
    }
    return reml_codegen_make_value(NULL, expr->type, false);
  }

  LLVMValueRef phi = LLVMBuildPhi(codegen->builder, phi_type, "match.result");
  if (incoming) {
    for (reml_codegen_phi_incoming *it = (reml_codegen_phi_incoming *)utarray_front(incoming);
         it != NULL; it = (reml_codegen_phi_incoming *)utarray_next(incoming, it)) {
      LLVMAddIncoming(phi, &it->value, &it->block, 1);
    }
    utarray_free(incoming);
  }

  return reml_codegen_make_value(phi, expr->type, false);
}

static reml_codegen_value reml_codegen_emit_expr(reml_codegen *codegen,
                                                 reml_codegen_scope_stack *scopes,
                                                 reml_expr *expr) {
  if (!expr) {
    return reml_codegen_make_value(NULL, NULL, false);
  }
  switch (expr->kind) {
    case REML_EXPR_LITERAL:
      return reml_codegen_emit_literal(codegen, expr);
    case REML_EXPR_IDENT:
      return reml_codegen_emit_ident(codegen, scopes, expr);
    case REML_EXPR_UNARY:
      return reml_codegen_emit_unary(codegen, scopes, expr);
    case REML_EXPR_BINARY:
      return reml_codegen_emit_binary(codegen, scopes, expr);
    case REML_EXPR_BLOCK:
      return reml_codegen_emit_block(codegen, scopes, &expr->data.block, expr->type);
    case REML_EXPR_IF:
      return reml_codegen_emit_if(codegen, scopes, expr);
    case REML_EXPR_WHILE:
      return reml_codegen_emit_while(codegen, scopes, expr);
    case REML_EXPR_MATCH:
      return reml_codegen_emit_match(codegen, scopes, expr);
    default:
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, expr->span,
                          "unknown expression kind in codegen");
      return reml_codegen_make_value(NULL, expr->type, false);
  }
}

bool reml_codegen_init(reml_codegen *codegen, const char *module_name) {
  if (!codegen) {
    return false;
  }
  memset(codegen, 0, sizeof(*codegen));
  reml_diagnostics_init(&codegen->diagnostics);

  if (LLVMInitializeNativeTarget() != 0 || LLVMInitializeNativeAsmParser() != 0 ||
      LLVMInitializeNativeAsmPrinter() != 0) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_LLVM_FAILURE, reml_span_make(0, 0, 0, 0, 0, 0),
                        "failed to initialize LLVM native target");
    return false;
  }

  codegen->context = LLVMContextCreate();
  if (!codegen->context) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, reml_span_make(0, 0, 0, 0, 0, 0),
                        "failed to create LLVM context");
    return false;
  }

  codegen->module =
      LLVMModuleCreateWithNameInContext(module_name ? module_name : "reml", codegen->context);
  codegen->builder = LLVMCreateBuilderInContext(codegen->context);
  codegen->alloca_builder = LLVMCreateBuilderInContext(codegen->context);

  codegen->target_triple = LLVMGetDefaultTargetTriple();
  if (!codegen->target_triple) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_LLVM_FAILURE, reml_span_make(0, 0, 0, 0, 0, 0),
                        "failed to get default target triple");
    return false;
  }
  LLVMSetTarget(codegen->module, codegen->target_triple);

  LLVMTargetRef target = NULL;
  char *target_error = NULL;
  if (LLVMGetTargetFromTriple(codegen->target_triple, &target, &target_error) != 0) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_LLVM_FAILURE, reml_span_make(0, 0, 0, 0, 0, 0),
                        target_error ? target_error : "failed to get LLVM target");
    if (target_error) {
      LLVMDisposeMessage(target_error);
    }
    return false;
  }

  codegen->target_machine =
      LLVMCreateTargetMachine(target, codegen->target_triple, "", "", LLVMCodeGenLevelDefault,
                              LLVMRelocDefault, LLVMCodeModelDefault);
  if (!codegen->target_machine) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_LLVM_FAILURE, reml_span_make(0, 0, 0, 0, 0, 0),
                        "failed to create LLVM target machine");
    return false;
  }

  codegen->target_data = LLVMCreateTargetDataLayout(codegen->target_machine);
  if (codegen->target_data) {
    char *layout = LLVMCopyStringRepOfTargetData(codegen->target_data);
    LLVMSetDataLayout(codegen->module, layout);
    LLVMDisposeMessage(layout);
  }

  return true;
}

void reml_codegen_deinit(reml_codegen *codegen) {
  if (!codegen) {
    return;
  }
  if (codegen->builder) {
    LLVMDisposeBuilder(codegen->builder);
  }
  if (codegen->alloca_builder) {
    LLVMDisposeBuilder(codegen->alloca_builder);
  }
  if (codegen->module) {
    LLVMDisposeModule(codegen->module);
  }
  if (codegen->target_machine) {
    LLVMDisposeTargetMachine(codegen->target_machine);
  }
  if (codegen->target_data) {
    LLVMDisposeTargetData(codegen->target_data);
  }
  if (codegen->context) {
    LLVMContextDispose(codegen->context);
  }
  if (codegen->target_triple) {
    LLVMDisposeMessage(codegen->target_triple);
  }
  reml_diagnostics_deinit(&codegen->diagnostics);
  memset(codegen, 0, sizeof(*codegen));
}

bool reml_codegen_generate(reml_codegen *codegen, reml_compilation_unit *unit) {
  if (!codegen || !unit || !codegen->module) {
    return false;
  }

  LLVMTypeRef i64 = LLVMInt64TypeInContext(codegen->context);
  LLVMTypeRef fn_type = LLVMFunctionType(i64, NULL, 0, 0);
  codegen->current_function = LLVMAddFunction(codegen->module, "reml_main", fn_type);
  LLVMBasicBlockRef entry =
      LLVMAppendBasicBlockInContext(codegen->context, codegen->current_function, "entry");
  LLVMPositionBuilderAtEnd(codegen->builder, entry);

  reml_codegen_scope_stack scopes;
  reml_codegen_scope_stack_init(&scopes);
  reml_codegen_scope_stack_push(&scopes);

  bool terminated = false;
  if (unit->statements) {
    for (reml_stmt **it = (reml_stmt **)utarray_front(unit->statements); it != NULL;
         it = (reml_stmt **)utarray_next(unit->statements, it)) {
      if (reml_codegen_emit_statement(codegen, &scopes, *it)) {
        terminated = true;
        break;
      }
    }
  }

  if (!terminated) {
    LLVMValueRef default_ret = LLVMConstInt(i64, 0, 1);
    LLVMBuildRet(codegen->builder, default_ret);
  }

  reml_codegen_scope_stack_deinit(&scopes);

  LLVMTypeRef i32 = LLVMInt32TypeInContext(codegen->context);
  LLVMTypeRef i8 = LLVMInt8TypeInContext(codegen->context);
  LLVMTypeRef i8_ptr = LLVMPointerType(i8, 0);
  LLVMTypeRef params[] = {i32, LLVMPointerType(i8_ptr, 0)};
  LLVMTypeRef main_type = LLVMFunctionType(i32, params, 2, 0);
  LLVMValueRef main_fn = LLVMAddFunction(codegen->module, "main", main_type);
  LLVMBasicBlockRef main_entry =
      LLVMAppendBasicBlockInContext(codegen->context, main_fn, "entry");
  LLVMPositionBuilderAtEnd(codegen->builder, main_entry);
  LLVMValueRef main_call = LLVMBuildCall2(codegen->builder, fn_type, codegen->current_function,
                                          NULL, 0, "call");
  LLVMValueRef main_ret = LLVMBuildTrunc(codegen->builder, main_call, i32, "ret");
  LLVMBuildRet(codegen->builder, main_ret);

  char *verify_error = NULL;
  if (LLVMVerifyModule(codegen->module, LLVMReturnStatusAction, &verify_error) != 0) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_LLVM_FAILURE, reml_span_make(0, 0, 0, 0, 0, 0),
                        verify_error ? verify_error : "LLVM module verification failed");
    if (verify_error) {
      LLVMDisposeMessage(verify_error);
    }
    return false;
  }

  return reml_diagnostics_count(&codegen->diagnostics) == 0;
}

bool reml_codegen_emit_ir(reml_codegen *codegen, const char *path) {
  if (!codegen || !codegen->module || !path) {
    return false;
  }
  char *error = NULL;
  if (LLVMPrintModuleToFile(codegen->module, path, &error) != 0) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_LLVM_FAILURE, reml_span_make(0, 0, 0, 0, 0, 0),
                        error ? error : "failed to emit LLVM IR");
    if (error) {
      LLVMDisposeMessage(error);
    }
    return false;
  }
  return true;
}

bool reml_codegen_emit_object(reml_codegen *codegen, const char *path) {
  if (!codegen || !codegen->target_machine || !codegen->module || !path) {
    return false;
  }
  char *error = NULL;
  if (LLVMTargetMachineEmitToFile(codegen->target_machine, codegen->module, (char *)path,
                                  LLVMObjectFile, &error) != 0) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_LLVM_FAILURE, reml_span_make(0, 0, 0, 0, 0, 0),
                        error ? error : "failed to emit object file");
    if (error) {
      LLVMDisposeMessage(error);
    }
    return false;
  }
  return true;
}

const reml_diagnostic_list *reml_codegen_diagnostics(const reml_codegen *codegen) {
  if (!codegen) {
    return NULL;
  }
  return &codegen->diagnostics;
}
