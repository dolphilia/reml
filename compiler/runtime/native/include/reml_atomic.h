#ifndef REML_ATOMIC_H
#define REML_ATOMIC_H

#include "reml_platform.h"
#include <stdint.h>
#include <stddef.h>

#if REML_COMPILER_MSVC
#include <windows.h>

typedef volatile LONG64 atomic_uint_fast64_t;
typedef volatile LONG64 atomic_size_t;

typedef enum {
    memory_order_relaxed = 0,
    memory_order_consume,
    memory_order_acquire,
    memory_order_release,
    memory_order_acq_rel,
    memory_order_seq_cst
} memory_order;

static inline uint64_t atomic_load_explicit(const atomic_uint_fast64_t* obj,
                                            memory_order order) {
    (void)order;
    return (uint64_t)InterlockedCompareExchange64((volatile LONG64*)obj, 0, 0);
}

static inline void atomic_store_explicit(atomic_uint_fast64_t* obj,
                                         uint64_t value,
                                         memory_order order) {
    (void)order;
    InterlockedExchange64((volatile LONG64*)obj, (LONG64)value);
}

static inline uint64_t atomic_fetch_add_explicit(atomic_uint_fast64_t* obj,
                                                 uint64_t value,
                                                 memory_order order) {
    (void)order;
    return (uint64_t)InterlockedExchangeAdd64((volatile LONG64*)obj,
                                              (LONG64)value);
}

static inline size_t atomic_load(const atomic_size_t* obj) {
    return (size_t)InterlockedCompareExchange64((volatile LONG64*)obj, 0, 0);
}

static inline size_t atomic_fetch_add(atomic_size_t* obj, size_t value) {
    return (size_t)InterlockedExchangeAdd64((volatile LONG64*)obj,
                                            (LONG64)value);
}

static inline void atomic_store(atomic_size_t* obj, size_t value) {
    InterlockedExchange64((volatile LONG64*)obj, (LONG64)value);
}

#else
#include <stdatomic.h>
#endif /* REML_COMPILER_MSVC */

#endif /* REML_ATOMIC_H */

