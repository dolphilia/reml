; ModuleID = 'main'
source_filename = "main"
target datalayout = "e-m:e-p:64:64-f64:64:64-v128:128:128-a:0:64"
target triple = "x86_64-unknown-linux-gnu"

declare ptr @mem_alloc(i64)

declare void @inc_ref(ptr)

declare void @dec_ref(ptr)

; Function Attrs: noreturn
declare void @panic(ptr, i64) #0

attributes #0 = { noreturn }
