#include "reml/sema/sema.h"

#include <stdlib.h>
#include <string.h>

#include <uthash.h>
#include <utarray.h>

typedef enum {
  REML_SYMBOL_FUNC,
  REML_SYMBOL_VAR,
  REML_SYMBOL_TYPE,
  REML_SYMBOL_MODULE
} reml_symbol_kind;

typedef struct {
  reml_type *type;
  UT_array *generics;
} reml_scheme;

typedef struct reml_symbol {
  reml_symbol_kind kind;
  reml_string_view name;
  reml_span span;
  reml_scheme scheme;
  bool is_builtin;
  bool is_predeclared;
  reml_symbol_id id;
  UT_hash_handle hh;
} reml_symbol;

typedef struct {
  reml_symbol *symbols;
} reml_scope;

struct reml_symbol_table {
  UT_array *scopes;
  reml_symbol_id next_id;
};

typedef uint8_t reml_effect_set;

enum {
  REML_EFFECT_NONE = 0,
  REML_EFFECT_MUT = 1 << 0,
  REML_EFFECT_IO = 1 << 1,
  REML_EFFECT_PANIC = 1 << 2
};

static void reml_scheme_init(reml_scheme *scheme, reml_type *type) {
  if (!scheme) {
    return;
  }
  scheme->type = type;
  UT_icd id_icd = {sizeof(uint32_t), NULL, NULL, NULL};
  utarray_new(scheme->generics, &id_icd);
}

static void reml_scheme_reset(reml_scheme *scheme, reml_type *type) {
  if (!scheme) {
    return;
  }
  if (scheme->generics) {
    utarray_clear(scheme->generics);
  }
  scheme->type = type;
}

static void reml_scheme_deinit(reml_scheme *scheme) {
  if (!scheme || !scheme->generics) {
    return;
  }
  utarray_free(scheme->generics);
  scheme->generics = NULL;
  scheme->type = NULL;
}

static reml_scope *reml_scope_new(void) {
  reml_scope *scope = (reml_scope *)calloc(1, sizeof(reml_scope));
  if (!scope) {
    return NULL;
  }
  scope->symbols = NULL;
  return scope;
}

static void reml_scope_free(reml_scope *scope) {
  if (!scope) {
    return;
  }
  reml_symbol *sym = NULL;
  reml_symbol *tmp = NULL;
  HASH_ITER(hh, scope->symbols, sym, tmp) {
    HASH_DEL(scope->symbols, sym);
    reml_scheme_deinit(&sym->scheme);
    free(sym);
  }
  free(scope);
}

static void reml_symbol_table_init(reml_symbol_table *table) {
  if (!table) {
    return;
  }
  UT_icd scope_icd = {sizeof(reml_scope *), NULL, NULL, NULL};
  utarray_new(table->scopes, &scope_icd);
  table->next_id = 1;
}

static void reml_symbol_table_deinit(reml_symbol_table *table) {
  if (!table || !table->scopes) {
    return;
  }
  for (reml_scope **it = (reml_scope **)utarray_front(table->scopes); it != NULL;
       it = (reml_scope **)utarray_next(table->scopes, it)) {
    reml_scope_free(*it);
  }
  utarray_free(table->scopes);
  table->scopes = NULL;
}

static reml_scope *reml_symbol_table_current(reml_symbol_table *table) {
  if (!table || !table->scopes || utarray_len(table->scopes) == 0) {
    return NULL;
  }
  return *(reml_scope **)utarray_back(table->scopes);
}

static void reml_symbol_table_enter(reml_symbol_table *table) {
  if (!table || !table->scopes) {
    return;
  }
  reml_scope *scope = reml_scope_new();
  utarray_push_back(table->scopes, &scope);
}

static void reml_symbol_table_exit(reml_symbol_table *table) {
  if (!table || !table->scopes || utarray_len(table->scopes) == 0) {
    return;
  }
  reml_scope **scope_ptr = (reml_scope **)utarray_back(table->scopes);
  reml_scope_free(*scope_ptr);
  utarray_pop_back(table->scopes);
}

static reml_symbol *reml_scope_lookup(reml_scope *scope, reml_string_view name) {
  if (!scope) {
    return NULL;
  }
  reml_symbol *symbol = NULL;
  HASH_FIND(hh, scope->symbols, name.data, name.length, symbol);
  return symbol;
}

static reml_symbol *reml_symbol_table_lookup(reml_symbol_table *table, reml_string_view name) {
  if (!table || !table->scopes) {
    return NULL;
  }
  for (reml_scope **it = (reml_scope **)utarray_back(table->scopes); it != NULL;
       it = (reml_scope **)utarray_prev(table->scopes, it)) {
    reml_symbol *symbol = reml_scope_lookup(*it, name);
    if (symbol) {
      return symbol;
    }
  }
  return NULL;
}

static bool reml_symbol_table_has_builtin(reml_symbol_table *table, reml_string_view name) {
  if (!table || !table->scopes) {
    return false;
  }
  for (reml_scope **it = (reml_scope **)utarray_back(table->scopes); it != NULL;
       it = (reml_scope **)utarray_prev(table->scopes, it)) {
    reml_symbol *symbol = reml_scope_lookup(*it, name);
    if (symbol && symbol->is_builtin) {
      return true;
    }
  }
  return false;
}

static reml_symbol *reml_symbol_table_define(reml_symbol_table *table, reml_symbol_kind kind,
                                             reml_string_view name, reml_span span,
                                             reml_type *type, bool is_builtin,
                                             bool is_predeclared) {
  if (!table) {
    return NULL;
  }
  reml_scope *scope = reml_symbol_table_current(table);
  if (!scope) {
    return NULL;
  }
  reml_symbol *existing = reml_scope_lookup(scope, name);
  if (existing) {
    return existing;
  }

  reml_symbol *symbol = (reml_symbol *)calloc(1, sizeof(reml_symbol));
  if (!symbol) {
    return NULL;
  }
  symbol->kind = kind;
  symbol->name = name;
  symbol->span = span;
  symbol->is_builtin = is_builtin;
  symbol->is_predeclared = is_predeclared;
  symbol->id = table->next_id++;
  reml_scheme_init(&symbol->scheme, type);
  HASH_ADD_KEYPTR(hh, scope->symbols, symbol->name.data, symbol->name.length, symbol);
  return symbol;
}

static bool reml_var_ids_contains(UT_array *vars, uint32_t id) {
  if (!vars) {
    return false;
  }
  for (uint32_t *it = (uint32_t *)utarray_front(vars); it != NULL;
       it = (uint32_t *)utarray_next(vars, it)) {
    if (*it == id) {
      return true;
    }
  }
  return false;
}

static void reml_var_ids_push_unique(UT_array *vars, uint32_t id) {
  if (!vars) {
    return;
  }
  if (reml_var_ids_contains(vars, id)) {
    return;
  }
  utarray_push_back(vars, &id);
}

static void reml_type_collect_vars(reml_type *type, UT_array *vars) {
  if (!type || !vars) {
    return;
  }
  type = reml_type_prune(type);
  if (type->kind == REML_TYPE_VAR) {
    reml_var_ids_push_unique(vars, type->data.var.id);
    return;
  }
  if (type->kind == REML_TYPE_TUPLE && type->data.tuple.items) {
    for (reml_type **it = (reml_type **)utarray_front(type->data.tuple.items); it != NULL;
         it = (reml_type **)utarray_next(type->data.tuple.items, it)) {
      reml_type_collect_vars(*it, vars);
    }
  }
  if (type->kind == REML_TYPE_FUNCTION) {
    if (type->data.function.params) {
      for (reml_type **it = (reml_type **)utarray_front(type->data.function.params); it != NULL;
           it = (reml_type **)utarray_next(type->data.function.params, it)) {
        reml_type_collect_vars(*it, vars);
      }
    }
    reml_type_collect_vars(type->data.function.result, vars);
  }
}

static void reml_scheme_collect_free_vars(const reml_scheme *scheme, UT_array *vars) {
  if (!scheme || !vars) {
    return;
  }
  UT_icd tmp_icd = {sizeof(uint32_t), NULL, NULL, NULL};
  UT_array *all_vars = NULL;
  utarray_new(all_vars, &tmp_icd);
  reml_type_collect_vars(scheme->type, all_vars);
  for (uint32_t *it = (uint32_t *)utarray_front(all_vars); it != NULL;
       it = (uint32_t *)utarray_next(all_vars, it)) {
    if (!reml_var_ids_contains(scheme->generics, *it)) {
      reml_var_ids_push_unique(vars, *it);
    }
  }
  utarray_free(all_vars);
}

static void reml_env_collect_free_vars(reml_symbol_table *table, const reml_symbol *skip,
                                       UT_array *vars) {
  if (!table || !table->scopes || !vars) {
    return;
  }
  for (reml_scope **it = (reml_scope **)utarray_front(table->scopes); it != NULL;
       it = (reml_scope **)utarray_next(table->scopes, it)) {
    for (reml_symbol *sym = (*it)->symbols; sym != NULL; sym = sym->hh.next) {
      if (sym == skip) {
        continue;
      }
      reml_scheme_collect_free_vars(&sym->scheme, vars);
    }
  }
}

typedef struct {
  uint32_t id;
  reml_type *replacement;
} reml_type_subst;

static reml_type *reml_type_instantiate_inner(reml_type_ctx *ctx, reml_type *type,
                                              UT_array *generics, UT_array *substs) {
  type = reml_type_prune(type);
  if (!type) {
    return NULL;
  }
  if (type->kind == REML_TYPE_VAR && reml_var_ids_contains(generics, type->data.var.id)) {
    for (reml_type_subst *it = (reml_type_subst *)utarray_front(substs); it != NULL;
         it = (reml_type_subst *)utarray_next(substs, it)) {
      if (it->id == type->data.var.id) {
        return it->replacement;
      }
    }
    reml_type *fresh = reml_type_make_var(ctx);
    reml_type_subst subst = {.id = type->data.var.id, .replacement = fresh};
    utarray_push_back(substs, &subst);
    return fresh;
  }
  return type;
}

static reml_type *reml_type_instantiate(reml_type_ctx *ctx, const reml_scheme *scheme) {
  if (!scheme || !scheme->type) {
    return NULL;
  }
  if (!scheme->generics || utarray_len(scheme->generics) == 0) {
    return scheme->type;
  }
  UT_icd subst_icd = {sizeof(reml_type_subst), NULL, NULL, NULL};
  UT_array *substs = NULL;
  utarray_new(substs, &subst_icd);
  reml_type *result = reml_type_instantiate_inner(ctx, scheme->type, scheme->generics, substs);
  utarray_free(substs);
  return result;
}

static void reml_report_diag(reml_sema *sema, reml_diagnostic_code code, reml_span span,
                             const char *message) {
  if (!sema) {
    return;
  }
  reml_diagnostic diag = {.code = code, .span = span, .message = message};
  reml_diagnostics_push(&sema->diagnostics, diag);
}

static bool reml_expect_type(reml_sema *sema, reml_type *actual, reml_type *expected,
                             reml_span span) {
  if (reml_type_unify(&sema->types, actual, expected)) {
    return true;
  }
  reml_report_diag(sema, REML_DIAG_TYPE_MISMATCH, span, "type mismatch");
  return false;
}

static reml_type *reml_infer_expr(reml_sema *sema, reml_expr *expr, reml_effect_set *effect);
static void reml_check_pattern(reml_sema *sema, reml_pattern *pattern, reml_type *expected,
                               reml_effect_set *effect, bool allow_define);

static reml_type *reml_infer_literal(reml_sema *sema, reml_literal literal) {
  switch (literal.kind) {
    case REML_LITERAL_INT:
      return reml_type_int(&sema->types);
    case REML_LITERAL_BIGINT:
      return reml_type_bigint(&sema->types);
    case REML_LITERAL_FLOAT:
      return reml_type_float(&sema->types);
    case REML_LITERAL_STRING:
      return reml_type_string(&sema->types);
    case REML_LITERAL_CHAR:
      return reml_type_char(&sema->types);
    case REML_LITERAL_BOOL:
      return reml_type_bool(&sema->types);
    default:
      return reml_type_error(&sema->types);
  }
}

static bool reml_is_numeric_type(reml_type *type, reml_type_ctx *ctx) {
  type = reml_type_prune(type);
  return type == reml_type_int(ctx) || type == reml_type_bigint(ctx) ||
         type == reml_type_float(ctx);
}

static reml_type *reml_infer_unary(reml_sema *sema, reml_expr *expr, reml_effect_set *effect) {
  reml_type *operand = reml_infer_expr(sema, expr->data.unary.operand, effect);
  if (!operand) {
    return reml_type_error(&sema->types);
  }
  switch (expr->data.unary.op) {
    case REML_TOKEN_MINUS:
      if (operand->kind == REML_TYPE_VAR) {
        reml_expect_type(sema, operand, reml_type_int(&sema->types), expr->span);
        return operand;
      }
      if (!reml_is_numeric_type(operand, &sema->types)) {
        reml_report_diag(sema, REML_DIAG_TYPE_MISMATCH, expr->span,
                         "unary '-' expects numeric type");
        return reml_type_error(&sema->types);
      }
      return operand;
    case REML_TOKEN_BANG:
      reml_expect_type(sema, operand, reml_type_bool(&sema->types), expr->span);
      return reml_type_bool(&sema->types);
    default:
      reml_report_diag(sema, REML_DIAG_UNSUPPORTED_FEATURE, expr->span,
                       "unsupported unary operator");
      return reml_type_error(&sema->types);
  }
}

static bool reml_unify_binary_numeric(reml_sema *sema, reml_type *left, reml_type *right,
                                      reml_span span) {
  left = reml_type_prune(left);
  right = reml_type_prune(right);
  if (left->kind == REML_TYPE_VAR && right->kind == REML_TYPE_VAR) {
    return reml_expect_type(sema, left, reml_type_int(&sema->types), span) &&
           reml_expect_type(sema, right, reml_type_int(&sema->types), span);
  }
  if (left->kind == REML_TYPE_VAR && reml_is_numeric_type(right, &sema->types)) {
    return reml_expect_type(sema, left, right, span);
  }
  if (right->kind == REML_TYPE_VAR && reml_is_numeric_type(left, &sema->types)) {
    return reml_expect_type(sema, right, left, span);
  }
  if (!reml_is_numeric_type(left, &sema->types) || !reml_is_numeric_type(right, &sema->types)) {
    reml_report_diag(sema, REML_DIAG_TYPE_MISMATCH, span, "numeric operator expects numbers");
    return false;
  }
  if (!reml_type_unify(&sema->types, left, right)) {
    reml_report_diag(sema, REML_DIAG_TYPE_MISMATCH, span, "numeric operands must match");
    return false;
  }
  return true;
}

static reml_type *reml_infer_binary(reml_sema *sema, reml_expr *expr, reml_effect_set *effect) {
  reml_type *left = reml_infer_expr(sema, expr->data.binary.left, effect);
  reml_type *right = reml_infer_expr(sema, expr->data.binary.right, effect);
  if (!left || !right) {
    return reml_type_error(&sema->types);
  }
  switch (expr->data.binary.op) {
    case REML_TOKEN_PLUS:
    case REML_TOKEN_MINUS:
    case REML_TOKEN_STAR:
    case REML_TOKEN_SLASH:
    case REML_TOKEN_PERCENT:
    case REML_TOKEN_CARET:
      if (!reml_unify_binary_numeric(sema, left, right, expr->span)) {
        return reml_type_error(&sema->types);
      }
      return reml_type_prune(left);
    case REML_TOKEN_LT:
    case REML_TOKEN_LE:
    case REML_TOKEN_GT:
    case REML_TOKEN_GE:
      if (!reml_unify_binary_numeric(sema, left, right, expr->span)) {
        return reml_type_error(&sema->types);
      }
      return reml_type_bool(&sema->types);
    case REML_TOKEN_EQEQ:
    case REML_TOKEN_NOTEQ:
      if (!reml_type_unify(&sema->types, left, right)) {
        reml_report_diag(sema, REML_DIAG_TYPE_MISMATCH, expr->span, "equality types must match");
        return reml_type_error(&sema->types);
      }
      return reml_type_bool(&sema->types);
    case REML_TOKEN_LOGICAL_AND:
    case REML_TOKEN_LOGICAL_OR:
      if (!reml_expect_type(sema, left, reml_type_bool(&sema->types), expr->span) ||
          !reml_expect_type(sema, right, reml_type_bool(&sema->types), expr->span)) {
        return reml_type_error(&sema->types);
      }
      return reml_type_bool(&sema->types);
    case REML_TOKEN_DOTDOT:
    case REML_TOKEN_PIPE_FORWARD:
      reml_report_diag(sema, REML_DIAG_UNSUPPORTED_FEATURE, expr->span,
                       "unsupported binary operator");
      return reml_type_error(&sema->types);
    default:
      reml_report_diag(sema, REML_DIAG_UNSUPPORTED_FEATURE, expr->span,
                       "unsupported binary operator");
      return reml_type_error(&sema->types);
  }
}

static reml_effect_set reml_effect_union(reml_effect_set left, reml_effect_set right) {
  return (reml_effect_set)(left | right);
}

static reml_type *reml_infer_block(reml_sema *sema, reml_expr *expr, reml_effect_set *effect) {
  reml_symbol_table_enter(sema->symbols);
  reml_effect_set block_effect = REML_EFFECT_NONE;

  if (expr->data.block.statements) {
    for (reml_stmt **it = (reml_stmt **)utarray_front(expr->data.block.statements); it != NULL;
         it = (reml_stmt **)utarray_next(expr->data.block.statements, it)) {
      reml_stmt *stmt = *it;
      reml_effect_set stmt_effect = REML_EFFECT_NONE;
      switch (stmt->kind) {
        case REML_STMT_VAL_DECL: {
          reml_type *value_type =
              reml_infer_expr(sema, stmt->data.val_decl.value, &stmt_effect);
          reml_check_pattern(sema, stmt->data.val_decl.pattern, value_type, &stmt_effect, true);
          break;
        }
        case REML_STMT_RETURN:
          reml_infer_expr(sema, stmt->data.expr, &stmt_effect);
          break;
        case REML_STMT_EXPR:
          reml_infer_expr(sema, stmt->data.expr, &stmt_effect);
          break;
        default:
          break;
      }
      block_effect = reml_effect_union(block_effect, stmt_effect);
    }
  }

  reml_type *result_type = reml_type_unit(&sema->types);
  if (expr->data.block.tail) {
    reml_effect_set tail_effect = REML_EFFECT_NONE;
    result_type = reml_infer_expr(sema, expr->data.block.tail, &tail_effect);
    block_effect = reml_effect_union(block_effect, tail_effect);
  }

  reml_symbol_table_exit(sema->symbols);
  if (effect) {
    *effect = reml_effect_union(*effect, block_effect);
  }
  return result_type;
}

static reml_type *reml_infer_if(reml_sema *sema, reml_expr *expr, reml_effect_set *effect) {
  reml_effect_set cond_effect = REML_EFFECT_NONE;
  reml_type *cond_type = reml_infer_expr(sema, expr->data.if_expr.condition, &cond_effect);
  reml_expect_type(sema, cond_type, reml_type_bool(&sema->types), expr->data.if_expr.condition->span);

  reml_effect_set then_effect = REML_EFFECT_NONE;
  reml_type *then_type = reml_infer_expr(sema, expr->data.if_expr.then_branch, &then_effect);

  reml_type *result_type = reml_type_unit(&sema->types);
  if (expr->data.if_expr.else_branch) {
    reml_effect_set else_effect = REML_EFFECT_NONE;
    reml_type *else_type = reml_infer_expr(sema, expr->data.if_expr.else_branch, &else_effect);
    reml_expect_type(sema, then_type, else_type, expr->span);
    result_type = reml_type_prune(then_type);
    *effect = reml_effect_union(*effect, else_effect);
  } else {
    reml_expect_type(sema, then_type, reml_type_unit(&sema->types), expr->span);
  }

  *effect = reml_effect_union(*effect, cond_effect);
  *effect = reml_effect_union(*effect, then_effect);
  return result_type;
}

static reml_type *reml_infer_while(reml_sema *sema, reml_expr *expr, reml_effect_set *effect) {
  reml_effect_set cond_effect = REML_EFFECT_NONE;
  reml_type *cond_type = reml_infer_expr(sema, expr->data.while_expr.condition, &cond_effect);
  reml_expect_type(sema, cond_type, reml_type_bool(&sema->types),
                   expr->data.while_expr.condition->span);

  reml_effect_set body_effect = REML_EFFECT_NONE;
  reml_type *body_type = reml_infer_expr(sema, expr->data.while_expr.body, &body_effect);
  reml_expect_type(sema, body_type, reml_type_unit(&sema->types), expr->data.while_expr.body->span);

  *effect = reml_effect_union(*effect, cond_effect);
  *effect = reml_effect_union(*effect, body_effect);
  return reml_type_unit(&sema->types);
}

static reml_type *reml_infer_match(reml_sema *sema, reml_expr *expr, reml_effect_set *effect) {
  reml_effect_set scrutinee_effect = REML_EFFECT_NONE;
  reml_type *scrutinee = reml_infer_expr(sema, expr->data.match_expr.scrutinee, &scrutinee_effect);
  reml_type *result = NULL;

  if (expr->data.match_expr.arms) {
    for (reml_match_arm *it = (reml_match_arm *)utarray_front(expr->data.match_expr.arms);
         it != NULL; it = (reml_match_arm *)utarray_next(expr->data.match_expr.arms, it)) {
      reml_symbol_table_enter(sema->symbols);
      reml_effect_set arm_effect = REML_EFFECT_NONE;
      reml_check_pattern(sema, it->pattern, scrutinee, &arm_effect, true);
      reml_type *arm_type = reml_infer_expr(sema, it->body, &arm_effect);
      if (!result) {
        result = arm_type;
      } else {
        reml_expect_type(sema, result, arm_type, it->body->span);
        result = reml_type_prune(result);
      }
      reml_symbol_table_exit(sema->symbols);
      *effect = reml_effect_union(*effect, arm_effect);
    }
  }

  *effect = reml_effect_union(*effect, scrutinee_effect);
  return result ? result : reml_type_error(&sema->types);
}

static reml_type *reml_infer_expr(reml_sema *sema, reml_expr *expr, reml_effect_set *effect) {
  if (!expr) {
    return reml_type_error(&sema->types);
  }
  reml_effect_set local_effect = REML_EFFECT_NONE;
  reml_type *result = NULL;
  switch (expr->kind) {
    case REML_EXPR_LITERAL:
      result = reml_infer_literal(sema, expr->data.literal);
      break;
    case REML_EXPR_IDENT: {
      reml_symbol *symbol = reml_symbol_table_lookup(sema->symbols, expr->data.ident);
      if (!symbol) {
        reml_report_diag(sema, REML_DIAG_UNDEFINED_SYMBOL, expr->span, "undefined symbol");
        result = reml_type_error(&sema->types);
      } else {
        expr->symbol_id = symbol->id;
        result = reml_type_instantiate(&sema->types, &symbol->scheme);
      }
      break;
    }
    case REML_EXPR_UNARY:
      result = reml_infer_unary(sema, expr, &local_effect);
      break;
    case REML_EXPR_BINARY:
      result = reml_infer_binary(sema, expr, &local_effect);
      break;
    case REML_EXPR_BLOCK:
      result = reml_infer_block(sema, expr, &local_effect);
      break;
    case REML_EXPR_IF:
      result = reml_infer_if(sema, expr, &local_effect);
      break;
    case REML_EXPR_WHILE:
      result = reml_infer_while(sema, expr, &local_effect);
      break;
    case REML_EXPR_MATCH:
      result = reml_infer_match(sema, expr, &local_effect);
      break;
    default:
      reml_report_diag(sema, REML_DIAG_UNSUPPORTED_FEATURE, expr->span,
                       "unsupported expression");
      result = reml_type_error(&sema->types);
      break;
  }
  expr->type = result;
  if (effect) {
    *effect = reml_effect_union(*effect, local_effect);
  }
  return result;
}

static void reml_generalize(reml_sema *sema, reml_symbol *symbol, reml_type *type,
                            bool allow_poly) {
  if (!symbol) {
    return;
  }
  reml_scheme_reset(&symbol->scheme, type);
  if (!allow_poly) {
    return;
  }
  UT_icd var_icd = {sizeof(uint32_t), NULL, NULL, NULL};
  UT_array *type_vars = NULL;
  UT_array *env_vars = NULL;
  utarray_new(type_vars, &var_icd);
  utarray_new(env_vars, &var_icd);
  reml_type_collect_vars(type, type_vars);
  reml_env_collect_free_vars(sema->symbols, symbol, env_vars);

  for (uint32_t *it = (uint32_t *)utarray_front(type_vars); it != NULL;
       it = (uint32_t *)utarray_next(type_vars, it)) {
    if (!reml_var_ids_contains(env_vars, *it)) {
      reml_var_ids_push_unique(symbol->scheme.generics, *it);
    }
  }

  utarray_free(type_vars);
  utarray_free(env_vars);
}

static void reml_define_pattern_symbol(reml_sema *sema, reml_pattern *pattern,
                                       reml_type *expected, bool allow_define,
                                       reml_effect_set *effect) {
  if (!pattern || !allow_define) {
    return;
  }
  if (pattern->kind != REML_PATTERN_IDENT) {
    return;
  }
  if (reml_symbol_table_has_builtin(sema->symbols, pattern->data.ident)) {
    reml_report_diag(sema, REML_DIAG_DUPLICATE_SYMBOL, pattern->span,
                     "cannot redefine builtin");
    return;
  }
  reml_scope *scope = reml_symbol_table_current(sema->symbols);
  reml_symbol *existing = reml_scope_lookup(scope, pattern->data.ident);
  if (existing && !existing->is_predeclared) {
    reml_report_diag(sema, REML_DIAG_DUPLICATE_SYMBOL, pattern->span,
                     "duplicate symbol in scope");
    return;
  }
  reml_symbol *symbol = existing;
  if (!symbol) {
    symbol = reml_symbol_table_define(sema->symbols, REML_SYMBOL_VAR, pattern->data.ident,
                                      pattern->span, expected, false, false);
  }
  if (!symbol) {
    return;
  }
  if (existing && existing->is_predeclared) {
    reml_expect_type(sema, existing->scheme.type, expected, pattern->span);
    expected = reml_type_prune(existing->scheme.type);
  }
  symbol->is_predeclared = false;
  pattern->symbol_id = symbol->id;
  pattern->type = expected;

  bool allow_poly = effect ? (*effect == REML_EFFECT_NONE) : true;
  reml_generalize(sema, symbol, expected, allow_poly);
}

static void reml_check_pattern(reml_sema *sema, reml_pattern *pattern, reml_type *expected,
                               reml_effect_set *effect, bool allow_define) {
  if (!pattern) {
    return;
  }
  switch (pattern->kind) {
    case REML_PATTERN_WILDCARD:
      pattern->type = expected;
      return;
    case REML_PATTERN_IDENT:
      reml_define_pattern_symbol(sema, pattern, expected, allow_define, effect);
      return;
    case REML_PATTERN_LITERAL: {
      reml_type *literal_type = reml_infer_literal(sema, pattern->data.literal);
      if (!reml_expect_type(sema, literal_type, expected, pattern->span)) {
        return;
      }
      pattern->type = literal_type;
      return;
    }
    case REML_PATTERN_TUPLE:
    case REML_PATTERN_RECORD:
      reml_report_diag(sema, REML_DIAG_UNSUPPORTED_FEATURE, pattern->span,
                       "pattern kind not supported in phase 3");
      pattern->type = reml_type_error(&sema->types);
      return;
    default:
      return;
  }
}

static void reml_first_pass_decls(reml_sema *sema, reml_compilation_unit *unit) {
  if (!unit || !unit->statements) {
    return;
  }
  for (reml_stmt **it = (reml_stmt **)utarray_front(unit->statements); it != NULL;
       it = (reml_stmt **)utarray_next(unit->statements, it)) {
    reml_stmt *stmt = *it;
    if (stmt->kind != REML_STMT_VAL_DECL) {
      continue;
    }
    reml_pattern *pattern = stmt->data.val_decl.pattern;
    if (!pattern || pattern->kind != REML_PATTERN_IDENT) {
      continue;
    }
    if (reml_symbol_table_has_builtin(sema->symbols, pattern->data.ident)) {
      reml_report_diag(sema, REML_DIAG_DUPLICATE_SYMBOL, pattern->span,
                       "cannot redefine builtin");
      continue;
    }
    reml_scope *scope = reml_symbol_table_current(sema->symbols);
    if (reml_scope_lookup(scope, pattern->data.ident)) {
      reml_report_diag(sema, REML_DIAG_DUPLICATE_SYMBOL, pattern->span,
                       "duplicate symbol in scope");
      continue;
    }
    reml_symbol *symbol =
        reml_symbol_table_define(sema->symbols, REML_SYMBOL_VAR, pattern->data.ident,
                                 pattern->span, reml_type_make_var(&sema->types), false, true);
    if (symbol) {
      pattern->symbol_id = symbol->id;
      pattern->type = symbol->scheme.type;
    }
  }
}

static void reml_check_stmt(reml_sema *sema, reml_stmt *stmt, reml_effect_set *effect) {
  if (!stmt) {
    return;
  }
  switch (stmt->kind) {
    case REML_STMT_VAL_DECL: {
      reml_effect_set value_effect = REML_EFFECT_NONE;
      reml_type *value_type = reml_infer_expr(sema, stmt->data.val_decl.value, &value_effect);
      reml_check_pattern(sema, stmt->data.val_decl.pattern, value_type, &value_effect, true);
      if (effect) {
        *effect = reml_effect_union(*effect, value_effect);
      }
      break;
    }
    case REML_STMT_RETURN: {
      reml_effect_set expr_effect = REML_EFFECT_NONE;
      reml_infer_expr(sema, stmt->data.expr, &expr_effect);
      if (effect) {
        *effect = reml_effect_union(*effect, expr_effect);
      }
      break;
    }
    case REML_STMT_EXPR: {
      reml_effect_set expr_effect = REML_EFFECT_NONE;
      reml_infer_expr(sema, stmt->data.expr, &expr_effect);
      if (effect) {
        *effect = reml_effect_union(*effect, expr_effect);
      }
      break;
    }
    default:
      break;
  }
}

void reml_sema_init(reml_sema *sema) {
  if (!sema) {
    return;
  }
  sema->symbols = (reml_symbol_table *)calloc(1, sizeof(reml_symbol_table));
  reml_symbol_table_init(sema->symbols);
  reml_symbol_table_enter(sema->symbols);
  reml_type_ctx_init(&sema->types);
  reml_diagnostics_init(&sema->diagnostics);
}

void reml_sema_deinit(reml_sema *sema) {
  if (!sema) {
    return;
  }
  if (sema->symbols) {
    while (sema->symbols->scopes && utarray_len(sema->symbols->scopes) > 0) {
      reml_symbol_table_exit(sema->symbols);
    }
    reml_symbol_table_deinit(sema->symbols);
    free(sema->symbols);
    sema->symbols = NULL;
  }
  reml_type_ctx_deinit(&sema->types);
  reml_diagnostics_deinit(&sema->diagnostics);
}

bool reml_sema_check(reml_sema *sema, reml_compilation_unit *unit) {
  if (!sema || !unit) {
    return false;
  }
  reml_first_pass_decls(sema, unit);

  if (unit->statements) {
    for (reml_stmt **it = (reml_stmt **)utarray_front(unit->statements); it != NULL;
         it = (reml_stmt **)utarray_next(unit->statements, it)) {
      reml_effect_set effect = REML_EFFECT_NONE;
      reml_check_stmt(sema, *it, &effect);
    }
  }

  return reml_diagnostics_count(&sema->diagnostics) == 0;
}

const reml_diagnostic_list *reml_sema_diagnostics(const reml_sema *sema) {
  if (!sema) {
    return NULL;
  }
  return &sema->diagnostics;
}
