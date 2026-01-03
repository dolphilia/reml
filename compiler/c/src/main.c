#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <inttypes.h>

#include <utarray.h>

#include "argparse.h"
#include "reml/ast/printer.h"
#include "reml/codegen/codegen.h"
#include "reml/lexer/lexer.h"
#include "reml/manifest/manifest.h"
#include "reml/parser/parser.h"
#include "reml/sema/sema.h"
#include "reml/util/logger.h"

#define REML_VERSION "0.1.0-dev"

static const char *const kUsage[] = {
  "reml [options] <command>",
  NULL,
};

static int filter_verbosity(int argc, const char **argv, int *verbosity, const char **out_argv) {
  int outc = 0;

  out_argv[outc++] = argv[0];

  for (int i = 1; i < argc; i++) {
    if (strcmp(argv[i], "-vv") == 0) {
      *verbosity = 2;
      continue;
    }
    if (strcmp(argv[i], "-v") == 0) {
      if (*verbosity < 1) {
        *verbosity = 1;
      }
      continue;
    }

    out_argv[outc++] = argv[i];
  }

  return outc;
}

static void print_version(void) {
  printf("reml %s\n", REML_VERSION);
}

static char *read_file(const char *path, size_t *out_length) {
  FILE *fp = fopen(path, "rb");
  if (!fp) {
    return NULL;
  }
  if (fseek(fp, 0, SEEK_END) != 0) {
    fclose(fp);
    return NULL;
  }
  long size = ftell(fp);
  if (size < 0) {
    fclose(fp);
    return NULL;
  }
  if (fseek(fp, 0, SEEK_SET) != 0) {
    fclose(fp);
    return NULL;
  }
  char *buffer = (char *)malloc((size_t)size + 1);
  if (!buffer) {
    fclose(fp);
    return NULL;
  }
  size_t read_bytes = fread(buffer, 1, (size_t)size, fp);
  fclose(fp);
  if (read_bytes != (size_t)size) {
    free(buffer);
    return NULL;
  }
  buffer[size] = '\0';
  if (out_length) {
    *out_length = (size_t)size;
  }
  return buffer;
}

static void print_diagnostics(const char *stage, const reml_diagnostic_list *list) {
  if (!list) {
    return;
  }
  size_t count = reml_diagnostics_count(list);
  for (size_t i = 0; i < count; ++i) {
    const reml_diagnostic *diag = reml_diagnostics_at(list, i);
    if (!diag) {
      continue;
    }
    if (reml_span_is_valid(&diag->span)) {
      fprintf(stderr, "%s:%d:%d: %s\n", stage, diag->span.start_line, diag->span.start_column,
              diag->message);
    } else {
      fprintf(stderr, "%s: %s\n", stage, diag->message);
    }
  }
}

static void json_write_escaped(FILE *out, const char *text) {
  if (!out || !text) {
    return;
  }
  for (const char *p = text; *p != '\0'; ++p) {
    unsigned char c = (unsigned char)*p;
    switch (c) {
      case '"':
        fputs("\\\"", out);
        break;
      case '\\':
        fputs("\\\\", out);
        break;
      case '\n':
        fputs("\\n", out);
        break;
      case '\r':
        fputs("\\r", out);
        break;
      case '\t':
        fputs("\\t", out);
        break;
      default:
        if (c < 0x20) {
          fprintf(out, "\\u%04x", (unsigned int)c);
        } else {
          fputc(c, out);
        }
        break;
    }
  }
}

static void json_write_string_view(FILE *out, reml_string_view view) {
  fputc('"', out);
  for (size_t i = 0; i < view.length; ++i) {
    unsigned char c = (unsigned char)view.data[i];
    switch (c) {
      case '"':
        fputs("\\\"", out);
        break;
      case '\\':
        fputs("\\\\", out);
        break;
      case '\n':
        fputs("\\n", out);
        break;
      case '\r':
        fputs("\\r", out);
        break;
      case '\t':
        fputs("\\t", out);
        break;
      default:
        if (c < 0x20) {
          fprintf(out, "\\u%04x", (unsigned int)c);
        } else {
          fputc(c, out);
        }
        break;
    }
  }
  fputc('"', out);
}

static void print_diagnostics_json(FILE *out, const reml_diagnostic_list *list) {
  if (!out) {
    return;
  }
  fputc('[', out);
  if (list) {
    size_t count = reml_diagnostics_count(list);
    for (size_t i = 0; i < count; ++i) {
      const reml_diagnostic *diag = reml_diagnostics_at(list, i);
      if (!diag) {
        continue;
      }
      if (i > 0) {
        fputc(',', out);
      }
      fputc('{', out);
      fprintf(out, "\"code\":%d,", (int)diag->code);
      fputs("\"message\":\"", out);
      json_write_escaped(out, diag->message ? diag->message : "");
      fputs("\",\"span\":{", out);
      fprintf(out, "\"start_line\":%d,\"start_column\":%d,", diag->span.start_line,
              diag->span.start_column);
      fprintf(out, "\"end_line\":%d,\"end_column\":%d", diag->span.end_line,
              diag->span.end_column);
      fputs("}", out);

      if (diag->pattern) {
        fputs(",\"extensions\":{\"pattern\":{", out);
        fputs("\"missing_variants\":[", out);
        if (diag->pattern->missing_variants) {
          size_t vcount = utarray_len(diag->pattern->missing_variants);
          size_t vindex = 0;
          for (reml_string_view *it =
                   (reml_string_view *)utarray_front(diag->pattern->missing_variants);
               it != NULL;
               it = (reml_string_view *)utarray_next(diag->pattern->missing_variants, it)) {
            if (vindex++ > 0) {
              fputc(',', out);
            }
            json_write_string_view(out, *it);
          }
          (void)vcount;
        }
        fputs("],\"missing_ranges\":[", out);
        if (diag->pattern->missing_ranges) {
          size_t rindex = 0;
          for (reml_diagnostic_range *it =
                   (reml_diagnostic_range *)utarray_front(diag->pattern->missing_ranges);
               it != NULL;
               it = (reml_diagnostic_range *)utarray_next(diag->pattern->missing_ranges, it)) {
            if (rindex++ > 0) {
              fputc(',', out);
            }
            fprintf(out,
                    "{\"start\":%" PRId64 ",\"end\":%" PRId64 ",\"inclusive\":%s}",
                    it->start, it->end, it->inclusive ? "true" : "false");
          }
        }
        fputs("]}", out);
        fputs("}", out);
      }

      fputc('}', out);
    }
  }
  fputc(']', out);
}

static void print_diagnostics_json_object(FILE *out, const reml_diagnostic_list *sema_list,
                                          const reml_diagnostic_list *codegen_list) {
  if (!out) {
    return;
  }
  fputs("{\"sema\":", out);
  print_diagnostics_json(out, sema_list);
  fputs(",\"codegen\":", out);
  print_diagnostics_json(out, codegen_list);
  fputs("}\n", out);
}

static char *derive_object_path(const char *bin_path) {
  size_t len = strlen(bin_path);
  char *path = (char *)malloc(len + 3);
  if (!path) {
    return NULL;
  }
  memcpy(path, bin_path, len);
  path[len] = '\0';
  strcat(path, ".o");
  return path;
}

static bool run_linker(const char *obj_path, const char *bin_path) {
  if (!obj_path || !bin_path) {
    return false;
  }
  const char *cc_cmd = getenv("CC");
  if (!cc_cmd || cc_cmd[0] == '\0') {
#ifdef __APPLE__
    cc_cmd = "xcrun --sdk macosx clang";
#else
    cc_cmd = "cc";
#endif
  }
  size_t cmd_len = strlen(cc_cmd) + strlen(obj_path) + strlen(bin_path) + 8;
  char *command = (char *)malloc(cmd_len + 16);
  if (!command) {
    return false;
  }
  snprintf(command, cmd_len + 16, "%s \"%s\" -o \"%s\"", cc_cmd, obj_path, bin_path);
  int result = system(command);
  free(command);
  return result == 0;
}

static int command_internal_codegen(int argc, const char **argv) {
  if (argc < 1) {
    fprintf(stderr, "missing file path\n");
    return 1;
  }

  const char *input_path = argv[0];
  const char *ir_path = NULL;
  const char *obj_path = NULL;
  const char *bin_path = NULL;
  char *derived_obj_path = NULL;
  bool remove_obj = false;
  bool diag_json = false;

  for (int i = 1; i < argc; ++i) {
    if (strcmp(argv[i], "--emit-ir") == 0 || strcmp(argv[i], "--emit-llvm") == 0) {
      if (i + 1 >= argc) {
        fprintf(stderr, "missing path after %s\n", argv[i]);
        return 1;
      }
      ir_path = argv[++i];
      continue;
    }
    if (strcmp(argv[i], "--diag-json") == 0) {
      diag_json = true;
      continue;
    }
    if (strcmp(argv[i], "--emit-obj") == 0) {
      if (i + 1 >= argc) {
        fprintf(stderr, "missing path after --emit-obj\n");
        return 1;
      }
      obj_path = argv[++i];
      continue;
    }
    if (strcmp(argv[i], "--emit-bin") == 0 || strcmp(argv[i], "--emit-exe") == 0) {
      if (i + 1 >= argc) {
        fprintf(stderr, "missing path after %s\n", argv[i]);
        return 1;
      }
      bin_path = argv[++i];
      continue;
    }
    fprintf(stderr, "unknown option: %s\n", argv[i]);
    return 1;
  }

  if (!ir_path && !obj_path) {
    obj_path = "out.o";
  }

  if (bin_path && !obj_path) {
    derived_obj_path = derive_object_path(bin_path);
    if (!derived_obj_path) {
      fprintf(stderr, "failed to allocate object path\n");
      return 1;
    }
    obj_path = derived_obj_path;
    remove_obj = true;
  }

  size_t length = 0;
  char *content = read_file(input_path, &length);
  if (!content) {
    fprintf(stderr, "failed to read file: %s\n", input_path);
    return 1;
  }

  reml_parser parser;
  reml_parser_init(&parser, content, length);
  reml_compilation_unit *unit = reml_parse_compilation_unit(&parser);
  if (!unit) {
    const reml_parse_error *error = reml_parser_error(&parser);
    if (error) {
      fprintf(stderr, "parse error: %s\n", error->message);
    } else {
      fprintf(stderr, "parse error\n");
    }
    free(content);
    return 1;
  }

  reml_sema sema;
  reml_sema_init(&sema);
  bool ok = reml_sema_check(&sema, unit);
  if (!ok) {
    if (diag_json) {
      print_diagnostics_json_object(stdout, reml_sema_diagnostics(&sema), NULL);
    } else {
      print_diagnostics("sema", reml_sema_diagnostics(&sema));
    }
    reml_sema_deinit(&sema);
    reml_compilation_unit_free(unit);
    free(content);
    return 1;
  }

  reml_codegen codegen;
  if (!reml_codegen_init(&codegen, "reml_module")) {
    if (diag_json) {
      print_diagnostics_json_object(stdout, reml_sema_diagnostics(&sema),
                                    reml_codegen_diagnostics(&codegen));
    } else {
      print_diagnostics("codegen", reml_codegen_diagnostics(&codegen));
    }
    reml_codegen_deinit(&codegen);
    reml_sema_deinit(&sema);
    reml_compilation_unit_free(unit);
    free(content);
    return 1;
  }

  if (!reml_codegen_generate(&codegen, unit)) {
    if (diag_json) {
      print_diagnostics_json_object(stdout, reml_sema_diagnostics(&sema),
                                    reml_codegen_diagnostics(&codegen));
    } else {
      print_diagnostics("codegen", reml_codegen_diagnostics(&codegen));
    }
    reml_codegen_deinit(&codegen);
    reml_sema_deinit(&sema);
    reml_compilation_unit_free(unit);
    free(content);
    return 1;
  }

  if (ir_path && !reml_codegen_emit_ir(&codegen, ir_path)) {
    if (diag_json) {
      print_diagnostics_json_object(stdout, reml_sema_diagnostics(&sema),
                                    reml_codegen_diagnostics(&codegen));
    } else {
      print_diagnostics("codegen", reml_codegen_diagnostics(&codegen));
    }
    reml_codegen_deinit(&codegen);
    reml_sema_deinit(&sema);
    reml_compilation_unit_free(unit);
    free(content);
    return 1;
  }

  if (obj_path && !reml_codegen_emit_object(&codegen, obj_path)) {
    if (diag_json) {
      print_diagnostics_json_object(stdout, reml_sema_diagnostics(&sema),
                                    reml_codegen_diagnostics(&codegen));
    } else {
      print_diagnostics("codegen", reml_codegen_diagnostics(&codegen));
    }
    reml_codegen_deinit(&codegen);
    reml_sema_deinit(&sema);
    reml_compilation_unit_free(unit);
    free(content);
    free(derived_obj_path);
    return 1;
  }

  if (bin_path && !run_linker(obj_path, bin_path)) {
    fprintf(stderr, "linker error: failed to produce executable\n");
    reml_codegen_deinit(&codegen);
    reml_sema_deinit(&sema);
    reml_compilation_unit_free(unit);
    free(content);
    if (remove_obj && obj_path) {
      remove(obj_path);
    }
    free(derived_obj_path);
    return 1;
  }

  if (diag_json) {
    print_diagnostics_json_object(stdout, reml_sema_diagnostics(&sema), NULL);
  }
  reml_codegen_deinit(&codegen);
  reml_sema_deinit(&sema);
  reml_compilation_unit_free(unit);
  free(content);
  if (remove_obj && obj_path) {
    remove(obj_path);
  }
  free(derived_obj_path);
  return 0;
}

static int command_internal_lex(const char *path) {
  size_t length = 0;
  char *content = read_file(path, &length);
  if (!content) {
    fprintf(stderr, "failed to read file: %s\n", path);
    return 1;
  }

  reml_lexer lexer;
  reml_lexer_init(&lexer, content, length);

  for (;;) {
    reml_token token = reml_lexer_next(&lexer);
    printf("%s\t%.*s\n", reml_token_kind_name(token.kind), (int)token.lexeme.length,
           token.lexeme.data);
    if (token.kind == REML_TOKEN_EOF || token.kind == REML_TOKEN_INVALID) {
      break;
    }
  }

  if (lexer.has_error) {
    fprintf(stderr, "lex error: %s\n", lexer.error.message);
    free(content);
    return 1;
  }

  free(content);
  return 0;
}

static int command_internal_parse(const char *path) {
  size_t length = 0;
  char *content = read_file(path, &length);
  if (!content) {
    fprintf(stderr, "failed to read file: %s\n", path);
    return 1;
  }

  reml_parser parser;
  reml_parser_init(&parser, content, length);
  reml_compilation_unit *unit = reml_parse_compilation_unit(&parser);
  if (!unit) {
    const reml_parse_error *error = reml_parser_error(&parser);
    if (error) {
      fprintf(stderr, "parse error: %s\n", error->message);
    } else {
      fprintf(stderr, "parse error\n");
    }
    free(content);
    return 1;
  }

  reml_ast_write_compilation_unit(stdout, unit);
  fputc('\n', stdout);

  reml_compilation_unit_free(unit);
  free(content);
  return 0;
}

int main(int argc, const char **argv) {
  int verbosity = 0;
  const char *filtered_argv[argc];
  int filtered_argc = filter_verbosity(argc, argv, &verbosity, filtered_argv);

  struct argparse_option options[] = {
    OPT_HELP(),
    OPT_INTEGER('v', "verbose", &verbosity, "verbosity level (0-2)"),
    OPT_END(),
  };

  struct argparse argparse;
  argparse_init(&argparse, options, kUsage, ARGPARSE_STOP_AT_NON_OPTION);
  argparse_describe(&argparse, "Reml C compiler (bootstrap)", NULL);
  filtered_argc = argparse_parse(&argparse, filtered_argc, filtered_argv);

  reml_log_init(verbosity);

  if (filtered_argc < 1) {
    argparse_usage(&argparse);
    return 0;
  }

  const char *command = filtered_argv[0];

  if (strcmp(command, "version") == 0 || strcmp(command, "--version") == 0) {
    print_version();
    return 0;
  }

  if (strcmp(command, "help") == 0) {
    argparse_usage(&argparse);
    return 0;
  }

  if (strcmp(command, "internal") == 0) {
    if (filtered_argc < 2) {
      fprintf(stderr, "missing internal command\n");
      return 1;
    }
    const char *subcommand = filtered_argv[1];
    if (strcmp(subcommand, "lex") == 0) {
      if (filtered_argc < 3) {
        fprintf(stderr, "missing file path\n");
        return 1;
      }
      return command_internal_lex(filtered_argv[2]);
    }
    if (strcmp(subcommand, "parse") == 0) {
      if (filtered_argc < 3) {
        fprintf(stderr, "missing file path\n");
        return 1;
      }
      return command_internal_parse(filtered_argv[2]);
    }
    if (strcmp(subcommand, "codegen") == 0) {
      if (filtered_argc < 3) {
        fprintf(stderr, "missing file path\n");
        return 1;
      }
      return command_internal_codegen(filtered_argc - 2, filtered_argv + 2);
    }
    if (strcmp(subcommand, "manifest") == 0) {
      if (filtered_argc < 3) {
        fprintf(stderr, "missing manifest path\n");
        return 1;
      }
      reml_manifest manifest;
      reml_manifest_error error;
      if (!reml_manifest_load(filtered_argv[2], &manifest, &error)) {
        fprintf(stderr, "manifest error: %s\n", error.message);
        return 1;
      }
      printf("package.name = %s\n", manifest.package_name);
      printf("package.version = %s\n", manifest.package_version);
      reml_manifest_free(&manifest);
      return 0;
    }
    fprintf(stderr, "unknown internal command: %s\n", subcommand);
    return 1;
  }

  fprintf(stderr, "Unknown command: %s\n", command);
  argparse_usage(&argparse);
  return 1;
}
