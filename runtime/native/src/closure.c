/**
 * closure.c - クロージャ（関数値）サポート
 *
 * 最小 ABI として {env*, code_ptr} 形式のクロージャを提供する。
 * env は RC 管理対象のヒープオブジェクト（または NULL）を想定し、
 * 閉包生成時に inc_ref、破棄時に dec_ref される。
 */

#include "../include/reml_runtime.h"

void* reml_closure_new(void* env, void* code_ptr) {
    reml_closure_t* closure = (reml_closure_t*)mem_alloc(sizeof(reml_closure_t));
    reml_set_type_tag(closure, REML_TAG_CLOSURE);
    closure->env = env;
    closure->code_ptr = code_ptr;
    if (env != NULL) {
        inc_ref(env);
    }
    return closure;
}

void* reml_closure_env(void* closure_ptr) {
    if (closure_ptr == NULL) {
        panic("closure env target is null");
    }
    if (reml_get_type_tag(closure_ptr) != REML_TAG_CLOSURE) {
        panic("closure env type tag mismatch");
    }
    return ((reml_closure_t*)closure_ptr)->env;
}

void* reml_closure_code_ptr(void* closure_ptr) {
    if (closure_ptr == NULL) {
        panic("closure code_ptr target is null");
    }
    if (reml_get_type_tag(closure_ptr) != REML_TAG_CLOSURE) {
        panic("closure code_ptr type tag mismatch");
    }
    return ((reml_closure_t*)closure_ptr)->code_ptr;
}

