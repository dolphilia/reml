#ifndef REML_OS_H
#define REML_OS_H

#include "reml_platform.h"
#include <stddef.h>

#ifdef REML_PLATFORM_WINDOWS
#include <windows.h>
#else
#include <pthread.h>
#endif

typedef enum {
    REML_OS_SUCCESS = 0,
    REML_OS_ERROR_INVALID_ARGUMENT = 1,
    REML_OS_ERROR_SYSTEM_FAILURE = 2,
    REML_OS_ERROR_NOT_SUPPORTED = 3
} reml_os_result_t;

typedef struct {
#ifdef REML_PLATFORM_WINDOWS
    HANDLE handle;
#else
    int fd;
#endif
} reml_os_file_t;

typedef struct {
#ifdef REML_PLATFORM_WINDOWS
    HANDLE handle;
#else
    pthread_t thread;
#endif
    int active;
} reml_os_thread_t;

typedef void (*reml_os_thread_entry_t)(void* context);

reml_os_result_t reml_os_file_open_read(const char* utf8_path,
                                        reml_os_file_t* out_file);
reml_os_result_t reml_os_file_open_write(const char* utf8_path,
                                         int truncate,
                                         reml_os_file_t* out_file);
reml_os_result_t reml_os_file_read(reml_os_file_t* file,
                                   void* buffer,
                                   size_t buffer_size,
                                   size_t* bytes_read);
reml_os_result_t reml_os_file_write(reml_os_file_t* file,
                                    const void* buffer,
                                    size_t buffer_size,
                                    size_t* bytes_written);
void reml_os_file_close(reml_os_file_t* file);

reml_os_result_t reml_os_file_write_all(reml_os_file_t* file,
                                        const void* buffer,
                                        size_t buffer_size);

reml_os_file_t reml_os_stdout(void);
reml_os_file_t reml_os_stderr(void);

reml_os_result_t reml_os_thread_start(reml_os_thread_t* thread,
                                      reml_os_thread_entry_t entry,
                                      void* context);
reml_os_result_t reml_os_thread_join(reml_os_thread_t* thread);

size_t reml_os_last_error_message(char* buffer, size_t buffer_size);
void reml_os_clear_last_error(void);

#endif /* REML_OS_H */
