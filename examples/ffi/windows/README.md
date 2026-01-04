# Windows FFI サンプル

Phase 2-3 FFI契約拡張のWindows x64検証用サンプルコード

## 概要

このディレクトリには、Windows x64 (MSVC ABI) 向けのFFI検証サンプルが含まれています。
各サンプルは `docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md` および `docs/spec/3-9-core-async-ffi-unsafe.md` に基づいて作成されています。

## サンプル一覧

### 1. `messagebox.reml` - Win32 API 呼び出し

**目的**: Windows MessageBoxW API の呼び出し検証

**検証項目**:
- `calling_convention("win64")` の正しいlowering (CallConv = 79)
- 借用所有権 (`ownership("borrowed")`) のポインタ引数
- UTF-16文字列の受け渡し (`*const u16`)
- LLVM IR メタデータ `reml.bridge.platform = x86_64-pc-windows-msvc`

**依存**: `user32.dll` (Windows標準)

**コンパイル方法**:
```bash
cargo run --manifest-path compiler/frontend/Cargo.toml --bin remlc -- --emit-ir examples/ffi/windows/messagebox.reml
opt -verify messagebox.ll -S -o messagebox.opt.ll
llc messagebox.opt.ll -filetype=obj -o messagebox.obj
```

**実行** (要 Win32 リンク):
```bash
# MSVC linker で実行可能ファイル作成
link /SUBSYSTEM:WINDOWS messagebox.obj user32.lib
```

---

### 2. `struct_passing.reml` - 構造体受け渡し

**目的**: MSVC ABI の 8バイト閾値による `sret`/`byval` 属性検証

**検証項目**:
- 小構造体 (≤8 bytes) → レジスタ渡し (`rax`/`rdx`)
- 大構造体 (>8 bytes) → スタック渡し + `sret` 属性
- 構造体引数 (>8 bytes) → `byval` 属性
- `Rectangle` (16 bytes) の戻り値・引数での lowering

**LLVM IR 期待値**:
```llvm
; 小構造体戻り値 (レジスタ)
define win64cc { i32, i32 } @ffi_get_origin() {
  ; ...
}

; 大構造体戻り値 (sret)
define win64cc void @ffi_get_screen_rect(
  %Rectangle* sret(%Rectangle) align 4 %ret
) {
  ; ...
}

; 大構造体引数 (byval)
define win64cc i32 @ffi_calculate_area(
  %Rectangle* byval(%Rectangle) align 4 %rect
) {
  ; ...
}
```

**コンパイル方法**:
```bash
cargo run --manifest-path compiler/frontend/Cargo.toml --bin remlc -- --emit-ir examples/ffi/windows/struct_passing.reml
grep -E "(sret|byval)" struct_passing.ll  # 属性確認
```

---

### 3. `ownership_transfer.reml` - 所有権契約

**目的**: `borrowed` / `transferred` 所有権の動作検証

**検証項目**:
- `ownership("transferred")` - 所有権移転 (inc_ref/dec_ref)
- `ownership("borrowed")` - 借用 (inc_ref のみ)
- ランタイムヘルパ (`reml_ffi_acquire_borrowed` 等) の呼び出し
- `AuditEnvelope.metadata.bridge.ownership` の記録

**LLVM IR 期待値**:
```llvm
; transferred: inc_ref 不要
%handle = call win64cc i8* @ffi_allocate_resource(i64 1024)

; borrowed: inc_ref 自動挿入
call void @inc_ref(i8* %handle)
%value = call win64cc i32 @ffi_read_resource(i8* %handle)

; transferred (解放): dec_ref + C側 free
call void @dec_ref(i8* %handle)
call win64cc void @ffi_free_resource(i8* %handle)
```

**コンパイル方法**:
```bash
cargo run --manifest-path compiler/frontend/Cargo.toml --bin remlc -- --emit-ir examples/ffi/windows/ownership_transfer.reml
grep -E "(inc_ref|dec_ref)" ownership_transfer.ll  # 参照カウント確認
```

---

## 検証手順

### Phase 2-3 での使用

1. **コンパイル**:
   ```bash
   cargo build --manifest-path compiler/frontend/Cargo.toml
   ```

2. **IR生成**:
   ```bash
   cargo run --manifest-path compiler/frontend/Cargo.toml --bin remlc -- --emit-ir examples/ffi/windows/<sample>.reml
   ```

3. **メタデータ確認**:
   ```bash
   grep "reml.bridge" <sample>.ll
   # 期待:
   # !llvm.module.flags = !{!0}
   # !0 = !{i32 1, !"reml.bridge.version", i32 1}
   # !reml.bridge.stubs = !{!1}
   # !1 = !{!"bridge.platform=x86_64-pc-windows-msvc", ...}
   ```

4. **LLVM検証**:
   ```bash
   opt -verify <sample>.ll -S -o <sample>.opt.ll
   llc <sample>.opt.ll -filetype=obj -o <sample>.obj
   ```

5. **監査ログ**:
   ```bash
   cargo run --manifest-path compiler/frontend/Cargo.toml --bin remlc -- --emit-audit examples/ffi/windows/<sample>.reml
   # AuditEnvelope.metadata.bridge.* を確認
   ```

6. **結果記録**:
   ```bash
   # reports/ffi-windows-summary.md へ追記
   echo "## <sample>.reml" >> reports/ffi-windows-summary.md
   echo "- IR生成: ✅" >> reports/ffi-windows-summary.md
   echo "- CallConv確認: win64 (79)" >> reports/ffi-windows-summary.md
   # ...
   ```

---

## 技術的制約 (Phase 2-3)

### LLVM 16.0.4 使用

- **現状**: MSYS2 LLVM 16.0.4 (MinGW-w64)
- **Target**: `x86_64-w64-windows-gnu`
- **制限**: MSVC ABI完全対応は Phase 3 以降
- **対処**: IR検証はLLVM 16で実施、実行検証は任意

詳細: `docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md`

---

## 参照資料

- [2-3-ffi-contract-extension.md](../../../docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md) - Phase 2-3計画
- [3-9-core-async-ffi-unsafe.md](../../../docs/spec/3-9-core-async-ffi-unsafe.md) - FFI仕様
- [windows-llvm-build-investigation.md](../../../docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md) - LLVM調査
- [ffi-windows-summary.md](../../../reports/ffi-windows-summary.md) - Windows検証結果

---

**作成日**: 2025-10-19
**Phase**: 2-3 FFI契約拡張
**環境**: Windows 11 (MSYS2) + LLVM 16.0.4
