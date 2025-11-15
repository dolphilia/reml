# LLVM バックエンドインベントリ（W2: Rust ラッパ層）

## 目的
- `compiler/ocaml/src/llvm_gen/` が担ってきたミドルレイヤ（MIR→LLVM IR、`TargetMachine`/`DataLayout`、診断/verify）の責務を Rust ラッパ層として再構築し、`docs/plans/rust-migration/2-0-llvm-backend-plan.md` の W1 で整理した差分チェックリストを W2 で反映する。
- `docs/spec/0-1-project-purpose.md` と `docs/plans/rust-migration/unified-porting-principles.md` に基づき、「振る舞いの同一性」を起点とした `TargetMachine`/`DataLayout` 設定と呼び出し順を `compiler/rust/backend/llvm/` に設計する。
- `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` で定義された差分監査手順に従い OCaml 参照実装と Rust 実装の差異を `docs-migrations.log` に記録し、仕様やガイドとの整合性を残す。

## OCaml 実装のモジュールと責務（参照: `compiler/ocaml/src/llvm_gen/`）
| モジュール | 主な責務 | Rust 側の対応候補 | 参照ドキュメント |
| --- | --- | --- | --- |
| `codegen.ml` | MIR→LLVM IR 変換、関数/ブロック構成、builder 管理 | `compiler/rust/backend/llvm/src/codegen.rs`（予定）、`Builder`/`Module` ラッパ | `docs/guides/llvm-integration-notes.md` §5.0、`docs/plans/rust-migration/2-1-runtime-integration.md` |
| `type_mapping.ml` | Reml 型・ADT → LLVM 型と構造体アラインメント | `type_mapping.rs` + `data_layout.rs` | `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md`（チェックリスト） |
| `target_config.ml` | `TargetMachine`/`DataLayout`/Triple/CallingConvention/最適化レベル | `target_machine.rs` + `target_config.rs`（ビルダー） | `docs/guides/llvm-integration-notes.md` §5.0、`docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` |
| `llvm_attr.ml` | 関数属性と呼出規約（`noalias`/`nounwind` 等） | `llvm_attr.rs`（属性マップ）、`FunctionAttributeSet` | `docs/spec/3-6-core-diagnostics-audit.md` |
| `verify.ml` | `opt -verify`/`llc` 呼び出し、診断ログ・`audit`/`Diagnostic` 拡張 | `verify.rs` + CLI 側 `audit` フック | `reports/diagnostic-format-regression.md` |
| `ffi_value_lowering.ml` + `runtime_link.ml` | RC/panic/リンク処理、runtime 境界 | `ffi_lowering.rs` + `runtime_link.rs`（予定）、`docs/plans/rust-migration/2-1-runtime-integration.md` との照合 | `docs/guides/reml-ffi-handbook.md`, `docs/plans/rust-migration/2-2-adapter-layer-guidelines.md` |

> **補足**: `ffi_value_lowering` 以降は P2-1/2-2 の境界をまたぐため、Rust 側でも呼び出し順を表形式で追跡し、`docs-migrations.log` の W2 セクションで差分や未対応項目を記録する。

## Rust ラッパ層の設計方針（W2 での成果）

### 1. TargetMachine + DataLayout API
- `TargetMachineBuilder` 構造体を導入し、以下をチェーン可能な設定 API として提供する。
  - `.with_triple(triple: Triple)` : `x86_64-unknown-linux-gnu` / `x86_64-apple-darwin` / `x86_64-pc-windows-gnu` / `x86_64-pc-windows-msvc`（`docs/guides/llvm-integration-notes.md` §5.0 の表に整合）。
  - `.with_cpu(cpu: &str)` / `.with_features(features: &str)` : OCaml `target_config` の `cpu`/`features` を再現し、`TargetMachine::getTargetCPU` / `getTargetFeatureString` と一致させる。
  - `.with_relocation_model(model: RelocModel)` / `.with_code_model(model: CodeModel)` / `.with_optimization_level(level: OptimizationLevel)` : `target_config` と照合し、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` で監査する項目に含める。
  - `.with_data_layout(layout: DataLayoutSpec)` : `docs/guides/llvm-integration-notes.md` §5.0 に記録された文字列（例: `e-m:e-i64:64-f64:64:64-v128:128:128-a:0:64`) を生成し、モジュールに設定する。
- Windows では `TargetMachineBuilder` に `windows_toolchain` を注入し、`docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` が示す MSYS2 LLVM 16 と公式 LLVM 19.1.1 を切り替えるオプションを持ち、選択理由を `docs-migrations.log` に残して監査経路につなげる。

### 2. Type mapping と LLVM 属性
- `type_mapping.rs` では `RemlType` → LLVM 型のマップ、ADT の `{i32 tag, payload}` 表現、文字列/スライスの `{i8*, i64}` を関数として定義し、`DataLayoutSpec` との整合性を維持する。
- `llvm_attr.rs` では `AttributeKind` を `EnumMap` で定義し、`noalias`/`readonly`/`nounwind`/`uwtable` などの属性を `FunctionAttributeSet` に追加する API を提供する。`TargetMachineBuilder` から受け取った `calling_conv` を基に `cc ccc`/`win64` など呼出規約属性を設定し、`docs/spec/3-6-core-diagnostics-audit.md` に記録される監査メタデータと連携する。

### 3. Builder/Module レイヤとランタイム境界
- `codegen.rs` は LLVM の `IRBuilder`/`Module` API をラップし、MIR から `Function`/`Block` を構築しながら `type_mapping` の LLVM 型を利用する。OCaml の `codegen` の呼び出し順（`codegen`→`type_mapping`→`target_config`→`verify`）を Rust 側でも再現するよう、`Builder` の複数段階インタフェースを提供する。
- `ffi_lowering.rs` は RC 参照カウント操作、`panic`/`abort` 呼び出し、`runtime_link::link_module` を扱い、`docs/plans/rust-migration/2-1-runtime-integration.md` の `ffi_value_lowering` との対応表を補足資料として `docs/plans/rust-migration/appendix/llvm-backend-inventory.md` に記録し、差分があれば `docs-migrations.log` にエントリを作成する。
- `runtime_link.rs` は `llc`/`opt` 生成物とのリンク順序を `docs/plans/rust-migration/2-2-adapter-layer-guidelines.md` の指針に沿って整理し、FFI の `extern` シンボルを shim で束ねる設計を明文化する。

### 4. 診断/verify の統合
- CLI 側で `--emit-llvm --backend=rust` が選択されると `verify.rs` を起動し、`opt -verify` → `llc` → `llvm-dis` の順にプロセスを呼び出す。各コマンドの出力は `reports/diagnostic-format-regression.md` に記載された診断 ID（`target.config.*`, `effects.contract.stage_mismatch` など）と比較し、差分は `Diagnostic.extensions["backend"]` / `audit.log("llvm.verify", ...)` で JSON 化して保存する。
- 差分や検証結果は `docs/spec/3-6-core-diagnostics-audit.md` の監査メタデータ定義と合わせて `docs-migrations.log` の W2 章に追加し、P3 の CI/監査パイプラインに引き継ぐ。

## W3 差分スナップショット

- `compiler/rust/backend/llvm::integration::generate_w3_snapshot` を通じて、`CodegenContext` に `MirFunction`（`@k__main`）を流し込み、`TargetMachineBuilder` の `Triple::WindowsMSVC` / `DataLayout` / `OptimizationLevel` 設定と属性・FFI 呼び出しを含む `GeneratedFunction` を構築する。
- `Verifier` で `opt -verify`/`llc` 相当の検証をシミュレートし、`Diagnostic.extensions["backend"]="rust"` と `audit.log("llvm.verify", …)` 形式のエントリを署名付きで残すことで、`docs/spec/3-6-core-diagnostics-audit.md` と整合した差分監査のメタデータを得る。
- 設計検証の出力は `reports/backend-ir-diff/w3-demo-log.json` に JSON 形式で保存し、`DataLayout`/`calling_conv`/`ffi_calls` の観測値と `audit_entries` を `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の差分監査欄や `docs-migrations.log` の W2 章にリンクさせることで、W3 以降のハンドオーバー証跡とする。

## W2 チェックリスト（差分監査との接続）
1. `TargetMachineBuilder` の Triple/CPU/features/relocation/code_model/opt_level を `target_config.ml` の数値と照合し、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の差分欄に記帳。
2. `DataLayoutSpec` の文字列と `type_mapping` に記録したアラインメント（`i8:8` 等）を `docs/guides/llvm-integration-notes.md` §5.0 の表と突き合わせ、差異があれば `docs-migrations.log` に対応箇所を記録。
3. `verify.rs` で起動する `opt -verify`/`llc` の出力を `reports/diagnostic-format-regression.md` の診断一覧にマップし、`diagnostic.extensions["backend"]` に `backend=rust` を付けて `audit.log` に記録する。
4. Windows では `TargetMachineBuilder::windows_toolchain` が `docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` の MSYS2 LLVM 16 と公式 LLVM 19.1.1 を切り替える設定を持ち、`docs-migrations.log` に選択理由と `llc`/`opt` のパスを追記。
5. `ffi_lowering` → `runtime_link` の呼び出し順を `docs/plans/rust-migration/2-1-runtime-integration.md` の `ffi_value_lowering` フェーズと照合し、差分が生じた場合は `docs-migrations.log` の W2 セクションでコード行と理由を記録。
6. `docs/spec/3-6-core-diagnostics-audit.md` に定義された JSON フィールド（`Diagnostic.extensions`/`audit.log`）が Rust 実装でも同様の形式で出力されるようにガイドを追加し、文書化しておく。
7. 上記項目を `docs/plans/rust-migration/appendix/llvm-backend-inventory.md` に記録したトレース表と差分チェック操作で照合し、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` へ脚注を付ける。

## 次フェーズへの引継ぎ指針
- P2-1 (`docs/plans/rust-migration/2-1-runtime-integration.md`) では `ffi_lowering`/`runtime_link` と `TargetMachine` が生成する `Module` をランタイムに注入する手順を明記し、Rust 側ランタイムの `inc_ref`/`panic` へマッピングする。
- P2-2 (`docs/plans/rust-migration/2-2-adapter-layer-guidelines.md`) では `TargetMachine` を介した ABI 境界（`cc ccc`/`win64`）と `shim`/`adapter` の責務を整理し、Rust CLI の `--backend` フラグで `verify`/`llc` 呼び出しを共通化する。
- P3 (`docs/plans/bootstrap-roadmap/3-0-ci-and-dual-write-strategy.md` で管理) へのハンドオーバーでは、`reports/diagnostic-format-regression.md` と `docs-migrations.log` の W2 エントリを添付し、`TargetMachine` 設定・診断メタデータ・Windows toolchain の切り替え手順をまとめたサブセットを `docs/notes/` に転記する。
