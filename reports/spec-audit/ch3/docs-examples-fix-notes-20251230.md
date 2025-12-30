# docs-examples 修正メモ（ch3 / 2025-12-30）

## 対象
- `docs/spec/3-12-core-cli.md`
- `examples/docs-examples/spec/3-12-core-cli/sec_5.reml`

## 修正内容
- `match` を `match ... with` 形式へ修正し、アームで複数式を扱うためにブロック `{ ... }` を導入。
- `if` を `if ... then ... else ...` 形式へ統一。

## 検証
- `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- --output json examples/docs-examples/spec/3-12-core-cli/sec_2.reml`
- `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- --output json examples/docs-examples/spec/3-12-core-cli/sec_5.reml`

## 関連リンク
- `docs/plans/docs-examples-audit/1-2-spec-sample-fix-targets.md`

## 追記: 3-9 Core.Async/FFI/Unsafe

### 対象
- `docs/spec/3-9-core-async-ffi-unsafe.md`
- `examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_1_2.reml`
- `examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_1_4_5.reml`
- `examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_1_6.reml`
- `examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_1_9_3.reml`
- `examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_2_3.reml`
- `examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_2_4.reml`
- `examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_2_4_1.reml`
- `examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_2_4_1_1.reml`
- `examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_3_1.reml`
- `examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_3_6_1.reml`
- `examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_3_6_2.reml`
- `examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_3_6_3.reml`

### 修正内容
- フェーズ 3 の再検証対象サンプルは現行の正準例に一致していたため、内容の差分は無し。
- Rust Frontend 拡張後の再検証として diagnostics を再取得。

### 検証
- `compiler/rust/frontend/target/debug/reml_frontend --emit-diagnostics examples/docs-examples/spec/3-9-core-async-ffi-unsafe/<sec>.reml`
- `reports/spec-audit/ch3/3-9-core-async-ffi-unsafe__sec_1_2-20251230-diagnostics.json`
- `reports/spec-audit/ch3/3-9-core-async-ffi-unsafe__sec_1_4_5-20251230-diagnostics.json`
- `reports/spec-audit/ch3/3-9-core-async-ffi-unsafe__sec_1_6-20251230-diagnostics.json`
- `reports/spec-audit/ch3/3-9-core-async-ffi-unsafe__sec_1_9_3-20251230-diagnostics.json`
- `reports/spec-audit/ch3/3-9-core-async-ffi-unsafe__sec_2_3-20251230-diagnostics.json`
- `reports/spec-audit/ch3/3-9-core-async-ffi-unsafe__sec_2_4-20251230-diagnostics.json`
- `reports/spec-audit/ch3/3-9-core-async-ffi-unsafe__sec_2_4_1-20251230-diagnostics.json`
- `reports/spec-audit/ch3/3-9-core-async-ffi-unsafe__sec_2_4_1_1-20251230-diagnostics.json`
- `reports/spec-audit/ch3/3-9-core-async-ffi-unsafe__sec_3_1-20251230-diagnostics.json`
- `reports/spec-audit/ch3/3-9-core-async-ffi-unsafe__sec_3_6_1-20251230-diagnostics.json`
- `reports/spec-audit/ch3/3-9-core-async-ffi-unsafe__sec_3_6_2-20251230-diagnostics.json`
- `reports/spec-audit/ch3/3-9-core-async-ffi-unsafe__sec_3_6_3-20251230-diagnostics.json`

### 関連リンク
- `docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251230-1.md`
