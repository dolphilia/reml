#include "reml/sema/sema.h"

#include <errno.h>
#include <limits.h>
#include <stdlib.h>
#include <string.h>

#include <uthash.h>
#include <utarray.h>

typedef enum {
  REML_TRAIT_ADD,
  REML_TRAIT_SUB,
  REML_TRAIT_MUL,
  REML_TRAIT_DIV,
  REML_TRAIT_REM,
  REML_TRAIT_BITXOR,
  REML_TRAIT_EQ,
  REML_TRAIT_ORD
} reml_trait_kind;

typedef struct {
  reml_trait_kind trait;
  reml_type *left;
  reml_type *right;
  reml_type *result;
  reml_span span;
} reml_trait_constraint;

typedef enum {
  REML_SYMBOL_FUNC,
  REML_SYMBOL_VAR,
  REML_SYMBOL_TYPE,
  REML_SYMBOL_MODULE
} reml_symbol_kind;

typedef struct {
  reml_type *type;
  UT_array *generics;
  UT_array *effect_generics;
  UT_array *constraints;
} reml_scheme;

typedef struct reml_symbol {
  reml_symbol_kind kind;
  reml_string_view name;
  reml_span span;
  reml_scheme scheme;
  bool is_builtin;
  bool is_predeclared;
  bool is_mutable;
  uint32_t shared_borrows;
  bool mut_borrowed;
  reml_symbol_id id;
  UT_hash_handle hh;
} reml_symbol;

typedef struct reml_constructor_entry {
  reml_string_view name;
  reml_type *enum_type;
  reml_enum_variant *variant;
  UT_hash_handle hh;
} reml_constructor_entry;

typedef struct reml_enum_decl_entry {
  reml_string_view name;
  reml_type *enum_type;
  UT_hash_handle hh;
} reml_enum_decl_entry;

typedef struct {
  reml_symbol *symbols;
  UT_array *borrows;
} reml_scope;

typedef struct {
  reml_symbol *symbol;
  bool is_mutable;
} reml_borrow_record;

struct reml_symbol_table {
  UT_array *scopes;
  reml_symbol_id next_id;
};

static void reml_scheme_init(reml_scheme *scheme, reml_type *type) {
  if (!scheme) {
    return;
  }
  scheme->type = type;
  UT_icd id_icd = {sizeof(uint32_t), NULL, NULL, NULL};
  utarray_new(scheme->generics, &id_icd);
  utarray_new(scheme->effect_generics, &id_icd);
  UT_icd constraint_icd = {sizeof(reml_trait_constraint), NULL, NULL, NULL};
  utarray_new(scheme->constraints, &constraint_icd);
}

static void reml_scheme_reset(reml_scheme *scheme, reml_type *type) {
  if (!scheme) {
    return;
  }
  if (scheme->generics) {
    utarray_clear(scheme->generics);
  }
  if (scheme->effect_generics) {
    utarray_clear(scheme->effect_generics);
  }
  if (scheme->constraints) {
    utarray_clear(scheme->constraints);
  }
  scheme->type = type;
}

static void reml_scheme_deinit(reml_scheme *scheme) {
  if (!scheme) {
    return;
  }
  if (scheme->generics) {
    utarray_free(scheme->generics);
    scheme->generics = NULL;
  }
  if (scheme->effect_generics) {
    utarray_free(scheme->effect_generics);
    scheme->effect_generics = NULL;
  }
  if (scheme->constraints) {
    utarray_free(scheme->constraints);
    scheme->constraints = NULL;
  }
  scheme->type = NULL;
}

static reml_scope *reml_scope_new(void) {
  reml_scope *scope = (reml_scope *)calloc(1, sizeof(reml_scope));
  if (!scope) {
    return NULL;
  }
  scope->symbols = NULL;
  scope->borrows = NULL;
  UT_icd borrow_icd = {sizeof(reml_borrow_record), NULL, NULL, NULL};
  utarray_new(scope->borrows, &borrow_icd);
  return scope;
}

static void reml_scope_release_borrows(reml_scope *scope) {
  if (!scope || !scope->borrows) {
    return;
  }
  for (reml_borrow_record *it = (reml_borrow_record *)utarray_front(scope->borrows); it != NULL;
       it = (reml_borrow_record *)utarray_next(scope->borrows, it)) {
    if (!it->symbol) {
      continue;
    }
    if (it->is_mutable) {
      it->symbol->mut_borrowed = false;
    } else if (it->symbol->shared_borrows > 0) {
      it->symbol->shared_borrows -= 1;
    }
  }
  utarray_clear(scope->borrows);
}

static void reml_scope_free(reml_scope *scope) {
  if (!scope) {
    return;
  }
  reml_scope_release_borrows(scope);
  if (scope->borrows) {
    utarray_free(scope->borrows);
    scope->borrows = NULL;
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

static void reml_scope_record_borrow(reml_scope *scope, reml_symbol *symbol, bool is_mutable) {
  if (!scope || !scope->borrows || !symbol) {
    return;
  }
  reml_borrow_record record = {.symbol = symbol, .is_mutable = is_mutable};
  utarray_push_back(scope->borrows, &record);
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

static bool reml_symbol_table_is_top_level(reml_symbol_table *table) {
  return table && table->scopes && utarray_len(table->scopes) == 1;
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
                                             reml_type *type, bool is_builtin, bool is_predeclared,
                                             bool is_mutable) {
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
  symbol->is_mutable = is_mutable;
  symbol->shared_borrows = 0;
  symbol->mut_borrowed = false;
  symbol->id = table->next_id++;
  reml_scheme_init(&symbol->scheme, type);
  HASH_ADD_KEYPTR(hh, scope->symbols, symbol->name.data, symbol->name.length, symbol);
  return symbol;
}

static reml_constructor_entry *reml_constructor_lookup(reml_sema *sema, reml_string_view name) {
  if (!sema) {
    return NULL;
  }
  reml_constructor_entry *entry = NULL;
  HASH_FIND(hh, sema->constructors, name.data, name.length, entry);
  return entry;
}

static reml_enum_decl_entry *reml_enum_decl_lookup(reml_sema *sema, reml_string_view name) {
  if (!sema) {
    return NULL;
  }
  reml_enum_decl_entry *entry = NULL;
  HASH_FIND(hh, sema->enum_decls, name.data, name.length, entry);
  return entry;
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

static bool reml_string_view_equal(reml_string_view left, reml_string_view right) {
  if (left.length != right.length) {
    return false;
  }
  if (left.length == 0) {
    return true;
  }
  return memcmp(left.data, right.data, left.length) == 0;
}

static bool reml_literal_equal(reml_literal left, reml_literal right) {
  if (left.kind != right.kind) {
    return false;
  }
  if (left.kind == REML_LITERAL_STRING && left.string_kind != right.string_kind) {
    return false;
  }
  return reml_string_view_equal(left.text, right.text);
}

static reml_enum_variant *reml_enum_variant_find(UT_array *variants, reml_string_view name) {
  if (!variants) {
    return NULL;
  }
  for (reml_enum_variant *it = (reml_enum_variant *)utarray_front(variants); it != NULL;
       it = (reml_enum_variant *)utarray_next(variants, it)) {
    if (reml_string_view_equal(it->name, name)) {
      return it;
    }
  }
  return NULL;
}

static reml_enum_variant *reml_enum_variant_add(reml_type_ctx *ctx, reml_type *enum_type,
                                                 reml_string_view name, size_t field_count) {
  if (!enum_type || enum_type->kind != REML_TYPE_ENUM) {
    return NULL;
  }
  if (!enum_type->data.enum_type.variants) {
    UT_icd variant_icd = {sizeof(reml_enum_variant), NULL, NULL, NULL};
    utarray_new(enum_type->data.enum_type.variants, &variant_icd);
  }
  reml_enum_variant variant;
  variant.name = name;
  variant.tag = (int32_t)utarray_len(enum_type->data.enum_type.variants);
  variant.fields = NULL;
  if (field_count > 0) {
    UT_icd field_icd = {sizeof(reml_type *), NULL, NULL, NULL};
    utarray_new(variant.fields, &field_icd);
    for (size_t i = 0; i < field_count; ++i) {
      reml_type *field_type = reml_type_make_var(ctx);
      utarray_push_back(variant.fields, &field_type);
    }
  }
  utarray_push_back(enum_type->data.enum_type.variants, &variant);
  return reml_enum_variant_find(enum_type->data.enum_type.variants, name);
}

static size_t reml_enum_variant_count(reml_type *enum_type) {
  if (!enum_type || enum_type->kind != REML_TYPE_ENUM || !enum_type->data.enum_type.variants) {
    return 0;
  }
  return utarray_len(enum_type->data.enum_type.variants);
}

static int reml_string_view_cmp(const void *left, const void *right) {
  const reml_record_field *a = (const reml_record_field *)left;
  const reml_record_field *b = (const reml_record_field *)right;
  size_t len = a->name.length < b->name.length ? a->name.length : b->name.length;
  if (len > 0) {
    int cmp = memcmp(a->name.data, b->name.data, len);
    if (cmp != 0) {
      return cmp;
    }
  }
  if (a->name.length < b->name.length) {
    return -1;
  }
  if (a->name.length > b->name.length) {
    return 1;
  }
  return 0;
}

static void reml_record_fields_sort(UT_array *fields) {
  if (!fields || utarray_len(fields) == 0) {
    return;
  }
  reml_record_field *data = (reml_record_field *)utarray_front(fields);
  size_t count = utarray_len(fields);
  qsort(data, count, sizeof(reml_record_field), reml_string_view_cmp);
}

static reml_record_field *reml_record_field_find(reml_type *record, reml_string_view name) {
  if (!record || record->kind != REML_TYPE_RECORD || !record->data.record.fields) {
    return NULL;
  }
  for (reml_record_field *it =
           (reml_record_field *)utarray_front(record->data.record.fields);
       it != NULL;
       it = (reml_record_field *)utarray_next(record->data.record.fields, it)) {
    if (reml_string_view_equal(it->name, name)) {
      return it;
    }
  }
  return NULL;
}

static bool reml_pattern_fields_contains(UT_array *fields, reml_string_view name) {
  if (!fields) {
    return false;
  }
  for (reml_pattern_field *it = (reml_pattern_field *)utarray_front(fields); it != NULL;
       it = (reml_pattern_field *)utarray_next(fields, it)) {
    if (reml_string_view_equal(it->name, name)) {
      return true;
    }
  }
  return false;
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

static bool reml_parse_int_literal(reml_literal literal, int64_t *out_value) {
  if (!out_value) {
    return false;
  }
  char *text = reml_strip_numeric_literal(literal.text);
  if (!text) {
    return false;
  }
  errno = 0;
  char *end = NULL;
  long long value = strtoll(text, &end, 0);
  bool ok = (errno == 0 && end != NULL && *end == '\0');
  free(text);
  if (!ok) {
    return false;
  }
  *out_value = (int64_t)value;
  return true;
}

typedef struct {
  int64_t start;
  int64_t end;
} reml_int_interval;

static int reml_int_interval_cmp(const void *left, const void *right) {
  const reml_int_interval *a = (const reml_int_interval *)left;
  const reml_int_interval *b = (const reml_int_interval *)right;
  if (a->start < b->start) {
    return -1;
  }
  if (a->start > b->start) {
    return 1;
  }
  if (a->end < b->end) {
    return -1;
  }
  if (a->end > b->end) {
    return 1;
  }
  return 0;
}

static bool reml_interval_covers(UT_array *intervals, int64_t start, int64_t end) {
  if (!intervals) {
    return false;
  }
  for (reml_int_interval *it = (reml_int_interval *)utarray_front(intervals); it != NULL;
       it = (reml_int_interval *)utarray_next(intervals, it)) {
    if (it->start <= start && it->end >= end) {
      return true;
    }
  }
  return false;
}

static void reml_interval_insert(UT_array *intervals, int64_t start, int64_t end) {
  if (!intervals) {
    return;
  }
  reml_int_interval interval = {.start = start, .end = end};
  utarray_push_back(intervals, &interval);
  size_t count = utarray_len(intervals);
  if (count <= 1) {
    return;
  }
  reml_int_interval *data = (reml_int_interval *)utarray_front(intervals);
  qsort(data, count, sizeof(reml_int_interval), reml_int_interval_cmp);
  size_t write = 0;
  for (size_t i = 0; i < count; ++i) {
    reml_int_interval current = data[i];
    if (write == 0) {
      data[write++] = current;
      continue;
    }
    reml_int_interval *last = &data[write - 1];
    if (current.start <= last->end + 1) {
      if (current.end > last->end) {
        last->end = current.end;
      }
    } else {
      data[write++] = current;
    }
  }
  while (utarray_len(intervals) > write) {
    utarray_pop_back(intervals);
  }
}

static reml_string_view reml_string_view_from_cstr(const char *text) {
  return reml_string_view_make(text, text ? strlen(text) : 0);
}

static reml_diagnostic_pattern *reml_pattern_extension_new(void) {
  reml_diagnostic_pattern *pattern =
      (reml_diagnostic_pattern *)calloc(1, sizeof(reml_diagnostic_pattern));
  if (!pattern) {
    return NULL;
  }
  pattern->missing_variants = NULL;
  pattern->missing_ranges = NULL;
  return pattern;
}

static void reml_pattern_extension_add_variant(reml_diagnostic_pattern *pattern,
                                               reml_string_view name) {
  if (!pattern) {
    return;
  }
  if (!pattern->missing_variants) {
    UT_icd variant_icd = {sizeof(reml_string_view), NULL, NULL, NULL};
    utarray_new(pattern->missing_variants, &variant_icd);
  }
  utarray_push_back(pattern->missing_variants, &name);
}

static void reml_pattern_extension_add_range(reml_diagnostic_pattern *pattern, int64_t start,
                                             int64_t end, bool inclusive) {
  if (!pattern) {
    return;
  }
  if (!pattern->missing_ranges) {
    UT_icd range_icd = {sizeof(reml_diagnostic_range), NULL, NULL, NULL};
    utarray_new(pattern->missing_ranges, &range_icd);
  }
  reml_diagnostic_range range = {.start = start, .end = end, .inclusive = inclusive};
  utarray_push_back(pattern->missing_ranges, &range);
}

static void reml_pattern_extension_add_missing_ranges(reml_diagnostic_pattern *pattern,
                                                      UT_array *intervals) {
  if (!pattern) {
    return;
  }
  if (!intervals || utarray_len(intervals) == 0) {
    reml_pattern_extension_add_range(pattern, (int64_t)INT64_MIN, (int64_t)INT64_MAX, true);
    return;
  }
  int64_t cursor = (int64_t)INT64_MIN;
  bool covers_end = false;
  for (reml_int_interval *it = (reml_int_interval *)utarray_front(intervals); it != NULL;
       it = (reml_int_interval *)utarray_next(intervals, it)) {
    if (cursor < it->start) {
      int64_t gap_end = it->start - 1;
      reml_pattern_extension_add_range(pattern, cursor, gap_end, true);
    }
    if (it->end == (int64_t)INT64_MAX) {
      covers_end = true;
      break;
    }
    cursor = it->end + 1;
  }
  if (!covers_end && cursor <= (int64_t)INT64_MAX) {
    reml_pattern_extension_add_range(pattern, cursor, (int64_t)INT64_MAX, true);
  }
}

static bool reml_type_is_bool(reml_type *type) {
  type = type ? reml_type_prune(type) : NULL;
  return type && type->kind == REML_TYPE_BOOL;
}

static bool reml_pattern_is_bind_all(const reml_pattern *pattern) {
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
      if (!reml_pattern_is_bind_all(*it)) {
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
      if (!reml_pattern_is_bind_all(it->pattern)) {
        return false;
      }
    }
    return true;
  }
  return false;
}

static bool reml_pattern_is_catch_all(const reml_pattern *pattern, reml_type *scrutinee) {
  if (!pattern) {
    return false;
  }
  if (pattern->kind == REML_PATTERN_WILDCARD || pattern->kind == REML_PATTERN_IDENT) {
    return true;
  }
  scrutinee = scrutinee ? reml_type_prune(scrutinee) : NULL;
  if (pattern->kind == REML_PATTERN_TUPLE &&
      scrutinee && scrutinee->kind == REML_TYPE_TUPLE) {
    return reml_pattern_is_bind_all(pattern);
  }
  if (pattern->kind == REML_PATTERN_RECORD &&
      scrutinee && scrutinee->kind == REML_TYPE_RECORD) {
    return reml_pattern_is_bind_all(pattern);
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
    if (!reml_pattern_is_bind_all(*it)) {
      return false;
    }
  }
  return true;
}

static bool reml_pattern_is_bool_literal(const reml_pattern *pattern, bool *out_value) {
  if (!pattern || pattern->kind != REML_PATTERN_LITERAL) {
    return false;
  }
  if (pattern->data.literal.kind != REML_LITERAL_BOOL) {
    return false;
  }
  bool value = pattern->data.literal.text.length > 0 && pattern->data.literal.text.data[0] == 't';
  if (out_value) {
    *out_value = value;
  }
  return true;
}

static bool reml_match_literal_seen(UT_array *seen, reml_literal literal) {
  if (!seen) {
    return false;
  }
  for (reml_literal *it = (reml_literal *)utarray_front(seen); it != NULL;
       it = (reml_literal *)utarray_next(seen, it)) {
    if (reml_literal_equal(*it, literal)) {
      return true;
    }
  }
  utarray_push_back(seen, &literal);
  return false;
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
  if (type->kind == REML_TYPE_RECORD && type->data.record.fields) {
    for (reml_record_field *it =
             (reml_record_field *)utarray_front(type->data.record.fields);
         it != NULL;
         it = (reml_record_field *)utarray_next(type->data.record.fields, it)) {
      reml_type_collect_vars(it->type, vars);
    }
  }
  if (type->kind == REML_TYPE_ENUM && type->data.enum_type.variants) {
    for (reml_enum_variant *it =
             (reml_enum_variant *)utarray_front(type->data.enum_type.variants);
         it != NULL; it = (reml_enum_variant *)utarray_next(type->data.enum_type.variants, it)) {
      if (!it->fields) {
        continue;
      }
      for (reml_type **field = (reml_type **)utarray_front(it->fields); field != NULL;
           field = (reml_type **)utarray_next(it->fields, field)) {
        reml_type_collect_vars(*field, vars);
      }
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
  if (type->kind == REML_TYPE_REF) {
    reml_type_collect_vars(type->data.ref.target, vars);
  }
}

static reml_effect_row reml_effect_row_resolve(reml_effect_row row) {
  if (row.tail && row.tail->instance) {
    reml_effect_row resolved = reml_effect_row_resolve(*row.tail->instance);
    row.effects = reml_effect_union(row.effects, resolved.effects);
    row.tail = resolved.tail;
  }
  return row;
}

static void reml_effect_row_collect_vars_from_row(reml_effect_row row, UT_array *vars) {
  if (!vars) {
    return;
  }
  row = reml_effect_row_resolve(row);
  if (row.tail) {
    reml_var_ids_push_unique(vars, row.tail->id);
  }
}

static void reml_effect_row_collect_vars(reml_type *type, UT_array *vars) {
  if (!type || !vars) {
    return;
  }
  type = reml_type_prune(type);
  if (!type) {
    return;
  }
  if (type->kind == REML_TYPE_TUPLE && type->data.tuple.items) {
    for (reml_type **it = (reml_type **)utarray_front(type->data.tuple.items); it != NULL;
         it = (reml_type **)utarray_next(type->data.tuple.items, it)) {
      reml_effect_row_collect_vars(*it, vars);
    }
  }
  if (type->kind == REML_TYPE_RECORD && type->data.record.fields) {
    for (reml_record_field *it =
             (reml_record_field *)utarray_front(type->data.record.fields);
         it != NULL;
         it = (reml_record_field *)utarray_next(type->data.record.fields, it)) {
      reml_effect_row_collect_vars(it->type, vars);
    }
  }
  if (type->kind == REML_TYPE_ENUM && type->data.enum_type.variants) {
    for (reml_enum_variant *it =
             (reml_enum_variant *)utarray_front(type->data.enum_type.variants);
         it != NULL; it = (reml_enum_variant *)utarray_next(type->data.enum_type.variants, it)) {
      if (!it->fields) {
        continue;
      }
      for (reml_type **field = (reml_type **)utarray_front(it->fields); field != NULL;
           field = (reml_type **)utarray_next(it->fields, field)) {
        reml_effect_row_collect_vars(*field, vars);
      }
    }
  }
  if (type->kind == REML_TYPE_FUNCTION) {
    if (type->data.function.params) {
      for (reml_type **it = (reml_type **)utarray_front(type->data.function.params); it != NULL;
           it = (reml_type **)utarray_next(type->data.function.params, it)) {
        reml_effect_row_collect_vars(*it, vars);
      }
    }
    reml_effect_row_collect_vars(type->data.function.result, vars);
    reml_effect_row_collect_vars_from_row(type->data.function.effects, vars);
  }
  if (type->kind == REML_TYPE_REF) {
    reml_effect_row_collect_vars(type->data.ref.target, vars);
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

static void reml_scheme_collect_free_effect_vars(const reml_scheme *scheme, UT_array *vars) {
  if (!scheme || !vars) {
    return;
  }
  UT_icd tmp_icd = {sizeof(uint32_t), NULL, NULL, NULL};
  UT_array *all_vars = NULL;
  utarray_new(all_vars, &tmp_icd);
  reml_effect_row_collect_vars(scheme->type, all_vars);
  for (uint32_t *it = (uint32_t *)utarray_front(all_vars); it != NULL;
       it = (uint32_t *)utarray_next(all_vars, it)) {
    if (!reml_var_ids_contains(scheme->effect_generics, *it)) {
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

static void reml_env_collect_free_effect_vars(reml_symbol_table *table, const reml_symbol *skip,
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
      reml_scheme_collect_free_effect_vars(&sym->scheme, vars);
    }
  }
}

typedef struct {
  uint32_t id;
  reml_type *replacement;
} reml_type_subst;

typedef struct {
  uint32_t id;
  reml_effect_row_var *replacement;
} reml_effect_row_subst;

static reml_type *reml_type_instantiate_inner(reml_type_ctx *ctx, reml_type *type,
                                              UT_array *generics, UT_array *substs,
                                              UT_array *effect_generics,
                                              UT_array *effect_substs);

static reml_effect_row_var *reml_effect_row_apply_subst(reml_type_ctx *ctx, reml_effect_row_var *var,
                                                        UT_array *effect_generics,
                                                        UT_array *effect_substs) {
  if (!var) {
    return NULL;
  }
  if (!effect_generics || !reml_var_ids_contains(effect_generics, var->id)) {
    return var;
  }
  for (reml_effect_row_subst *it =
           (reml_effect_row_subst *)utarray_front(effect_substs);
       it != NULL;
       it = (reml_effect_row_subst *)utarray_next(effect_substs, it)) {
    if (it->id == var->id) {
      return it->replacement;
    }
  }
  reml_effect_row_var *fresh = reml_effect_row_var_make(ctx);
  reml_effect_row_subst subst = {.id = var->id, .replacement = fresh};
  utarray_push_back(effect_substs, &subst);
  return fresh;
}

static reml_effect_row reml_effect_row_instantiate(reml_type_ctx *ctx, reml_effect_row row,
                                                   UT_array *effect_generics,
                                                   UT_array *effect_substs) {
  if (!row.tail) {
    return row;
  }
  reml_effect_row resolved = reml_effect_row_resolve(row);
  reml_effect_row_var *tail =
      reml_effect_row_apply_subst(ctx, resolved.tail, effect_generics, effect_substs);
  return reml_effect_row_make(resolved.effects, tail);
}

static UT_array *reml_type_instantiate_items(reml_type_ctx *ctx, UT_array *items,
                                             UT_array *generics, UT_array *substs,
                                             UT_array *effect_generics,
                                             UT_array *effect_substs) {
  if (!items) {
    return NULL;
  }
  UT_icd item_icd = {sizeof(reml_type *), NULL, NULL, NULL};
  UT_array *out = NULL;
  utarray_new(out, &item_icd);
  for (reml_type **it = (reml_type **)utarray_front(items); it != NULL;
       it = (reml_type **)utarray_next(items, it)) {
    reml_type *item = reml_type_instantiate_inner(ctx, *it, generics, substs, effect_generics,
                                                  effect_substs);
    utarray_push_back(out, &item);
  }
  return out;
}

static UT_array *reml_type_instantiate_fields(reml_type_ctx *ctx, UT_array *fields,
                                              UT_array *generics, UT_array *substs,
                                              UT_array *effect_generics,
                                              UT_array *effect_substs) {
  if (!fields) {
    return NULL;
  }
  UT_icd field_icd = {sizeof(reml_record_field), NULL, NULL, NULL};
  UT_array *out = NULL;
  utarray_new(out, &field_icd);
  for (reml_record_field *it = (reml_record_field *)utarray_front(fields); it != NULL;
       it = (reml_record_field *)utarray_next(fields, it)) {
    reml_record_field field = {.name = it->name,
                               .type = reml_type_instantiate_inner(ctx, it->type, generics, substs,
                                                                   effect_generics, effect_substs)};
    utarray_push_back(out, &field);
  }
  return out;
}

static reml_type *reml_type_instantiate_inner(reml_type_ctx *ctx, reml_type *type,
                                              UT_array *generics, UT_array *substs,
                                              UT_array *effect_generics,
                                              UT_array *effect_substs) {
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
  if (type->kind == REML_TYPE_TUPLE) {
    UT_array *items = reml_type_instantiate_items(ctx, type->data.tuple.items, generics, substs,
                                                  effect_generics, effect_substs);
    return reml_type_make_tuple(ctx, items);
  }
  if (type->kind == REML_TYPE_RECORD) {
    UT_array *fields = reml_type_instantiate_fields(ctx, type->data.record.fields, generics, substs,
                                                    effect_generics, effect_substs);
    return reml_type_make_record(ctx, fields);
  }
  if (type->kind == REML_TYPE_FUNCTION) {
    UT_array *params = reml_type_instantiate_items(ctx, type->data.function.params, generics,
                                                   substs, effect_generics, effect_substs);
    reml_type *result = reml_type_instantiate_inner(ctx, type->data.function.result, generics,
                                                    substs, effect_generics, effect_substs);
    reml_effect_row effects =
        reml_effect_row_instantiate(ctx, type->data.function.effects, effect_generics,
                                    effect_substs);
    return reml_type_make_function(ctx, params, result, effects);
  }
  if (type->kind == REML_TYPE_REF) {
    reml_type *target = reml_type_instantiate_inner(ctx, type->data.ref.target, generics, substs,
                                                    effect_generics, effect_substs);
    return reml_type_make_ref(ctx, target, type->data.ref.is_mutable);
  }
  return type;
}

static reml_type *reml_type_apply_substs(reml_type *type, UT_array *substs) {
  type = reml_type_prune(type);
  if (!type || !substs) {
    return type;
  }
  if (type->kind != REML_TYPE_VAR) {
    return type;
  }
  for (reml_type_subst *it = (reml_type_subst *)utarray_front(substs); it != NULL;
       it = (reml_type_subst *)utarray_next(substs, it)) {
    if (it->id == type->data.var.id) {
      return it->replacement;
    }
  }
  return type;
}

static reml_type *reml_type_instantiate_with_substs(reml_type_ctx *ctx, const reml_scheme *scheme,
                                                     UT_array **out_substs) {
  if (out_substs) {
    *out_substs = NULL;
  }
  if (!scheme || !scheme->type) {
    return NULL;
  }
  bool has_type_generics = scheme->generics && utarray_len(scheme->generics) > 0;
  bool has_effect_generics = scheme->effect_generics && utarray_len(scheme->effect_generics) > 0;
  if (!has_type_generics && !has_effect_generics) {
    return scheme->type;
  }
  UT_icd subst_icd = {sizeof(reml_type_subst), NULL, NULL, NULL};
  UT_array *substs = NULL;
  utarray_new(substs, &subst_icd);
  UT_icd effect_subst_icd = {sizeof(reml_effect_row_subst), NULL, NULL, NULL};
  UT_array *effect_substs = NULL;
  utarray_new(effect_substs, &effect_subst_icd);
  reml_type *result = reml_type_instantiate_inner(ctx, scheme->type, scheme->generics, substs,
                                                  scheme->effect_generics, effect_substs);
  if (effect_substs) {
    utarray_free(effect_substs);
  }
  if (out_substs) {
    *out_substs = substs;
  } else {
    utarray_free(substs);
  }
  return result;
}

static reml_type *reml_type_instantiate(reml_type_ctx *ctx, const reml_scheme *scheme) {
  UT_array *substs = NULL;
  reml_type *result = reml_type_instantiate_with_substs(ctx, scheme, &substs);
  if (substs) {
    utarray_free(substs);
  }
  return result;
}

static void reml_report_diag(reml_sema *sema, reml_diagnostic_code code, reml_span span,
                             const char *message) {
  if (!sema) {
    return;
  }
  reml_diagnostic diag = {
      .code = code, .span = span, .message = message, .pattern = NULL, .effect = NULL};
  reml_diagnostics_push(&sema->diagnostics, diag);
}

static void reml_check_effect_contract(reml_sema *sema, const reml_attr_list *attrs,
                                       reml_effect_set effects) {
  if (!sema || !attrs) {
    return;
  }
  if (attrs->is_pure && effects != REML_EFFECT_NONE) {
    reml_report_diag(sema, REML_DIAG_EFFECT_VIOLATION, attrs->pure_span,
                     "effect violation: @pure requires no effects");
  }
  if (attrs->is_no_panic && (effects & REML_EFFECT_PANIC) != 0) {
    reml_report_diag(sema, REML_DIAG_EFFECT_VIOLATION, attrs->no_panic_span,
                     "effect violation: @no_panic forbids panic");
  }
}

static bool reml_effect_row_is_closed(reml_effect_row row) {
  row = reml_effect_row_resolve(row);
  return row.tail == NULL;
}

static void reml_report_effect_mismatch(reml_sema *sema, reml_span span, reml_effect_row expected,
                                        reml_effect_row actual) {
  if (!sema) {
    return;
  }
  reml_effect_row resolved_expected = reml_effect_row_resolve(expected);
  reml_effect_row resolved_actual = reml_effect_row_resolve(actual);
  reml_diagnostic_effect *effect =
      (reml_diagnostic_effect *)calloc(1, sizeof(reml_diagnostic_effect));
  if (effect) {
    effect->expected_effects = resolved_expected.effects;
    effect->actual_effects = resolved_actual.effects;
    effect->expected_open = resolved_expected.tail != NULL;
    effect->actual_open = resolved_actual.tail != NULL;
    if (resolved_expected.tail) {
      effect->expected_row_var = resolved_expected.tail->id;
    }
    if (resolved_actual.tail) {
      effect->actual_row_var = resolved_actual.tail->id;
    }
  }
  reml_diagnostic diag = {.code = REML_DIAG_EFFECT_CONTRACT_MISMATCH,
                          .span = span,
                          .message = "effect contract mismatch",
                          .pattern = NULL,
                          .effect = effect};
  reml_diagnostics_push(&sema->diagnostics, diag);
}

static void reml_register_type_decl(reml_sema *sema, const reml_type_decl *decl, reml_span span) {
  if (!sema || !decl) {
    return;
  }
  if (reml_enum_decl_lookup(sema, decl->name)) {
    reml_report_diag(sema, REML_DIAG_DUPLICATE_SYMBOL, span, "duplicate type declaration");
    return;
  }
  reml_type *enum_type = reml_type_make_enum(&sema->types);
  if (!enum_type) {
    return;
  }
  if (decl->variants) {
    for (reml_type_decl_variant *it =
             (reml_type_decl_variant *)utarray_front(decl->variants);
         it != NULL;
         it = (reml_type_decl_variant *)utarray_next(decl->variants, it)) {
      if (reml_constructor_lookup(sema, it->name)) {
        reml_report_diag(sema, REML_DIAG_DUPLICATE_SYMBOL, span,
                         "duplicate constructor declaration");
        continue;
      }
      size_t field_count = it->fields ? utarray_len(it->fields) : 0;
      reml_enum_variant *variant =
          reml_enum_variant_add(&sema->types, enum_type, it->name, field_count);
      if (!variant) {
        continue;
      }
      reml_constructor_entry *entry =
          (reml_constructor_entry *)calloc(1, sizeof(reml_constructor_entry));
      if (!entry) {
        continue;
      }
      entry->name = it->name;
      entry->enum_type = enum_type;
      entry->variant = variant;
      HASH_ADD_KEYPTR(hh, sema->constructors, entry->name.data, entry->name.length, entry);
    }
  }
  reml_enum_decl_entry *decl_entry =
      (reml_enum_decl_entry *)calloc(1, sizeof(reml_enum_decl_entry));
  if (!decl_entry) {
    return;
  }
  decl_entry->name = decl->name;
  decl_entry->enum_type = enum_type;
  HASH_ADD_KEYPTR(hh, sema->enum_decls, decl_entry->name.data, decl_entry->name.length,
                  decl_entry);
}

static bool reml_expect_type(reml_sema *sema, reml_type *actual, reml_type *expected,
                             reml_span span) {
  if (reml_type_unify(&sema->types, actual, expected)) {
    return true;
  }
  reml_type *left = reml_type_prune(actual);
  reml_type *right = reml_type_prune(expected);
  if (left && right && left->kind == REML_TYPE_FUNCTION && right->kind == REML_TYPE_FUNCTION) {
    if (reml_effect_row_is_closed(left->data.function.effects) &&
        reml_effect_row_is_closed(right->data.function.effects) &&
        left->data.function.effects.effects != right->data.function.effects.effects) {
      reml_report_effect_mismatch(sema, span, right->data.function.effects,
                                  left->data.function.effects);
      return false;
    }
  }
  reml_report_diag(sema, REML_DIAG_TYPE_MISMATCH, span, "type mismatch");
  return false;
}

static reml_type *reml_infer_expr(reml_sema *sema, reml_expr *expr, reml_effect_set *effect);
static void reml_check_pattern(reml_sema *sema, reml_pattern *pattern, reml_type *expected,
                               reml_effect_set *effect, bool allow_define, bool is_mutable,
                               size_t constraint_start, size_t constraint_end);
static reml_symbol *reml_symbol_from_ident(reml_sema *sema, reml_expr *expr) {
  if (!sema || !expr || expr->kind != REML_EXPR_IDENT) {
    return NULL;
  }
  return reml_symbol_table_lookup(sema->symbols, expr->data.ident);
}

static reml_type *reml_infer_literal(reml_sema *sema, reml_literal literal) {
  switch (literal.kind) {
    case REML_LITERAL_INT: {
      reml_type *literal_type = reml_type_make_var(&sema->types);
      if (!literal_type) {
        return reml_type_error(&sema->types);
      }
      reml_type_mark_numeric(&sema->types, literal_type);
      return literal_type;
    }
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

static bool reml_expect_numeric_type(reml_sema *sema, reml_type *type, reml_span span) {
  if (!sema || !type) {
    return false;
  }
  type = reml_type_prune(type);
  if (!type) {
    return false;
  }
  if (type->kind == REML_TYPE_VAR) {
    reml_type_mark_numeric(&sema->types, type);
    return true;
  }
  if (reml_is_numeric_type(type, &sema->types)) {
    return true;
  }
  reml_report_diag(sema, REML_DIAG_TYPE_MISMATCH, span, "numeric type expected");
  return false;
}

typedef struct {
  reml_trait_kind trait;
  reml_type_kind left;
  reml_type_kind right;
  reml_type_kind result;
} reml_trait_impl;

static reml_type *reml_type_from_kind(reml_type_ctx *ctx, reml_type_kind kind) {
  if (!ctx) {
    return NULL;
  }
  switch (kind) {
    case REML_TYPE_INT:
      return reml_type_int(ctx);
    case REML_TYPE_BIGINT:
      return reml_type_bigint(ctx);
    case REML_TYPE_FLOAT:
      return reml_type_float(ctx);
    case REML_TYPE_BOOL:
      return reml_type_bool(ctx);
    case REML_TYPE_CHAR:
      return reml_type_char(ctx);
    case REML_TYPE_STRING:
      return reml_type_string(ctx);
    case REML_TYPE_UNIT:
      return reml_type_unit(ctx);
    case REML_TYPE_ERROR:
      return reml_type_error(ctx);
    default:
      return reml_type_error(ctx);
  }
}

static const reml_trait_impl kTraitImpls[] = {
    {REML_TRAIT_ADD, REML_TYPE_INT, REML_TYPE_INT, REML_TYPE_INT},
    {REML_TRAIT_ADD, REML_TYPE_BIGINT, REML_TYPE_BIGINT, REML_TYPE_BIGINT},
    {REML_TRAIT_ADD, REML_TYPE_FLOAT, REML_TYPE_FLOAT, REML_TYPE_FLOAT},
    {REML_TRAIT_ADD, REML_TYPE_STRING, REML_TYPE_STRING, REML_TYPE_STRING},
    {REML_TRAIT_SUB, REML_TYPE_INT, REML_TYPE_INT, REML_TYPE_INT},
    {REML_TRAIT_SUB, REML_TYPE_BIGINT, REML_TYPE_BIGINT, REML_TYPE_BIGINT},
    {REML_TRAIT_SUB, REML_TYPE_FLOAT, REML_TYPE_FLOAT, REML_TYPE_FLOAT},
    {REML_TRAIT_MUL, REML_TYPE_INT, REML_TYPE_INT, REML_TYPE_INT},
    {REML_TRAIT_MUL, REML_TYPE_BIGINT, REML_TYPE_BIGINT, REML_TYPE_BIGINT},
    {REML_TRAIT_MUL, REML_TYPE_FLOAT, REML_TYPE_FLOAT, REML_TYPE_FLOAT},
    {REML_TRAIT_DIV, REML_TYPE_INT, REML_TYPE_INT, REML_TYPE_INT},
    {REML_TRAIT_DIV, REML_TYPE_BIGINT, REML_TYPE_BIGINT, REML_TYPE_BIGINT},
    {REML_TRAIT_DIV, REML_TYPE_FLOAT, REML_TYPE_FLOAT, REML_TYPE_FLOAT},
    {REML_TRAIT_REM, REML_TYPE_INT, REML_TYPE_INT, REML_TYPE_INT},
    {REML_TRAIT_REM, REML_TYPE_BIGINT, REML_TYPE_BIGINT, REML_TYPE_BIGINT},
    {REML_TRAIT_REM, REML_TYPE_FLOAT, REML_TYPE_FLOAT, REML_TYPE_FLOAT},
    {REML_TRAIT_BITXOR, REML_TYPE_INT, REML_TYPE_INT, REML_TYPE_INT},
    {REML_TRAIT_EQ, REML_TYPE_INT, REML_TYPE_INT, REML_TYPE_BOOL},
    {REML_TRAIT_EQ, REML_TYPE_BIGINT, REML_TYPE_BIGINT, REML_TYPE_BOOL},
    {REML_TRAIT_EQ, REML_TYPE_FLOAT, REML_TYPE_FLOAT, REML_TYPE_BOOL},
    {REML_TRAIT_EQ, REML_TYPE_BOOL, REML_TYPE_BOOL, REML_TYPE_BOOL},
    {REML_TRAIT_EQ, REML_TYPE_CHAR, REML_TYPE_CHAR, REML_TYPE_BOOL},
    {REML_TRAIT_EQ, REML_TYPE_STRING, REML_TYPE_STRING, REML_TYPE_BOOL},
    {REML_TRAIT_ORD, REML_TYPE_INT, REML_TYPE_INT, REML_TYPE_BOOL},
    {REML_TRAIT_ORD, REML_TYPE_BIGINT, REML_TYPE_BIGINT, REML_TYPE_BOOL},
    {REML_TRAIT_ORD, REML_TYPE_FLOAT, REML_TYPE_FLOAT, REML_TYPE_BOOL},
    {REML_TRAIT_ORD, REML_TYPE_CHAR, REML_TYPE_CHAR, REML_TYPE_BOOL},
    {REML_TRAIT_ORD, REML_TYPE_STRING, REML_TYPE_STRING, REML_TYPE_BOOL},
};

static void reml_trait_constraints_init(reml_sema *sema) {
  if (!sema) {
    return;
  }
  UT_icd constraint_icd = {sizeof(reml_trait_constraint), NULL, NULL, NULL};
  utarray_new(sema->trait_constraints, &constraint_icd);
}

static void reml_trait_constraints_deinit(reml_sema *sema) {
  if (!sema || !sema->trait_constraints) {
    return;
  }
  utarray_free(sema->trait_constraints);
  sema->trait_constraints = NULL;
}

static size_t reml_trait_constraints_count(const reml_sema *sema) {
  if (!sema || !sema->trait_constraints) {
    return 0;
  }
  return utarray_len(sema->trait_constraints);
}

static void reml_trait_constraints_add(reml_sema *sema, reml_trait_kind trait, reml_type *left,
                                       reml_type *right, reml_type *result, reml_span span) {
  if (!sema || !sema->trait_constraints) {
    return;
  }
  reml_trait_constraint constraint = {.trait = trait,
                                       .left = left,
                                       .right = right,
                                       .result = result,
                                       .span = span};
  utarray_push_back(sema->trait_constraints, &constraint);
}

static void reml_scheme_set_constraints(reml_scheme *scheme, reml_sema *sema, size_t start,
                                        size_t end) {
  if (!scheme || !scheme->constraints) {
    return;
  }
  utarray_clear(scheme->constraints);
  if (!sema || !sema->trait_constraints) {
    return;
  }
  size_t total = utarray_len(sema->trait_constraints);
  if (start >= total || end > total || start >= end) {
    return;
  }
  for (size_t i = start; i < end; ++i) {
    reml_trait_constraint *constraint =
        (reml_trait_constraint *)utarray_eltptr(sema->trait_constraints, i);
    if (constraint) {
      utarray_push_back(scheme->constraints, constraint);
    }
  }
}

static void reml_trait_constraints_instantiate(reml_sema *sema, const reml_scheme *scheme,
                                               UT_array *substs) {
  if (!sema || !scheme || !scheme->constraints) {
    return;
  }
  for (reml_trait_constraint *it =
           (reml_trait_constraint *)utarray_front(scheme->constraints);
       it != NULL;
       it = (reml_trait_constraint *)utarray_next(scheme->constraints, it)) {
    reml_type *left = reml_type_apply_substs(it->left, substs);
    reml_type *right = reml_type_apply_substs(it->right, substs);
    reml_type *result = reml_type_apply_substs(it->result, substs);
    if (!left || !right || !result) {
      continue;
    }
    reml_trait_constraints_add(sema, it->trait, left, right, result, it->span);
  }
}

static size_t reml_trait_match_candidates(reml_trait_kind trait, reml_type_kind left_kind,
                                          reml_type_kind right_kind, const reml_trait_impl **out,
                                          bool *out_duplicate) {
  size_t matches = 0;
  const reml_trait_impl *match = NULL;
  bool duplicate = false;
  for (size_t i = 0; i < sizeof(kTraitImpls) / sizeof(kTraitImpls[0]); ++i) {
    const reml_trait_impl *impl = &kTraitImpls[i];
    if (impl->trait != trait) {
      continue;
    }
    if (left_kind != REML_TYPE_VAR && left_kind != impl->left) {
      continue;
    }
    if (right_kind != REML_TYPE_VAR && right_kind != impl->right) {
      continue;
    }
    if (!match) {
      match = impl;
    } else if (match->left == impl->left && match->right == impl->right &&
               match->result == impl->result) {
      duplicate = true;
    }
    matches++;
  }
  if (out) {
    *out = match;
  }
  if (out_duplicate) {
    *out_duplicate = duplicate;
  }
  return matches;
}

static void reml_trait_constraints_resolve(reml_sema *sema) {
  if (!sema || !sema->trait_constraints) {
    return;
  }
  for (reml_trait_constraint *it =
           (reml_trait_constraint *)utarray_front(sema->trait_constraints);
       it != NULL;
       it = (reml_trait_constraint *)utarray_next(sema->trait_constraints, it)) {
    reml_type *left = reml_type_prune(it->left);
    reml_type *right = reml_type_prune(it->right);
    reml_type *result = reml_type_prune(it->result);
    if (!left || !right || !result) {
      continue;
    }
    if (left->kind == REML_TYPE_ERROR || right->kind == REML_TYPE_ERROR ||
        result->kind == REML_TYPE_ERROR) {
      continue;
    }
    if (left->kind == REML_TYPE_VAR || right->kind == REML_TYPE_VAR ||
        result->kind == REML_TYPE_VAR) {
      continue;
    }
    const reml_trait_impl *match = NULL;
    bool duplicate = false;
    size_t matches = reml_trait_match_candidates(it->trait, left->kind, right->kind, &match,
                                                  &duplicate);
    if (matches == 0) {
      reml_report_diag(sema, REML_DIAG_TRAIT_UNRESOLVED, it->span,
                       "cannot resolve operator trait");
      continue;
    }
    if (matches > 1) {
      if (left->kind != REML_TYPE_VAR && right->kind != REML_TYPE_VAR && duplicate) {
        reml_report_diag(sema, REML_DIAG_TRAIT_DUPLICATE, it->span,
                         "duplicate operator trait implementation");
      } else {
        reml_report_diag(sema, REML_DIAG_TRAIT_AMBIGUOUS, it->span,
                         "ambiguous operator trait");
      }
      continue;
    }
    reml_type *expected_left = reml_type_from_kind(&sema->types, match->left);
    reml_type *expected_right = reml_type_from_kind(&sema->types, match->right);
    reml_type *expected_result = reml_type_from_kind(&sema->types, match->result);
    if (!expected_left || !expected_right || !expected_result) {
      continue;
    }
    if (!reml_type_unify(&sema->types, left, expected_left) ||
        !reml_type_unify(&sema->types, right, expected_right) ||
        !reml_type_unify(&sema->types, result, expected_result)) {
      reml_report_diag(sema, REML_DIAG_TRAIT_UNRESOLVED, it->span,
                       "cannot resolve operator trait");
    }
  }
}

static reml_type *reml_resolve_binary_trait(reml_sema *sema, reml_trait_kind trait,
                                            reml_type *left, reml_type *right, reml_span span) {
  if (!sema) {
    return NULL;
  }
  left = reml_type_prune(left);
  right = reml_type_prune(right);
  if (!left || !right) {
    return reml_type_error(&sema->types);
  }
  if (left->kind == REML_TYPE_ERROR || right->kind == REML_TYPE_ERROR) {
    return reml_type_error(&sema->types);
  }

  reml_type_kind left_kind = left->kind;
  reml_type_kind right_kind = right->kind;
  const reml_trait_impl *match = NULL;
  bool has_duplicate = false;
  size_t matches =
      reml_trait_match_candidates(trait, left_kind, right_kind, &match, &has_duplicate);

  if (matches == 0) {
    reml_report_diag(sema, REML_DIAG_TRAIT_UNRESOLVED, span, "cannot resolve operator trait");
    return reml_type_error(&sema->types);
  }
  if (matches > 1) {
    if (left_kind == REML_TYPE_VAR || right_kind == REML_TYPE_VAR) {
      reml_type *result = reml_type_make_var(&sema->types);
      reml_trait_constraints_add(sema, trait, left, right, result, span);
      return result;
    }
    if (has_duplicate) {
      reml_report_diag(sema, REML_DIAG_TRAIT_DUPLICATE, span,
                       "duplicate operator trait implementation");
    } else {
      reml_report_diag(sema, REML_DIAG_TRAIT_AMBIGUOUS, span, "ambiguous operator trait");
    }
    return reml_type_error(&sema->types);
  }

  reml_type *expected_left = reml_type_from_kind(&sema->types, match->left);
  reml_type *expected_right = reml_type_from_kind(&sema->types, match->right);
  if (!expected_left || !expected_right) {
    return reml_type_error(&sema->types);
  }
  if (left_kind == REML_TYPE_VAR &&
      !reml_expect_type(sema, left, expected_left, span)) {
    return reml_type_error(&sema->types);
  }
  if (right_kind == REML_TYPE_VAR &&
      !reml_expect_type(sema, right, expected_right, span)) {
    return reml_type_error(&sema->types);
  }
  reml_type *result = reml_type_from_kind(&sema->types, match->result);
  reml_trait_constraints_add(sema, trait, left, right, result, span);
  return result;
}

static reml_type *reml_infer_unary(reml_sema *sema, reml_expr *expr, reml_effect_set *effect) {
  reml_type *operand = reml_infer_expr(sema, expr->data.unary.operand, effect);
  if (!operand) {
    return reml_type_error(&sema->types);
  }
  switch (expr->data.unary.op) {
    case REML_TOKEN_MINUS:
      if (!reml_expect_numeric_type(sema, operand, expr->span)) {
        return reml_type_error(&sema->types);
      }
      return reml_type_prune(operand);
    case REML_TOKEN_BANG:
      reml_expect_type(sema, operand, reml_type_bool(&sema->types), expr->span);
      return reml_type_bool(&sema->types);
    case REML_TOKEN_STAR: {
      reml_type *target = reml_type_prune(operand);
      if (!target || target->kind != REML_TYPE_REF) {
        reml_report_diag(sema, REML_DIAG_TYPE_MISMATCH, expr->span,
                         "deref expects reference type");
        return reml_type_error(&sema->types);
      }
      return target->data.ref.target ? target->data.ref.target : reml_type_error(&sema->types);
    }
    default:
      reml_report_diag(sema, REML_DIAG_UNSUPPORTED_FEATURE, expr->span,
                       "unsupported unary operator");
      return reml_type_error(&sema->types);
  }
}

static reml_type *reml_infer_ref(reml_sema *sema, reml_expr *expr, reml_effect_set *effect) {
  if (!expr) {
    return reml_type_error(&sema->types);
  }
  reml_expr *target_expr = expr->data.ref.target;
  if (!target_expr) {
    return reml_type_error(&sema->types);
  }
  reml_type *target_type = reml_infer_expr(sema, target_expr, effect);
  if (!target_type) {
    return reml_type_error(&sema->types);
  }
  if (reml_type_prune(target_type)->kind == REML_TYPE_ERROR) {
    return reml_type_error(&sema->types);
  }
  reml_symbol *symbol = reml_symbol_from_ident(sema, target_expr);
  if (!symbol) {
    reml_report_diag(sema, REML_DIAG_REF_EXPECTS_LVALUE, expr->span,
                     "reference expects lvalue");
    return reml_type_error(&sema->types);
  }
  if (expr->data.ref.is_mutable && !symbol->is_mutable) {
    reml_report_diag(sema, REML_DIAG_REF_NOT_MUTABLE, expr->span,
                     "mutable reference requires mutable binding");
    return reml_type_error(&sema->types);
  }
  if (expr->data.ref.is_mutable) {
    if (symbol->mut_borrowed || symbol->shared_borrows > 0) {
      reml_report_diag(sema, REML_DIAG_REF_ALIAS_CONFLICT, expr->span,
                       "mutable borrow conflicts with existing borrow");
      return reml_type_error(&sema->types);
    }
    symbol->mut_borrowed = true;
  } else {
    if (symbol->mut_borrowed) {
      reml_report_diag(sema, REML_DIAG_REF_ALIAS_CONFLICT, expr->span,
                       "shared borrow conflicts with mutable borrow");
      return reml_type_error(&sema->types);
    }
    symbol->shared_borrows += 1;
  }

  reml_scope *scope = reml_symbol_table_current(sema->symbols);
  reml_scope_record_borrow(scope, symbol, expr->data.ref.is_mutable);
  if (effect && expr->data.ref.is_mutable) {
    *effect = reml_effect_union(*effect, REML_EFFECT_MUT);
  }
  return reml_type_make_ref(&sema->types, target_type, expr->data.ref.is_mutable);
}

static reml_type *reml_infer_assignment(reml_sema *sema, reml_expr *expr,
                                        reml_effect_set *effect) {
  reml_expr *left = expr->data.binary.left;
  reml_expr *right = expr->data.binary.right;
  if (!left || !right) {
    return reml_type_error(&sema->types);
  }

  reml_type *left_type = NULL;
  if (left->kind == REML_EXPR_IDENT) {
    reml_symbol *symbol = reml_symbol_from_ident(sema, left);
    if (!symbol) {
      reml_report_diag(sema, REML_DIAG_UNDEFINED_SYMBOL, left->span, "undefined symbol");
      return reml_type_error(&sema->types);
    }
    if (!symbol->is_mutable) {
      reml_report_diag(sema, REML_DIAG_ASSIGN_NOT_MUTABLE, expr->span,
                       "assignment requires mutable binding");
      return reml_type_error(&sema->types);
    }
    if (symbol->mut_borrowed || symbol->shared_borrows > 0) {
      reml_report_diag(sema, REML_DIAG_REF_ALIAS_CONFLICT, expr->span,
                       "assignment conflicts with active borrow");
      return reml_type_error(&sema->types);
    }
    left_type = reml_infer_expr(sema, left, effect);
  } else if (left->kind == REML_EXPR_UNARY && left->data.unary.op == REML_TOKEN_STAR) {
    reml_effect_set left_effect = REML_EFFECT_NONE;
    left_type = reml_infer_expr(sema, left, &left_effect);
    if (effect) {
      *effect = reml_effect_union(*effect, left_effect);
    }
    reml_type *operand_type =
        left->data.unary.operand ? reml_type_prune(left->data.unary.operand->type) : NULL;
    if (!operand_type || operand_type->kind != REML_TYPE_REF || !operand_type->data.ref.is_mutable) {
      reml_report_diag(sema, REML_DIAG_ASSIGN_NOT_MUTABLE, expr->span,
                       "assignment requires mutable reference");
      return reml_type_error(&sema->types);
    }
  } else {
    reml_report_diag(sema, REML_DIAG_REF_EXPECTS_LVALUE, expr->span,
                     "assignment expects lvalue");
    return reml_type_error(&sema->types);
  }

  reml_effect_set right_effect = REML_EFFECT_NONE;
  reml_type *right_type = reml_infer_expr(sema, right, &right_effect);
  if (effect) {
    *effect = reml_effect_union(*effect, right_effect);
    *effect = reml_effect_union(*effect, REML_EFFECT_MUT);
  }
  if (!left_type || !right_type) {
    return reml_type_error(&sema->types);
  }
  reml_expect_type(sema, right_type, left_type, expr->span);
  return reml_type_unit(&sema->types);
}

static reml_type *reml_infer_binary(reml_sema *sema, reml_expr *expr, reml_effect_set *effect) {
  if (expr->data.binary.op == REML_TOKEN_COLONEQ) {
    return reml_infer_assignment(sema, expr, effect);
  }
  reml_type *left = reml_infer_expr(sema, expr->data.binary.left, effect);
  reml_type *right = reml_infer_expr(sema, expr->data.binary.right, effect);
  if (!left || !right) {
    return reml_type_error(&sema->types);
  }
  switch (expr->data.binary.op) {
    case REML_TOKEN_PLUS:
      return reml_resolve_binary_trait(sema, REML_TRAIT_ADD, left, right, expr->span);
    case REML_TOKEN_MINUS:
      return reml_resolve_binary_trait(sema, REML_TRAIT_SUB, left, right, expr->span);
    case REML_TOKEN_STAR:
      return reml_resolve_binary_trait(sema, REML_TRAIT_MUL, left, right, expr->span);
    case REML_TOKEN_SLASH:
      return reml_resolve_binary_trait(sema, REML_TRAIT_DIV, left, right, expr->span);
    case REML_TOKEN_PERCENT:
      return reml_resolve_binary_trait(sema, REML_TRAIT_REM, left, right, expr->span);
    case REML_TOKEN_CARET:
      return reml_resolve_binary_trait(sema, REML_TRAIT_BITXOR, left, right, expr->span);
    case REML_TOKEN_LT:
    case REML_TOKEN_LE:
    case REML_TOKEN_GT:
    case REML_TOKEN_GE:
      return reml_resolve_binary_trait(sema, REML_TRAIT_ORD, left, right, expr->span);
    case REML_TOKEN_EQEQ:
    case REML_TOKEN_NOTEQ:
      return reml_resolve_binary_trait(sema, REML_TRAIT_EQ, left, right, expr->span);
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
          size_t constraint_start = reml_trait_constraints_count(sema);
          reml_type *value_type =
              reml_infer_expr(sema, stmt->data.val_decl.value, &stmt_effect);
          size_t constraint_end = reml_trait_constraints_count(sema);
          reml_check_pattern(sema, stmt->data.val_decl.pattern, value_type, &stmt_effect, true,
                             stmt->data.val_decl.is_mutable, constraint_start, constraint_end);
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
  bool has_catch_all = false;
  bool bool_seen[2] = {false, false};
  UT_icd literal_icd = {sizeof(reml_literal), NULL, NULL, NULL};
  UT_array *seen_literals = NULL;
  utarray_new(seen_literals, &literal_icd);
  UT_icd tag_icd = {sizeof(int32_t), NULL, NULL, NULL};
  UT_array *seen_tags = NULL;
  utarray_new(seen_tags, &tag_icd);
  UT_icd interval_icd = {sizeof(reml_int_interval), NULL, NULL, NULL};
  UT_array *seen_intervals = NULL;
  utarray_new(seen_intervals, &interval_icd);

  if (expr->data.match_expr.arms) {
    for (reml_match_arm *it = (reml_match_arm *)utarray_front(expr->data.match_expr.arms);
         it != NULL; it = (reml_match_arm *)utarray_next(expr->data.match_expr.arms, it)) {
      bool has_guard = it->guard != NULL;
      if (has_catch_all) {
        reml_report_diag(sema, REML_DIAG_PATTERN_UNREACHABLE_ARM, it->pattern->span,
                         "unreachable match arm");
      } else if (reml_pattern_is_catch_all(it->pattern, scrutinee) && !has_guard) {
        has_catch_all = true;
      } else if (it->pattern && it->pattern->kind == REML_PATTERN_LITERAL && !has_guard) {
        bool bool_value = false;
        if (reml_pattern_is_bool_literal(it->pattern, &bool_value)) {
          if (bool_seen[bool_value ? 1 : 0]) {
            reml_report_diag(sema, REML_DIAG_PATTERN_UNREACHABLE_ARM, it->pattern->span,
                             "unreachable match arm");
          } else {
            bool_seen[bool_value ? 1 : 0] = true;
          }
        } else if (reml_match_literal_seen(seen_literals, it->pattern->data.literal)) {
          reml_report_diag(sema, REML_DIAG_PATTERN_UNREACHABLE_ARM, it->pattern->span,
                           "unreachable match arm");
        }
        if (reml_type_prune(scrutinee) &&
            reml_type_prune(scrutinee)->kind == REML_TYPE_INT) {
          int64_t value = 0;
          if (reml_parse_int_literal(it->pattern->data.literal, &value)) {
            if (reml_interval_covers(seen_intervals, value, value)) {
              reml_report_diag(sema, REML_DIAG_PATTERN_UNREACHABLE_ARM, it->pattern->span,
                               "unreachable match arm");
            } else {
              reml_interval_insert(seen_intervals, value, value);
            }
          }
        }
      } else if (it->pattern && it->pattern->kind == REML_PATTERN_CONSTRUCTOR && !has_guard) {
        /* handled after reml_check_pattern to use resolved tag */
      } else if (it->pattern && it->pattern->kind == REML_PATTERN_RANGE && !has_guard) {
        if (reml_type_prune(scrutinee) &&
            reml_type_prune(scrutinee)->kind == REML_TYPE_INT) {
          int64_t start_value = 0;
          int64_t end_value = 0;
          if (reml_parse_int_literal(it->pattern->data.range.start, &start_value) &&
              reml_parse_int_literal(it->pattern->data.range.end, &end_value)) {
            int64_t last_value =
                it->pattern->data.range.inclusive ? end_value : (end_value - 1);
            if (start_value > last_value) {
              reml_report_diag(sema, REML_DIAG_PATTERN_UNREACHABLE_ARM, it->pattern->span,
                               "unreachable match arm");
            } else if (reml_interval_covers(seen_intervals, start_value, last_value)) {
              reml_report_diag(sema, REML_DIAG_PATTERN_UNREACHABLE_ARM, it->pattern->span,
                               "unreachable match arm");
            } else {
              reml_interval_insert(seen_intervals, start_value, last_value);
            }
          }
        }
      }
      reml_symbol_table_enter(sema->symbols);
      reml_effect_set arm_effect = REML_EFFECT_NONE;
      size_t constraint_mark = reml_trait_constraints_count(sema);
      reml_check_pattern(sema, it->pattern, scrutinee, &arm_effect, true, false, constraint_mark,
                         constraint_mark);
      if (it->pattern && it->pattern->kind == REML_PATTERN_CONSTRUCTOR && !has_guard) {
        int32_t tag = it->pattern->data.ctor.tag;
        bool payload_full = reml_pattern_ctor_payload_covers_all(it->pattern);
        bool seen = false;
        if (payload_full) {
          for (int32_t *it_tag = (int32_t *)utarray_front(seen_tags); it_tag != NULL;
               it_tag = (int32_t *)utarray_next(seen_tags, it_tag)) {
            if (*it_tag == tag) {
              seen = true;
              break;
            }
          }
          if (seen) {
            reml_report_diag(sema, REML_DIAG_PATTERN_UNREACHABLE_ARM, it->pattern->span,
                             "unreachable match arm");
          } else {
            utarray_push_back(seen_tags, &tag);
          }
        }
      }
      if (it->guard) {
        reml_effect_set guard_effect = REML_EFFECT_NONE;
        reml_type *guard_type = reml_infer_expr(sema, it->guard, &guard_effect);
        reml_expect_type(sema, guard_type, reml_type_bool(&sema->types), it->guard->span);
        arm_effect = reml_effect_union(arm_effect, guard_effect);
      }
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

  bool exhaustive = has_catch_all;
  if (!exhaustive && reml_type_is_bool(scrutinee)) {
    exhaustive = bool_seen[0] && bool_seen[1];
  }
  if (!exhaustive) {
    scrutinee = reml_type_prune(scrutinee);
    if (scrutinee && scrutinee->kind == REML_TYPE_ENUM &&
        scrutinee->data.enum_type.variants) {
      exhaustive = reml_enum_variant_count(scrutinee) > 0;
      for (reml_enum_variant *it =
               (reml_enum_variant *)utarray_front(scrutinee->data.enum_type.variants);
           it != NULL;
           it = (reml_enum_variant *)utarray_next(scrutinee->data.enum_type.variants, it)) {
        bool seen = false;
        for (int32_t *it_tag = (int32_t *)utarray_front(seen_tags); it_tag != NULL;
             it_tag = (int32_t *)utarray_next(seen_tags, it_tag)) {
          if (*it_tag == it->tag) {
            seen = true;
            break;
          }
        }
        if (!seen) {
          exhaustive = false;
          break;
        }
      }
    }
  }
  if (!exhaustive) {
    scrutinee = reml_type_prune(scrutinee);
    if (scrutinee && scrutinee->kind == REML_TYPE_INT) {
      exhaustive = reml_interval_covers(seen_intervals, (int64_t)INT64_MIN,
                                        (int64_t)INT64_MAX);
    }
  }
  if (!exhaustive) {
    reml_diagnostic_pattern *pattern_ext = NULL;
    reml_type *scrutinee_type = reml_type_prune(scrutinee);
    if (reml_type_is_bool(scrutinee_type)) {
      pattern_ext = reml_pattern_extension_new();
      if (pattern_ext) {
        if (!bool_seen[0]) {
          reml_pattern_extension_add_variant(pattern_ext,
                                             reml_string_view_from_cstr("false"));
        }
        if (!bool_seen[1]) {
          reml_pattern_extension_add_variant(pattern_ext,
                                             reml_string_view_from_cstr("true"));
        }
      }
    } else if (scrutinee_type && scrutinee_type->kind == REML_TYPE_ENUM) {
      pattern_ext = reml_pattern_extension_new();
      if (pattern_ext && scrutinee_type->data.enum_type.variants) {
        for (reml_enum_variant *it =
                 (reml_enum_variant *)utarray_front(scrutinee_type->data.enum_type.variants);
             it != NULL;
             it = (reml_enum_variant *)utarray_next(scrutinee_type->data.enum_type.variants,
                                                    it)) {
          bool seen = false;
          for (int32_t *it_tag = (int32_t *)utarray_front(seen_tags); it_tag != NULL;
               it_tag = (int32_t *)utarray_next(seen_tags, it_tag)) {
            if (*it_tag == it->tag) {
              seen = true;
              break;
            }
          }
          if (!seen) {
            reml_pattern_extension_add_variant(pattern_ext, it->name);
          }
        }
      }
    } else if (scrutinee_type && scrutinee_type->kind == REML_TYPE_INT) {
      pattern_ext = reml_pattern_extension_new();
      if (pattern_ext) {
        reml_pattern_extension_add_missing_ranges(pattern_ext, seen_intervals);
      }
    }

    reml_diagnostic diag = {.code = REML_DIAG_PATTERN_EXHAUSTIVENESS_MISSING,
                            .span = expr->span,
                            .message = "non-exhaustive match expression",
                            .pattern = pattern_ext,
                            .effect = NULL};
    reml_diagnostics_push(&sema->diagnostics, diag);
  }

  if (seen_literals) {
    utarray_free(seen_literals);
  }
  if (seen_tags) {
    utarray_free(seen_tags);
  }
  if (seen_intervals) {
    utarray_free(seen_intervals);
  }
  *effect = reml_effect_union(*effect, scrutinee_effect);
  return result ? result : reml_type_error(&sema->types);
}

static reml_type *reml_infer_constructor(reml_sema *sema, reml_expr *expr,
                                         reml_effect_set *effect) {
  if (!sema || !expr) {
    return reml_type_error(&sema->types);
  }
  reml_constructor_entry *entry = reml_constructor_lookup(sema, expr->data.ctor.name);
  if (!entry) {
    reml_report_diag(sema, REML_DIAG_CONSTRUCTOR_UNKNOWN, expr->span,
                     "unknown constructor");
    return reml_type_error(&sema->types);
  }
  reml_type *enum_type = entry->enum_type;
  reml_enum_variant *variant = entry->variant;
  if (!enum_type || !variant) {
    return reml_type_error(&sema->types);
  }
  size_t arg_count = expr->data.ctor.args ? utarray_len(expr->data.ctor.args) : 0;
  size_t field_count = variant->fields ? utarray_len(variant->fields) : 0;
  if (arg_count != field_count) {
    reml_report_diag(sema, REML_DIAG_TYPE_MISMATCH, expr->span,
                     "constructor arity mismatch");
    return reml_type_error(&sema->types);
  }
  expr->data.ctor.tag = variant->tag;

  if (expr->data.ctor.args && variant->fields) {
    size_t index = 0;
    for (reml_expr **it = (reml_expr **)utarray_front(expr->data.ctor.args); it != NULL;
         it = (reml_expr **)utarray_next(expr->data.ctor.args, it)) {
      reml_effect_set arg_effect = REML_EFFECT_NONE;
      reml_type *arg_type = reml_infer_expr(sema, *it, &arg_effect);
      if (effect) {
        *effect = reml_effect_union(*effect, arg_effect);
      }
      reml_type **field_type = (reml_type **)utarray_eltptr(variant->fields, index);
      if (field_type) {
        reml_expect_type(sema, arg_type, *field_type, (*it)->span);
      }
      index++;
    }
  }

  return enum_type;
}

static reml_type *reml_infer_tuple(reml_sema *sema, reml_expr *expr, reml_effect_set *effect) {
  if (!sema || !expr) {
    return reml_type_error(&sema->types);
  }
  UT_icd item_icd = {sizeof(reml_type *), NULL, NULL, NULL};
  UT_array *items = NULL;
  utarray_new(items, &item_icd);

  if (expr->data.tuple) {
    for (reml_expr **it = (reml_expr **)utarray_front(expr->data.tuple); it != NULL;
         it = (reml_expr **)utarray_next(expr->data.tuple, it)) {
      reml_effect_set item_effect = REML_EFFECT_NONE;
      reml_type *item_type = reml_infer_expr(sema, *it, &item_effect);
      if (effect) {
        *effect = reml_effect_union(*effect, item_effect);
      }
      utarray_push_back(items, &item_type);
    }
  }

  reml_type *tuple_type = reml_type_make_tuple(&sema->types, items);
  return tuple_type ? tuple_type : reml_type_error(&sema->types);
}

static reml_type *reml_infer_record(reml_sema *sema, reml_expr *expr, reml_effect_set *effect) {
  if (!sema || !expr) {
    return reml_type_error(&sema->types);
  }
  UT_icd field_icd = {sizeof(reml_record_field), NULL, NULL, NULL};
  UT_array *fields = NULL;
  utarray_new(fields, &field_icd);

  if (expr->data.record) {
    for (reml_record_expr_field *it =
             (reml_record_expr_field *)utarray_front(expr->data.record);
         it != NULL;
         it = (reml_record_expr_field *)utarray_next(expr->data.record, it)) {
      reml_effect_set field_effect = REML_EFFECT_NONE;
      reml_type *field_type = reml_infer_expr(sema, it->value, &field_effect);
      if (effect) {
        *effect = reml_effect_union(*effect, field_effect);
      }
      reml_record_field field;
      field.name = it->name;
      field.type = field_type;
      utarray_push_back(fields, &field);
    }
  }

  reml_record_fields_sort(fields);
  reml_type *record_type = reml_type_make_record(&sema->types, fields);
  return record_type ? record_type : reml_type_error(&sema->types);
}

static reml_type *reml_infer_record_update(reml_sema *sema, reml_expr *expr,
                                           reml_effect_set *effect) {
  if (!sema || !expr) {
    return reml_type_error(&sema->types);
  }
  reml_effect_set base_effect = REML_EFFECT_NONE;
  reml_type *base_type = reml_infer_expr(sema, expr->data.record_update.base, &base_effect);
  if (effect) {
    *effect = reml_effect_union(*effect, base_effect);
  }
  base_type = reml_type_prune(base_type);

  if (base_type && base_type->kind == REML_TYPE_VAR) {
    UT_icd field_icd = {sizeof(reml_record_field), NULL, NULL, NULL};
    UT_array *fields = NULL;
    utarray_new(fields, &field_icd);
    if (expr->data.record_update.fields) {
      for (reml_record_expr_field *it =
               (reml_record_expr_field *)utarray_front(expr->data.record_update.fields);
           it != NULL;
           it = (reml_record_expr_field *)utarray_next(expr->data.record_update.fields, it)) {
        reml_record_field field;
        field.name = it->name;
        field.type = reml_type_make_var(&sema->types);
        utarray_push_back(fields, &field);
      }
    }
    reml_record_fields_sort(fields);
    reml_type *record_type = reml_type_make_record(&sema->types, fields);
    reml_expect_type(sema, base_type, record_type, expr->span);
    base_type = reml_type_prune(base_type);
  }

  if (!base_type || base_type->kind != REML_TYPE_RECORD) {
    reml_report_diag(sema, REML_DIAG_TYPE_MISMATCH, expr->span,
                     "record update expects record type");
    return reml_type_error(&sema->types);
  }

  if (expr->data.record_update.fields) {
    for (reml_record_expr_field *it =
             (reml_record_expr_field *)utarray_front(expr->data.record_update.fields);
         it != NULL;
         it = (reml_record_expr_field *)utarray_next(expr->data.record_update.fields, it)) {
      reml_record_field *field = reml_record_field_find(base_type, it->name);
      if (!field) {
        reml_report_diag(sema, REML_DIAG_RECORD_FIELD_UNKNOWN, expr->span,
                         "unknown record field");
        return reml_type_error(&sema->types);
      }
      reml_effect_set field_effect = REML_EFFECT_NONE;
      reml_type *value_type = reml_infer_expr(sema, it->value, &field_effect);
      if (effect) {
        *effect = reml_effect_union(*effect, field_effect);
      }
      reml_expect_type(sema, value_type, field->type, it->value->span);
    }
  }

  return base_type;
}

static reml_type *reml_infer_perform(reml_sema *sema, reml_expr *expr, reml_effect_set *effect) {
  if (!sema || !expr) {
    return reml_type_error(&sema->types);
  }
  if (expr->data.perform.args) {
    for (reml_expr **it = (reml_expr **)utarray_front(expr->data.perform.args); it != NULL;
         it = (reml_expr **)utarray_next(expr->data.perform.args, it)) {
      reml_effect_set arg_effect = REML_EFFECT_NONE;
      reml_infer_expr(sema, *it, &arg_effect);
      if (effect) {
        *effect = reml_effect_union(*effect, arg_effect);
      }
    }
  }
  if (effect) {
    *effect = reml_effect_union(*effect, REML_EFFECT_IO);
  }
  return reml_type_unit(&sema->types);
}

static reml_type *reml_infer_resume(reml_sema *sema, reml_expr *expr, reml_effect_set *effect) {
  if (!sema || !expr) {
    return reml_type_error(&sema->types);
  }
  if (expr->data.resume.value) {
    reml_effect_set value_effect = REML_EFFECT_NONE;
    reml_type *value_type = reml_infer_expr(sema, expr->data.resume.value, &value_effect);
    if (effect) {
      *effect = reml_effect_union(*effect, value_effect);
    }
    return value_type ? value_type : reml_type_unit(&sema->types);
  }
  return reml_type_unit(&sema->types);
}

static reml_type *reml_infer_handle(reml_sema *sema, reml_expr *expr, reml_effect_set *effect) {
  if (!sema || !expr) {
    return reml_type_error(&sema->types);
  }
  reml_effect_set target_effect = REML_EFFECT_NONE;
  reml_type *target_type = reml_infer_expr(sema, expr->data.handle.target, &target_effect);
  if (effect) {
    *effect = reml_effect_union(*effect, target_effect);
  }

  if (expr->data.handle.handler.entries) {
    for (reml_handler_entry *it =
             (reml_handler_entry *)utarray_front(expr->data.handle.handler.entries);
         it != NULL;
         it = (reml_handler_entry *)utarray_next(expr->data.handle.handler.entries, it)) {
      reml_symbol_table_enter(sema->symbols);
      if (it->kind == REML_HANDLER_ENTRY_OPERATION && it->data.operation.params) {
        for (reml_string_view *param =
                 (reml_string_view *)utarray_front(it->data.operation.params);
             param != NULL;
             param = (reml_string_view *)utarray_next(it->data.operation.params, param)) {
          reml_symbol_table_define(sema->symbols, REML_SYMBOL_VAR, *param, expr->span,
                                   reml_type_make_var(&sema->types), false, false, false);
        }
      }
      reml_effect_set body_effect = REML_EFFECT_NONE;
      if (it->kind == REML_HANDLER_ENTRY_OPERATION) {
        reml_infer_expr(sema, it->data.operation.body, &body_effect);
      } else {
        reml_infer_expr(sema, it->data.ret.body, &body_effect);
      }
      if (effect) {
        *effect = reml_effect_union(*effect, body_effect);
      }
      reml_symbol_table_exit(sema->symbols);
    }
  }
  return target_type ? target_type : reml_type_unit(&sema->types);
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
        UT_array *substs = NULL;
        result = reml_type_instantiate_with_substs(&sema->types, &symbol->scheme, &substs);
        reml_trait_constraints_instantiate(sema, &symbol->scheme, substs);
        if (substs) {
          utarray_free(substs);
        }
      }
      break;
    }
    case REML_EXPR_UNARY:
      result = reml_infer_unary(sema, expr, &local_effect);
      break;
    case REML_EXPR_REF:
      result = reml_infer_ref(sema, expr, &local_effect);
      break;
    case REML_EXPR_BINARY:
      result = reml_infer_binary(sema, expr, &local_effect);
      break;
    case REML_EXPR_CONSTRUCTOR:
      result = reml_infer_constructor(sema, expr, &local_effect);
      break;
    case REML_EXPR_PERFORM:
      result = reml_infer_perform(sema, expr, &local_effect);
      break;
    case REML_EXPR_HANDLE:
      result = reml_infer_handle(sema, expr, &local_effect);
      break;
    case REML_EXPR_RESUME:
      result = reml_infer_resume(sema, expr, &local_effect);
      break;
    case REML_EXPR_TUPLE:
      result = reml_infer_tuple(sema, expr, &local_effect);
      break;
    case REML_EXPR_RECORD:
      result = reml_infer_record(sema, expr, &local_effect);
      break;
    case REML_EXPR_RECORD_UPDATE:
      result = reml_infer_record_update(sema, expr, &local_effect);
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

  bool allow_effect_poly = reml_symbol_table_is_top_level(sema->symbols);
  if (!allow_effect_poly) {
    return;
  }
  UT_array *effect_vars = NULL;
  UT_array *env_effect_vars = NULL;
  utarray_new(effect_vars, &var_icd);
  utarray_new(env_effect_vars, &var_icd);
  reml_effect_row_collect_vars(type, effect_vars);
  reml_env_collect_free_effect_vars(sema->symbols, symbol, env_effect_vars);
  for (uint32_t *it = (uint32_t *)utarray_front(effect_vars); it != NULL;
       it = (uint32_t *)utarray_next(effect_vars, it)) {
    if (!reml_var_ids_contains(env_effect_vars, *it)) {
      reml_var_ids_push_unique(symbol->scheme.effect_generics, *it);
    }
  }
  utarray_free(effect_vars);
  utarray_free(env_effect_vars);
}

static void reml_define_pattern_symbol(reml_sema *sema, reml_pattern *pattern,
                                       reml_type *expected, bool allow_define, bool is_mutable,
                                       reml_effect_set *effect, size_t constraint_start,
                                       size_t constraint_end) {
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
                                      pattern->span, expected, false, false, is_mutable);
  }
  if (!symbol) {
    return;
  }
  if (existing && existing->is_predeclared) {
    reml_expect_type(sema, existing->scheme.type, expected, pattern->span);
    expected = reml_type_prune(existing->scheme.type);
  }
  if (!existing) {
    symbol->is_mutable = is_mutable;
  }
  symbol->is_predeclared = false;
  pattern->symbol_id = symbol->id;
  pattern->type = expected;

  reml_type_apply_numeric_defaults(&sema->types, reml_type_int(&sema->types));
  bool allow_poly = effect ? (*effect == REML_EFFECT_NONE) : true;
  reml_generalize(sema, symbol, expected, allow_poly);
  reml_scheme_set_constraints(&symbol->scheme, sema, constraint_start, constraint_end);
}

static void reml_check_pattern(reml_sema *sema, reml_pattern *pattern, reml_type *expected,
                               reml_effect_set *effect, bool allow_define, bool is_mutable,
                               size_t constraint_start, size_t constraint_end) {
  if (!pattern) {
    return;
  }
  switch (pattern->kind) {
    case REML_PATTERN_WILDCARD:
      pattern->type = expected;
      return;
    case REML_PATTERN_IDENT:
      reml_define_pattern_symbol(sema, pattern, expected, allow_define, is_mutable, effect,
                                 constraint_start, constraint_end);
      return;
    case REML_PATTERN_LITERAL: {
      reml_type *literal_type = reml_infer_literal(sema, pattern->data.literal);
      if (!reml_expect_type(sema, literal_type, expected, pattern->span)) {
        return;
      }
      pattern->type = literal_type;
      return;
    }
    case REML_PATTERN_RANGE: {
      if (!reml_expect_type(sema, expected, reml_type_int(&sema->types), pattern->span)) {
        pattern->type = reml_type_error(&sema->types);
        return;
      }
      expected = reml_type_prune(expected);
      reml_type *start_type = reml_infer_literal(sema, pattern->data.range.start);
      reml_type *end_type = reml_infer_literal(sema, pattern->data.range.end);
      if (!reml_expect_type(sema, start_type, expected, pattern->span) ||
          !reml_expect_type(sema, end_type, expected, pattern->span)) {
        pattern->type = reml_type_error(&sema->types);
        return;
      }
      start_type = reml_type_prune(start_type);
      end_type = reml_type_prune(end_type);
      if (start_type->kind != REML_TYPE_INT || end_type->kind != REML_TYPE_INT) {
        reml_report_diag(sema, REML_DIAG_PATTERN_RANGE_TYPE_MISMATCH, pattern->span,
                         "range pattern expects integer bounds");
        pattern->type = reml_type_error(&sema->types);
        return;
      }
      int64_t start_value = 0;
      int64_t end_value = 0;
      if (reml_parse_int_literal(pattern->data.range.start, &start_value) &&
          reml_parse_int_literal(pattern->data.range.end, &end_value)) {
        bool inverted = pattern->data.range.inclusive ? (start_value > end_value)
                                                      : (start_value >= end_value);
        if (inverted) {
          reml_report_diag(sema, REML_DIAG_PATTERN_RANGE_INVERTED, pattern->span,
                           "range bound is inverted");
        }
      }
      pattern->type = expected;
      return;
    }
    case REML_PATTERN_CONSTRUCTOR: {
      reml_type *target = reml_type_prune(expected);
      if (target && target->kind == REML_TYPE_VAR) {
        reml_constructor_entry *entry =
            reml_constructor_lookup(sema, pattern->data.ctor.name);
        if (!entry) {
          reml_report_diag(sema, REML_DIAG_CONSTRUCTOR_UNKNOWN, pattern->span,
                           "unknown constructor");
          pattern->type = reml_type_error(&sema->types);
          return;
        }
        reml_expect_type(sema, target, entry->enum_type, pattern->span);
        target = reml_type_prune(target);
      }
      if (!target || target->kind != REML_TYPE_ENUM) {
        reml_report_diag(sema, REML_DIAG_TYPE_MISMATCH, pattern->span,
                         "constructor pattern expects enum type");
        pattern->type = reml_type_error(&sema->types);
        return;
      }
      size_t field_count =
          pattern->data.ctor.items ? utarray_len(pattern->data.ctor.items) : 0;
      reml_enum_variant *variant =
          reml_enum_variant_find(target->data.enum_type.variants, pattern->data.ctor.name);
      if (!variant) {
        reml_constructor_entry *entry =
            reml_constructor_lookup(sema, pattern->data.ctor.name);
        if (entry && entry->enum_type == target) {
          variant = entry->variant;
        }
      }
      if (!variant) {
        reml_report_diag(sema, REML_DIAG_CONSTRUCTOR_UNKNOWN, pattern->span,
                         "unknown constructor");
        pattern->type = reml_type_error(&sema->types);
        return;
      }
      size_t variant_fields = variant->fields ? utarray_len(variant->fields) : 0;
      if (variant_fields != field_count) {
        reml_report_diag(sema, REML_DIAG_PATTERN_CONSTRUCTOR_ARITY, pattern->span,
                         "constructor arity mismatch");
        pattern->type = reml_type_error(&sema->types);
        return;
      }
      pattern->data.ctor.tag = variant->tag;
      if (pattern->data.ctor.items && variant->fields) {
        size_t index = 0;
        for (reml_pattern **it = (reml_pattern **)utarray_front(pattern->data.ctor.items);
             it != NULL;
             it = (reml_pattern **)utarray_next(pattern->data.ctor.items, it)) {
          reml_type **field_type = (reml_type **)utarray_eltptr(variant->fields, index);
          reml_check_pattern(sema, *it, field_type ? *field_type : expected, effect, allow_define,
                             is_mutable, constraint_start, constraint_end);
          index++;
        }
      }
      pattern->type = expected;
      return;
    }
    case REML_PATTERN_TUPLE:
      {
        size_t item_count = pattern->data.items ? utarray_len(pattern->data.items) : 0;
        reml_type *target = reml_type_prune(expected);
        if (target && target->kind == REML_TYPE_VAR) {
          UT_icd item_icd = {sizeof(reml_type *), NULL, NULL, NULL};
          UT_array *items = NULL;
          utarray_new(items, &item_icd);
          for (size_t i = 0; i < item_count; ++i) {
            reml_type *item_type = reml_type_make_var(&sema->types);
            utarray_push_back(items, &item_type);
          }
          reml_type *tuple_type = reml_type_make_tuple(&sema->types, items);
          reml_expect_type(sema, target, tuple_type, pattern->span);
          target = reml_type_prune(target);
        }
        if (!target || target->kind != REML_TYPE_TUPLE) {
          reml_report_diag(sema, REML_DIAG_TYPE_MISMATCH, pattern->span,
                           "tuple pattern expects tuple type");
          pattern->type = reml_type_error(&sema->types);
          return;
        }
        size_t target_count = target->data.tuple.items ? utarray_len(target->data.tuple.items) : 0;
        if (target_count != item_count) {
          reml_report_diag(sema, REML_DIAG_TYPE_MISMATCH, pattern->span,
                           "tuple pattern arity mismatch");
          pattern->type = reml_type_error(&sema->types);
          return;
        }
        if (pattern->data.items && target->data.tuple.items) {
          size_t index = 0;
          for (reml_pattern **it = (reml_pattern **)utarray_front(pattern->data.items);
               it != NULL;
               it = (reml_pattern **)utarray_next(pattern->data.items, it)) {
            reml_type **item_type =
                (reml_type **)utarray_eltptr(target->data.tuple.items, index);
            reml_check_pattern(sema, *it, item_type ? *item_type : expected, effect, allow_define,
                               is_mutable, constraint_start, constraint_end);
            index++;
          }
        }
        pattern->type = expected;
        return;
      }
    case REML_PATTERN_RECORD:
      {
        reml_type *target = reml_type_prune(expected);
        if (target && target->kind == REML_TYPE_VAR) {
          UT_icd field_icd = {sizeof(reml_record_field), NULL, NULL, NULL};
          UT_array *fields = NULL;
          utarray_new(fields, &field_icd);
          if (pattern->data.fields) {
            for (reml_pattern_field *it =
                     (reml_pattern_field *)utarray_front(pattern->data.fields);
                 it != NULL;
                 it = (reml_pattern_field *)utarray_next(pattern->data.fields, it)) {
              reml_record_field field;
              field.name = it->name;
              field.type = reml_type_make_var(&sema->types);
              utarray_push_back(fields, &field);
            }
          }
          reml_record_fields_sort(fields);
          reml_type *record_type = reml_type_make_record(&sema->types, fields);
          reml_expect_type(sema, target, record_type, pattern->span);
          target = reml_type_prune(target);
        }
        if (!target || target->kind != REML_TYPE_RECORD) {
          reml_report_diag(sema, REML_DIAG_TYPE_MISMATCH, pattern->span,
                           "record pattern expects record type");
          pattern->type = reml_type_error(&sema->types);
          return;
        }
        bool missing = false;
        bool unknown = false;
        if (target->data.record.fields) {
          for (reml_record_field *it =
                   (reml_record_field *)utarray_front(target->data.record.fields);
               it != NULL;
               it = (reml_record_field *)utarray_next(target->data.record.fields, it)) {
            if (!reml_pattern_fields_contains(pattern->data.fields, it->name)) {
              missing = true;
              break;
            }
          }
        }
        if (pattern->data.fields) {
          for (reml_pattern_field *it =
                   (reml_pattern_field *)utarray_front(pattern->data.fields);
               it != NULL;
               it = (reml_pattern_field *)utarray_next(pattern->data.fields, it)) {
            if (!reml_record_field_find(target, it->name)) {
              unknown = true;
              break;
            }
          }
        }
        if (missing) {
          reml_report_diag(sema, REML_DIAG_RECORD_FIELD_MISSING, pattern->span,
                           "record field missing");
        }
        if (unknown) {
          reml_report_diag(sema, REML_DIAG_RECORD_FIELD_UNKNOWN, pattern->span,
                           "unknown record field");
        }
        if (missing || unknown) {
          pattern->type = reml_type_error(&sema->types);
          return;
        }
        if (pattern->data.fields) {
          for (reml_pattern_field *it =
                   (reml_pattern_field *)utarray_front(pattern->data.fields);
               it != NULL;
               it = (reml_pattern_field *)utarray_next(pattern->data.fields, it)) {
            reml_record_field *field = reml_record_field_find(target, it->name);
            if (!field) {
              reml_report_diag(sema, REML_DIAG_RECORD_FIELD_UNKNOWN, pattern->span,
                               "unknown record field");
              pattern->type = reml_type_error(&sema->types);
              return;
            }
            reml_check_pattern(sema, it->pattern, field->type, effect, allow_define, is_mutable,
                               constraint_start, constraint_end);
          }
        }
        pattern->type = expected;
        return;
      }
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
    if (stmt->kind == REML_STMT_TYPE_DECL) {
      reml_register_type_decl(sema, &stmt->data.type_decl, stmt->span);
      continue;
    }
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
    reml_symbol *symbol = reml_symbol_table_define(
        sema->symbols, REML_SYMBOL_VAR, pattern->data.ident, pattern->span,
        reml_type_make_var(&sema->types), false, true, stmt->data.val_decl.is_mutable);
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
      size_t constraint_start = reml_trait_constraints_count(sema);
      reml_type *value_type = reml_infer_expr(sema, stmt->data.val_decl.value, &value_effect);
      size_t constraint_end = reml_trait_constraints_count(sema);
      reml_check_pattern(sema, stmt->data.val_decl.pattern, value_type, &value_effect, true,
                         stmt->data.val_decl.is_mutable, constraint_start, constraint_end);
      reml_check_effect_contract(sema, &stmt->attrs, value_effect);
      if (effect) {
        *effect = reml_effect_union(*effect, value_effect);
      }
      break;
    }
    case REML_STMT_RETURN: {
      reml_effect_set expr_effect = REML_EFFECT_NONE;
      reml_infer_expr(sema, stmt->data.expr, &expr_effect);
      reml_check_effect_contract(sema, &stmt->attrs, expr_effect);
      if (effect) {
        *effect = reml_effect_union(*effect, expr_effect);
      }
      break;
    }
    case REML_STMT_EXPR: {
      reml_effect_set expr_effect = REML_EFFECT_NONE;
      reml_infer_expr(sema, stmt->data.expr, &expr_effect);
      reml_check_effect_contract(sema, &stmt->attrs, expr_effect);
      if (effect) {
        *effect = reml_effect_union(*effect, expr_effect);
      }
      break;
    }
    case REML_STMT_TYPE_DECL:
      reml_check_effect_contract(sema, &stmt->attrs, REML_EFFECT_NONE);
      break;
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
  sema->constructors = NULL;
  sema->enum_decls = NULL;
  reml_type_ctx_init(&sema->types);
  reml_diagnostics_init(&sema->diagnostics);
  reml_trait_constraints_init(sema);
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
  reml_constructor_entry *ctor = NULL;
  reml_constructor_entry *ctor_tmp = NULL;
  HASH_ITER(hh, sema->constructors, ctor, ctor_tmp) {
    HASH_DEL(sema->constructors, ctor);
    free(ctor);
  }
  reml_enum_decl_entry *decl = NULL;
  reml_enum_decl_entry *decl_tmp = NULL;
  HASH_ITER(hh, sema->enum_decls, decl, decl_tmp) {
    HASH_DEL(sema->enum_decls, decl);
    free(decl);
  }
  sema->constructors = NULL;
  sema->enum_decls = NULL;
  reml_type_ctx_deinit(&sema->types);
  reml_diagnostics_deinit(&sema->diagnostics);
  reml_trait_constraints_deinit(sema);
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

  reml_type_apply_numeric_defaults(&sema->types, reml_type_int(&sema->types));
  reml_trait_constraints_resolve(sema);
  return reml_diagnostics_count(&sema->diagnostics) == 0;
}

const reml_diagnostic_list *reml_sema_diagnostics(const reml_sema *sema) {
  if (!sema) {
    return NULL;
  }
  return &sema->diagnostics;
}
