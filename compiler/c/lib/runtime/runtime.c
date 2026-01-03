#include "runtime.h"

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#if defined(_WIN32)
#include <direct.h>
#include <windows.h>
#else
#include <unistd.h>
#endif

static const char *reml_diag_message_oom = "out of memory";
static const char *reml_diag_message_io = "io failure";
static const char *reml_diag_message_env = "environment variable not found";
static const char *reml_diag_message_args = "runtime arguments not initialized";
static const char *reml_diag_message_cwd = "failed to get current working directory";
static const char *reml_diag_message_time = "failed to get current time";

static reml_str *reml_runtime_args = NULL;
static size_t reml_runtime_args_len = 0;

static reml_str reml_str_from_cstr(const char *message) {
  reml_str result = {message, message ? strlen(message) : 0};
  return result;
}

static reml_result reml_result_from_errno(int32_t diagnostic_id, const char *message) {
  size_t len = message ? strlen(message) : 0;
  return reml_result_err(diagnostic_id, message, len);
}

static char *reml_copy_str_to_cstr(reml_str input) {
  char *buf = (char *)reml_alloc(input.len + 1);
  if (!buf) {
    return NULL;
  }
  if (input.len > 0 && input.ptr) {
    memcpy(buf, input.ptr, input.len);
  }
  buf[input.len] = '\0';
  return buf;
}

reml_result reml_result_ok(void) {
  reml_result result = {true, REML_DIAG_NONE, NULL, 0};
  return result;
}

reml_result reml_result_err(int32_t diagnostic_id, const char *message, size_t message_len) {
  reml_result result = {false, diagnostic_id, message, message_len};
  return result;
}

void reml_panic(reml_str message, int32_t diagnostic_id) {
  if (message.ptr && message.len > 0) {
    fwrite(message.ptr, 1, message.len, stderr);
    fputc('\n', stderr);
  }
  if (diagnostic_id != REML_DIAG_NONE) {
    fprintf(stderr, "diagnostic_id=%d\n", diagnostic_id);
  }
  fflush(stderr);
  exit(REML_PANIC_EXIT_CODE);
}

void *reml_alloc(size_t size) {
  void *ptr = malloc(size);
  if (!ptr && size > 0) {
    reml_panic(reml_str_from_cstr(reml_diag_message_oom), REML_DIAG_RUNTIME_OOM);
  }
  return ptr;
}

void reml_free(void *ptr) {
  free(ptr);
}

bool reml_arena_init(reml_arena *arena, size_t capacity) {
  if (!arena) {
    return false;
  }
  arena->buffer = (uint8_t *)reml_alloc(capacity);
  arena->capacity = capacity;
  arena->offset = 0;
  return arena->buffer != NULL;
}

void reml_arena_reset(reml_arena *arena) {
  if (!arena) {
    return;
  }
  arena->offset = 0;
}

void reml_arena_deinit(reml_arena *arena) {
  if (!arena) {
    return;
  }
  reml_free(arena->buffer);
  arena->buffer = NULL;
  arena->capacity = 0;
  arena->offset = 0;
}

void *reml_arena_alloc(reml_arena *arena, size_t size, size_t alignment) {
  if (!arena || size == 0) {
    return NULL;
  }
  if (alignment == 0) {
    alignment = sizeof(void *);
  }
  size_t aligned = (arena->offset + alignment - 1) / alignment * alignment;
  size_t next = aligned + size;
  if (next > arena->capacity) {
    size_t new_capacity = arena->capacity > 0 ? arena->capacity : 1024;
    while (next > new_capacity) {
      new_capacity *= 2;
    }
    uint8_t *new_buffer = (uint8_t *)realloc(arena->buffer, new_capacity);
    if (!new_buffer) {
      reml_panic(reml_str_from_cstr(reml_diag_message_oom), REML_DIAG_RUNTIME_OOM);
    }
    arena->buffer = new_buffer;
    arena->capacity = new_capacity;
  }
  void *ptr = arena->buffer + aligned;
  arena->offset = next;
  return ptr;
}

reml_result reml_print(reml_str message) {
  if (message.ptr && message.len > 0) {
    if (fwrite(message.ptr, 1, message.len, stdout) != message.len) {
      return reml_result_from_errno(REML_DIAG_RUNTIME_IO, reml_diag_message_io);
    }
  }
  fflush(stdout);
  return reml_result_ok();
}

reml_result reml_eprint(reml_str message) {
  if (message.ptr && message.len > 0) {
    if (fwrite(message.ptr, 1, message.len, stderr) != message.len) {
      return reml_result_from_errno(REML_DIAG_RUNTIME_IO, reml_diag_message_io);
    }
  }
  fflush(stderr);
  return reml_result_ok();
}

reml_result reml_read_file(reml_str path, reml_bytes *out_bytes) {
  if (!out_bytes) {
    return reml_result_from_errno(REML_DIAG_RUNTIME_IO, reml_diag_message_io);
  }
  char *path_c = reml_copy_str_to_cstr(path);
  if (!path_c) {
    return reml_result_from_errno(REML_DIAG_RUNTIME_OOM, reml_diag_message_oom);
  }
  FILE *fp = fopen(path_c, "rb");
  reml_free(path_c);
  if (!fp) {
    return reml_result_from_errno(REML_DIAG_RUNTIME_IO, reml_diag_message_io);
  }
  if (fseek(fp, 0, SEEK_END) != 0) {
    fclose(fp);
    return reml_result_from_errno(REML_DIAG_RUNTIME_IO, reml_diag_message_io);
  }
  long length = ftell(fp);
  if (length < 0) {
    fclose(fp);
    return reml_result_from_errno(REML_DIAG_RUNTIME_IO, reml_diag_message_io);
  }
  if (fseek(fp, 0, SEEK_SET) != 0) {
    fclose(fp);
    return reml_result_from_errno(REML_DIAG_RUNTIME_IO, reml_diag_message_io);
  }
  out_bytes->len = (size_t)length;
  out_bytes->ptr = (uint8_t *)reml_alloc(out_bytes->len ? out_bytes->len : 1);
  if (!out_bytes->ptr) {
    fclose(fp);
    return reml_result_from_errno(REML_DIAG_RUNTIME_OOM, reml_diag_message_oom);
  }
  size_t read = fread(out_bytes->ptr, 1, out_bytes->len, fp);
  fclose(fp);
  if (read != out_bytes->len) {
    reml_bytes_free(*out_bytes);
    return reml_result_from_errno(REML_DIAG_RUNTIME_IO, reml_diag_message_io);
  }
  return reml_result_ok();
}

reml_result reml_write_file(reml_str path, reml_bytes data) {
  char *path_c = reml_copy_str_to_cstr(path);
  if (!path_c) {
    return reml_result_from_errno(REML_DIAG_RUNTIME_OOM, reml_diag_message_oom);
  }
  FILE *fp = fopen(path_c, "wb");
  reml_free(path_c);
  if (!fp) {
    return reml_result_from_errno(REML_DIAG_RUNTIME_IO, reml_diag_message_io);
  }
  size_t written = 0;
  if (data.ptr && data.len > 0) {
    written = fwrite(data.ptr, 1, data.len, fp);
  }
  fclose(fp);
  if (written != data.len) {
    return reml_result_from_errno(REML_DIAG_RUNTIME_IO, reml_diag_message_io);
  }
  return reml_result_ok();
}

void reml_bytes_free(reml_bytes bytes) {
  reml_free(bytes.ptr);
}

reml_result reml_time_now(int64_t *out_unix_ms) {
  if (!out_unix_ms) {
    return reml_result_from_errno(REML_DIAG_RUNTIME_TIME, reml_diag_message_time);
  }
#if defined(_WIN32)
  FILETIME ft;
  GetSystemTimeAsFileTime(&ft);
  ULARGE_INTEGER uli;
  uli.LowPart = ft.dwLowDateTime;
  uli.HighPart = ft.dwHighDateTime;
  uint64_t windows_ticks = uli.QuadPart;
  uint64_t unix_ticks = windows_ticks - 116444736000000000ULL;
  *out_unix_ms = (int64_t)(unix_ticks / 10000ULL);
  return reml_result_ok();
#else
  struct timespec ts;
  if (clock_gettime(CLOCK_REALTIME, &ts) != 0) {
    return reml_result_from_errno(REML_DIAG_RUNTIME_TIME, reml_diag_message_time);
  }
  *out_unix_ms = (int64_t)ts.tv_sec * 1000 + (int64_t)(ts.tv_nsec / 1000000);
  return reml_result_ok();
#endif
}

reml_result reml_env_get(reml_str key, reml_str *out_value) {
  if (!out_value) {
    return reml_result_from_errno(REML_DIAG_RUNTIME_ENV, reml_diag_message_env);
  }
  char *key_c = reml_copy_str_to_cstr(key);
  if (!key_c) {
    return reml_result_from_errno(REML_DIAG_RUNTIME_OOM, reml_diag_message_oom);
  }
  const char *value = getenv(key_c);
  reml_free(key_c);
  if (!value) {
    return reml_result_from_errno(REML_DIAG_RUNTIME_ENV, reml_diag_message_env);
  }
  out_value->ptr = value;
  out_value->len = strlen(value);
  return reml_result_ok();
}

void reml_runtime_set_args(int argc, const char **argv) {
  if (reml_runtime_args) {
    reml_free(reml_runtime_args);
    reml_runtime_args = NULL;
    reml_runtime_args_len = 0;
  }
  if (argc <= 0 || !argv) {
    return;
  }
  reml_runtime_args = (reml_str *)reml_alloc(sizeof(reml_str) * (size_t)argc);
  reml_runtime_args_len = (size_t)argc;
  for (size_t i = 0; i < reml_runtime_args_len; i++) {
    const char *arg = argv[i];
    reml_runtime_args[i].ptr = arg;
    reml_runtime_args[i].len = arg ? strlen(arg) : 0;
  }
}

reml_result reml_args(reml_str **out_args, size_t *out_len) {
  if (!out_args || !out_len) {
    return reml_result_from_errno(REML_DIAG_RUNTIME_ARGS, reml_diag_message_args);
  }
  if (!reml_runtime_args) {
    *out_args = NULL;
    *out_len = 0;
    return reml_result_from_errno(REML_DIAG_RUNTIME_ARGS, reml_diag_message_args);
  }
  *out_args = reml_runtime_args;
  *out_len = reml_runtime_args_len;
  return reml_result_ok();
}

reml_result reml_cwd(reml_string *out_cwd) {
  if (!out_cwd) {
    return reml_result_from_errno(REML_DIAG_RUNTIME_CWD, reml_diag_message_cwd);
  }
  size_t size = 256;
  for (;;) {
    char *buffer = (char *)reml_alloc(size);
    if (!buffer) {
      return reml_result_from_errno(REML_DIAG_RUNTIME_OOM, reml_diag_message_oom);
    }
#if defined(_WIN32)
    char *result = _getcwd(buffer, (int)size);
#else
    char *result = getcwd(buffer, size);
#endif
    if (result) {
      out_cwd->ptr = buffer;
      out_cwd->len = strlen(buffer);
      return reml_result_ok();
    }
    reml_free(buffer);
    if (errno != ERANGE) {
      return reml_result_from_errno(REML_DIAG_RUNTIME_CWD, reml_diag_message_cwd);
    }
    size *= 2;
  }
}
