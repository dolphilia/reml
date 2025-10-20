; Darwin arm64 struct return sample for verify_llvm_ir.sh
target triple = "arm64-apple-darwin"

%Pair = type { double, double }

declare void @sum_pair(%Pair* sret(%Pair) align 16, double, double)

define double @call_sum_pair(double %lhs, double %rhs) {
entry:
  %result = alloca %Pair, align 16
  call void @sum_pair(%Pair* sret(%Pair) align 16 %result, double %lhs, double %rhs)
  %first = getelementptr inbounds %Pair, %Pair* %result, i32 0, i32 0
  %value = load double, double* %first, align 8
  ret double %value
}
