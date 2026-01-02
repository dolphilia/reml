#include "reml/util/logger.h"

void reml_log_init(int verbosity) {
  int level = LOG_INFO;

  if (verbosity <= 0) {
    level = LOG_INFO;
  } else if (verbosity == 1) {
    level = LOG_DEBUG;
  } else {
    level = LOG_TRACE;
  }

  log_set_quiet(0);
  log_set_level(level);
}
