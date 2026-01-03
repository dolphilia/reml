#include "reml/lexer/lexer.h"

#include <ctype.h>
#include <stdint.h>
#include <string.h>
#include <sys/types.h>

#include <utf8proc.h>

#include "reml/text/grapheme.h"
#include "reml/text/unicode.h"

static bool reml_is_xid_start(int32_t codepoint) {
  if (codepoint == '_') {
    return true;
  }
  utf8proc_category_t cat = utf8proc_category(codepoint);
  return cat == UTF8PROC_CATEGORY_LU || cat == UTF8PROC_CATEGORY_LL || cat == UTF8PROC_CATEGORY_LT ||
         cat == UTF8PROC_CATEGORY_LM || cat == UTF8PROC_CATEGORY_LO || cat == UTF8PROC_CATEGORY_NL;
}

static bool reml_is_xid_continue(int32_t codepoint) {
  if (reml_is_xid_start(codepoint)) {
    return true;
  }
  utf8proc_category_t cat = utf8proc_category(codepoint);
  return cat == UTF8PROC_CATEGORY_MN || cat == UTF8PROC_CATEGORY_MC || cat == UTF8PROC_CATEGORY_ME ||
         cat == UTF8PROC_CATEGORY_ND || cat == UTF8PROC_CATEGORY_PC;
}

static bool reml_is_ascii_digit(int c) {
  return c >= '0' && c <= '9';
}

static bool reml_is_ascii_hex(int c) {
  return (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F');
}

static bool reml_is_ascii_oct(int c) {
  return c >= '0' && c <= '7';
}

static bool reml_is_ascii_bin(int c) {
  return c == '0' || c == '1';
}

static int reml_peek_byte(const reml_lexer *lexer) {
  if (lexer->index >= lexer->length) {
    return 0;
  }
  return (unsigned char)lexer->input[lexer->index];
}

static int reml_peek_next_byte(const reml_lexer *lexer) {
  if (lexer->index + 1 >= lexer->length) {
    return 0;
  }
  return (unsigned char)lexer->input[lexer->index + 1];
}

static bool reml_is_hex_digit(int c) {
  return (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F');
}

static void reml_lexer_set_error(reml_lexer *lexer, const char *message, size_t start_offset,
                                 size_t end_offset, int start_line, int start_column, int end_line,
                                 int end_column) {
  lexer->has_error = true;
  lexer->error.span =
      reml_span_make(start_offset, end_offset, start_line, start_column, end_line, end_column);
  lexer->error.message = message;
}

static void reml_advance_bytes(reml_lexer *lexer, size_t count) {
  if (count == 0) {
    return;
  }
  if (count == 1) {
    unsigned char c = (unsigned char)lexer->input[lexer->index];
    lexer->index += 1;
    if (c == '\t') {
      lexer->column += 4;
    } else {
      lexer->column += 1;
    }
    return;
  }
  lexer->index += count;
  lexer->column += (int)count;
}

static bool reml_advance_codepoint(reml_lexer *lexer, int32_t *out_cp, size_t *out_bytes) {
  size_t remaining = lexer->length - lexer->index;
  if (remaining == 0) {
    return false;
  }
  int32_t codepoint = 0;
  ssize_t len = utf8proc_iterate((const uint8_t *)lexer->input + lexer->index, (ssize_t)remaining,
                                 &codepoint);
  if (len < 0) {
    reml_lexer_set_error(lexer, "unicode.invalid_utf8", lexer->index, lexer->index + 1,
                         lexer->line, lexer->column, lexer->line, lexer->column + 1);
    return false;
  }
  if (!utf8proc_codepoint_valid(codepoint)) {
    reml_lexer_set_error(lexer, "unicode.invalid_scalar", lexer->index, lexer->index + (size_t)len,
                         lexer->line, lexer->column, lexer->line, lexer->column + 1);
    return false;
  }
  lexer->index += (size_t)len;
  lexer->column += 1;
  if (out_cp) {
    *out_cp = codepoint;
  }
  if (out_bytes) {
    *out_bytes = (size_t)len;
  }
  return true;
}

static bool reml_advance_grapheme(reml_lexer *lexer) {
  reml_unicode_error error;
  size_t advance = reml_grapheme_advance(lexer->input, lexer->length, lexer->index, &error);
  if (advance == 0) {
    const char *message = error.kind == REML_UNICODE_INVALID_SCALAR ? "unicode.invalid_scalar"
                                                                    : "unicode.invalid_utf8";
    size_t end_offset = lexer->index + (error.length ? error.length : 1);
    reml_lexer_set_error(lexer, message, lexer->index, end_offset, lexer->line, lexer->column,
                         lexer->line, lexer->column + 1);
    return false;
  }
  lexer->index += advance;
  lexer->column += 1;
  return true;
}

static void reml_advance_line_break(reml_lexer *lexer) {
  if (!lexer || lexer->index >= lexer->length) {
    return;
  }
  int c = reml_peek_byte(lexer);
  if (c == '\r') {
    reml_advance_bytes(lexer, 1);
    if (reml_peek_byte(lexer) == '\n') {
      reml_advance_bytes(lexer, 1);
    }
    lexer->line += 1;
    lexer->column = 1;
    return;
  }
  if (c == '\n') {
    reml_advance_bytes(lexer, 1);
    lexer->line += 1;
    lexer->column = 1;
  }
}

static bool reml_lex_unicode_escape(reml_lexer *lexer, size_t escape_offset, int escape_line,
                                    int escape_column) {
  if (reml_peek_byte(lexer) != 'u') {
    reml_lexer_set_error(lexer, "invalid escape sequence", escape_offset, lexer->index,
                         escape_line, escape_column, lexer->line, lexer->column);
    return false;
  }
  reml_advance_bytes(lexer, 1);
  if (reml_peek_byte(lexer) != '{') {
    reml_lexer_set_error(lexer, "invalid unicode escape", escape_offset, lexer->index,
                         escape_line, escape_column, lexer->line, lexer->column);
    return false;
  }
  reml_advance_bytes(lexer, 1);
  int digits = 0;
  uint32_t codepoint = 0;
  while (lexer->index < lexer->length) {
    int c = reml_peek_byte(lexer);
    if (c == '}') {
      break;
    }
    if (!reml_is_hex_digit(c) || digits >= 6) {
      reml_lexer_set_error(lexer, "invalid unicode escape", escape_offset, lexer->index + 1,
                           escape_line, escape_column, lexer->line, lexer->column + 1);
      return false;
    }
    codepoint <<= 4;
    if (c >= '0' && c <= '9') {
      codepoint |= (uint32_t)(c - '0');
    } else if (c >= 'a' && c <= 'f') {
      codepoint |= (uint32_t)(c - 'a' + 10);
    } else {
      codepoint |= (uint32_t)(c - 'A' + 10);
    }
    digits += 1;
    reml_advance_bytes(lexer, 1);
  }
  if (digits == 0 || reml_peek_byte(lexer) != '}') {
    reml_lexer_set_error(lexer, "invalid unicode escape", escape_offset, lexer->index,
                         escape_line, escape_column, lexer->line, lexer->column);
    return false;
  }
  reml_advance_bytes(lexer, 1);
  if (codepoint > 0x10FFFF || (codepoint >= 0xD800 && codepoint <= 0xDFFF)) {
    reml_lexer_set_error(lexer, "unicode.invalid_scalar", escape_offset, lexer->index,
                         escape_line, escape_column, lexer->line, lexer->column);
    return false;
  }
  return true;
}

static bool reml_is_raw_string_start(const reml_lexer *lexer, size_t *out_hashes) {
  if (!lexer || reml_peek_byte(lexer) != 'r') {
    return false;
  }
  size_t index = lexer->index + 1;
  size_t hashes = 0;
  while (index < lexer->length && lexer->input[index] == '#') {
    hashes += 1;
    index += 1;
  }
  if (index >= lexer->length || lexer->input[index] != '"') {
    return false;
  }
  if (out_hashes) {
    *out_hashes = hashes;
  }
  return true;
}

static void reml_skip_whitespace_and_comments(reml_lexer *lexer) {
  while (lexer->index < lexer->length) {
    int c = reml_peek_byte(lexer);
    if (c == ' ' || c == '\t' || c == '\v' || c == '\f') {
      reml_advance_bytes(lexer, 1);
      continue;
    }
    if (c == '\r') {
      size_t start_offset = lexer->index;
      reml_advance_bytes(lexer, 1);
      if (reml_peek_byte(lexer) == '\n') {
        reml_advance_bytes(lexer, 1);
      }
      (void)start_offset;
      lexer->line += 1;
      lexer->column = 1;
      continue;
    }
    if (c == '\n') {
      reml_advance_bytes(lexer, 1);
      lexer->line += 1;
      lexer->column = 1;
      continue;
    }

    if (c == '/' && reml_peek_next_byte(lexer) == '/') {
      reml_advance_bytes(lexer, 2);
      while (lexer->index < lexer->length && reml_peek_byte(lexer) != '\n') {
        if (reml_peek_byte(lexer) < 0x80) {
          reml_advance_bytes(lexer, 1);
        } else if (!reml_advance_grapheme(lexer)) {
          return;
        }
      }
      continue;
    }

    if (c == '/' && reml_peek_next_byte(lexer) == '*') {
      reml_advance_bytes(lexer, 2);
      while (lexer->index < lexer->length) {
        int d = reml_peek_byte(lexer);
        if (d == '\r') {
          reml_advance_bytes(lexer, 1);
          if (reml_peek_byte(lexer) == '\n') {
            reml_advance_bytes(lexer, 1);
          }
          lexer->line += 1;
          lexer->column = 1;
          continue;
        }
        if (d == '\n') {
          reml_advance_bytes(lexer, 1);
          lexer->line += 1;
          lexer->column = 1;
          continue;
        }
        if (d == '*' && reml_peek_next_byte(lexer) == '/') {
          reml_advance_bytes(lexer, 2);
          break;
        }
        if (d < 0x80) {
          reml_advance_bytes(lexer, 1);
        } else if (!reml_advance_grapheme(lexer)) {
          return;
        }
      }
      continue;
    }

    break;
  }
}

static reml_token reml_make_token(reml_token_kind kind, const reml_lexer *lexer, size_t start_offset,
                                  int start_line, int start_column, size_t end_offset, int end_line,
                                  int end_column) {
  reml_token token;
  token.kind = kind;
  token.lexeme =
      reml_string_view_make(lexer->input + start_offset, end_offset - start_offset);
  token.span = reml_span_make(start_offset, end_offset, start_line, start_column, end_line, end_column);
  return token;
}

static reml_token_kind reml_keyword_kind(const reml_string_view *view) {
  if (view->length == 6 && strncmp(view->data, "return", 6) == 0) {
    return REML_TOKEN_KW_RETURN;
  }
  if (view->length == 4 && strncmp(view->data, "true", 4) == 0) {
    return REML_TOKEN_KW_TRUE;
  }
  if (view->length == 5 && strncmp(view->data, "false", 5) == 0) {
    return REML_TOKEN_KW_FALSE;
  }
  if (view->length == 2 && strncmp(view->data, "if", 2) == 0) {
    return REML_TOKEN_KW_IF;
  }
  if (view->length == 5 && strncmp(view->data, "while", 5) == 0) {
    return REML_TOKEN_KW_WHILE;
  }
  if (view->length == 4 && strncmp(view->data, "when", 4) == 0) {
    return REML_TOKEN_KW_WHEN;
  }
  if (view->length == 4 && strncmp(view->data, "then", 4) == 0) {
    return REML_TOKEN_KW_THEN;
  }
  if (view->length == 4 && strncmp(view->data, "else", 4) == 0) {
    return REML_TOKEN_KW_ELSE;
  }
  if (view->length == 5 && strncmp(view->data, "match", 5) == 0) {
    return REML_TOKEN_KW_MATCH;
  }
  if (view->length == 4 && strncmp(view->data, "with", 4) == 0) {
    return REML_TOKEN_KW_WITH;
  }
  if (view->length == 3 && strncmp(view->data, "let", 3) == 0) {
    return REML_TOKEN_KW_LET;
  }
  if (view->length == 3 && strncmp(view->data, "var", 3) == 0) {
    return REML_TOKEN_KW_VAR;
  }
  if (view->length == 3 && strncmp(view->data, "mut", 3) == 0) {
    return REML_TOKEN_KW_MUT;
  }
  if (view->length == 2 && strncmp(view->data, "fn", 2) == 0) {
    return REML_TOKEN_KW_FN;
  }
  if (view->length == 3 && strncmp(view->data, "pub", 3) == 0) {
    return REML_TOKEN_KW_PUB;
  }
  if (view->length == 3 && strncmp(view->data, "use", 3) == 0) {
    return REML_TOKEN_KW_USE;
  }
  if (view->length == 6 && strncmp(view->data, "module", 6) == 0) {
    return REML_TOKEN_KW_MODULE;
  }
  if (view->length == 4 && strncmp(view->data, "type", 4) == 0) {
    return REML_TOKEN_KW_TYPE;
  }
  return REML_TOKEN_IDENT;
}

void reml_lexer_init(reml_lexer *lexer, const char *input, size_t length) {
  lexer->input = input;
  lexer->length = length;
  lexer->index = 0;
  lexer->line = 1;
  lexer->column = 1;
  lexer->has_error = false;
  lexer->pending_error = false;
  lexer->error.message = NULL;
  lexer->error.span = reml_span_make(0, 0, 1, 1, 1, 1);

  reml_unicode_error error;
  if (!reml_unicode_validate_utf8(input, length, &error)) {
    const char *message = error.kind == REML_UNICODE_INVALID_SCALAR ? "unicode.invalid_scalar"
                                                                    : "unicode.invalid_utf8";
    reml_span span = reml_span_from_offsets(input, length, error.offset, error.offset + error.length);
    reml_lexer_set_error(lexer, message, span.start_offset, span.end_offset, span.start_line,
                         span.start_column, span.end_line, span.end_column);
    lexer->pending_error = true;
    return;
  }

  if (!reml_unicode_is_nfc(input, length, &error)) {
    reml_span span = reml_span_from_offsets(input, length, error.offset, error.offset + error.length);
    reml_lexer_set_error(lexer, "unicode.normalize.required", span.start_offset, span.end_offset,
                         span.start_line, span.start_column, span.end_line, span.end_column);
    lexer->pending_error = true;
  }
}

reml_token reml_lexer_next(reml_lexer *lexer) {
  if (lexer->pending_error) {
    lexer->pending_error = false;
    reml_token token;
    token.kind = REML_TOKEN_INVALID;
    token.span = lexer->error.span;
    token.lexeme =
        reml_string_view_make(lexer->input + token.span.start_offset,
                              token.span.end_offset > token.span.start_offset
                                  ? token.span.end_offset - token.span.start_offset
                                  : 0);
    return token;
  }

  reml_skip_whitespace_and_comments(lexer);

  size_t start_offset = lexer->index;
  int start_line = lexer->line;
  int start_column = lexer->column;

  if (lexer->index >= lexer->length) {
    return reml_make_token(REML_TOKEN_EOF, lexer, start_offset, start_line, start_column,
                           lexer->index, lexer->line, lexer->column);
  }

  int c = reml_peek_byte(lexer);

  if (c == '\r' || c == '\n') {
    reml_skip_whitespace_and_comments(lexer);
    return reml_lexer_next(lexer);
  }

  if (c == 'r') {
    size_t hashes = 0;
    if (reml_is_raw_string_start(lexer, &hashes)) {
      reml_advance_bytes(lexer, 1);
      for (size_t i = 0; i < hashes; ++i) {
        reml_advance_bytes(lexer, 1);
      }
      reml_advance_bytes(lexer, 1);
      while (lexer->index < lexer->length) {
        int d = reml_peek_byte(lexer);
        if (d == '"') {
          bool match = true;
          for (size_t i = 0; i < hashes; ++i) {
            if (lexer->index + 1 + i >= lexer->length ||
                lexer->input[lexer->index + 1 + i] != '#') {
              match = false;
              break;
            }
          }
          if (match) {
            reml_advance_bytes(lexer, 1 + hashes);
            return reml_make_token(REML_TOKEN_STRING_RAW, lexer, start_offset, start_line,
                                   start_column, lexer->index, lexer->line, lexer->column);
          }
        }
        if (d == '\n' || d == '\r') {
          reml_advance_line_break(lexer);
          continue;
        }
        if (d < 0x80) {
          reml_advance_bytes(lexer, 1);
        } else if (!reml_advance_grapheme(lexer)) {
          return reml_make_token(REML_TOKEN_INVALID, lexer, start_offset, start_line, start_column,
                                 lexer->index, lexer->line, lexer->column);
        }
      }
      reml_lexer_set_error(lexer, "unterminated raw string literal", start_offset, lexer->index,
                           start_line, start_column, lexer->line, lexer->column);
      return reml_make_token(REML_TOKEN_INVALID, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
    }
  }

  if (c == '"' && reml_peek_next_byte(lexer) == '"' &&
      lexer->index + 2 < lexer->length && lexer->input[lexer->index + 2] == '"') {
    reml_advance_bytes(lexer, 3);
    while (lexer->index < lexer->length) {
      int d = reml_peek_byte(lexer);
      if (d == '"' && lexer->index + 2 < lexer->length &&
          lexer->input[lexer->index + 1] == '"' && lexer->input[lexer->index + 2] == '"') {
        reml_advance_bytes(lexer, 3);
        return reml_make_token(REML_TOKEN_STRING_MULTILINE, lexer, start_offset, start_line,
                               start_column, lexer->index, lexer->line, lexer->column);
      }
      if (d == '\\') {
        size_t escape_offset = lexer->index;
        int escape_line = lexer->line;
        int escape_column = lexer->column;
        reml_advance_bytes(lexer, 1);
        int next = reml_peek_byte(lexer);
        if (next == 'u') {
          if (!reml_lex_unicode_escape(lexer, escape_offset, escape_line, escape_column)) {
            return reml_make_token(REML_TOKEN_INVALID, lexer, start_offset, start_line,
                                   start_column, lexer->index, lexer->line, lexer->column);
          }
          continue;
        }
        if (next == 'n' || next == 'r' || next == 't' || next == '\\' || next == '"' ||
            next == '\'' || next == '/') {
          reml_advance_bytes(lexer, 1);
          continue;
        }
        reml_lexer_set_error(lexer, "invalid escape sequence", escape_offset, lexer->index,
                             escape_line, escape_column, lexer->line, lexer->column);
        return reml_make_token(REML_TOKEN_INVALID, lexer, start_offset, start_line, start_column,
                               lexer->index, lexer->line, lexer->column);
      }
      if (d == '\n' || d == '\r') {
        reml_advance_line_break(lexer);
        continue;
      }
      if (d < 0x80) {
        reml_advance_bytes(lexer, 1);
      } else if (!reml_advance_grapheme(lexer)) {
        return reml_make_token(REML_TOKEN_INVALID, lexer, start_offset, start_line, start_column,
                               lexer->index, lexer->line, lexer->column);
      }
    }
    reml_lexer_set_error(lexer, "unterminated multiline string literal", start_offset,
                         lexer->index, start_line, start_column, lexer->line, lexer->column);
    return reml_make_token(REML_TOKEN_INVALID, lexer, start_offset, start_line, start_column,
                           lexer->index, lexer->line, lexer->column);
  }

  if (c == '"') {
    reml_advance_bytes(lexer, 1);
    while (lexer->index < lexer->length) {
      int d = reml_peek_byte(lexer);
      if (d == '\\') {
        size_t escape_offset = lexer->index;
        int escape_line = lexer->line;
        int escape_column = lexer->column;
        reml_advance_bytes(lexer, 1);
        int next = reml_peek_byte(lexer);
        if (next == 'u') {
          if (!reml_lex_unicode_escape(lexer, escape_offset, escape_line, escape_column)) {
            return reml_make_token(REML_TOKEN_INVALID, lexer, start_offset, start_line,
                                   start_column, lexer->index, lexer->line, lexer->column);
          }
          continue;
        }
        if (next == 'n' || next == 'r' || next == 't' || next == '\\' || next == '"' ||
            next == '\'' || next == '/') {
          reml_advance_bytes(lexer, 1);
          continue;
        }
        reml_lexer_set_error(lexer, "invalid escape sequence", escape_offset, lexer->index,
                             escape_line, escape_column, lexer->line, lexer->column);
        return reml_make_token(REML_TOKEN_INVALID, lexer, start_offset, start_line, start_column,
                               lexer->index, lexer->line, lexer->column);
      }
      if (d == '"') {
        reml_advance_bytes(lexer, 1);
        return reml_make_token(REML_TOKEN_STRING, lexer, start_offset, start_line, start_column,
                               lexer->index, lexer->line, lexer->column);
      }
      if (d == '\n' || d == '\r') {
        reml_lexer_set_error(lexer, "unterminated string literal", start_offset, lexer->index,
                             start_line, start_column, lexer->line, lexer->column);
        return reml_make_token(REML_TOKEN_INVALID, lexer, start_offset, start_line, start_column,
                               lexer->index, lexer->line, lexer->column);
      }
      if (d < 0x80) {
        reml_advance_bytes(lexer, 1);
      } else if (!reml_advance_grapheme(lexer)) {
        return reml_make_token(REML_TOKEN_INVALID, lexer, start_offset, start_line, start_column,
                               lexer->index, lexer->line, lexer->column);
      }
    }
    reml_lexer_set_error(lexer, "unterminated string literal", start_offset, lexer->index,
                         start_line, start_column, lexer->line, lexer->column);
    return reml_make_token(REML_TOKEN_INVALID, lexer, start_offset, start_line, start_column,
                           lexer->index, lexer->line, lexer->column);
  }

  if (c == '\'') {
    reml_advance_bytes(lexer, 1);
    if (lexer->index < lexer->length && reml_peek_byte(lexer) == '\\') {
      size_t escape_offset = lexer->index;
      int escape_line = lexer->line;
      int escape_column = lexer->column;
      reml_advance_bytes(lexer, 1);
      int next = reml_peek_byte(lexer);
      if (next == 'u') {
        if (!reml_lex_unicode_escape(lexer, escape_offset, escape_line, escape_column)) {
          return reml_make_token(REML_TOKEN_INVALID, lexer, start_offset, start_line,
                                 start_column, lexer->index, lexer->line, lexer->column);
        }
      } else if (next == 'n' || next == 'r' || next == 't' || next == '\\' || next == '"' ||
                 next == '\'' || next == '/') {
        reml_advance_bytes(lexer, 1);
      } else {
        reml_lexer_set_error(lexer, "invalid escape sequence", escape_offset, lexer->index,
                             escape_line, escape_column, lexer->line, lexer->column);
        return reml_make_token(REML_TOKEN_INVALID, lexer, start_offset, start_line, start_column,
                               lexer->index, lexer->line, lexer->column);
      }
    } else if (lexer->index < lexer->length) {
      int d = reml_peek_byte(lexer);
      if (d < 0x80) {
        reml_advance_bytes(lexer, 1);
      } else if (!reml_advance_codepoint(lexer, NULL, NULL)) {
        return reml_make_token(REML_TOKEN_INVALID, lexer, start_offset, start_line, start_column,
                               lexer->index, lexer->line, lexer->column);
      }
    }
    if (lexer->index < lexer->length && reml_peek_byte(lexer) == '\'') {
      reml_advance_bytes(lexer, 1);
      return reml_make_token(REML_TOKEN_CHAR, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
    }
    reml_lexer_set_error(lexer, "unterminated char literal", start_offset, lexer->index,
                         start_line, start_column, lexer->line, lexer->column);
    return reml_make_token(REML_TOKEN_INVALID, lexer, start_offset, start_line, start_column,
                           lexer->index, lexer->line, lexer->column);
  }

  if (reml_is_ascii_digit(c)) {
    bool is_float = false;
    int base = 10;
    reml_advance_bytes(lexer, 1);

    if (c == '0') {
      int n = reml_peek_byte(lexer);
      if (n == 'x' || n == 'X') {
        base = 16;
        reml_advance_bytes(lexer, 1);
      } else if (n == 'o' || n == 'O') {
        base = 8;
        reml_advance_bytes(lexer, 1);
      } else if (n == 'b' || n == 'B') {
        base = 2;
        reml_advance_bytes(lexer, 1);
      }
    }

    while (lexer->index < lexer->length) {
      int d = reml_peek_byte(lexer);
      if (d == '_') {
        reml_advance_bytes(lexer, 1);
        continue;
      }
      if (base == 16 && reml_is_ascii_hex(d)) {
        reml_advance_bytes(lexer, 1);
        continue;
      }
      if (base == 8 && reml_is_ascii_oct(d)) {
        reml_advance_bytes(lexer, 1);
        continue;
      }
      if (base == 2 && reml_is_ascii_bin(d)) {
        reml_advance_bytes(lexer, 1);
        continue;
      }
      if (base == 10 && reml_is_ascii_digit(d)) {
        reml_advance_bytes(lexer, 1);
        continue;
      }
      if (base == 10 && d == '.' && reml_peek_next_byte(lexer) != '.') {
        is_float = true;
        reml_advance_bytes(lexer, 1);
        continue;
      }
      if (base == 10 && (d == 'e' || d == 'E')) {
        is_float = true;
        reml_advance_bytes(lexer, 1);
        if (reml_peek_byte(lexer) == '+' || reml_peek_byte(lexer) == '-') {
          reml_advance_bytes(lexer, 1);
        }
        continue;
      }
      break;
    }

    return reml_make_token(is_float ? REML_TOKEN_FLOAT : REML_TOKEN_INT, lexer, start_offset,
                           start_line, start_column, lexer->index, lexer->line, lexer->column);
  }

  if (c < 0x80 ? (isalpha(c) || c == '_') : true) {
    size_t start = lexer->index;
    int32_t cp = 0;
    size_t bytes = 0;

    if (c < 0x80) {
      if (!reml_is_xid_start(c)) {
        reml_lexer_set_error(lexer, "invalid identifier start", start_offset, lexer->index + 1,
                             start_line, start_column, lexer->line, lexer->column + 1);
        reml_advance_bytes(lexer, 1);
        return reml_make_token(REML_TOKEN_INVALID, lexer, start_offset, start_line, start_column,
                               lexer->index, lexer->line, lexer->column);
      }
      reml_advance_bytes(lexer, 1);
    } else {
      if (!reml_advance_codepoint(lexer, &cp, &bytes)) {
        return reml_make_token(REML_TOKEN_INVALID, lexer, start_offset, start_line, start_column,
                               lexer->index, lexer->line, lexer->column);
      }
      if (!reml_is_xid_start(cp)) {
        reml_lexer_set_error(lexer, "invalid identifier start", start_offset, lexer->index,
                             start_line, start_column, lexer->line, lexer->column);
        return reml_make_token(REML_TOKEN_INVALID, lexer, start_offset, start_line, start_column,
                               lexer->index, lexer->line, lexer->column);
      }
    }

    while (lexer->index < lexer->length) {
      int d = reml_peek_byte(lexer);
      if (d < 0x80) {
        if (isalnum(d) || d == '_') {
          reml_advance_bytes(lexer, 1);
          continue;
        }
        break;
      }
      if (!reml_advance_codepoint(lexer, &cp, &bytes)) {
        break;
      }
      if (!reml_is_xid_continue(cp)) {
        lexer->index -= bytes;
        lexer->column -= 1;
        break;
      }
    }

    reml_string_view view =
        reml_string_view_make(lexer->input + start, lexer->index - start);
    reml_token_kind kind = reml_keyword_kind(&view);
    return reml_make_token(kind, lexer, start_offset, start_line, start_column, lexer->index,
                           lexer->line, lexer->column);
  }

  reml_advance_bytes(lexer, 1);

  switch (c) {
    case '(':
      return reml_make_token(REML_TOKEN_LPAREN, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
    case ')':
      return reml_make_token(REML_TOKEN_RPAREN, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
    case '{':
      return reml_make_token(REML_TOKEN_LBRACE, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
    case '}':
      return reml_make_token(REML_TOKEN_RBRACE, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
    case '[':
      return reml_make_token(REML_TOKEN_LBRACKET, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
    case ']':
      return reml_make_token(REML_TOKEN_RBRACKET, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
    case ',':
      return reml_make_token(REML_TOKEN_COMMA, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
    case ';':
      return reml_make_token(REML_TOKEN_SEMI, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
    case ':':
      if (reml_peek_byte(lexer) == '=') {
        reml_advance_bytes(lexer, 1);
        return reml_make_token(REML_TOKEN_COLONEQ, lexer, start_offset, start_line, start_column,
                               lexer->index, lexer->line, lexer->column);
      }
      return reml_make_token(REML_TOKEN_COLON, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
    case '?':
      return reml_make_token(REML_TOKEN_QUESTION, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
    case '.':
      if (reml_peek_byte(lexer) == '.') {
        reml_advance_bytes(lexer, 1);
        if (reml_peek_byte(lexer) == '=') {
          reml_advance_bytes(lexer, 1);
          return reml_make_token(REML_TOKEN_DOTDOTEQ, lexer, start_offset, start_line,
                                 start_column, lexer->index, lexer->line, lexer->column);
        }
        return reml_make_token(REML_TOKEN_DOTDOT, lexer, start_offset, start_line, start_column,
                               lexer->index, lexer->line, lexer->column);
      }
      return reml_make_token(REML_TOKEN_DOT, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
    case '+':
      return reml_make_token(REML_TOKEN_PLUS, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
    case '-':
      if (reml_peek_byte(lexer) == '>') {
        reml_advance_bytes(lexer, 1);
        return reml_make_token(REML_TOKEN_ARROW, lexer, start_offset, start_line, start_column,
                               lexer->index, lexer->line, lexer->column);
      }
      return reml_make_token(REML_TOKEN_MINUS, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
    case '*':
      return reml_make_token(REML_TOKEN_STAR, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
    case '/':
      return reml_make_token(REML_TOKEN_SLASH, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
    case '%':
      return reml_make_token(REML_TOKEN_PERCENT, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
    case '^':
      return reml_make_token(REML_TOKEN_CARET, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
    case '!':
      if (reml_peek_byte(lexer) == '=') {
        reml_advance_bytes(lexer, 1);
        return reml_make_token(REML_TOKEN_NOTEQ, lexer, start_offset, start_line, start_column,
                               lexer->index, lexer->line, lexer->column);
      }
      return reml_make_token(REML_TOKEN_BANG, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
    case '=':
      if (reml_peek_byte(lexer) == '=') {
        reml_advance_bytes(lexer, 1);
        return reml_make_token(REML_TOKEN_EQEQ, lexer, start_offset, start_line, start_column,
                               lexer->index, lexer->line, lexer->column);
      }
      return reml_make_token(REML_TOKEN_EQ, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
    case '<':
      if (reml_peek_byte(lexer) == '=') {
        reml_advance_bytes(lexer, 1);
        return reml_make_token(REML_TOKEN_LE, lexer, start_offset, start_line, start_column,
                               lexer->index, lexer->line, lexer->column);
      }
      return reml_make_token(REML_TOKEN_LT, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
    case '>':
      if (reml_peek_byte(lexer) == '=') {
        reml_advance_bytes(lexer, 1);
        return reml_make_token(REML_TOKEN_GE, lexer, start_offset, start_line, start_column,
                               lexer->index, lexer->line, lexer->column);
      }
      return reml_make_token(REML_TOKEN_GT, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
    case '&':
      if (reml_peek_byte(lexer) == '&') {
        reml_advance_bytes(lexer, 1);
        return reml_make_token(REML_TOKEN_LOGICAL_AND, lexer, start_offset, start_line,
                               start_column, lexer->index, lexer->line, lexer->column);
      }
      return reml_make_token(REML_TOKEN_AMP, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
    case '|':
      if (reml_peek_byte(lexer) == '|') {
        reml_advance_bytes(lexer, 1);
        return reml_make_token(REML_TOKEN_LOGICAL_OR, lexer, start_offset, start_line, start_column,
                               lexer->index, lexer->line, lexer->column);
      }
      if (reml_peek_byte(lexer) == '>') {
        reml_advance_bytes(lexer, 1);
        return reml_make_token(REML_TOKEN_PIPE_FORWARD, lexer, start_offset, start_line,
                               start_column, lexer->index, lexer->line, lexer->column);
      }
      return reml_make_token(REML_TOKEN_PIPE, lexer, start_offset, start_line, start_column,
                             lexer->index, lexer->line, lexer->column);
      break;
    default:
      break;
  }

  reml_lexer_set_error(lexer, "unexpected character", start_offset, lexer->index, start_line,
                       start_column, lexer->line, lexer->column);
  return reml_make_token(REML_TOKEN_INVALID, lexer, start_offset, start_line, start_column,
                         lexer->index, lexer->line, lexer->column);
}

const char *reml_token_kind_name(reml_token_kind kind) {
  switch (kind) {
    case REML_TOKEN_INVALID:
      return "INVALID";
    case REML_TOKEN_EOF:
      return "EOF";
    case REML_TOKEN_IDENT:
      return "IDENT";
    case REML_TOKEN_INT:
      return "INT";
    case REML_TOKEN_FLOAT:
      return "FLOAT";
    case REML_TOKEN_STRING:
      return "STRING";
    case REML_TOKEN_STRING_RAW:
      return "STRING_RAW";
    case REML_TOKEN_STRING_MULTILINE:
      return "STRING_MULTILINE";
    case REML_TOKEN_CHAR:
      return "CHAR";
    case REML_TOKEN_KW_RETURN:
      return "KW_RETURN";
    case REML_TOKEN_KW_TRUE:
      return "KW_TRUE";
    case REML_TOKEN_KW_FALSE:
      return "KW_FALSE";
    case REML_TOKEN_KW_IF:
      return "KW_IF";
    case REML_TOKEN_KW_WHILE:
      return "KW_WHILE";
    case REML_TOKEN_KW_WHEN:
      return "KW_WHEN";
    case REML_TOKEN_KW_THEN:
      return "KW_THEN";
    case REML_TOKEN_KW_ELSE:
      return "KW_ELSE";
    case REML_TOKEN_KW_MATCH:
      return "KW_MATCH";
    case REML_TOKEN_KW_WITH:
      return "KW_WITH";
    case REML_TOKEN_KW_LET:
      return "KW_LET";
    case REML_TOKEN_KW_VAR:
      return "KW_VAR";
    case REML_TOKEN_KW_MUT:
      return "KW_MUT";
    case REML_TOKEN_KW_FN:
      return "KW_FN";
    case REML_TOKEN_KW_PUB:
      return "KW_PUB";
    case REML_TOKEN_KW_USE:
      return "KW_USE";
    case REML_TOKEN_KW_MODULE:
      return "KW_MODULE";
    case REML_TOKEN_LPAREN:
      return "LPAREN";
    case REML_TOKEN_RPAREN:
      return "RPAREN";
    case REML_TOKEN_LBRACE:
      return "LBRACE";
    case REML_TOKEN_RBRACE:
      return "RBRACE";
    case REML_TOKEN_LBRACKET:
      return "LBRACKET";
    case REML_TOKEN_RBRACKET:
      return "RBRACKET";
    case REML_TOKEN_COMMA:
      return "COMMA";
    case REML_TOKEN_SEMI:
      return "SEMI";
    case REML_TOKEN_COLON:
      return "COLON";
    case REML_TOKEN_COLONEQ:
      return "COLONEQ";
    case REML_TOKEN_DOT:
      return "DOT";
    case REML_TOKEN_DOTDOTEQ:
      return "DOTDOTEQ";
    case REML_TOKEN_QUESTION:
      return "QUESTION";
    case REML_TOKEN_ARROW:
      return "ARROW";
    case REML_TOKEN_EQ:
      return "EQ";
    case REML_TOKEN_AMP:
      return "AMP";
    case REML_TOKEN_CARET:
      return "CARET";
    case REML_TOKEN_LOGICAL_AND:
      return "LOGICAL_AND";
    case REML_TOKEN_LOGICAL_OR:
      return "LOGICAL_OR";
    case REML_TOKEN_PIPE:
      return "PIPE";
    case REML_TOKEN_PIPE_FORWARD:
      return "PIPE_FORWARD";
    case REML_TOKEN_PLUS:
      return "PLUS";
    case REML_TOKEN_MINUS:
      return "MINUS";
    case REML_TOKEN_STAR:
      return "STAR";
    case REML_TOKEN_SLASH:
      return "SLASH";
    case REML_TOKEN_PERCENT:
      return "PERCENT";
    case REML_TOKEN_DOTDOT:
      return "DOTDOT";
    case REML_TOKEN_EQEQ:
      return "EQEQ";
    case REML_TOKEN_NOTEQ:
      return "NOTEQ";
    case REML_TOKEN_LT:
      return "LT";
    case REML_TOKEN_LE:
      return "LE";
    case REML_TOKEN_GT:
      return "GT";
    case REML_TOKEN_GE:
      return "GE";
    case REML_TOKEN_BANG:
      return "BANG";
    default:
      return "UNKNOWN";
  }
}
