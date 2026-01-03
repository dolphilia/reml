include(FetchContent)

set(FETCHCONTENT_QUIET OFF)

if(POLICY CMP0169)
  cmake_policy(SET CMP0169 OLD)
endif()

function(reml_declare_dependency name repo tag)
  FetchContent_Declare(
    ${name}
    GIT_REPOSITORY ${repo}
    GIT_TAG ${tag}
  )
endfunction()

# Version tags are placeholders until first fetch; validate before pinning.
reml_declare_dependency(uthash https://github.com/troydhanson/uthash.git v2.3.0)
reml_declare_dependency(argparse https://github.com/cofyc/argparse.git v1.1.0)
reml_declare_dependency(yyjson https://github.com/ibireme/yyjson.git 0.9.0)
reml_declare_dependency(tomlc99 https://github.com/cktan/tomlc99.git 26b9c1ea770dab2378e5041b695d24ccebe58a7a)
reml_declare_dependency(libtommath https://github.com/libtom/libtommath.git v1.2.0)
reml_declare_dependency(utf8proc https://github.com/JuliaStrings/utf8proc.git v2.8.0)
reml_declare_dependency(libgrapheme https://github.com/tekknolagi/libgrapheme.git 2.0.1)
reml_declare_dependency(cmocka https://git.cryptomilk.org/projects/cmocka.git cmocka-1.1.7)
reml_declare_dependency(logc https://github.com/rxi/log.c.git f9ea34994bd58ed342d2245cd4110bb5c6790153)
reml_declare_dependency(tinydir https://github.com/cxong/tinydir.git 1.2.6)
reml_declare_dependency(uuid4 https://github.com/kokke/tiny-uuid4.git v1.0.0)
reml_declare_dependency(blake3 https://github.com/BLAKE3-team/BLAKE3.git 1.5.0)

function(reml_make_core_dependencies)
  FetchContent_GetProperties(uthash)
  if(NOT uthash_POPULATED)
    FetchContent_Populate(uthash)
  endif()

  FetchContent_GetProperties(argparse)
  if(NOT argparse_POPULATED)
    FetchContent_Populate(argparse)
  endif()

  FetchContent_GetProperties(logc)
  if(NOT logc_POPULATED)
    FetchContent_Populate(logc)
  endif()

  FetchContent_GetProperties(utf8proc)
  if(NOT utf8proc_POPULATED)
    FetchContent_Populate(utf8proc)
  endif()

  FetchContent_GetProperties(libgrapheme)
  if(NOT libgrapheme_POPULATED)
    FetchContent_Populate(libgrapheme)
  endif()

  FetchContent_GetProperties(tomlc99)
  if(NOT tomlc99_POPULATED)
    FetchContent_Populate(tomlc99)
  endif()

  FetchContent_GetProperties(libtommath)
  if(NOT libtommath_POPULATED)
    FetchContent_Populate(libtommath)
  endif()

  add_library(reml_argparse STATIC ${argparse_SOURCE_DIR}/argparse.c)
  target_include_directories(reml_argparse PUBLIC ${argparse_SOURCE_DIR})

  add_library(reml_logc STATIC ${logc_SOURCE_DIR}/src/log.c)
  target_include_directories(reml_logc PUBLIC ${logc_SOURCE_DIR}/src)

  add_library(reml_utf8proc STATIC ${utf8proc_SOURCE_DIR}/utf8proc.c)
  target_include_directories(reml_utf8proc PUBLIC ${utf8proc_SOURCE_DIR})

  set(REML_GRAPHEME_LIB ${libgrapheme_SOURCE_DIR}/libgrapheme.a)
  add_custom_command(
    OUTPUT ${REML_GRAPHEME_LIB}
    COMMAND ${CMAKE_MAKE_PROGRAM} libgrapheme.a
    WORKING_DIRECTORY ${libgrapheme_SOURCE_DIR}
    COMMENT "Building libgrapheme via upstream Makefile"
  )
  add_custom_target(reml_grapheme_build DEPENDS ${REML_GRAPHEME_LIB})
  add_library(reml_grapheme STATIC IMPORTED GLOBAL)
  set_target_properties(reml_grapheme PROPERTIES IMPORTED_LOCATION ${REML_GRAPHEME_LIB})

  add_library(reml_tomlc99 STATIC ${tomlc99_SOURCE_DIR}/toml.c)
  target_include_directories(reml_tomlc99 PUBLIC ${tomlc99_SOURCE_DIR})

  file(GLOB REML_TOMMATH_SOURCES "${libtommath_SOURCE_DIR}/*.c")
  add_library(reml_tommath STATIC ${REML_TOMMATH_SOURCES})
  target_include_directories(reml_tommath PUBLIC ${libtommath_SOURCE_DIR})

  set(REML_TOMMATH_TARGET reml_tommath PARENT_SCOPE)
  set(REML_TOMMATH_INCLUDE_DIR ${libtommath_SOURCE_DIR} PARENT_SCOPE)
  set(REML_GRAPHEME_TARGET reml_grapheme PARENT_SCOPE)
  set(REML_GRAPHEME_BUILD_TARGET reml_grapheme_build PARENT_SCOPE)

  set(REML_UTHASH_INCLUDE_DIR ${uthash_SOURCE_DIR}/src PARENT_SCOPE)
endfunction()

function(reml_make_test_dependencies)
  FetchContent_MakeAvailable(cmocka)
  if(TARGET cmocka)
    set(REML_CMOCKA_TARGET cmocka PARENT_SCOPE)
  elseif(TARGET cmocka-static)
    set(REML_CMOCKA_TARGET cmocka-static PARENT_SCOPE)
  else()
    message(FATAL_ERROR "cmocka target not found")
  endif()
endfunction()
