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
    case REML_EXPR_BINARY:
      fputs("(", out);
      fputs(reml_token_symbol(expr->data.binary.op), out);
      fputs(" ", out);
      reml_ast_write_expr(out, expr->data.binary.left);
      fputs(" ", out);
      reml_ast_write_expr(out, expr->data.binary.right);
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
