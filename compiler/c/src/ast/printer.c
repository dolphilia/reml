#include "reml/ast/printer.h"

#include <string.h>

#include "reml/parser/operator_table.h"

static void reml_write_view(FILE *out, reml_string_view view) {
  fwrite(view.data, 1, view.length, out);
}

static const char *reml_token_symbol(reml_token_kind kind) {
  reml_operator_entry entry = {0};
  if (reml_operator_lookup(kind, &entry)) {
    return entry.symbol;
  }
  switch (kind) {
    case REML_TOKEN_MINUS:
      return "-";
    case REML_TOKEN_BANG:
      return "!";
    default:
      return "?";
  }
}

static void reml_ast_write_pattern(FILE *out, const reml_pattern *pattern) {
  if (!pattern) {
    fputs("(pattern null)", out);
    return;
  }

  switch (pattern->kind) {
    case REML_PATTERN_WILDCARD:
      fputs("(_)", out);
      return;
    case REML_PATTERN_IDENT:
      fputs("(pident ", out);
      reml_write_view(out, pattern->data.ident);
      fputs(")", out);
      return;
    case REML_PATTERN_LITERAL:
      fputs("(plit ", out);
      reml_write_view(out, pattern->data.literal.text);
      fputs(")", out);
      return;
    case REML_PATTERN_RANGE:
      fputs("(prange ", out);
      reml_write_view(out, pattern->data.range.start.text);
      fputs(pattern->data.range.inclusive ? " ..= " : " .. ", out);
      reml_write_view(out, pattern->data.range.end.text);
      fputs(")", out);
      return;
    case REML_PATTERN_TUPLE:
      fputs("(ptuple", out);
      if (pattern->data.items) {
        for (reml_pattern **it = (reml_pattern **)utarray_front(pattern->data.items); it != NULL;
             it = (reml_pattern **)utarray_next(pattern->data.items, it)) {
          fputs(" ", out);
          reml_ast_write_pattern(out, *it);
        }
      }
      fputs(")", out);
      return;
    case REML_PATTERN_RECORD:
      fputs("(precord", out);
      if (pattern->data.fields) {
        for (reml_pattern_field *it =
                 (reml_pattern_field *)utarray_front(pattern->data.fields);
             it != NULL;
             it = (reml_pattern_field *)utarray_next(pattern->data.fields, it)) {
          fputs(" (field ", out);
          reml_write_view(out, it->name);
          if (it->pattern) {
            fputs(" ", out);
            reml_ast_write_pattern(out, it->pattern);
          }
          fputs(")", out);
        }
      }
      fputs(")", out);
      return;
    case REML_PATTERN_CONSTRUCTOR:
      fputs("(pctor ", out);
      reml_write_view(out, pattern->data.ctor.name);
      if (pattern->data.ctor.items) {
        for (reml_pattern **it = (reml_pattern **)utarray_front(pattern->data.ctor.items);
             it != NULL;
             it = (reml_pattern **)utarray_next(pattern->data.ctor.items, it)) {
          fputs(" ", out);
          reml_ast_write_pattern(out, *it);
        }
      }
      fputs(")", out);
      return;
    default:
      fputs("(pattern ?)", out);
      return;
  }
}

void reml_ast_write_expr(FILE *out, const reml_expr *expr) {
  if (!expr) {
    fputs("(null)", out);
    return;
  }

  switch (expr->kind) {
    case REML_EXPR_LITERAL:
      switch (expr->data.literal.kind) {
        case REML_LITERAL_INT:
          fputs("(int ", out);
          reml_write_view(out, expr->data.literal.text);
          fputs(")", out);
          return;
        case REML_LITERAL_BIGINT:
          fputs("(bigint ", out);
          reml_write_view(out, expr->data.literal.text);
          fputs(")", out);
          return;
        case REML_LITERAL_FLOAT:
          fputs("(float ", out);
          reml_write_view(out, expr->data.literal.text);
          fputs(")", out);
          return;
        case REML_LITERAL_STRING:
          fputs("(string ", out);
          reml_write_view(out, expr->data.literal.text);
          fputs(")", out);
          return;
        case REML_LITERAL_CHAR:
          fputs("(char ", out);
          reml_write_view(out, expr->data.literal.text);
          fputs(")", out);
          return;
        case REML_LITERAL_BOOL:
          fputs("(bool ", out);
          reml_write_view(out, expr->data.literal.text);
          fputs(")", out);
          return;
        default:
          fputs("(literal ?)", out);
          return;
      }
    case REML_EXPR_IDENT:
      fputs("(ident ", out);
      reml_write_view(out, expr->data.ident);
      fputs(")", out);
      return;
    case REML_EXPR_UNARY:
      fputs("(", out);
      fputs(reml_token_symbol(expr->data.unary.op), out);
      fputs(" ", out);
      reml_ast_write_expr(out, expr->data.unary.operand);
      fputs(")", out);
      return;
    case REML_EXPR_REF:
      fputs("(", out);
      fputs(expr->data.ref.is_mutable ? "&mut" : "&", out);
      fputs(" ", out);
      reml_ast_write_expr(out, expr->data.ref.target);
      fputs(")", out);
      return;
    case REML_EXPR_BINARY:
      fputs("(", out);
      fputs(reml_token_symbol(expr->data.binary.op), out);
      fputs(" ", out);
      reml_ast_write_expr(out, expr->data.binary.left);
      fputs(" ", out);
      reml_ast_write_expr(out, expr->data.binary.right);
      fputs(")", out);
      return;
    case REML_EXPR_CONSTRUCTOR:
      fputs("(ctor ", out);
      reml_write_view(out, expr->data.ctor.name);
      if (expr->data.ctor.args) {
        for (reml_expr **it = (reml_expr **)utarray_front(expr->data.ctor.args); it != NULL;
             it = (reml_expr **)utarray_next(expr->data.ctor.args, it)) {
          fputs(" ", out);
          reml_ast_write_expr(out, *it);
        }
      }
      fputs(")", out);
      return;
    case REML_EXPR_TUPLE:
      fputs("(tuple", out);
      if (expr->data.tuple) {
        for (reml_expr **it = (reml_expr **)utarray_front(expr->data.tuple); it != NULL;
             it = (reml_expr **)utarray_next(expr->data.tuple, it)) {
          fputs(" ", out);
          reml_ast_write_expr(out, *it);
        }
      }
      fputs(")", out);
      return;
    case REML_EXPR_RECORD:
      fputs("(record", out);
      if (expr->data.record) {
        for (reml_record_expr_field *it =
                 (reml_record_expr_field *)utarray_front(expr->data.record);
             it != NULL;
             it = (reml_record_expr_field *)utarray_next(expr->data.record, it)) {
          fputs(" (field ", out);
          reml_write_view(out, it->name);
          fputs(" ", out);
          reml_ast_write_expr(out, it->value);
          fputs(")", out);
        }
      }
      fputs(")", out);
      return;
    case REML_EXPR_RECORD_UPDATE:
      fputs("(record_update ", out);
      reml_ast_write_expr(out, expr->data.record_update.base);
      if (expr->data.record_update.fields) {
        for (reml_record_expr_field *it =
                 (reml_record_expr_field *)utarray_front(expr->data.record_update.fields);
             it != NULL;
             it = (reml_record_expr_field *)utarray_next(expr->data.record_update.fields, it)) {
          fputs(" (field ", out);
          reml_write_view(out, it->name);
          fputs(" ", out);
          reml_ast_write_expr(out, it->value);
          fputs(")", out);
        }
      }
      fputs(")", out);
      return;
    case REML_EXPR_BLOCK:
      fputs("(block", out);
      if (expr->data.block.statements) {
        for (reml_stmt **it = (reml_stmt **)utarray_front(expr->data.block.statements); it != NULL;
             it = (reml_stmt **)utarray_next(expr->data.block.statements, it)) {
          fputs(" ", out);
          reml_ast_write_stmt(out, *it);
        }
      }
      if (expr->data.block.tail) {
        fputs(" (tail ", out);
        reml_ast_write_expr(out, expr->data.block.tail);
        fputs(")", out);
      }
      fputs(")", out);
      return;
    case REML_EXPR_IF:
      fputs("(if ", out);
      reml_ast_write_expr(out, expr->data.if_expr.condition);
      fputs(" ", out);
      reml_ast_write_expr(out, expr->data.if_expr.then_branch);
      if (expr->data.if_expr.else_branch) {
        fputs(" ", out);
        reml_ast_write_expr(out, expr->data.if_expr.else_branch);
      }
      fputs(")", out);
      return;
    case REML_EXPR_WHILE:
      fputs("(while ", out);
      reml_ast_write_expr(out, expr->data.while_expr.condition);
      fputs(" ", out);
      reml_ast_write_expr(out, expr->data.while_expr.body);
      fputs(")", out);
      return;
    case REML_EXPR_MATCH:
      fputs("(match ", out);
      reml_ast_write_expr(out, expr->data.match_expr.scrutinee);
      if (expr->data.match_expr.arms) {
        for (reml_match_arm *it = (reml_match_arm *)utarray_front(expr->data.match_expr.arms);
             it != NULL;
             it = (reml_match_arm *)utarray_next(expr->data.match_expr.arms, it)) {
          fputs(" (arm ", out);
          reml_ast_write_pattern(out, it->pattern);
          if (it->guard) {
            fputs(" (guard ", out);
            reml_ast_write_expr(out, it->guard);
            fputs(")", out);
          }
          fputs(" ", out);
          reml_ast_write_expr(out, it->body);
          fputs(")", out);
        }
      }
      fputs(")", out);
      return;
    default:
      fputs("(expr ?)", out);
      return;
  }
}

void reml_ast_write_stmt(FILE *out, const reml_stmt *stmt) {
  if (!stmt) {
    fputs("(null)", out);
    return;
  }

  switch (stmt->kind) {
    case REML_STMT_EXPR:
      fputs("(expr ", out);
      reml_ast_write_expr(out, stmt->data.expr);
      fputs(")", out);
      return;
    case REML_STMT_RETURN:
      fputs("(return ", out);
      reml_ast_write_expr(out, stmt->data.expr);
      fputs(")", out);
      return;
    case REML_STMT_VAL_DECL:
      fputs("(let ", out);
      reml_ast_write_pattern(out, stmt->data.val_decl.pattern);
      fputs(" ", out);
      reml_ast_write_expr(out, stmt->data.val_decl.value);
      fputs(")", out);
      return;
    case REML_STMT_TYPE_DECL:
      fputs("(type ", out);
      reml_write_view(out, stmt->data.type_decl.name);
      if (stmt->data.type_decl.variants) {
        for (reml_type_decl_variant *it =
                 (reml_type_decl_variant *)utarray_front(stmt->data.type_decl.variants);
             it != NULL;
             it = (reml_type_decl_variant *)utarray_next(stmt->data.type_decl.variants, it)) {
          fputs(" (variant ", out);
          reml_write_view(out, it->name);
          if (it->fields) {
            for (reml_string_view *field =
                     (reml_string_view *)utarray_front(it->fields);
                 field != NULL;
                 field = (reml_string_view *)utarray_next(it->fields, field)) {
              fputs(" ", out);
              reml_write_view(out, *field);
            }
          }
          fputs(")", out);
        }
      }
      fputs(")", out);
      return;
    default:
      fputs("(stmt ?)", out);
      return;
  }
}

void reml_ast_write_compilation_unit(FILE *out, const reml_compilation_unit *unit) {
  if (!unit) {
    fputs("(unit)", out);
    return;
  }
  fputs("(unit", out);
  if (unit->statements) {
    for (reml_stmt **it = (reml_stmt **)utarray_front(unit->statements); it != NULL;
         it = (reml_stmt **)utarray_next(unit->statements, it)) {
      fputs(" ", out);
      reml_ast_write_stmt(out, *it);
    }
  }
  fputs(")", out);
}
