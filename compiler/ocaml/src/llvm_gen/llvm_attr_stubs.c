#include <caml/mlvalues.h>
#include <caml/memory.h>

/* llvm-ocaml バインディングで公開されているヘルパ関数を利用する */
extern void *from_val(value v);
extern value to_val(void *ptr);

/* 必要な LLVM C API の型と関数を最小限だけ前方宣言する */
typedef struct LLVMOpaqueContext *LLVMContextRef;
typedef struct LLVMOpaqueType *LLVMTypeRef;
typedef struct LLVMOpaqueAttributeRef *LLVMAttributeRef;
LLVMAttributeRef LLVMCreateTypeAttribute(LLVMContextRef C, unsigned KindID,
                                         LLVMTypeRef type_ref);

#define Context_val(v) ((LLVMContextRef)from_val(v))
#define Type_val(v) ((LLVMTypeRef)from_val(v))

CAMLprim value reml_llvm_create_type_attr_by_kind(value ctx, value kind,
                                                  value ty) {
  CAMLparam3(ctx, kind, ty);
  LLVMAttributeRef attr =
      LLVMCreateTypeAttribute(Context_val(ctx), Int_val(kind), Type_val(ty));
  CAMLreturn(to_val(attr));
}
