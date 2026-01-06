# 付録B: エンドツーエンド実行トレース 調査メモ

## 参照資料
- `compiler/frontend/src/bin/reml_frontend.rs`: CLI 入口から解析・型検査・出力までの実行フロー。
- `compiler/frontend/src/typeck/driver.rs`: 型検査から MIR 生成までの流れ。
- `compiler/frontend/src/semantics/mir.rs`: MIR のスキーマと構築ロジック。
- `compiler/backend/llvm/src/integration.rs`: MIR JSON をバックエンドへ渡す統合ポイント。
- `compiler/backend/llvm/src/runtime_link.rs`: LLVM IR とランタイムのリンク手順。

## E2E トレースの入口 (reml_frontend)
- `main` で FFI 初期化 → CLI 引数解析 → 監査イベント開始 → `run_frontend` 実行。
  - `compiler/frontend/src/bin/reml_frontend.rs:545-625`
- `run_frontend` が入力ファイル読み込み、パース、型検査、診断生成、成果物出力を直列に実行。
  - `compiler/frontend/src/bin/reml_frontend.rs:628-885`
- パースは `ParserDriver::parse_with_options_and_run_config` / `StreamingRunner` の分岐で実行される。
  - `compiler/frontend/src/bin/reml_frontend.rs:656-678`
- 型検査は `TypecheckDriver::infer_module` を起点にして `TypecheckReport` を得る。
  - `compiler/frontend/src/bin/reml_frontend.rs:680-687`
- `--emit-mir` 指定時に `artifacts.mir` を JSON 出力する。
  - `compiler/frontend/src/bin/reml_frontend.rs:820-825`

## MIR 生成の起点 (typecheck / semantics)
- `TypecheckDriver::infer_module` が AST 有無で分岐し、AST があれば `infer_module_from_ast` を実行。
  - `compiler/frontend/src/typeck/driver.rs:119-133`
- 型検査完了後、`typed::TypedModule` から `mir::MirModule` を生成し、impl 情報などを付与する。
  - `compiler/frontend/src/typeck/driver.rs:666-712`
- MIR のスキーマバージョンは `frontend-mir/0.2` として定義される。
  - `compiler/frontend/src/semantics/mir.rs:8`
- `MirModule::from_typed_module` が TypedModule を MIR に変換する入口。
  - `compiler/frontend/src/semantics/mir.rs:31-73`

## バックエンドへの接続 (MIR JSON → Backend)
- `generate_snapshot_from_mir_json` が MIR JSON を読み込み、`generate_snapshot` に渡して差分スナップショットを生成する。
  - `compiler/backend/llvm/src/integration.rs:1336-1364`
- `generate_snapshot_from_mir_json` は MIR 側の `metadata`/`runtime_symbols` を統合してからバックエンドを実行する。
  - `compiler/backend/llvm/src/integration.rs:1344-1355`

## ランタイムリンク (LLVM IR → 実行ファイル)
- ランタイムライブラリは `REML_RUNTIME_PATH` か既定候補 (`compiler/runtime/native/build/libreml_runtime.a`) から検索する。
  - `compiler/backend/llvm/src/runtime_link.rs:144-169`
- `compile_ir_with_llc` が LLVM IR からオブジェクトファイルを生成する。
  - `compiler/backend/llvm/src/runtime_link.rs:202-216`
- `generate_link_command` と `link_object_with_runtime` がオブジェクトとランタイムをリンクする。
  - `compiler/backend/llvm/src/runtime_link.rs:218-264`
- `link_with_runtime` が IR → obj → 実行ファイルまでをまとめて実行し、オブジェクトを掃除する。
  - `compiler/backend/llvm/src/runtime_link.rs:266-275`

## 実行トレースで明示すべき成果物
- フロントエンドの JSON 成果物: AST / typed AST / MIR / constraints / parse debug。
  - `compiler/frontend/src/bin/reml_frontend.rs:814-839`
- バックエンドの差分スナップショット: `BackendDiffSnapshot` (MIR 関数ごとの LLVM IR と監査ログ)。
  - `compiler/backend/llvm/src/integration.rs:1300-1333`

