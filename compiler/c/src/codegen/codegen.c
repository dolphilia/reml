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
  reml_symbol_id id;
  LLVMValueRef alloca;
  reml_type *type;
} reml_codegen_drop;

typedef struct {
  UT_array *bindings;
  UT_array *drops;
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
  size_t index;
  bool requires_check;
} reml_codegen_switch_case;

static bool reml_type_is_int(reml_type *type);
static bool reml_type_is_bigint(reml_type *type);
static bool reml_type_is_float(reml_type *type);
static bool reml_type_is_bool(reml_type *type);
static bool reml_type_is_unit(reml_type *type);
static bool reml_type_is_enum(reml_type *type);
static bool reml_type_is_tuple(reml_type *type);
static bool reml_type_is_record(reml_type *type);
static bool reml_string_view_equal(reml_string_view left, reml_string_view right);
static reml_enum_variant *reml_codegen_enum_variant(reml_type *type, reml_string_view name);
static bool reml_record_field_index(reml_type *type, reml_string_view name, size_t *out_index,
                                    reml_type **out_type);
static LLVMTypeRef reml_codegen_tuple_struct_type(reml_codegen *codegen, reml_type *type);
static LLVMTypeRef reml_codegen_record_struct_type(reml_codegen *codegen, reml_type *type);

static void reml_codegen_report(reml_codegen *codegen, reml_diagnostic_code code, reml_span span,
                                const char *message) {
  if (!codegen) {
    return;
  }
  reml_diagnostic diag = {.code = code, .span = span, .message = message, .pattern = NULL};
  reml_diagnostics_push(&codegen->diagnostics, diag);
}

static reml_codegen_scope *reml_codegen_scope_new(void) {
  reml_codegen_scope *scope = (reml_codegen_scope *)calloc(1, sizeof(reml_codegen_scope));
  if (!scope) {
    return NULL;
  }
  UT_icd binding_icd = {sizeof(reml_codegen_binding), NULL, NULL, NULL};
  utarray_new(scope->bindings, &binding_icd);
  UT_icd drop_icd = {sizeof(reml_codegen_drop), NULL, NULL, NULL};
  utarray_new(scope->drops, &drop_icd);
  return scope;
}

static void reml_codegen_scope_free(reml_codegen_scope *scope) {
  if (!scope) {
    return;
  }
  if (scope->bindings) {
    utarray_free(scope->bindings);
  }
  if (scope->drops) {
    utarray_free(scope->drops);
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

static void reml_codegen_scope_register_drop(reml_codegen_scope_stack *stack, reml_symbol_id id,
                                             LLVMValueRef alloca, reml_type *type) {
  if (!stack || id == REML_SYMBOL_ID_INVALID || !alloca || !type) {
    return;
  }
  reml_codegen_scope *scope = reml_codegen_scope_stack_current(stack);
  if (!scope || !scope->drops) {
    return;
  }
  reml_codegen_drop drop = {.id = id, .alloca = alloca, .type = type};
  utarray_push_back(scope->drops, &drop);
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
    case REML_TYPE_ENUM:
      return LLVMPointerType(codegen->enum_repr_type, 0);
    case REML_TYPE_TUPLE:
    case REML_TYPE_RECORD:
      return LLVMPointerType(LLVMInt8TypeInContext(codegen->context), 0);
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

static reml_symbol_id reml_codegen_expr_skip_id(const reml_expr *expr) {
  if (!expr || expr->kind != REML_EXPR_IDENT) {
    return REML_SYMBOL_ID_INVALID;
  }
  return expr->symbol_id;
}

static void reml_codegen_emit_enum_free(reml_codegen *codegen, LLVMValueRef value) {
  if (!codegen || !value) {
    return;
  }
  LLVMTypeRef enum_ptr = LLVMPointerType(codegen->enum_repr_type, 0);
  LLVMTypeRef params[1] = {enum_ptr};
  LLVMTypeRef fn_type = LLVMFunctionType(LLVMVoidTypeInContext(codegen->context), params, 1, 0);
  LLVMValueRef fn = reml_codegen_get_runtime_fn(codegen, "reml_enum_free", fn_type);
  LLVMValueRef args[1] = {value};
  LLVMBuildCall2(codegen->builder, fn_type, fn, args, 1, "");
}

static void reml_codegen_scope_emit_drops(reml_codegen *codegen, reml_codegen_scope *scope,
                                          reml_symbol_id skip_id) {
  if (!codegen || !scope || !scope->drops) {
    return;
  }
  for (reml_codegen_drop *it = (reml_codegen_drop *)utarray_front(scope->drops); it != NULL;
       it = (reml_codegen_drop *)utarray_next(scope->drops, it)) {
    if (it->id == skip_id) {
      continue;
    }
    if (!reml_type_is_enum(it->type)) {
      continue;
    }
    LLVMValueRef enum_value =
        LLVMBuildLoad2(codegen->builder, LLVMPointerType(codegen->enum_repr_type, 0),
                       it->alloca, "enum.load");
    reml_codegen_emit_enum_free(codegen, enum_value);
  }
}

static void reml_codegen_scope_emit_all_drops(reml_codegen *codegen,
                                              reml_codegen_scope_stack *stack,
                                              reml_symbol_id skip_id) {
  if (!codegen || !stack || !stack->scopes) {
    return;
  }
  for (reml_codegen_scope **it = (reml_codegen_scope **)utarray_back(stack->scopes); it != NULL;
       it = (reml_codegen_scope **)utarray_prev(stack->scopes, it)) {
    reml_codegen_scope_emit_drops(codegen, *it, skip_id);
  }
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
  if (pattern->kind == REML_PATTERN_WILDCARD || pattern->kind == REML_PATTERN_IDENT) {
    return true;
  }
  if (pattern->kind == REML_PATTERN_TUPLE) {
    if (!pattern->data.items) {
      return true;
    }
    for (reml_pattern **it = (reml_pattern **)utarray_front(pattern->data.items); it != NULL;
         it = (reml_pattern **)utarray_next(pattern->data.items, it)) {
      if (!reml_pattern_is_catch_all(*it)) {
        return false;
      }
    }
    return true;
  }
  if (pattern->kind == REML_PATTERN_RECORD) {
    if (!pattern->data.fields) {
      return true;
    }
    for (reml_pattern_field *it =
             (reml_pattern_field *)utarray_front(pattern->data.fields);
         it != NULL;
         it = (reml_pattern_field *)utarray_next(pattern->data.fields, it)) {
      if (!reml_pattern_is_catch_all(it->pattern)) {
        return false;
      }
    }
    return true;
  }
  return false;
}

static bool reml_pattern_ctor_payload_covers_all(const reml_pattern *pattern) {
  if (!pattern || pattern->kind != REML_PATTERN_CONSTRUCTOR) {
    return false;
  }
  if (!pattern->data.ctor.items) {
    return true;
  }
  for (reml_pattern **it = (reml_pattern **)utarray_front(pattern->data.ctor.items); it != NULL;
       it = (reml_pattern **)utarray_next(pattern->data.ctor.items, it)) {
    if (!reml_pattern_is_catch_all(*it)) {
      return false;
    }
  }
  return true;
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
      reml_codegen_scope_emit_all_drops(codegen, scopes,
                                        reml_codegen_expr_skip_id(stmt->data.expr));
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
      if (reml_type_is_enum(value.type)) {
        reml_codegen_scope_register_drop(scopes, pattern->symbol_id, alloca, value.type);
      }
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
    reml_codegen_scope_emit_drops(codegen, reml_codegen_scope_stack_current(scopes),
                                  reml_codegen_expr_skip_id(block->tail));
    reml_codegen_scope_stack_pop(scopes);
    return value;
  }

  reml_codegen_scope_emit_drops(codegen, reml_codegen_scope_stack_current(scopes),
                                REML_SYMBOL_ID_INVALID);
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
  if (pattern->kind == REML_PATTERN_WILDCARD || pattern->kind == REML_PATTERN_IDENT) {
    return LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 1, 0);
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
  if (pattern->kind == REML_PATTERN_RANGE) {
    if (!reml_type_is_int(type)) {
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, pattern->span,
                          "range pattern expects integer scrutinee");
      return LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 0, 0);
    }
    int64_t start_value = 0;
    int64_t end_value = 0;
    reml_numeric_parse_result start_status =
        reml_parse_int_literal(pattern->data.range.start, &start_value);
    reml_numeric_parse_result end_status =
        reml_parse_int_literal(pattern->data.range.end, &end_value);
    if (start_status != REML_NUMERIC_OK || end_status != REML_NUMERIC_OK) {
      reml_codegen_report(codegen, REML_DIAG_NUMERIC_INVALID, pattern->span,
                          "invalid range bound literal");
      return LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 0, 0);
    }
    LLVMValueRef start_const = LLVMConstInt(LLVMInt64TypeInContext(codegen->context),
                                            (unsigned long long)start_value, 1);
    LLVMValueRef end_const = LLVMConstInt(LLVMInt64TypeInContext(codegen->context),
                                          (unsigned long long)end_value, 1);
    LLVMValueRef ge =
        LLVMBuildICmp(codegen->builder, LLVMIntSGE, scrutinee, start_const, "range.ge");
    LLVMIntPredicate end_pred = pattern->data.range.inclusive ? LLVMIntSLE : LLVMIntSLT;
    LLVMValueRef le =
        LLVMBuildICmp(codegen->builder, end_pred, scrutinee, end_const, "range.le");
    return LLVMBuildAnd(codegen->builder, ge, le, "range.and");
  }
  if (pattern->kind == REML_PATTERN_TUPLE) {
    if (!reml_type_is_tuple(type)) {
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, pattern->span,
                          "tuple pattern expects tuple scrutinee");
      return LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 0, 0);
    }
    reml_type *tuple_type = reml_type_prune(type);
    size_t field_count =
        pattern->data.items ? utarray_len(pattern->data.items) : 0;
    size_t tuple_count =
        tuple_type->data.tuple.items ? utarray_len(tuple_type->data.tuple.items) : 0;
    if (field_count != tuple_count) {
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, pattern->span,
                          "tuple pattern arity mismatch");
      return LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 0, 0);
    }
    LLVMTypeRef tuple_struct = reml_codegen_tuple_struct_type(codegen, tuple_type);
    if (!tuple_struct) {
      return LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 0, 0);
    }
    LLVMValueRef tuple_typed =
        LLVMBuildBitCast(codegen->builder, scrutinee, LLVMPointerType(tuple_struct, 0),
                         "tuple.cast");
    LLVMValueRef all_match = LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 1, 0);
    size_t index = 0;
    for (reml_pattern **it = (reml_pattern **)utarray_front(pattern->data.items); it != NULL;
         it = (reml_pattern **)utarray_next(pattern->data.items, it)) {
      reml_type **field_type = (reml_type **)utarray_eltptr(tuple_type->data.tuple.items, index);
      LLVMTypeRef field_llvm =
          reml_codegen_lower_type(codegen, field_type ? *field_type : NULL);
      if (!field_llvm) {
        field_llvm = LLVMInt64TypeInContext(codegen->context);
      }
      LLVMValueRef field_ptr =
          LLVMBuildStructGEP2(codegen->builder, tuple_struct, tuple_typed, (unsigned)index,
                              "tuple.field.ptr");
      LLVMValueRef field_value =
          LLVMBuildLoad2(codegen->builder, field_llvm, field_ptr, "tuple.field");
      LLVMValueRef field_match =
          reml_codegen_emit_pattern_check(codegen, *it, field_value,
                                          field_type ? *field_type : NULL);
      if (!field_match) {
        field_match = LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 0, 0);
      }
      all_match = LLVMBuildAnd(codegen->builder, all_match, field_match, "match.and");
      index++;
    }
    return all_match;
  }
  if (pattern->kind == REML_PATTERN_RECORD) {
    if (!reml_type_is_record(type)) {
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, pattern->span,
                          "record pattern expects record scrutinee");
      return LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 0, 0);
    }
    reml_type *record_type = reml_type_prune(type);
    size_t field_count =
        pattern->data.fields ? utarray_len(pattern->data.fields) : 0;
    size_t record_count =
        record_type->data.record.fields ? utarray_len(record_type->data.record.fields) : 0;
    if (field_count != record_count) {
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, pattern->span,
                          "record pattern field mismatch");
      return LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 0, 0);
    }
    LLVMTypeRef record_struct = reml_codegen_record_struct_type(codegen, record_type);
    if (!record_struct) {
      return LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 0, 0);
    }
    LLVMValueRef record_typed =
        LLVMBuildBitCast(codegen->builder, scrutinee, LLVMPointerType(record_struct, 0),
                         "record.cast");
    LLVMValueRef all_match = LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 1, 0);
    for (reml_pattern_field *it =
             (reml_pattern_field *)utarray_front(pattern->data.fields);
         it != NULL;
         it = (reml_pattern_field *)utarray_next(pattern->data.fields, it)) {
      size_t field_index = 0;
      reml_type *field_type = NULL;
      if (!reml_record_field_index(record_type, it->name, &field_index, &field_type)) {
        reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, pattern->span,
                            "record field missing in type");
        return LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 0, 0);
      }
      LLVMTypeRef field_llvm = reml_codegen_lower_type(codegen, field_type);
      if (!field_llvm) {
        field_llvm = LLVMInt64TypeInContext(codegen->context);
      }
      LLVMValueRef field_ptr =
          LLVMBuildStructGEP2(codegen->builder, record_struct, record_typed,
                              (unsigned)field_index, "record.field.ptr");
      LLVMValueRef field_value =
          LLVMBuildLoad2(codegen->builder, field_llvm, field_ptr, "record.field");
      LLVMValueRef field_match =
          reml_codegen_emit_pattern_check(codegen, it->pattern, field_value, field_type);
      if (!field_match) {
        field_match = LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 0, 0);
      }
      all_match = LLVMBuildAnd(codegen->builder, all_match, field_match, "match.and");
    }
    return all_match;
  }
  if (pattern->kind == REML_PATTERN_CONSTRUCTOR) {
    if (!reml_type_is_enum(type)) {
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, pattern->span,
                          "constructor pattern expects enum scrutinee");
      return LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 0, 0);
    }
    reml_type *enum_type = reml_type_prune(type);
    reml_enum_variant *variant = reml_codegen_enum_variant(enum_type, pattern->data.ctor.name);
    if (!variant) {
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, pattern->span,
                          "unknown enum constructor");
      return LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 0, 0);
    }
    LLVMValueRef tag_ptr =
        LLVMBuildStructGEP2(codegen->builder, codegen->enum_repr_type, scrutinee, 0,
                            "enum.tag.ptr");
    LLVMValueRef tag_value =
        LLVMBuildLoad2(codegen->builder, LLVMInt32TypeInContext(codegen->context), tag_ptr,
                       "enum.tag");
    LLVMValueRef tag_const =
        LLVMConstInt(LLVMInt32TypeInContext(codegen->context),
                     (unsigned long long)variant->tag, 1);
    LLVMValueRef tag_match =
        LLVMBuildICmp(codegen->builder, LLVMIntEQ, tag_value, tag_const, "match.tag");

    size_t field_count =
        pattern->data.ctor.items ? utarray_len(pattern->data.ctor.items) : 0;
    if (field_count == 0) {
      return tag_match;
    }

    if (!variant || !variant->fields || utarray_len(variant->fields) != field_count) {
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, pattern->span,
                          "missing enum variant payload information");
      return tag_match;
    }

    LLVMValueRef payload_ptr =
        LLVMBuildStructGEP2(codegen->builder, codegen->enum_repr_type, scrutinee, 1,
                            "enum.payload.ptr");
    LLVMValueRef payload_raw =
        LLVMBuildLoad2(codegen->builder, LLVMPointerType(LLVMInt8TypeInContext(codegen->context), 0),
                       payload_ptr, "enum.payload");

    size_t field_total = utarray_len(variant->fields);
    LLVMTypeRef *field_types = (LLVMTypeRef *)calloc(field_total, sizeof(LLVMTypeRef));
    if (!field_types) {
      return tag_match;
    }
    for (size_t i = 0; i < field_total; ++i) {
      reml_type **field_type = (reml_type **)utarray_eltptr(variant->fields, i);
      field_types[i] = reml_codegen_lower_type(codegen, field_type ? *field_type : NULL);
      if (!field_types[i]) {
        field_types[i] = LLVMInt64TypeInContext(codegen->context);
      }
    }
    LLVMTypeRef payload_struct =
        LLVMStructTypeInContext(codegen->context, field_types, (unsigned)field_total, 0);
    free(field_types);

    LLVMValueRef payload_typed =
        LLVMBuildBitCast(codegen->builder, payload_raw, LLVMPointerType(payload_struct, 0),
                         "enum.payload.cast");

    LLVMValueRef all_match = tag_match;
    size_t index = 0;
    for (reml_pattern **it = (reml_pattern **)utarray_front(pattern->data.ctor.items);
         it != NULL;
         it = (reml_pattern **)utarray_next(pattern->data.ctor.items, it)) {
      reml_type **field_type = (reml_type **)utarray_eltptr(variant->fields, index);
      LLVMTypeRef field_llvm =
          reml_codegen_lower_type(codegen, field_type ? *field_type : NULL);
      if (!field_llvm) {
        field_llvm = LLVMInt64TypeInContext(codegen->context);
      }
      LLVMValueRef field_ptr =
          LLVMBuildStructGEP2(codegen->builder, payload_struct, payload_typed, (unsigned)index,
                              "enum.field.ptr");
      LLVMValueRef field_value =
          LLVMBuildLoad2(codegen->builder, field_llvm, field_ptr, "enum.field");
      reml_type *field_reml = field_type ? *field_type : NULL;
      LLVMValueRef field_match =
          reml_codegen_emit_pattern_check(codegen, *it, field_value, field_reml);
      if (!field_match) {
        field_match = LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 0, 0);
      }
      all_match = LLVMBuildAnd(codegen->builder, all_match, field_match, "match.and");
      index++;
    }
    return all_match;
  }

  reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, pattern->span,
                      "unsupported pattern in codegen");
  return LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 0, 0);
}

static void reml_codegen_bind_pattern_value(reml_codegen *codegen, reml_codegen_scope_stack *scopes,
                                            reml_pattern *pattern, LLVMValueRef value,
                                            reml_type *type);

static void reml_codegen_bind_pattern(reml_codegen *codegen, reml_codegen_scope_stack *scopes,
                                      reml_pattern *pattern, LLVMValueRef scrutinee,
                                      reml_type *type) {
  if (!pattern) {
    return;
  }
  if (pattern->kind == REML_PATTERN_IDENT) {
    reml_codegen_bind_pattern_value(codegen, scopes, pattern, scrutinee, type);
    return;
  }
  if (pattern->kind == REML_PATTERN_TUPLE) {
    if (!reml_type_is_tuple(type)) {
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, pattern->span,
                          "tuple binding expects tuple type");
      return;
    }
    reml_type *tuple_type = reml_type_prune(type);
    size_t field_count =
        pattern->data.items ? utarray_len(pattern->data.items) : 0;
    size_t tuple_count =
        tuple_type->data.tuple.items ? utarray_len(tuple_type->data.tuple.items) : 0;
    if (field_count != tuple_count) {
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, pattern->span,
                          "tuple binding arity mismatch");
      return;
    }
    LLVMTypeRef tuple_struct = reml_codegen_tuple_struct_type(codegen, tuple_type);
    if (!tuple_struct) {
      return;
    }
    LLVMValueRef tuple_typed =
        LLVMBuildBitCast(codegen->builder, scrutinee, LLVMPointerType(tuple_struct, 0),
                         "tuple.bind.cast");
    size_t index = 0;
    for (reml_pattern **it = (reml_pattern **)utarray_front(pattern->data.items); it != NULL;
         it = (reml_pattern **)utarray_next(pattern->data.items, it)) {
      reml_type **field_type = (reml_type **)utarray_eltptr(tuple_type->data.tuple.items, index);
      LLVMTypeRef field_llvm =
          reml_codegen_lower_type(codegen, field_type ? *field_type : NULL);
      if (!field_llvm) {
        field_llvm = LLVMInt64TypeInContext(codegen->context);
      }
      LLVMValueRef field_ptr =
          LLVMBuildStructGEP2(codegen->builder, tuple_struct, tuple_typed, (unsigned)index,
                              "tuple.bind.ptr");
      LLVMValueRef field_value =
          LLVMBuildLoad2(codegen->builder, field_llvm, field_ptr, "tuple.bind");
      reml_codegen_bind_pattern_value(codegen, scopes, *it, field_value,
                                      field_type ? *field_type : NULL);
      index++;
    }
    return;
  }
  if (pattern->kind == REML_PATTERN_RECORD) {
    if (!reml_type_is_record(type)) {
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, pattern->span,
                          "record binding expects record type");
      return;
    }
    reml_type *record_type = reml_type_prune(type);
    size_t field_count =
        pattern->data.fields ? utarray_len(pattern->data.fields) : 0;
    size_t record_count =
        record_type->data.record.fields ? utarray_len(record_type->data.record.fields) : 0;
    if (field_count != record_count) {
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, pattern->span,
                          "record binding field mismatch");
      return;
    }
    LLVMTypeRef record_struct = reml_codegen_record_struct_type(codegen, record_type);
    if (!record_struct) {
      return;
    }
    LLVMValueRef record_typed =
        LLVMBuildBitCast(codegen->builder, scrutinee, LLVMPointerType(record_struct, 0),
                         "record.bind.cast");
    for (reml_pattern_field *it =
             (reml_pattern_field *)utarray_front(pattern->data.fields);
         it != NULL;
         it = (reml_pattern_field *)utarray_next(pattern->data.fields, it)) {
      size_t field_index = 0;
      reml_type *field_type = NULL;
      if (!reml_record_field_index(record_type, it->name, &field_index, &field_type)) {
        reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, pattern->span,
                            "record field missing in type");
        return;
      }
      LLVMTypeRef field_llvm = reml_codegen_lower_type(codegen, field_type);
      if (!field_llvm) {
        field_llvm = LLVMInt64TypeInContext(codegen->context);
      }
      LLVMValueRef field_ptr =
          LLVMBuildStructGEP2(codegen->builder, record_struct, record_typed,
                              (unsigned)field_index, "record.bind.ptr");
      LLVMValueRef field_value =
          LLVMBuildLoad2(codegen->builder, field_llvm, field_ptr, "record.bind");
      reml_codegen_bind_pattern_value(codegen, scopes, it->pattern, field_value, field_type);
    }
    return;
  }
  if (pattern->kind == REML_PATTERN_CONSTRUCTOR) {
    size_t field_count =
        pattern->data.ctor.items ? utarray_len(pattern->data.ctor.items) : 0;
    if (field_count == 0) {
      return;
    }
    reml_type *enum_type = reml_type_prune(type);
    if (!enum_type || enum_type->kind != REML_TYPE_ENUM) {
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, pattern->span,
                          "constructor binding expects enum type");
      return;
    }
    reml_enum_variant *variant = reml_codegen_enum_variant(enum_type, pattern->data.ctor.name);
    if (!variant || !variant->fields || utarray_len(variant->fields) != field_count) {
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, pattern->span,
                          "missing enum payload information");
      return;
    }
    LLVMValueRef payload_ptr =
        LLVMBuildStructGEP2(codegen->builder, codegen->enum_repr_type, scrutinee, 1,
                            "enum.payload.ptr");
    LLVMValueRef payload_raw =
        LLVMBuildLoad2(codegen->builder, LLVMPointerType(LLVMInt8TypeInContext(codegen->context), 0),
                       payload_ptr, "enum.payload");

    size_t field_total = utarray_len(variant->fields);
    LLVMTypeRef *field_types = (LLVMTypeRef *)calloc(field_total, sizeof(LLVMTypeRef));
    if (!field_types) {
      return;
    }
    for (size_t i = 0; i < field_total; ++i) {
      reml_type **field_type = (reml_type **)utarray_eltptr(variant->fields, i);
      field_types[i] = reml_codegen_lower_type(codegen, field_type ? *field_type : NULL);
      if (!field_types[i]) {
        field_types[i] = LLVMInt64TypeInContext(codegen->context);
      }
    }
    LLVMTypeRef payload_struct =
        LLVMStructTypeInContext(codegen->context, field_types, (unsigned)field_total, 0);
    free(field_types);

    LLVMValueRef payload_typed =
        LLVMBuildBitCast(codegen->builder, payload_raw, LLVMPointerType(payload_struct, 0),
                         "enum.payload.cast");

    size_t index = 0;
    for (reml_pattern **it = (reml_pattern **)utarray_front(pattern->data.ctor.items);
         it != NULL;
         it = (reml_pattern **)utarray_next(pattern->data.ctor.items, it)) {
      reml_type **field_type = (reml_type **)utarray_eltptr(variant->fields, index);
      LLVMTypeRef field_llvm =
          reml_codegen_lower_type(codegen, field_type ? *field_type : NULL);
      if (!field_llvm) {
        field_llvm = LLVMInt64TypeInContext(codegen->context);
      }
      LLVMValueRef field_ptr =
          LLVMBuildStructGEP2(codegen->builder, payload_struct, payload_typed, (unsigned)index,
                              "enum.field.ptr");
      LLVMValueRef field_value =
          LLVMBuildLoad2(codegen->builder, field_llvm, field_ptr, "enum.field");
      reml_codegen_bind_pattern_value(codegen, scopes, *it, field_value,
                                      field_type ? *field_type : NULL);
      index++;
    }
    return;
  }
  if (pattern->kind == REML_PATTERN_RANGE) {
    return;
  }
}

static void reml_codegen_bind_pattern_value(reml_codegen *codegen, reml_codegen_scope_stack *scopes,
                                            reml_pattern *pattern, LLVMValueRef value,
                                            reml_type *type) {
  if (!pattern) {
    return;
  }
  switch (pattern->kind) {
    case REML_PATTERN_IDENT: {
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
      LLVMBuildStore(codegen->builder, value, alloca);
      reml_codegen_scope_define(scopes, pattern->symbol_id, alloca, llvm_type);
      return;
    }
    case REML_PATTERN_CONSTRUCTOR:
      reml_codegen_bind_pattern(codegen, scopes, pattern, value, type);
      return;
    case REML_PATTERN_TUPLE:
    case REML_PATTERN_RECORD:
      reml_codegen_bind_pattern(codegen, scopes, pattern, value, type);
      return;
    default:
      return;
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

static bool reml_type_is_enum(reml_type *type) {
  type = type ? reml_type_prune(type) : NULL;
  return type && type->kind == REML_TYPE_ENUM;
}

static bool reml_type_is_tuple(reml_type *type) {
  type = type ? reml_type_prune(type) : NULL;
  return type && type->kind == REML_TYPE_TUPLE;
}

static bool reml_type_is_record(reml_type *type) {
  type = type ? reml_type_prune(type) : NULL;
  return type && type->kind == REML_TYPE_RECORD;
}

static bool reml_string_view_equal(reml_string_view left, reml_string_view right) {
  if (left.length != right.length) {
    return false;
  }
  if (left.length == 0) {
    return true;
  }
  return memcmp(left.data, right.data, left.length) == 0;
}

static reml_enum_variant *reml_codegen_enum_variant(reml_type *type, reml_string_view name) {
  type = type ? reml_type_prune(type) : NULL;
  if (!type || type->kind != REML_TYPE_ENUM || !type->data.enum_type.variants) {
    return NULL;
  }
  for (reml_enum_variant *it =
           (reml_enum_variant *)utarray_front(type->data.enum_type.variants);
       it != NULL;
       it = (reml_enum_variant *)utarray_next(type->data.enum_type.variants, it)) {
    if (reml_string_view_equal(it->name, name)) {
      return it;
    }
  }
  return NULL;
}

static reml_record_expr_field *reml_record_expr_field_find(UT_array *fields,
                                                           reml_string_view name) {
  if (!fields) {
    return NULL;
  }
  for (reml_record_expr_field *it =
           (reml_record_expr_field *)utarray_front(fields);
       it != NULL;
       it = (reml_record_expr_field *)utarray_next(fields, it)) {
    if (reml_string_view_equal(it->name, name)) {
      return it;
    }
  }
  return NULL;
}

static bool reml_record_field_index(reml_type *type, reml_string_view name, size_t *out_index,
                                    reml_type **out_type) {
  type = type ? reml_type_prune(type) : NULL;
  if (!type || type->kind != REML_TYPE_RECORD || !type->data.record.fields) {
    return false;
  }
  size_t index = 0;
  for (reml_record_field *it =
           (reml_record_field *)utarray_front(type->data.record.fields);
       it != NULL;
       it = (reml_record_field *)utarray_next(type->data.record.fields, it)) {
    if (reml_string_view_equal(it->name, name)) {
      if (out_index) {
        *out_index = index;
      }
      if (out_type) {
        *out_type = it->type;
      }
      return true;
    }
    index++;
  }
  return false;
}

static LLVMTypeRef reml_codegen_tuple_struct_type(reml_codegen *codegen, reml_type *type) {
  if (!codegen || !type) {
    return NULL;
  }
  type = reml_type_prune(type);
  if (!type || type->kind != REML_TYPE_TUPLE) {
    return NULL;
  }
  size_t count = type->data.tuple.items ? utarray_len(type->data.tuple.items) : 0;
  LLVMTypeRef *field_types = NULL;
  if (count > 0) {
    field_types = (LLVMTypeRef *)calloc(count, sizeof(LLVMTypeRef));
    if (!field_types) {
      return NULL;
    }
    for (size_t i = 0; i < count; ++i) {
      reml_type **item_type = (reml_type **)utarray_eltptr(type->data.tuple.items, i);
      field_types[i] = reml_codegen_lower_type(codegen, item_type ? *item_type : NULL);
      if (!field_types[i]) {
        field_types[i] = LLVMInt64TypeInContext(codegen->context);
      }
    }
  }
  LLVMTypeRef tuple_struct =
      LLVMStructTypeInContext(codegen->context, field_types, (unsigned)count, 0);
  free(field_types);
  return tuple_struct;
}

static LLVMTypeRef reml_codegen_record_struct_type(reml_codegen *codegen, reml_type *type) {
  if (!codegen || !type) {
    return NULL;
  }
  type = reml_type_prune(type);
  if (!type || type->kind != REML_TYPE_RECORD) {
    return NULL;
  }
  size_t count = type->data.record.fields ? utarray_len(type->data.record.fields) : 0;
  LLVMTypeRef *field_types = NULL;
  if (count > 0) {
    field_types = (LLVMTypeRef *)calloc(count, sizeof(LLVMTypeRef));
    if (!field_types) {
      return NULL;
    }
    size_t index = 0;
    for (reml_record_field *it =
             (reml_record_field *)utarray_front(type->data.record.fields);
         it != NULL;
         it = (reml_record_field *)utarray_next(type->data.record.fields, it)) {
      field_types[index] = reml_codegen_lower_type(codegen, it->type);
      if (!field_types[index]) {
        field_types[index] = LLVMInt64TypeInContext(codegen->context);
      }
      index++;
    }
  }
  LLVMTypeRef record_struct =
      LLVMStructTypeInContext(codegen->context, field_types, (unsigned)count, 0);
  free(field_types);
  return record_struct;
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

static reml_codegen_value reml_codegen_emit_constructor(reml_codegen *codegen,
                                                        reml_codegen_scope_stack *scopes,
                                                        reml_expr *expr) {
  reml_type *enum_type = reml_type_prune(expr->type);
  if (!enum_type || enum_type->kind != REML_TYPE_ENUM) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, expr->span,
                        "constructor expects enum type");
    return reml_codegen_make_value(NULL, expr->type, false);
  }
  reml_enum_variant *variant = reml_codegen_enum_variant(enum_type, expr->data.ctor.name);
  if (!variant) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, expr->span,
                        "unknown enum constructor");
    return reml_codegen_make_value(NULL, expr->type, false);
  }

  size_t arg_count = expr->data.ctor.args ? utarray_len(expr->data.ctor.args) : 0;
  size_t field_count = variant->fields ? utarray_len(variant->fields) : 0;
  if (arg_count != field_count) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, expr->span,
                        "constructor payload arity mismatch");
    return reml_codegen_make_value(NULL, expr->type, false);
  }

  LLVMValueRef *arg_values = NULL;
  if (arg_count > 0) {
    arg_values = (LLVMValueRef *)calloc(arg_count, sizeof(LLVMValueRef));
    if (!arg_values) {
      return reml_codegen_make_value(NULL, expr->type, false);
    }
    size_t index = 0;
    for (reml_expr **it = (reml_expr **)utarray_front(expr->data.ctor.args); it != NULL;
         it = (reml_expr **)utarray_next(expr->data.ctor.args, it)) {
      reml_codegen_value arg = reml_codegen_emit_expr(codegen, scopes, *it);
      if (arg.terminated || !arg.value) {
        free(arg_values);
        return reml_codegen_make_value(NULL, expr->type, arg.terminated);
      }
      arg_values[index++] = arg.value;
    }
  }

  LLVMTypeRef enum_ptr = LLVMPointerType(codegen->enum_repr_type, 0);
  LLVMTypeRef params[2] = {LLVMInt32TypeInContext(codegen->context),
                           LLVMInt64TypeInContext(codegen->context)};
  LLVMTypeRef fn_type = LLVMFunctionType(enum_ptr, params, 2, 0);
  LLVMValueRef fn = reml_codegen_get_runtime_fn(codegen, "reml_enum_make", fn_type);

  LLVMTypeRef payload_struct = NULL;
  LLVMValueRef payload_size_value =
      LLVMConstInt(LLVMInt64TypeInContext(codegen->context), 0, 0);
  if (field_count > 0) {
    LLVMTypeRef *field_types = (LLVMTypeRef *)calloc(field_count, sizeof(LLVMTypeRef));
    if (!field_types) {
      free(arg_values);
      return reml_codegen_make_value(NULL, expr->type, false);
    }
    for (size_t i = 0; i < field_count; ++i) {
      reml_type **field_type = (reml_type **)utarray_eltptr(variant->fields, i);
      field_types[i] = reml_codegen_lower_type(codegen, field_type ? *field_type : NULL);
      if (!field_types[i]) {
        field_types[i] = LLVMInt64TypeInContext(codegen->context);
      }
    }
    payload_struct =
        LLVMStructTypeInContext(codegen->context, field_types, (unsigned)field_count, 0);
    free(field_types);
    unsigned long long payload_size =
        LLVMABISizeOfType(codegen->target_data, payload_struct);
    payload_size_value = LLVMConstInt(LLVMInt64TypeInContext(codegen->context), payload_size, 0);
  }

  LLVMValueRef args[2] = {
      LLVMConstInt(LLVMInt32TypeInContext(codegen->context),
                   (unsigned long long)variant->tag, 1),
      payload_size_value};
  LLVMValueRef enum_value = LLVMBuildCall2(codegen->builder, fn_type, fn, args, 2, "enum.new");

  if (field_count > 0 && payload_struct) {
    LLVMValueRef payload_ptr =
        LLVMBuildStructGEP2(codegen->builder, codegen->enum_repr_type, enum_value, 1,
                            "enum.payload.ptr");
    LLVMValueRef payload_raw =
        LLVMBuildLoad2(codegen->builder, LLVMPointerType(LLVMInt8TypeInContext(codegen->context), 0),
                       payload_ptr, "enum.payload");
    LLVMValueRef payload_typed =
        LLVMBuildBitCast(codegen->builder, payload_raw, LLVMPointerType(payload_struct, 0),
                         "enum.payload.cast");

    for (size_t i = 0; i < field_count; ++i) {
      LLVMValueRef field_ptr =
          LLVMBuildStructGEP2(codegen->builder, payload_struct, payload_typed, (unsigned)i,
                              "enum.field.ptr");
      LLVMBuildStore(codegen->builder, arg_values[i], field_ptr);
    }
  }

  free(arg_values);
  return reml_codegen_make_value(enum_value, expr->type, false);
}

static reml_codegen_value reml_codegen_emit_tuple(reml_codegen *codegen,
                                                  reml_codegen_scope_stack *scopes,
                                                  reml_expr *expr) {
  reml_type *tuple_type = reml_type_prune(expr->type);
  if (!tuple_type || tuple_type->kind != REML_TYPE_TUPLE) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, expr->span,
                        "tuple expression expects tuple type");
    return reml_codegen_make_value(NULL, expr->type, false);
  }
  LLVMTypeRef tuple_struct = reml_codegen_tuple_struct_type(codegen, tuple_type);
  if (!tuple_struct) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, expr->span,
                        "failed to lower tuple type");
    return reml_codegen_make_value(NULL, expr->type, false);
  }
  LLVMValueRef alloca =
      reml_codegen_create_entry_alloca(codegen, tuple_struct, "tuple.tmp");
  if (!alloca) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, expr->span,
                        "failed to allocate tuple");
    return reml_codegen_make_value(NULL, expr->type, false);
  }
  size_t index = 0;
  if (expr->data.tuple) {
    for (reml_expr **it = (reml_expr **)utarray_front(expr->data.tuple); it != NULL;
         it = (reml_expr **)utarray_next(expr->data.tuple, it)) {
      reml_codegen_value value = reml_codegen_emit_expr(codegen, scopes, *it);
      if (value.terminated) {
        return value;
      }
      reml_type **field_type =
          (reml_type **)utarray_eltptr(tuple_type->data.tuple.items, index);
      LLVMTypeRef field_llvm =
          reml_codegen_lower_type(codegen, field_type ? *field_type : NULL);
      if (!field_llvm) {
        field_llvm = LLVMInt64TypeInContext(codegen->context);
      }
      LLVMValueRef field_ptr =
          LLVMBuildStructGEP2(codegen->builder, tuple_struct, alloca, (unsigned)index,
                              "tuple.field.ptr");
      LLVMBuildStore(codegen->builder, value.value, field_ptr);
      index++;
    }
  }
  LLVMValueRef cast =
      LLVMBuildBitCast(codegen->builder, alloca,
                       LLVMPointerType(LLVMInt8TypeInContext(codegen->context), 0),
                       "tuple.cast");
  return reml_codegen_make_value(cast, expr->type, false);
}

static reml_codegen_value reml_codegen_emit_record(reml_codegen *codegen,
                                                   reml_codegen_scope_stack *scopes,
                                                   reml_expr *expr) {
  reml_type *record_type = reml_type_prune(expr->type);
  if (!record_type || record_type->kind != REML_TYPE_RECORD) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_UNSUPPORTED, expr->span,
                        "record expression expects record type");
    return reml_codegen_make_value(NULL, expr->type, false);
  }
  LLVMTypeRef record_struct = reml_codegen_record_struct_type(codegen, record_type);
  if (!record_struct) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, expr->span,
                        "failed to lower record type");
    return reml_codegen_make_value(NULL, expr->type, false);
  }
  LLVMValueRef alloca =
      reml_codegen_create_entry_alloca(codegen, record_struct, "record.tmp");
  if (!alloca) {
    reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, expr->span,
                        "failed to allocate record");
    return reml_codegen_make_value(NULL, expr->type, false);
  }
  size_t index = 0;
  for (reml_record_field *it =
           (reml_record_field *)utarray_front(record_type->data.record.fields);
       it != NULL;
       it = (reml_record_field *)utarray_next(record_type->data.record.fields, it)) {
    reml_record_expr_field *expr_field = reml_record_expr_field_find(expr->data.record, it->name);
    if (!expr_field) {
      reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, expr->span,
                          "record field missing in expression");
      return reml_codegen_make_value(NULL, expr->type, false);
    }
    reml_codegen_value value = reml_codegen_emit_expr(codegen, scopes, expr_field->value);
    if (value.terminated) {
      return value;
    }
    LLVMTypeRef field_llvm = reml_codegen_lower_type(codegen, it->type);
    if (!field_llvm) {
      field_llvm = LLVMInt64TypeInContext(codegen->context);
    }
    LLVMValueRef field_ptr =
        LLVMBuildStructGEP2(codegen->builder, record_struct, alloca, (unsigned)index,
                            "record.field.ptr");
    LLVMBuildStore(codegen->builder, value.value, field_ptr);
    index++;
  }
  LLVMValueRef cast =
      LLVMBuildBitCast(codegen->builder, alloca,
                       LLVMPointerType(LLVMInt8TypeInContext(codegen->context), 0),
                       "record.cast");
  return reml_codegen_make_value(cast, expr->type, false);
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

static LLVMBasicBlockRef reml_codegen_emit_match_chain(
    reml_codegen *codegen, reml_codegen_scope_stack *scopes, UT_array *arms, size_t start_index,
    reml_codegen_value scrutinee, reml_expr *expr, LLVMBasicBlockRef merge_bb, bool is_unit,
    bool report_non_exhaustive, bool emit_fallthrough, bool *has_fallthrough,
    bool *any_non_terminated, UT_array *incoming, LLVMTypeRef phi_type) {
  if (!arms) {
    return LLVMGetInsertBlock(codegen->builder);
  }
  size_t arm_count = utarray_len(arms);
  LLVMBasicBlockRef current_bb = LLVMGetInsertBlock(codegen->builder);
  for (size_t index = start_index; index < arm_count && current_bb != NULL; ++index) {
    reml_match_arm *arm = (reml_match_arm *)utarray_eltptr(arms, index);
    if (!arm) {
      continue;
    }
    bool catch_all = reml_pattern_is_catch_all(arm->pattern);
    bool has_guard = arm->guard != NULL;
    LLVMBasicBlockRef arm_bb =
        LLVMAppendBasicBlockInContext(codegen->context, codegen->current_function, "match.arm");
    LLVMBasicBlockRef next_bb = NULL;

    if (catch_all) {
      if (has_guard) {
        next_bb = LLVMAppendBasicBlockInContext(codegen->context, codegen->current_function,
                                                "match.next");
      }
      LLVMBuildBr(codegen->builder, arm_bb);
      if (!has_guard && has_fallthrough) {
        *has_fallthrough = false;
      }
    } else {
      next_bb = LLVMAppendBasicBlockInContext(codegen->context, codegen->current_function,
                                              "match.next");
      LLVMValueRef cond =
          reml_codegen_emit_pattern_check(codegen, arm->pattern, scrutinee.value, scrutinee.type);
      if (!cond) {
        cond = LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 0, 0);
      }
      LLVMBuildCondBr(codegen->builder, cond, arm_bb, next_bb);
    }

    LLVMPositionBuilderAtEnd(codegen->builder, arm_bb);
    reml_codegen_scope_stack_push(scopes);
    reml_codegen_bind_pattern(codegen, scopes, arm->pattern, scrutinee.value, scrutinee.type);

    reml_codegen_value body = {0};
    LLVMBasicBlockRef body_bb = arm_bb;
    if (has_guard) {
      reml_codegen_value guard = reml_codegen_emit_expr(codegen, scopes, arm->guard);
      if (guard.terminated || !guard.value) {
        reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, arm->guard->span,
                            "missing match guard value");
        LLVMBuildBr(codegen->builder, next_bb);
        reml_codegen_scope_emit_drops(codegen, reml_codegen_scope_stack_current(scopes),
                                      reml_codegen_expr_skip_id(arm->body));
        reml_codegen_scope_stack_pop(scopes);
        LLVMPositionBuilderAtEnd(codegen->builder, next_bb);
        current_bb = next_bb;
        continue;
      }
      LLVMBasicBlockRef guard_bb = LLVMAppendBasicBlockInContext(
          codegen->context, codegen->current_function, "match.guard");
      LLVMBuildCondBr(codegen->builder, guard.value, guard_bb, next_bb);
      LLVMPositionBuilderAtEnd(codegen->builder, guard_bb);
      body_bb = guard_bb;
      body = reml_codegen_emit_expr(codegen, scopes, arm->body);
    } else {
      body = reml_codegen_emit_expr(codegen, scopes, arm->body);
    }
    reml_codegen_scope_stack_pop(scopes);

    if (!body.terminated) {
      LLVMBuildBr(codegen->builder, merge_bb);
      if (any_non_terminated) {
        *any_non_terminated = true;
      }
      if (!is_unit) {
        if (!body.value) {
          reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, arm->body->span,
                              "missing match arm value");
        } else if (incoming) {
          reml_codegen_phi_incoming entry = {.value = body.value, .block = body_bb};
          utarray_push_back(incoming, &entry);
        }
      }
    }

    if (catch_all && !has_guard) {
      current_bb = NULL;
      break;
    }

    LLVMPositionBuilderAtEnd(codegen->builder, next_bb);
    current_bb = next_bb;
  }

  if (current_bb != NULL && emit_fallthrough) {
    if (report_non_exhaustive && has_fallthrough && *has_fallthrough) {
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
      if (any_non_terminated) {
        *any_non_terminated = true;
      }
    } else {
      LLVMBuildBr(codegen->builder, merge_bb);
    }
  }

  return current_bb;
}

static LLVMBasicBlockRef reml_codegen_match_get_fallback(
    reml_codegen *codegen, reml_codegen_scope_stack *scopes, UT_array *arms, size_t start_index,
    reml_codegen_value scrutinee, reml_expr *expr, LLVMBasicBlockRef merge_bb, bool is_unit,
    UT_array *incoming, LLVMTypeRef phi_type, UT_array *fallback_blocks,
    bool *any_non_terminated) {
  if (!fallback_blocks) {
    return NULL;
  }
  LLVMBasicBlockRef *entry =
      (LLVMBasicBlockRef *)utarray_eltptr(fallback_blocks, (unsigned)start_index);
  if (entry && *entry) {
    return *entry;
  }
  LLVMBasicBlockRef fallback_bb = LLVMAppendBasicBlockInContext(
      codegen->context, codegen->current_function, "match.fallback");
  if (entry) {
    *entry = fallback_bb;
  }
  LLVMBasicBlockRef resume_bb = LLVMGetInsertBlock(codegen->builder);
  LLVMPositionBuilderAtEnd(codegen->builder, fallback_bb);
  bool fallback_fallthrough = true;
  reml_codegen_emit_match_chain(codegen, scopes, arms, start_index, scrutinee, expr, merge_bb,
                                is_unit, false, true, &fallback_fallthrough, any_non_terminated,
                                incoming, phi_type);
  LLVMPositionBuilderAtEnd(codegen->builder, resume_bb);
  return fallback_bb;
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
                                     reml_type_is_int(scrutinee.type) ||
                                     reml_type_is_enum(scrutinee.type))) {
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
        reml_codegen_switch_case entry = {.arm = it,
                                          .value = literal_value,
                                          .index = index,
                                          .requires_check = false};
        utarray_push_back(cases, &entry);
      } else if (it->pattern && it->pattern->kind == REML_PATTERN_RANGE &&
                 reml_type_is_int(scrutinee.type)) {
        int64_t start_value = 0;
        int64_t end_value = 0;
        reml_numeric_parse_result start_status =
            reml_parse_int_literal(it->pattern->data.range.start, &start_value);
        reml_numeric_parse_result end_status =
            reml_parse_int_literal(it->pattern->data.range.end, &end_value);
        if (start_status != REML_NUMERIC_OK || end_status != REML_NUMERIC_OK) {
          valid = false;
          break;
        }
        bool inclusive = it->pattern->data.range.inclusive;
        int64_t last_value = inclusive ? end_value : end_value - 1;
        int64_t count = last_value - start_value + 1;
        if (count <= 0 || count > 8) {
          valid = false;
          break;
        }
        for (int64_t value = start_value; value <= last_value; ++value) {
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
          LLVMValueRef literal_value =
              LLVMConstInt(LLVMInt64TypeInContext(codegen->context),
                           (unsigned long long)value, 1);
          reml_codegen_switch_case entry = {.arm = it,
                                            .value = literal_value,
                                            .index = index,
                                            .requires_check = false};
          utarray_push_back(cases, &entry);
        }
        if (!valid) {
          break;
        }
      } else if (it->pattern && it->pattern->kind == REML_PATTERN_CONSTRUCTOR &&
                 reml_type_is_enum(scrutinee.type)) {
        size_t field_count =
            it->pattern->data.ctor.items ? utarray_len(it->pattern->data.ctor.items) : 0;
        (void)field_count;
        reml_type *enum_type = reml_type_prune(scrutinee.type);
        reml_enum_variant *variant = reml_codegen_enum_variant(
            enum_type, it->pattern->data.ctor.name);
        if (!variant) {
          valid = false;
          break;
        }
        bool payload_full = reml_pattern_ctor_payload_covers_all(it->pattern);
        int64_t value = variant->tag;
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
        LLVMValueRef literal_value =
            LLVMConstInt(LLVMInt32TypeInContext(codegen->context),
                         (unsigned long long)value, 1);
        reml_codegen_switch_case entry = {.arm = it,
                                          .value = literal_value,
                                          .index = index,
                                          .requires_check = !payload_full};
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

      UT_array *fallback_blocks = NULL;
      UT_icd fallback_icd = {sizeof(LLVMBasicBlockRef), NULL, NULL, NULL};
      utarray_new(fallback_blocks, &fallback_icd);
      for (size_t i = 0; i <= arm_count; ++i) {
        LLVMBasicBlockRef none = NULL;
        utarray_push_back(fallback_blocks, &none);
      }

      LLVMValueRef switch_value = scrutinee.value;
      if (reml_type_is_enum(scrutinee.type)) {
        LLVMValueRef tag_ptr =
            LLVMBuildStructGEP2(codegen->builder, codegen->enum_repr_type, scrutinee.value, 0,
                                "enum.tag.ptr");
        switch_value =
            LLVMBuildLoad2(codegen->builder, LLVMInt32TypeInContext(codegen->context), tag_ptr,
                           "enum.tag");
      }
      LLVMValueRef switch_inst =
          LLVMBuildSwitch(codegen->builder, switch_value, default_bb, (unsigned)utarray_len(cases));

      for (reml_codegen_switch_case *it = (reml_codegen_switch_case *)utarray_front(cases);
           it != NULL; it = (reml_codegen_switch_case *)utarray_next(cases, it)) {
        LLVMBasicBlockRef arm_bb =
            LLVMAppendBasicBlockInContext(codegen->context, codegen->current_function, "match.arm");
        LLVMAddCase(switch_inst, it->value, arm_bb);

        LLVMPositionBuilderAtEnd(codegen->builder, arm_bb);
        LLVMBasicBlockRef case_bb = arm_bb;
        LLVMBasicBlockRef case_fallback_bb = NULL;
        if (it->requires_check) {
          LLVMValueRef cond =
              reml_codegen_emit_pattern_check(codegen, it->arm->pattern, scrutinee.value,
                                              scrutinee.type);
          if (!cond) {
            cond = LLVMConstInt(LLVMInt1TypeInContext(codegen->context), 0, 0);
          }
          case_fallback_bb = reml_codegen_match_get_fallback(
              codegen, scopes, expr->data.match_expr.arms, it->index + 1, scrutinee, expr,
              merge_bb, is_unit, incoming, phi_type, fallback_blocks, &any_non_terminated);
          LLVMBasicBlockRef payload_bb = LLVMAppendBasicBlockInContext(
              codegen->context, codegen->current_function, "match.payload");
          LLVMBuildCondBr(codegen->builder, cond, payload_bb, case_fallback_bb);
          LLVMPositionBuilderAtEnd(codegen->builder, payload_bb);
          case_bb = payload_bb;
        }
        reml_codegen_scope_stack_push(scopes);
        reml_codegen_bind_pattern(codegen, scopes, it->arm->pattern, scrutinee.value,
                                  scrutinee.type);
        reml_codegen_value body = {0};
        LLVMBasicBlockRef body_bb = case_bb;
        if (it->arm->guard) {
          reml_codegen_value guard = reml_codegen_emit_expr(codegen, scopes, it->arm->guard);
          if (guard.terminated || !guard.value) {
            reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, it->arm->guard->span,
                                "missing match guard value");
            reml_codegen_scope_emit_drops(codegen, reml_codegen_scope_stack_current(scopes),
                                          reml_codegen_expr_skip_id(it->arm->body));
            reml_codegen_scope_stack_pop(scopes);
          } else {
            LLVMBasicBlockRef guard_bb = LLVMAppendBasicBlockInContext(
                codegen->context, codegen->current_function, "match.guard");
            if (!case_fallback_bb) {
              case_fallback_bb = reml_codegen_match_get_fallback(
                  codegen, scopes, expr->data.match_expr.arms, it->index + 1, scrutinee, expr,
                  merge_bb, is_unit, incoming, phi_type, fallback_blocks, &any_non_terminated);
            }
            LLVMBuildCondBr(codegen->builder, guard.value, guard_bb, case_fallback_bb);
            LLVMPositionBuilderAtEnd(codegen->builder, guard_bb);
            body_bb = guard_bb;
            body = reml_codegen_emit_expr(codegen, scopes, it->arm->body);
            reml_codegen_scope_emit_drops(codegen, reml_codegen_scope_stack_current(scopes),
                                          reml_codegen_expr_skip_id(it->arm->body));
            reml_codegen_scope_stack_pop(scopes);
          }
        } else {
          body = reml_codegen_emit_expr(codegen, scopes, it->arm->body);
          reml_codegen_scope_emit_drops(codegen, reml_codegen_scope_stack_current(scopes),
                                        reml_codegen_expr_skip_id(it->arm->body));
          reml_codegen_scope_stack_pop(scopes);
        }

        if (!body.terminated) {
          LLVMBuildBr(codegen->builder, merge_bb);
          any_non_terminated = true;
          if (!is_unit) {
            if (!body.value) {
              reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, it->arm->body->span,
                                  "missing match arm value");
            } else if (incoming) {
              reml_codegen_phi_incoming entry = {.value = body.value, .block = body_bb};
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
        reml_codegen_value body = {0};
        LLVMBasicBlockRef body_bb = default_bb;
        if (default_arm->guard) {
          reml_codegen_value guard = reml_codegen_emit_expr(codegen, scopes, default_arm->guard);
          if (guard.terminated || !guard.value) {
            reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, default_arm->guard->span,
                                "missing match guard value");
            reml_codegen_scope_emit_drops(codegen, reml_codegen_scope_stack_current(scopes),
                                          reml_codegen_expr_skip_id(default_arm->body));
            reml_codegen_scope_stack_pop(scopes);
          } else {
            LLVMBasicBlockRef guard_bb = LLVMAppendBasicBlockInContext(
                codegen->context, codegen->current_function, "match.guard");
            LLVMBasicBlockRef fallback_bb = reml_codegen_match_get_fallback(
                codegen, scopes, expr->data.match_expr.arms, arm_count, scrutinee, expr, merge_bb,
                is_unit, incoming, phi_type, fallback_blocks, &any_non_terminated);
            LLVMBuildCondBr(codegen->builder, guard.value, guard_bb, fallback_bb);
            LLVMPositionBuilderAtEnd(codegen->builder, guard_bb);
            body_bb = guard_bb;
            body = reml_codegen_emit_expr(codegen, scopes, default_arm->body);
            reml_codegen_scope_emit_drops(codegen, reml_codegen_scope_stack_current(scopes),
                                          reml_codegen_expr_skip_id(default_arm->body));
            reml_codegen_scope_stack_pop(scopes);
          }
        } else {
          body = reml_codegen_emit_expr(codegen, scopes, default_arm->body);
          reml_codegen_scope_emit_drops(codegen, reml_codegen_scope_stack_current(scopes),
                                        reml_codegen_expr_skip_id(default_arm->body));
          reml_codegen_scope_stack_pop(scopes);
        }

        if (!body.terminated) {
          LLVMBuildBr(codegen->builder, merge_bb);
          any_non_terminated = true;
          if (!is_unit) {
            if (!body.value) {
              reml_codegen_report(codegen, REML_DIAG_CODEGEN_INTERNAL, default_arm->body->span,
                                  "missing match arm value");
            } else if (incoming) {
              reml_codegen_phi_incoming entry = {.value = body.value, .block = body_bb};
              utarray_push_back(incoming, &entry);
            }
          }
        }
      } else {
        bool exhaustive = reml_type_is_bool(scrutinee.type) && utarray_len(cases) == 2;
        if (!exhaustive && reml_type_is_enum(scrutinee.type)) {
          reml_type *enum_type = reml_type_prune(scrutinee.type);
          size_t variant_count =
              enum_type && enum_type->data.enum_type.variants
                  ? utarray_len(enum_type->data.enum_type.variants)
                  : 0;
          exhaustive = variant_count > 0 && utarray_len(cases) == variant_count;
        }
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

      if (fallback_blocks) {
        utarray_free(fallback_blocks);
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
    current_bb = reml_codegen_emit_match_chain(codegen, scopes, expr->data.match_expr.arms, 0,
                                               scrutinee, expr, merge_bb, is_unit, true, true,
                                               &has_fallthrough, &any_non_terminated, incoming,
                                               phi_type);
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
    case REML_EXPR_CONSTRUCTOR:
      return reml_codegen_emit_constructor(codegen, scopes, expr);
    case REML_EXPR_TUPLE:
      return reml_codegen_emit_tuple(codegen, scopes, expr);
    case REML_EXPR_RECORD:
      return reml_codegen_emit_record(codegen, scopes, expr);
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
  codegen->enum_repr_type = LLVMStructTypeInContext(
      codegen->context,
      (LLVMTypeRef[]){LLVMInt32TypeInContext(codegen->context),
                      LLVMPointerType(LLVMInt8TypeInContext(codegen->context), 0)},
      2, 0);

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
    reml_codegen_scope_emit_all_drops(codegen, &scopes, REML_SYMBOL_ID_INVALID);
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
