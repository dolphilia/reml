cmake_minimum_required(VERSION 3.10)

project(reml_c LANGUAGES C CXX)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_C_EXTENSIONS OFF)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

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
)

target_include_directories(reml_core
  PUBLIC
    include
    ${REML_UTHASH_INCLUDE_DIR}
)

target_link_libraries(reml_core
  PUBLIC
    reml_logc
    reml_utf8proc
    reml_tomlc99
)

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
)

target_link_libraries(reml_unit_tests
  PRIVATE
    reml_core
    ${REML_CMOCKA_TARGET}
)

add_test(NAME reml_unit_tests COMMAND reml_unit_tests)
