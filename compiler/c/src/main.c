#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "argparse.h"
#include "reml/ast/printer.h"
#include "reml/lexer/lexer.h"
#include "reml/manifest/manifest.h"
#include "reml/parser/parser.h"
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
  argparse_init(&argparse, options, kUsage, 0);
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
