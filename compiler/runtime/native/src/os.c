#include "../include/reml_os.h"
#include "../include/reml_platform.h"
#include <errno.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef REML_PLATFORM_WINDOWS
#include <wchar.h>
#else
#include <fcntl.h>
#include <unistd.h>
#endif

typedef struct {
#ifdef REML_PLATFORM_WINDOWS
    DWORD win32_error;
#else
    int posix_errno;
#endif
} reml_os_error_state_t;

static REML_THREAD_LOCAL reml_os_error_state_t g_last_error = {0};

void reml_os_clear_last_error(void) {
#ifdef REML_PLATFORM_WINDOWS
    g_last_error.win32_error = 0;
#else
    g_last_error.posix_errno = 0;
#endif
}

static void reml_os_set_system_error(void) {
#ifdef REML_PLATFORM_WINDOWS
    g_last_error.win32_error = GetLastError();
#else
    g_last_error.posix_errno = errno;
#endif
}

size_t reml_os_last_error_message(char* buffer, size_t buffer_size) {
    if (buffer == NULL || buffer_size == 0) {
        return 0;
    }

#ifdef REML_PLATFORM_WINDOWS
    DWORD code = g_last_error.win32_error;
    if (code == 0) {
        buffer[0] = '\0';
        return 0;
    }

    DWORD length = FormatMessageA(
        FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
        NULL,
        code,
        MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT),
        buffer,
        (DWORD)buffer_size,
        NULL);

    if (length == 0) {
        snprintf(buffer, buffer_size, "Win32 error %lu", code);
        return strnlen(buffer, buffer_size);
    }

    // FormatMessageA で末尾に改行が付与される場合があるため除去
    while (length > 0 &&
           (buffer[length - 1] == '\r' || buffer[length - 1] == '\n')) {
        buffer[--length] = '\0';
    }
    return (size_t)length;
#else
    int code = g_last_error.posix_errno;
    if (code == 0) {
        buffer[0] = '\0';
        return 0;
    }

    const char* msg = strerror(code);
    if (msg == NULL) {
        snprintf(buffer, buffer_size, "POSIX error %d", code);
    } else {
        strncpy(buffer, msg, buffer_size);
        buffer[buffer_size - 1] = '\0';
    }
    return strnlen(buffer, buffer_size);
#endif
}

#ifdef REML_PLATFORM_WINDOWS
static wchar_t* reml_os_utf8_to_wide(const char* utf8_path) {
    if (utf8_path == NULL) {
        return NULL;
    }

    int required = MultiByteToWideChar(CP_UTF8,
                                       MB_ERR_INVALID_CHARS,
                                       utf8_path,
                                       -1,
                                       NULL,
                                       0);
    if (required <= 0) {
        reml_os_set_system_error();
        return NULL;
    }

    wchar_t* buffer =
        (wchar_t*)malloc((size_t)required * sizeof(wchar_t));
    if (buffer == NULL) {
        return NULL;
    }

    int converted = MultiByteToWideChar(CP_UTF8,
                                        MB_ERR_INVALID_CHARS,
                                        utf8_path,
                                        -1,
                                        buffer,
                                        required);
    if (converted == 0) {
        reml_os_set_system_error();
        free(buffer);
        return NULL;
    }

    return buffer;
}
#endif

static reml_os_result_t reml_os_validate_path(const char* path) {
    if (path == NULL || path[0] == '\0') {
        return REML_OS_ERROR_INVALID_ARGUMENT;
    }
    return REML_OS_SUCCESS;
}

reml_os_result_t reml_os_file_open_read(const char* utf8_path,
                                        reml_os_file_t* out_file) {
    if (out_file == NULL) {
        return REML_OS_ERROR_INVALID_ARGUMENT;
    }

    reml_os_result_t validation = reml_os_validate_path(utf8_path);
    if (validation != REML_OS_SUCCESS) {
        return validation;
    }

#ifdef REML_PLATFORM_WINDOWS
    wchar_t* wide_path = reml_os_utf8_to_wide(utf8_path);
    if (wide_path == NULL) {
        return REML_OS_ERROR_SYSTEM_FAILURE;
    }

    HANDLE handle = CreateFileW(wide_path,
                                GENERIC_READ,
                                FILE_SHARE_READ | FILE_SHARE_WRITE,
                                NULL,
                                OPEN_EXISTING,
                                FILE_ATTRIBUTE_NORMAL,
                                NULL);
    free(wide_path);

    if (handle == INVALID_HANDLE_VALUE) {
        reml_os_set_system_error();
        return REML_OS_ERROR_SYSTEM_FAILURE;
    }

    out_file->handle = handle;
#else
    int fd = open(utf8_path, O_RDONLY);
    if (fd == -1) {
        reml_os_set_system_error();
        return REML_OS_ERROR_SYSTEM_FAILURE;
    }
    out_file->fd = fd;
#endif

    reml_os_clear_last_error();
    return REML_OS_SUCCESS;
}

reml_os_result_t reml_os_file_open_write(const char* utf8_path,
                                         int truncate,
                                         reml_os_file_t* out_file) {
    if (out_file == NULL) {
        return REML_OS_ERROR_INVALID_ARGUMENT;
    }

    reml_os_result_t validation = reml_os_validate_path(utf8_path);
    if (validation != REML_OS_SUCCESS) {
        return validation;
    }

#ifdef REML_PLATFORM_WINDOWS
    wchar_t* wide_path = reml_os_utf8_to_wide(utf8_path);
    if (wide_path == NULL) {
        return REML_OS_ERROR_SYSTEM_FAILURE;
    }

    DWORD creation = truncate ? CREATE_ALWAYS : OPEN_ALWAYS;
    HANDLE handle = CreateFileW(wide_path,
                                GENERIC_WRITE,
                                FILE_SHARE_READ,
                                NULL,
                                creation,
                                FILE_ATTRIBUTE_NORMAL,
                                NULL);
    free(wide_path);

    if (handle == INVALID_HANDLE_VALUE) {
        reml_os_set_system_error();
        return REML_OS_ERROR_SYSTEM_FAILURE;
    }

    if (!truncate) {
        SetFilePointer(handle, 0, NULL, FILE_END);
    }

    out_file->handle = handle;
#else
    int flags = O_WRONLY | O_CREAT;
    if (truncate) {
        flags |= O_TRUNC;
    } else {
        flags |= O_APPEND;
    }

    int fd = open(utf8_path, flags, 0666);
    if (fd == -1) {
        reml_os_set_system_error();
        return REML_OS_ERROR_SYSTEM_FAILURE;
    }
    out_file->fd = fd;
#endif

    reml_os_clear_last_error();
    return REML_OS_SUCCESS;
}

reml_os_result_t reml_os_file_read(reml_os_file_t* file,
                                   void* buffer,
                                   size_t buffer_size,
                                   size_t* bytes_read) {
    if (file == NULL || buffer == NULL || bytes_read == NULL) {
        return REML_OS_ERROR_INVALID_ARGUMENT;
    }

#ifdef REML_PLATFORM_WINDOWS
    DWORD read_bytes = 0;
    if (!ReadFile(file->handle, buffer, (DWORD)buffer_size, &read_bytes, NULL)) {
        reml_os_set_system_error();
        *bytes_read = 0;
        return REML_OS_ERROR_SYSTEM_FAILURE;
    }
    *bytes_read = (size_t)read_bytes;
#else
    ssize_t result = read(file->fd, buffer, buffer_size);
    if (result < 0) {
        reml_os_set_system_error();
        *bytes_read = 0;
        return REML_OS_ERROR_SYSTEM_FAILURE;
    }
    *bytes_read = (size_t)result;
#endif

    reml_os_clear_last_error();
    return REML_OS_SUCCESS;
}

reml_os_result_t reml_os_file_write(reml_os_file_t* file,
                                    const void* buffer,
                                    size_t buffer_size,
                                    size_t* bytes_written) {
    if (file == NULL || buffer == NULL || bytes_written == NULL) {
        return REML_OS_ERROR_INVALID_ARGUMENT;
    }

#ifdef REML_PLATFORM_WINDOWS
    DWORD written = 0;
    if (!WriteFile(file->handle,
                   buffer,
                   (DWORD)buffer_size,
                   &written,
                   NULL)) {
        reml_os_set_system_error();
        *bytes_written = 0;
        return REML_OS_ERROR_SYSTEM_FAILURE;
    }
    *bytes_written = (size_t)written;
#else
    ssize_t result = write(file->fd, buffer, buffer_size);
    if (result < 0) {
        reml_os_set_system_error();
        *bytes_written = 0;
        return REML_OS_ERROR_SYSTEM_FAILURE;
    }
    *bytes_written = (size_t)result;
#endif

    reml_os_clear_last_error();
    return REML_OS_SUCCESS;
}

reml_os_result_t reml_os_file_write_all(reml_os_file_t* file,
                                        const void* buffer,
                                        size_t buffer_size) {
    if (file == NULL || buffer == NULL) {
        return REML_OS_ERROR_INVALID_ARGUMENT;
    }

    const uint8_t* bytes = (const uint8_t*)buffer;
    size_t total_written = 0;

    while (total_written < buffer_size) {
        size_t chunk_written = 0;
        reml_os_result_t result = reml_os_file_write(
            file, bytes + total_written, buffer_size - total_written,
            &chunk_written);
        if (result != REML_OS_SUCCESS) {
            return result;
        }
        if (chunk_written == 0) {
            return REML_OS_ERROR_SYSTEM_FAILURE;
        }
        total_written += chunk_written;
    }

    return REML_OS_SUCCESS;
}

reml_os_file_t reml_os_stdout(void) {
    reml_os_file_t file;
#ifdef REML_PLATFORM_WINDOWS
    file.handle = GetStdHandle(STD_OUTPUT_HANDLE);
#else
    file.fd = STDOUT_FILENO;
#endif
    return file;
}

reml_os_file_t reml_os_stderr(void) {
    reml_os_file_t file;
#ifdef REML_PLATFORM_WINDOWS
    file.handle = GetStdHandle(STD_ERROR_HANDLE);
#else
    file.fd = STDERR_FILENO;
#endif
    return file;
}

void reml_os_file_close(reml_os_file_t* file) {
    if (file == NULL) {
        return;
    }

#ifdef REML_PLATFORM_WINDOWS
    if (file->handle != NULL && file->handle != INVALID_HANDLE_VALUE) {
        CloseHandle(file->handle);
        file->handle = INVALID_HANDLE_VALUE;
    }
#else
    if (file->fd >= 0) {
        close(file->fd);
        file->fd = -1;
    }
#endif
}

#ifdef REML_PLATFORM_WINDOWS
typedef struct {
    reml_os_thread_entry_t entry;
    void* context;
} reml_thread_payload_t;

static DWORD WINAPI reml_thread_start(LPVOID param) {
    reml_thread_payload_t* payload = (reml_thread_payload_t*)param;
    if (payload != NULL && payload->entry != NULL) {
        payload->entry(payload->context);
    }
    free(payload);
    return 0;
}
#else
typedef struct {
    reml_os_thread_entry_t entry;
    void* context;
} reml_thread_payload_t;

static void* reml_thread_start(void* param) {
    reml_thread_payload_t* payload = (reml_thread_payload_t*)param;
    if (payload != NULL && payload->entry != NULL) {
        payload->entry(payload->context);
    }
    free(payload);
    return NULL;
}
#endif

reml_os_result_t reml_os_thread_start(reml_os_thread_t* thread,
                                      reml_os_thread_entry_t entry,
                                      void* context) {
    if (thread == NULL || entry == NULL) {
        return REML_OS_ERROR_INVALID_ARGUMENT;
    }

    reml_thread_payload_t* payload =
        (reml_thread_payload_t*)malloc(sizeof(reml_thread_payload_t));
    if (payload == NULL) {
        return REML_OS_ERROR_SYSTEM_FAILURE;
    }

    payload->entry = entry;
    payload->context = context;

#ifdef REML_PLATFORM_WINDOWS
    HANDLE handle = CreateThread(NULL,
                                 0,
                                 reml_thread_start,
                                 payload,
                                 0,
                                 NULL);
    if (handle == NULL) {
        free(payload);
        reml_os_set_system_error();
        return REML_OS_ERROR_SYSTEM_FAILURE;
    }
    thread->handle = handle;
#else
    pthread_t tid;
    int result = pthread_create(&tid, NULL, reml_thread_start, payload);
    if (result != 0) {
        free(payload);
        g_last_error.posix_errno = result;
        return REML_OS_ERROR_SYSTEM_FAILURE;
    }
    thread->thread = tid;
#endif

    thread->active = 1;
    reml_os_clear_last_error();
    return REML_OS_SUCCESS;
}

reml_os_result_t reml_os_thread_join(reml_os_thread_t* thread) {
    if (thread == NULL || !thread->active) {
        return REML_OS_ERROR_INVALID_ARGUMENT;
    }

#ifdef REML_PLATFORM_WINDOWS
    DWORD wait_result = WaitForSingleObject(thread->handle, INFINITE);
    if (wait_result != WAIT_OBJECT_0) {
        reml_os_set_system_error();
        return REML_OS_ERROR_SYSTEM_FAILURE;
    }
    CloseHandle(thread->handle);
    thread->handle = NULL;
#else
    int result = pthread_join(thread->thread, NULL);
    if (result != 0) {
        g_last_error.posix_errno = result;
        return REML_OS_ERROR_SYSTEM_FAILURE;
    }
#endif

    thread->active = 0;
    reml_os_clear_last_error();
    return REML_OS_SUCCESS;
}
