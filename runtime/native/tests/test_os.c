#include "../include/reml_os.h"
#include "../include/reml_platform.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef REML_PLATFORM_WINDOWS
#include <windows.h>
#else
#include <unistd.h>
#endif

static const char* test_make_temp_path(char* buffer, size_t buffer_size) {
#if REML_COMPILER_MSVC
    errno_t err = tmpnam_s(buffer, buffer_size);
    if (err != 0) {
        return NULL;
    }
    return buffer;
#else
    const char* pattern = "/tmp/reml_os_test_XXXXXX";
    size_t pattern_len = strlen(pattern);
    if (pattern_len + 1 > buffer_size) {
        return NULL;
    }
    memcpy(buffer, pattern, pattern_len + 1);
    int fd = mkstemp(buffer);
    if (fd == -1) {
        return NULL;
    }
    close(fd);
    return buffer;
#endif
}

static void test_file_roundtrip(void) {
    char path_buffer[L_tmpnam];
    const char* path = test_make_temp_path(path_buffer, sizeof(path_buffer));
    assert(path != NULL);

    reml_os_file_t file;
    reml_os_result_t result =
        reml_os_file_open_write(path, /*truncate=*/1, &file);
    assert(result == REML_OS_SUCCESS);

    const char* payload = "runtime_os_test_payload";
    result =
        reml_os_file_write_all(&file, payload, strlen(payload));
    assert(result == REML_OS_SUCCESS);
    reml_os_file_close(&file);

    result = reml_os_file_open_read(path, &file);
    assert(result == REML_OS_SUCCESS);

    char read_buffer[64];
    size_t bytes_read = 0;
    result = reml_os_file_read(&file, read_buffer,
                               sizeof(read_buffer) - 1, &bytes_read);
    assert(result == REML_OS_SUCCESS);
    read_buffer[bytes_read] = '\0';
    assert(strcmp(read_buffer, payload) == 0);

    reml_os_file_close(&file);
    remove(path);
}

static void test_error_message(void) {
    reml_os_file_t file;
    reml_os_result_t result = reml_os_file_open_read(
        "this_file_should_not_exist.reml", &file);
    assert(result != REML_OS_SUCCESS);

    char message[128];
    size_t length = reml_os_last_error_message(message, sizeof(message));
    assert(length > 0);
    reml_os_clear_last_error();
}

static void test_standard_handles(void) {
    reml_os_file_t stdout_file = reml_os_stdout();
    reml_os_file_t stderr_file = reml_os_stderr();

#ifdef REML_PLATFORM_WINDOWS
    assert(stdout_file.handle != NULL);
    assert(stdout_file.handle != INVALID_HANDLE_VALUE);
    assert(stderr_file.handle != NULL);
    assert(stderr_file.handle != INVALID_HANDLE_VALUE);
#else
    assert(stdout_file.fd >= 0);
    assert(stderr_file.fd >= 0);
#endif
}

int main(void) {
    printf("==================================================\n");
    printf("OS Abstraction Test Suite\n");
    printf("==================================================\n\n");

    test_file_roundtrip();
    test_error_message();
    test_standard_handles();

    printf("\nAll tests passed.\n");
    return 0;
}
