; ModuleID = 'main'
source_filename = "main"
target datalayout = "e-m:e-p:64:64-f64:64:64-v128:128:128-a:0:64"
target triple = "x86_64-unknown-linux-gnu"

declare ptr @mem_alloc(i64)

declare void @inc_ref(ptr)

declare void @dec_ref(ptr)

; Function Attrs: noreturn
declare void @panic(ptr, i64) #0

declare void @print_i64(i64)

; Function Attrs: nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memcpy.p0.p0.i64(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i64, i1 immarg) #1

define i64 @add(i64 %a, i64 %b) {
entry:
  %add_tmp = add i64 %a, %b
  ret i64 %add_tmp
}

define i64 @main() {
entry:
  %call_tmp = call i64 @add(i64 21, i64 21)
  ret i64 %call_tmp
}

attributes #0 = { noreturn }
attributes #1 = { nocallback nofree nounwind willreturn memory(argmem: readwrite) }
