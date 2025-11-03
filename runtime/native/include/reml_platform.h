#ifndef REML_PLATFORM_H
#define REML_PLATFORM_H

/**
 * reml_platform.h - プラットフォーム判定と補助マクロ
 *
 * ランタイム実装が対象 OS/コンパイラごとの差異を扱いやすくするための
 * 定義を提供する。MSVC 向けビルドでは __declspec ベースの属性や
 * Windows API を使用し、POSIX 系ビルドでは既存の GCC/Clang 拡張を使用する。
 */

#if defined(_WIN32) || defined(_WIN64)
#define REML_PLATFORM_WINDOWS 1
#else
#define REML_PLATFORM_POSIX 1
#endif

#if defined(_MSC_VER) && !defined(__clang__)
#define REML_COMPILER_MSVC 1
#else
#define REML_COMPILER_MSVC 0
#endif

#if defined(__clang__)
#define REML_COMPILER_CLANG 1
#else
#define REML_COMPILER_CLANG 0
#endif

#if defined(__GNUC__) && !defined(__clang__)
#define REML_COMPILER_GCC 1
#else
#define REML_COMPILER_GCC 0
#endif

#if REML_COMPILER_MSVC
#define REML_NORETURN __declspec(noreturn)
#else
#define REML_NORETURN __attribute__((noreturn))
#endif

#if REML_COMPILER_MSVC
#define REML_THREAD_LOCAL __declspec(thread)
#else
#define REML_THREAD_LOCAL _Thread_local
#endif

#endif /* REML_PLATFORM_H */
