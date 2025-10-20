; Darwin arm64 varargs sample for verify_llvm_ir.sh
target triple = "arm64-apple-darwin"

@.str = private unnamed_addr constant [7 x i8] c"%d %d\0A\00"

declare i32 @printf(i8*, ...)

define i32 @reml_vararg_demo(i32 %value) {
entry:
  %fmt = getelementptr inbounds [7 x i8], [7 x i8]* @.str, i64 0, i64 0
  %call = call i32 (i8*, ...) @printf(i8* %fmt, i32 %value, i32 99)
  ret i32 %call
}
