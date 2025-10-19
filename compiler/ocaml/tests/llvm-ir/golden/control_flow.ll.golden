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

declare void @reml_ffi_bridge_record_status(i32)

; Function Attrs: nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memcpy.p0.p0.i64(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i64, i1 immarg) #1

define i1 @__Eq_i64_eq(i64 %x, i64 %y) {
entry:
  %eq_result = icmp eq i64 %x, %y
  ret i1 %eq_result
}

define i1 @__Eq_String_eq(ptr %s1, ptr %s2) {
entry:
  %string_eq_result = call i32 @string_eq(ptr %s1, ptr %s2)
  %to_bool = icmp ne i32 %string_eq_result, 0
  ret i1 %to_bool
}

declare i32 @string_eq(ptr, ptr)

define i1 @__Eq_Bool_eq(i1 %b1, i1 %b2) {
entry:
  %eq_result = icmp eq i1 %b1, %b2
  ret i1 %eq_result
}

define i32 @__Ord_i64_compare(i64 %x, i64 %y) {
entry:
  %lt = icmp slt i64 %x, %y
  %gt = icmp sgt i64 %x, %y
  %sel1 = select i1 %lt, i32 -1, i32 0
  %sel2 = select i1 %gt, i32 1, i32 %sel1
  ret i32 %sel2
}

define i32 @__Ord_String_compare(ptr %s1, ptr %s2) {
entry:
  %string_compare_result = call i32 @string_compare(ptr %s1, ptr %s2)
  ret i32 %string_compare_result
}

declare i32 @string_compare(ptr, ptr)

define i64 @factorial(i64 %n) {
entry:
  %icmp_le = icmp sle i64 %n, 1
  br i1 %icmp_le, label %if_then, label %if_else

if_then:                                          ; preds = %entry
  br label %if_merge

if_else:                                          ; preds = %entry
  %sub_tmp = sub i64 %n, 1
  %call_tmp = call i64 @factorial(i64 %sub_tmp)
  %mul_tmp = mul i64 %n, %call_tmp
  br label %if_merge

if_merge:                                         ; preds = %if_else, %if_then
  %if_tmp = phi i64 [ 1, %if_then ], [ %mul_tmp, %if_else ]
  ret i64 %if_tmp
}

define i64 @fib(i64 %n) {
entry:
  %icmp_le = icmp sle i64 %n, 1
  br i1 %icmp_le, label %if_then, label %if_else

if_then:                                          ; preds = %entry
  br label %if_merge

if_else:                                          ; preds = %entry
  %sub_tmp = sub i64 %n, 1
  %call_tmp = call i64 @fib(i64 %sub_tmp)
  %sub_tmp1 = sub i64 %n, 2
  %call_tmp2 = call i64 @fib(i64 %sub_tmp1)
  %add_tmp = add i64 %call_tmp, %call_tmp2
  br label %if_merge

if_merge:                                         ; preds = %if_else, %if_then
  %if_tmp = phi i64 [ %n, %if_then ], [ %add_tmp, %if_else ]
  ret i64 %if_tmp
}

define i64 @max(i64 %a, i64 %b) {
entry:
  %icmp_gt = icmp sgt i64 %a, %b
  br i1 %icmp_gt, label %if_then, label %if_else

if_then:                                          ; preds = %entry
  br label %if_merge

if_else:                                          ; preds = %entry
  br label %if_merge

if_merge:                                         ; preds = %if_else, %if_then
  %if_tmp = phi i64 [ %a, %if_then ], [ %b, %if_else ]
  ret i64 %if_tmp
}

attributes #0 = { noreturn }
attributes #1 = { nocallback nofree nounwind willreturn memory(argmem: readwrite) }
