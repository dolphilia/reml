#ifndef REML_C_RUNTIME_H
#define REML_C_RUNTIME_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "reml/text/string.h"

#ifdef __cplusplus
extern "C" {
#endif

#define REML_PANIC_EXIT_CODE 70

typedef enum {
  REML_DIAG_NONE = 0,
  REML_DIAG_RUNTIME_OOM = 1000,
  REML_DIAG_RUNTIME_IO = 1001,
  REML_DIAG_RUNTIME_ENV = 1002,
  REML_DIAG_RUNTIME_ARGS = 1003,
  REML_DIAG_RUNTIME_CWD = 1004,
  REML_DIAG_RUNTIME_TIME = 1005
} reml_diag_id;

typedef struct {
  bool ok;
  int32_t diagnostic_id;
  const char *message;
  size_t message_len;
} reml_result;

reml_result reml_result_ok(void);
reml_result reml_result_err(int32_t diagnostic_id, const char *message, size_t message_len);

void reml_panic(reml_str message, int32_t diagnostic_id);

typedef struct {
  uint8_t *ptr;
  size_t len;
} reml_bytes;

void *reml_alloc(size_t size);
void reml_free(void *ptr);

typedef struct {
  uint8_t *buffer;
  size_t capacity;
  size_t offset;
} reml_arena;

bool reml_arena_init(reml_arena *arena, size_t capacity);
void reml_arena_reset(reml_arena *arena);
void reml_arena_deinit(reml_arena *arena);
void *reml_arena_alloc(reml_arena *arena, size_t size, size_t alignment);

reml_result reml_print(reml_str message);
reml_result reml_eprint(reml_str message);
reml_result reml_read_file(reml_str path, reml_bytes *out_bytes);
reml_result reml_write_file(reml_str path, reml_bytes data);
void reml_bytes_free(reml_bytes bytes);

reml_result reml_time_now(int64_t *out_unix_ms);
reml_result reml_env_get(reml_str key, reml_str *out_value);
void reml_runtime_set_args(int argc, const char **argv);
reml_result reml_args(reml_str **out_args, size_t *out_len);
reml_result reml_cwd(reml_string *out_cwd);

#ifdef __cplusplus
}
#endif

#endif
