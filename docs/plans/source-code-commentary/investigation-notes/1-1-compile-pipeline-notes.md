# 第1章 調査メモ: Remlコンパイルパイプライン

## 参照した資料
- `compiler/README.md:5-11`（コンパイル関連コンポーネントの分類）
- `compiler/frontend/README.md:3-15`（フロントエンドの責務と CLI）
- `compiler/backend/README.md:3-6`（バックエンドが LLVM スケルトンである点）
- `compiler/runtime/README.md:3-8`（ランタイムの構成）
- `compiler/frontend/src/bin/reml_frontend.rs:545-687`（CLI 入口から解析・型検査までの流れ）
- `compiler/frontend/src/pipeline/mod.rs:16-199`（パイプライン識別子と監査イベント）
- `compiler/backend/llvm/src/runtime_link.rs:10-199`（ランタイムリンクの準備とエラー）

## 調査メモ

### コンパイルパイプラインの全体像（現時点のコードから確認できる範囲）
- リポジトリ上のコンパイル関連コンポーネントは `frontend` / `backend` / `runtime` / `adapter` / `ffi_bindgen` / `xtask` に分割されている。`backend` は LLVM スケルトンで、`runtime` は Rust + native/ffi 構成。 (`compiler/README.md:5-11`, `compiler/backend/README.md:3-6`, `compiler/runtime/README.md:3-8`)
- `frontend` の CLI として `reml_frontend` が定義され、入力ソースを解析し JSON を出力することが明記されている。 (`compiler/frontend/README.md:13-15`)

### フロントエンドの実行フロー（CLI 実装）
- `reml_frontend` の `main` は FFI 実行エンジンの初期化後、plugin/capability コマンドを判定し、通常処理に入る。パイプライン識別子 (`PipelineDescriptor`) と監査エミッタ (`AuditEmitter`) を準備し、開始イベントを出力してから `run_frontend` を呼ぶ。 (`compiler/frontend/src/bin/reml_frontend.rs:545-594`)
- 成功時は `PipelineOutcome` を作成し、監査イベントの完了処理を行ったあと `emit_cli_output` を呼んで出力する。失敗時は `PipelineFailure` を作成して失敗イベントを送る。 (`compiler/frontend/src/bin/reml_frontend.rs:595-623`)
- `run_frontend` では入力ファイルを読み込み、必要に応じてトークン列を出力し、パーサ実行（`ParserDriver` or `StreamingRunner`）後に型推論を行う。 (`compiler/frontend/src/bin/reml_frontend.rs:628-687`)

### パイプライン監査のデータ構造
- `PipelineDescriptor` は CLI 1 回分の識別情報を保持し、監査イベント共通のメタデータを生成する。 (`compiler/frontend/src/pipeline/mod.rs:16-94`)
- `PipelineOutcome` / `PipelineFailure` が成功・失敗時の付帯情報として利用される。 (`compiler/frontend/src/pipeline/mod.rs:109-150`)
- `AuditEmitter` は `pipeline_started`/`pipeline_completed` などのイベントを生成するユーティリティ。 (`compiler/frontend/src/pipeline/mod.rs:153-199`)

### バックエンドとランタイム連携の状況
- `backend` は LLVM バックエンドのスケルトンとして実装が進められている。 (`compiler/backend/README.md:3-6`)
- `backend/llvm/src/runtime_link.rs` にランタイムライブラリ探索 (`find_runtime_library`) とリンク失敗時のエラー型が定義されている。 (`compiler/backend/llvm/src/runtime_link.rs:72-170`)
- `reml_frontend` から `backend` に直接移譲する実行パスは、今回確認した範囲では見つからない（章内で「概念上のパイプライン」と「現実の接続状況」を区別する必要がある）。

## 未確認事項 / TODO
- `frontend` と `backend` を接続する実行パス（もし存在するなら）を後続章で再調査する。
- パイプライン全体に対応する `docs/spec` 節が存在するかの確認。
