#ifndef REML_CODEGEN_CODEGEN_H
#define REML_CODEGEN_CODEGEN_H

#include <stdbool.h>

#include <llvm-c/Analysis.h>
#include <llvm-c/Core.h>
#include <llvm-c/Target.h>
#include <llvm-c/TargetMachine.h>

#include "reml/ast/ast.h"
#include "reml/sema/diagnostic.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
  LLVMContextRef context;
  LLVMModuleRef module;
  LLVMBuilderRef builder;
  LLVMBuilderRef alloca_builder;
  LLVMTargetMachineRef target_machine;
  LLVMTargetDataRef target_data;
  LLVMValueRef current_function;
  char *target_triple;
  LLVMTypeRef enum_repr_type;
  reml_diagnostic_list diagnostics;
} reml_codegen;

bool reml_codegen_init(reml_codegen *codegen, const char *module_name);
void reml_codegen_deinit(reml_codegen *codegen);

bool reml_codegen_generate(reml_codegen *codegen, reml_compilation_unit *unit);
bool reml_codegen_emit_ir(reml_codegen *codegen, const char *path);
bool reml_codegen_emit_object(reml_codegen *codegen, const char *path);

const reml_diagnostic_list *reml_codegen_diagnostics(const reml_codegen *codegen);

#ifdef __cplusplus
}
#endif

#endif
