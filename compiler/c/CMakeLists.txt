cmake_minimum_required(VERSION 3.10)

project(reml_c LANGUAGES C CXX)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_C_EXTENSIONS OFF)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

option(REML_ENABLE_LLVM "Enable LLVM backend" ON)

if(NOT CMAKE_CONFIGURATION_TYPES AND NOT CMAKE_BUILD_TYPE)
  set(CMAKE_BUILD_TYPE Debug CACHE STRING "Build type" FORCE)
endif()

include(cmake/FetchDependencies.cmake)
reml_make_core_dependencies()

add_library(reml_core
  src/util/logger.c
  src/util/span.c
  src/lexer/lexer.c
  src/ast/ast.c
  src/ast/printer.c
  src/parser/parser.c
  src/parser/operator_table.c
  src/manifest/manifest.c
  src/numeric/bigint.c
  src/numeric/core_numeric.c
  src/text/string.c
  src/text/unicode.c
  src/text/grapheme.c
  src/runtime/enum.c
  src/runtime/effects.c
  src/runtime/effects_core.c
  src/mir/mir.c
  src/typeck/type.c
  src/sema/diagnostic.c
  src/sema/sema.c
)

target_include_directories(reml_core
  PUBLIC
    include
    ${REML_UTHASH_INCLUDE_DIR}
    ${REML_TOMMATH_INCLUDE_DIR}
)

target_link_libraries(reml_core
  PUBLIC
    reml_logc
    reml_utf8proc
    ${REML_GRAPHEME_TARGET}
    reml_tomlc99
    ${REML_TOMMATH_TARGET}
)

if(REML_ENABLE_LLVM)
  find_package(LLVM REQUIRED CONFIG)
  message(STATUS "Found LLVM ${LLVM_PACKAGE_VERSION}")

  add_definitions(${LLVM_DEFINITIONS})
  llvm_map_components_to_libnames(LLVM_LIBS core analysis target asmparser asmprinter support native nativecodegen)

  add_library(reml_codegen
    src/codegen/codegen.c
  )

  target_include_directories(reml_codegen
    SYSTEM PUBLIC
      ${LLVM_INCLUDE_DIRS}
  )

  target_link_libraries(reml_codegen
    PUBLIC
      reml_core
      ${LLVM_LIBS}
  )

  target_include_directories(reml_codegen
    PUBLIC
      include
  )
endif()

add_executable(reml
  src/main.c
)

target_link_libraries(reml
  PRIVATE
    reml_core
    reml_argparse
)

target_include_directories(reml
  PRIVATE
    include
)

if(REML_ENABLE_LLVM)
  target_link_libraries(reml
    PRIVATE
      reml_codegen
  )
endif()

if(MSVC)
  target_compile_options(reml_core PRIVATE /W4)
  target_compile_options(reml PRIVATE /W4)
else()
  target_compile_options(reml_core PRIVATE -Wall -Wextra -Wpedantic)
  target_compile_options(reml PRIVATE -Wall -Wextra -Wpedantic)

  if(CMAKE_BUILD_TYPE STREQUAL "Debug")
    target_compile_options(reml_core PRIVATE -fsanitize=address,undefined)
    target_compile_options(reml PRIVATE -fsanitize=address,undefined)
    target_link_options(reml_core PRIVATE -fsanitize=address,undefined)
    target_link_options(reml PRIVATE -fsanitize=address,undefined)
  endif()
endif()

enable_testing()
reml_make_test_dependencies()

add_executable(reml_unit_tests
  tests/unit/test_main.c
  tests/unit/test_parser_operator_table.c
  tests/unit/test_lexer_basic.c
  tests/unit/test_parser_basic.c
  tests/unit/test_sema.c
  tests/unit/test_bigint_runtime.c
  tests/unit/test_effects_runtime.c
  tests/unit/test_text_unicode.c
)

target_link_libraries(reml_unit_tests
  PRIVATE
    reml_core
    ${REML_CMOCKA_TARGET}
)

add_test(NAME reml_unit_tests COMMAND reml_unit_tests)

if(REML_ENABLE_LLVM)
  add_executable(reml_codegen_tests
    tests/unit/test_codegen_main.c
    tests/unit/test_codegen.c
  )

  target_link_libraries(reml_codegen_tests
    PRIVATE
      reml_codegen
      ${REML_CMOCKA_TARGET}
  )

  add_test(NAME reml_codegen_tests COMMAND reml_codegen_tests)

  find_package(Python3 COMPONENTS Interpreter)
  if(Python3_Interpreter_FOUND)
    add_test(
      NAME reml_codegen_hello_world
      COMMAND ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/integration/codegen_hello.py
              $<TARGET_FILE:reml>
              ${CMAKE_CURRENT_SOURCE_DIR}/tests/integration/hello_world.reml
              ${CMAKE_CURRENT_BINARY_DIR}
    )
  endif()
endif()
