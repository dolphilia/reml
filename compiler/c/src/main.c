#include <stdio.h>
#include <string.h>

#include "argparse.h"
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

  fprintf(stderr, "Unknown command: %s\n", command);
  argparse_usage(&argparse);
  return 1;
}
