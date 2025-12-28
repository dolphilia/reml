cmake_minimum_required(VERSION 3.20)
project(reml_runtime C)

option(REML_RUNTIME_ENABLE_DEBUG "Enable runtime debug instrumentation" OFF)

set(RUNTIME_SOURCES
    src/array.c
    src/boxing.c
    src/closure.c
    src/ffi_bridge.c
    src/mem_alloc.c
    src/intrinsics.c
    src/panic.c
    src/print_i64.c
    src/refcount.c
    src/record.c
    src/os.c
    src/string_compare.c
    src/string_data.c
    src/set.c
)

add_library(reml_runtime STATIC ${RUNTIME_SOURCES})
target_include_directories(reml_runtime PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)

set_target_properties(reml_runtime PROPERTIES
    C_STANDARD 11
    C_STANDARD_REQUIRED ON
)

if(MSVC)
    target_compile_options(reml_runtime PRIVATE /W4 /WX- /O2 /permissive-)
    target_compile_definitions(reml_runtime PRIVATE _CRT_SECURE_NO_WARNINGS WIN32_LEAN_AND_MEAN)
else()
    target_compile_options(reml_runtime PRIVATE -Wall -Wextra -Wpedantic -O2)
endif()

if(REML_RUNTIME_ENABLE_DEBUG)
    target_compile_definitions(reml_runtime PUBLIC DEBUG)
endif()

function(reml_runtime_add_test name)
    add_executable(${name} tests/${name}.c)
    target_link_libraries(${name} PRIVATE reml_runtime)
    if(MSVC)
        target_compile_options(${name} PRIVATE /W4 /WX- /permissive-)
        target_compile_definitions(${name} PRIVATE _CRT_SECURE_NO_WARNINGS WIN32_LEAN_AND_MEAN)
    else()
        target_compile_options(${name} PRIVATE -Wall -Wextra -Wpedantic)
    endif()
    add_test(NAME ${name} COMMAND ${name})
endfunction()

include(CTest)
if(BUILD_TESTING)
    reml_runtime_add_test(test_mem_alloc)
    reml_runtime_add_test(test_refcount)
    reml_runtime_add_test(test_ffi_bridge)
    reml_runtime_add_test(test_os)
    reml_runtime_add_test(test_set)
endif()
