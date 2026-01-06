# 調査メモ: 第12章 ランタイム連携と検証

## 対象モジュール

- `compiler/backend/llvm/src/runtime_link.rs`
- `compiler/backend/llvm/src/verify.rs`
- `compiler/backend/llvm/src/target_machine.rs`
- `compiler/backend/llvm/src/target_diagnostics.rs`
- `compiler/backend/llvm/src/codegen.rs`
- `compiler/backend/llvm/src/integration.rs`

## 入口と全体像

- バックエンドの検証は `Verifier::verify_module` が入口。`ModuleIr` が持つ `target_context` からターゲット診断を作成し、監査ログと診断を統合する。
  - `compiler/backend/llvm/src/verify.rs:94-268`
  - `compiler/backend/llvm/src/codegen.rs:1081-1096`
- ランタイム連携は `runtime_link` が担当。`link_with_runtime` が LLVM IR を `llc` でオブジェクト化し、`clang` でランタイム静的ライブラリとリンクする。
  - `compiler/backend/llvm/src/runtime_link.rs:202-275`
- `generate_snapshot` と `generate_w3_snapshot` が検証結果を `BackendDiffSnapshot` に詰め、診断・監査情報をログ化する。
  - `compiler/backend/llvm/src/integration.rs:1288-1459`

## データ構造

- **LinkCommand / RuntimeLinkError**: リンカー呼び出しのコマンド表現と、I/O・コマンド失敗・ランタイム不在を表すエラー列挙。
  - `compiler/backend/llvm/src/runtime_link.rs:46-141`
- **TargetMachine / TargetMachineBuilder**: 目的の Triple/ABI/DataLayout を持ち、`from_run_config` で `RunConfigTarget` から設定を組み立てる。
  - `compiler/backend/llvm/src/target_machine.rs:5-330`
- **TargetDiagnosticContext / RunConfigTarget / PlatformInfo**: 実行環境と要求ターゲットの差分を保持し、診断や監査用の JSON を生成。
  - `compiler/backend/llvm/src/target_diagnostics.rs:6-260`
- **VerificationResult / AuditLog**: 検証結果と監査エントリを保持するコンテナ。
  - `compiler/backend/llvm/src/verify.rs:9-83`

## コアロジック

- **ランタイムライブラリ探索**: `REML_RUNTIME_PATH` を優先し、見つからない場合は固定の候補を探索する。
  - `compiler/backend/llvm/src/runtime_link.rs:144-169`
- **リンクフロー**: `llc` による IR→obj と、OS ごとのリンク引数生成 (`-lSystem` / `-lc -lm`) を組み合わせる。
  - `compiler/backend/llvm/src/runtime_link.rs:202-275`
- **ターゲット診断**: `TargetDiagnosticEmitter::emit` が要求値と実環境を比較し、`profile_id` 未設定や `os/arch/family` の不一致を診断化する。
  - `compiler/backend/llvm/src/target_diagnostics.rs:229-260`
- **検証ログ**: `Verifier` が `target_report`、Bridge メタデータ、`native.*` 系メタデータを監査ログへ集約し、`audit.verdict` を確定させる。
  - `compiler/backend/llvm/src/verify.rs:94-268`

## 仕様との対応メモ

- 監査ログの形式は `docs/spec/3-6-core-diagnostics-audit.md` の `AuditEnvelope` と整合を取る必要がある（監査キーの構造化方針）。
- FFI/unsafe 由来の監査キー (`native.*`) は `docs/spec/3-9-core-async-ffi-unsafe.md` の監査要件と合わせて参照する。

## TODO / 不明点

- `TargetDiagnosticEmitter` が `profile_id` 未指定をエラー扱いにする意図（CLI のどの設定で補完されるか）を確認したい。
- `TargetSpec` の `Triple::AppleDarwin` が `x86-64` CPU を指す点と `platform_label` の `macos-arm64` の整合性を再確認したい。
