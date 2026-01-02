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
reml_declare_dependency(tomlc99 https://github.com/cktan/tomlc99.git v1.0.0)
reml_declare_dependency(libtommath https://github.com/libtom/libtommath.git v1.2.0)
reml_declare_dependency(utf8proc https://github.com/JuliaStrings/utf8proc.git v2.8.0)
reml_declare_dependency(libgrapheme https://github.com/tekknolagi/libgrapheme.git v2.0.1)
reml_declare_dependency(cmocka https://git.cryptomilk.org/projects/cmocka.git cmocka-1.1.7)
reml_declare_dependency(logc https://github.com/rxi/log.c.git f9ea34994bd58ed342d2245cd4110bb5c6790153)
reml_declare_dependency(tinydir https://github.com/cxong/tinydir.git 1.2.6)
reml_declare_dependency(uuid4 https://github.com/kokke/tiny-uuid4.git v1.0.0)
reml_declare_dependency(blake3 https://github.com/BLAKE3-team/BLAKE3.git 1.5.0)

function(reml_make_core_dependencies)
  FetchContent_GetProperties(argparse)
  if(NOT argparse_POPULATED)
    FetchContent_Populate(argparse)
  endif()

  FetchContent_GetProperties(logc)
  if(NOT logc_POPULATED)
    FetchContent_Populate(logc)
  endif()

  add_library(reml_argparse STATIC ${argparse_SOURCE_DIR}/argparse.c)
  target_include_directories(reml_argparse PUBLIC ${argparse_SOURCE_DIR})

  add_library(reml_logc STATIC ${logc_SOURCE_DIR}/src/log.c)
  target_include_directories(reml_logc PUBLIC ${logc_SOURCE_DIR}/src)
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
